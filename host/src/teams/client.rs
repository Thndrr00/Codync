//! The Microsoft Bot Framework connector: OAuth tokens for outbound replies, the service URL and
//! conversation it names, activity posts, and the JWT check every incoming activity passes.
//!
//! The host registers one Azure Bot resource by hand; this module never creates it. App id and
//! password (or a managed identity) are the Azure registration's, and the messaging endpoint points
//! at `/hooks/teams/{id}` on this host (through an HTTPS reverse proxy or tunnel).

use anyhow::{Context, Result, bail};
use aws_lc_rs::signature::{RSA_PKCS1_2048_8192_SHA256, RsaPublicKeyComponents};
use base64::Engine as _;
use serde_json::{Value, json};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::egress;

/// Bot Framework's token audience and issuer.
const AUDIENCE: &str = "https://api.botframework.com";
const ISSUER: &str = "https://api.botframework.com";
const OPENID: &str = "https://login.botframework.com/v1/.well-known/openidconfiguration";
const IMDS: &str = "http://169.254.169.254/metadata/identity/oauth2/token";

/// Tokens live for about an hour; ask again after 50 minutes.
const TOKEN_FOR: Duration = Duration::from_mins(50);
/// Microsoft's signing keys rotate; an hour is the documented courtesy.
const KEYS_FOR: Duration = Duration::from_hours(1);

static TOKENS: Mutex<Option<(Instant, String, String)>> = Mutex::new(None);
static KEYS: Mutex<Option<(Instant, Value)>> = Mutex::new(None);

fn b64(value: &str) -> Result<Vec<u8>> {
    let padded = match value.len() % 4 {
        2 => format!("{value}=="),
        3 => format!("{value}="),
        _ => value.to_owned(),
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(padded).with_context(|| "a token part is not base64url")
}

fn token_endpoint(tenant: &str) -> String {
    let tenant = if tenant.trim().is_empty() { "botframework" } else { tenant.trim() };
    format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token")
}

/// The token outbound calls carry. Managed identity asks the Azure instance metadata service with
/// the bot's client id; otherwise the app's client credentials are exchanged for it.
pub(super) async fn token(app_id: &str, tenant: &str, password: &str, managed: bool) -> Result<String> {
    if let Some((at, key, token)) = TOKENS.lock().ok().and_then(|cache| cache.as_ref().cloned())
        && key == app_id
        && at.elapsed() < TOKEN_FOR
    {
        return Ok(token);
    }
    let token =
        if managed { managed_token(app_id).await? } else { client_credentials(app_id, tenant, password).await? };
    if let Ok(mut cache) = TOKENS.lock() {
        *cache = Some((Instant::now(), app_id.to_owned(), token.clone()));
    }
    Ok(token)
}

async fn client_credentials(app_id: &str, tenant: &str, password: &str) -> Result<String> {
    anyhow::ensure!(!app_id.trim().is_empty() && !password.is_empty(), "The Teams app id and password are required.");
    let url = token_endpoint(tenant);
    let client = egress::client(egress::Policy::Only(&["login.microsoftonline.com"]), &url).await?;
    let response = client
        .post(&url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", app_id),
            ("client_secret", password),
            ("scope", "https://api.botframework.com/.default"),
        ])
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("Microsoft sign-in failed: {}", error.without_url()))?;
    let status = response.status();
    let body = egress::json(response, 64 * 1024).await.unwrap_or_default();
    if !status.is_success() {
        let why = body["error_description"].as_str().or_else(|| body["error"].as_str()).unwrap_or("refused");
        bail!("Microsoft refused the Teams app credentials: {why}");
    }
    body["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .context("Microsoft signed in without an access token")
}

/// Managed identity uses the instance metadata endpoint, which is link-local and therefore outside
/// the normal egress policy. Only this fixed URL, with no redirects, is ever called.
async fn managed_token(app_id: &str) -> Result<String> {
    let url = format!("{IMDS}?api-version=2019-08-01&resource={AUDIENCE}&client_id={app_id}");
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()?;
    let response = client
        .get(&url)
        .header("Metadata", "true")
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("The managed identity is not available: {}", error.without_url()))?;
    let status = response.status();
    let body = egress::json(response, 64 * 1024).await.unwrap_or_default();
    if !status.is_success() {
        bail!("The managed identity could not get a Teams token ({}).", status);
    }
    body["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .context("The managed identity returned no access token")
}

async fn keys() -> Result<Value> {
    if let Some((at, keys)) = KEYS.lock().ok().and_then(|keys| keys.as_ref().cloned())
        && at.elapsed() < KEYS_FOR
    {
        return Ok(keys);
    }
    let client = egress::client(egress::Policy::Public, OPENID).await?;
    let metadata = egress::json(client.get(OPENID).timeout(Duration::from_secs(20)).send().await?, 256 * 1024).await?;
    let uri = metadata["jwks_uri"].as_str().context("Bot Framework has no key set")?;
    let client = egress::client(egress::Policy::Public, uri).await?;
    let keys = egress::json(client.get(uri).timeout(Duration::from_secs(20)).send().await?, 1024 * 1024).await?;
    if let Ok(mut cache) = KEYS.lock() {
        *cache = Some((Instant::now(), keys.clone()));
    }
    Ok(keys)
}

/// The claims a verified Bot Framework token must carry.
fn claims_ok(claims: &Value, app_id: &str, service_url: &str, now: i64) -> bool {
    let audience = claims["aud"].as_str() == Some(app_id)
        || claims["aud"].as_array().is_some_and(|list| list.iter().any(|a| a.as_str() == Some(app_id)));
    let issuer = claims["iss"].as_str() == Some(ISSUER);
    let service =
        claims["serviceurl"].as_str().is_some_and(|url| url.trim_end_matches('/') == service_url.trim_end_matches('/'));
    let exp = claims["exp"].as_i64().unwrap_or_default();
    let nbf = claims["nbf"].as_i64().unwrap_or(now);
    audience && issuer && service && exp > now - 300 && nbf < now + 300
}

/// Verifies the bearer token a Bot Framework activity carries: RS256 against Microsoft's published
/// keys, then the audience, issuer, service URL and lifetime.
pub(super) async fn verify(token: &str, app_id: &str, service_url: &str) -> Result<Value> {
    let mut parts = token.split('.');
    let (Some(header), Some(payload), Some(signature), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        bail!("The Teams token is not a JWT.");
    };
    let head: Value = serde_json::from_slice(&b64(header)?)?;
    anyhow::ensure!(head["alg"] == "RS256", "The Teams token is not signed with RS256.");
    let kid = head["kid"].as_str().context("The Teams token names no key")?;
    let claims: Value = serde_json::from_slice(&b64(payload)?)?;
    let now = chrono::Utc::now().timestamp();
    anyhow::ensure!(claims_ok(&claims, app_id, service_url, now), "The Teams token is not for this bot and service.");
    let keys = keys().await?;
    let key = keys["keys"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|key| key["kid"].as_str() == Some(kid))
        .context("Microsoft does not publish that signing key")?;
    let n = b64(key["n"].as_str().context("The signing key has no modulus")?)?;
    let e = b64(key["e"].as_str().context("The signing key has no exponent")?)?;
    let components = RsaPublicKeyComponents { n: n.as_slice(), e: e.as_slice() };
    let signed = format!("{header}.{payload}");
    components
        .verify(&RSA_PKCS1_2048_8192_SHA256, signed.as_bytes(), &b64(signature)?)
        .map_err(|_| anyhow::anyhow!("The Teams token signature does not check out."))?;
    Ok(claims)
}

fn activity_body(app_id: &str, conversation: &str, reply_to: &str, body: Value) -> Value {
    let mut activity = json!({
        "type": "message",
        "from": {"id": app_id},
        "conversation": {"id": conversation},
    });
    if !reply_to.is_empty() {
        activity["replyToId"] = reply_to.into();
    }
    if let Some(text) = body["text"].as_str() {
        activity["text"] = text.into();
    }
    if let Some(attachments) = body.get("attachments") {
        activity["attachments"] = attachments.clone();
    }
    activity
}

async fn post(cfg: &super::Config, service_url: &str, path: &str, activity: &Value) -> Result<Value> {
    let base = service_url.trim_end_matches('/');
    anyhow::ensure!(
        base.starts_with("https://") && !base.contains('@') && !base.contains(' '),
        "The Teams service URL must be HTTPS."
    );
    let url = format!("{base}{path}");
    let token = token(&cfg.app_id, &cfg.tenant_id, &cfg.app_password, cfg.managed_identity).await?;
    let client = egress::client(egress::Policy::Public, &url).await?;
    let response = client
        .post(&url)
        .bearer_auth(token)
        .json(activity)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("Teams did not take the reply: {}", error.without_url()))?;
    let status = response.status();
    let body = egress::json(response, 256 * 1024).await.unwrap_or_default();
    if !status.is_success() {
        let why = body["error"]["message"].as_str().or_else(|| body["message"].as_str()).unwrap_or("refused");
        bail!("Teams refused the reply ({status}): {why}");
    }
    Ok(body)
}

/// Posts a text activity and returns its id.
pub(super) async fn send_text(
    cfg: &super::Config,
    service_url: &str,
    conversation: &str,
    reply_to: &str,
    text: &str,
) -> Result<String> {
    let activity = activity_body(&cfg.app_id, conversation, reply_to, json!({"text": text}));
    let answer = post(cfg, service_url, &format!("/v3/conversations/{conversation}/activities"), &activity).await?;
    Ok(answer["id"].as_str().map(str::to_owned).unwrap_or_default())
}

/// Posts an Adaptive Card and returns the activity that shows it.
pub(super) async fn send_card(
    cfg: &super::Config,
    service_url: &str,
    conversation: &str,
    card: &Value,
) -> Result<String> {
    let activity = activity_body(&cfg.app_id, conversation, "", card.clone());
    let answer = post(cfg, service_url, &format!("/v3/conversations/{conversation}/activities"), &activity).await?;
    Ok(answer["id"].as_str().map(str::to_owned).unwrap_or_default())
}

/// Replaces a card with its settled form.
pub(super) async fn update_card(
    cfg: &super::Config,
    service_url: &str,
    conversation: &str,
    activity: &str,
    card: &Value,
) -> Result<()> {
    let body = activity_body(&cfg.app_id, conversation, "", card.clone());
    let path = format!("/v3/conversations/{conversation}/activities/{activity}");
    let base = service_url.trim_end_matches('/');
    let url = format!("{base}{path}");
    let token = token(&cfg.app_id, &cfg.tenant_id, &cfg.app_password, cfg.managed_identity).await?;
    let client = egress::client(egress::Policy::Public, &url).await?;
    let response = client
        .put(&url)
        .bearer_auth(token)
        .json(&body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("Teams did not update the card: {}", error.without_url()))?;
    if !response.status().is_success() {
        bail!("Teams refused the card update ({})", response.status());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_must_be_for_this_bot_and_service() {
        let now = 1_700_000_000;
        let claims = json!({"aud": "app-1", "iss": ISSUER, "serviceurl": "https://smba.example/",
            "exp": now + 3600, "nbf": now - 10});
        assert!(claims_ok(&claims, "app-1", "https://smba.example", now));
        assert!(!claims_ok(&claims, "app-2", "https://smba.example", now));
        assert!(!claims_ok(&claims, "app-1", "https://other.example", now));
        assert!(!claims_ok(
            &json!({"aud":"app-1","iss":"https://evil.example","serviceurl":"https://smba.example",
            "exp": now + 3600}),
            "app-1",
            "https://smba.example",
            now
        ));
        assert!(!claims_ok(
            &json!({"aud":"app-1","iss":ISSUER,"serviceurl":"https://smba.example",
            "exp": now - 400}),
            "app-1",
            "https://smba.example",
            now
        ));
        let list = json!({"aud": ["other", "app-1"], "iss": ISSUER, "serviceurl": "https://smba.example",
            "exp": now + 3600});
        assert!(claims_ok(&list, "app-1", "https://smba.example", now));
    }

    #[test]
    fn an_activity_carries_text_or_an_adaptive_card() {
        let text = activity_body("app", "conv", "parent", json!({"text": "hola"}));
        assert_eq!(text["type"], "message");
        assert_eq!(text["conversation"]["id"], "conv");
        assert_eq!(text["replyToId"], "parent");
        assert_eq!(text["text"], "hola");
        let card = activity_body("app", "conv", "", json!({"attachments": [{"contentType": "adaptive"}]}));
        assert_eq!(card["attachments"][0]["contentType"], "adaptive");
        assert!(card.get("replyToId").is_none());
    }
}
