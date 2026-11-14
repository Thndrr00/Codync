//! OpenAI-compatible speech endpoint: where `OpenAI` speech requests go and what they may carry.
//! A local voice server saved in Voice chat settings wins; `CODYNC_OPENAI_BASE_URL` is its
//! fallback, so a developer's environment still applies on a computer with no setting.

use crate::store::Store;
use anyhow::{Result, anyhow, bail};

/// What a base URL is called in errors: the settings field, or the variable it came from.
pub(super) const SETTING: &str = "The local voice server URL";
const ENV: &str = "CODYNC_OPENAI_BASE_URL";

/// `voice.baseUrl` (a compatible server on this computer) when set and valid, else
/// `CODYNC_OPENAI_BASE_URL`, else `None` for `api.openai.com`. An invalid saved URL is
/// reported once and skipped; nothing ever unwinds here.
pub(super) fn openai_base(store: &Store) -> Result<Option<reqwest::Url>> {
    let saved = super::server::base_url(store);
    resolve(saved.as_deref(), std::env::var(ENV).ok().as_deref())
}

/// The saved setting over the environment variable, validated the same way. A saved URL
/// that doesn't validate (edited by hand, or by an older build) falls back with a log line.
fn resolve(saved: Option<&str>, env: Option<&str>) -> Result<Option<reqwest::Url>> {
    if let Some(value) = saved.map(str::trim).filter(|s| !s.is_empty()) {
        match parse_base(value, SETTING) {
            Ok(url) => return Ok(Some(url)),
            Err(error) => tracing::warn!(
                error = format!("{error:#}"),
                "ignoring the saved local voice server; OpenAI speech uses the fallback"
            ),
        }
    }
    parse_openai_base(env)
}

/// Whether speech requests leave for a server other than `api.openai.com`.
pub(super) fn is_custom(store: &Store) -> Result<bool> {
    Ok(openai_base(store)?.is_some())
}

/// A base URL as saved (`http://127.0.0.1:8890`): HTTPS, or HTTP only on this computer, and
/// no credentials, query or fragment. A trailing `/v1` is dropped; a path prefix stays.
pub(super) fn parse_base(value: &str, label: &str) -> Result<reqwest::Url> {
    let value = value.trim();
    let mut url = reqwest::Url::parse(value)
        .map_err(|_| anyhow!("{label} must be an absolute HTTP(S) URL, such as http://127.0.0.1:8890"))?;
    let userinfo = value
        .split_once("://")
        .is_some_and(|(_, rest)| rest.split(['/', '?', '#']).next().is_some_and(|a| a.contains('@')));
    if userinfo
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("{label} must not contain credentials, a query or a fragment");
    }
    let loopback = url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host.trim_matches(['[', ']']).parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
    });
    if url.host_str().is_none() || !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        bail!("{label} requires HTTPS, except HTTP on localhost or a loopback IP");
    }
    let path = url.path().trim_end_matches('/');
    let path = path.strip_suffix("/v1").unwrap_or(path).to_owned();
    url.set_path(&path);
    Ok(url)
}

fn parse_openai_base(value: Option<&str>) -> Result<Option<reqwest::Url>> {
    let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) else { return Ok(None) };
    parse_base(value, ENV).map(Some)
}

pub(super) fn openai_url(base: Option<&reqwest::Url>, path: &str) -> String {
    format!("{}{path}", base.map_or("https://api.openai.com", |url| url.as_str().trim_end_matches('/')))
}

pub(super) fn realtime_url(local: bool) -> Result<String> {
    if local {
        bail!(
            "OpenAI Realtime isn't available with a local voice server. Use Speech mode, or remove the server in Voice chat settings."
        );
    }
    Ok(openai_url(None, "/v1/realtime/client_secrets"))
}

// Never forward a saved bearer key through a custom server's redirect (even on the same host).
pub(super) fn openai_http() -> Result<&'static reqwest::Client> {
    static CLIENT: std::sync::LazyLock<std::result::Result<reqwest::Client, reqwest::Error>> =
        std::sync::LazyLock::new(|| reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build());
    CLIENT.as_ref().map_err(|_| anyhow!("Could not initialize the OpenAI speech client"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn compatible_base_normalizes_version_and_preserves_proxy_prefix() {
        for base in
            ["https://speech.example/proxy", "https://speech.example/proxy/", "https://speech.example/proxy/v1/"]
        {
            let base = parse_base(base, ENV).unwrap();
            assert_eq!(openai_url(Some(&base), "/v1/audio/speech"), "https://speech.example/proxy/v1/audio/speech");
        }
        for base in ["http://localhost:8890/v1", "http://127.0.0.2:8890", "http://[::1]:8890/"] {
            assert!(parse_base(base, ENV).is_ok());
        }
        assert!(parse_openai_base(Some("  ")).unwrap().is_none());
        assert_eq!(openai_url(None, "/v1/models"), "https://api.openai.com/v1/models");
    }

    #[test]
    fn compatible_base_rejects_insecure_or_ambiguous_destinations() {
        for base in [
            "http://speech.example",
            "http://192.168.1.10:8890",
            "http://localhost.example",
            "ftp://localhost",
            "/relative",
            "https://user:secret@speech.example",
            "https://@speech.example",
            "https://speech.example?key=secret",
            "https://speech.example/#fragment",
        ] {
            assert!(parse_base(base, ENV).is_err(), "accepted {base}");
        }
    }

    #[test]
    fn the_saved_server_wins_and_the_environment_is_its_fallback() {
        let saved = resolve(Some("http://127.0.0.1:8890"), Some("https://speech.example/v1")).unwrap();
        assert_eq!(openai_url(saved.as_ref(), "/v1/models"), "http://127.0.0.1:8890/v1/models");
        let env = resolve(Some("  "), Some("https://speech.example/v1")).unwrap();
        assert_eq!(openai_url(env.as_ref(), "/v1/models"), "https://speech.example/v1/models");
        // An invalid saved URL never stops speech: the environment, then OpenAI, still work.
        let fallback = resolve(Some("http://speech.example"), Some("http://127.0.0.1:8890")).unwrap();
        assert_eq!(openai_url(fallback.as_ref(), "/v1/models"), "http://127.0.0.1:8890/v1/models");
        let official = resolve(Some("not a url"), None).unwrap();
        assert!(official.is_none());
        assert_eq!(openai_url(official.as_ref(), "/v1/models"), "https://api.openai.com/v1/models");
    }

    #[test]
    fn compatible_speech_endpoint_cannot_mint_realtime_credentials() {
        assert!(parse_base("http://127.0.0.1:8890/v1", ENV).is_ok());
        assert!(realtime_url(true).is_err());
        assert_eq!(realtime_url(false).unwrap(), "https://api.openai.com/v1/realtime/client_secrets");
    }

    #[tokio::test]
    async fn speech_client_does_not_follow_credential_redirects() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 2048];
            let _ = stream.read(&mut buffer).await.unwrap();
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/collect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        });
        let response = openai_http()
            .unwrap()
            .get(format!("http://{address}/v1/models"))
            .bearer_auth("test-only-key")
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        server.await.unwrap();
    }
}
