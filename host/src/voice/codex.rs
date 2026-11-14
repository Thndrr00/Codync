//! Live voice with the computer's own Codex / ChatGPT sign-in, as OpenBot 0.65 offers it.
//!
//! The sign-in lives in Codex's `auth.json` (`CODEX_HOME`, else `~/.codex/auth.json`). A ChatGPT sign-in can
//! carry a realtime conversation; an API key sign-in already reaches OpenAI through Codync's own OpenAI engine.
//! Codync decides here whether the sign-in can carry a call, with the reason when it can't, and Voice settings
//! shows that answer. Codync never copies the token anywhere: the file is read only to decide, and the token
//! itself is never logged or sent.
//!
//! A ChatGPT sign-in does not hand out a client secret the apps could dial with. Codex itself brokers the
//! call: it takes the app's WebRTC offer and answers with the SDP (`codex app-server`, `thread/realtime/start`
//! with `transport: {type: "webrtc", sdp}`). That handoff is a host API of its own; until a call engine uses it,
//! the option says so instead of failing a call. See `docs/features/voice-call.md`.

use serde_json::Value;
use std::path::PathBuf;

/// How Codex is signed in on this computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A ChatGPT account: the plan can carry realtime voice.
    Chatgpt,
    /// An API key: OpenAI is reached with it, not with the plan's voice.
    ApiKey,
    /// A sign-in file Codync can't place.
    Unknown,
}

/// What the sign-in offers live voice, as `voiceStatus.codex` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    pub available: bool,
    pub reason: Option<String>,
    /// The account the file names, when it names one (never a token).
    pub account: Option<String>,
}

impl Access {
    fn ready(account: Option<String>) -> Self {
        Self { available: true, reason: None, account }
    }

    fn no(reason: impl Into<String>) -> Self {
        Self { available: false, reason: Some(reason.into()), account: None }
    }

    pub fn json(&self) -> Value {
        serde_json::json!({
            "available": self.available,
            "reason": self.reason,
            "account": self.account,
        })
    }
}

/// The sign-in file: `CODEX_HOME` when set, else `~/.codex/auth.json`.
pub fn auth_path() -> Option<PathBuf> {
    let home =
        std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| dirs::home_dir().map(|h| h.join(".codex")))?;
    Some(home.join("auth.json"))
}

/// Whether this computer's Codex sign-in can carry live voice, and why not when it can't. A missing,
/// unreadable or expired file is an answer, not an error: Voice settings has to say something useful.
pub fn access() -> Access {
    access_at(auth_path().as_deref())
}

/// The same for a named file, so a test never reads the user's own login.
pub fn access_at(path: Option<&std::path::Path>) -> Access {
    let Some(path) = path else {
        return Access::no(SIGNED_OUT);
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Access::no(SIGNED_OUT),
        Err(_) => return Access::no("Codync can't read Codex's sign-in on this computer."),
    };
    let auth: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Access::no("Codex's sign-in file is damaged. Run `codex login` again."),
    };
    let mode = mode_of(&auth);
    match mode {
        Mode::ApiKey => Access::no(
            "Codex is signed in with an API key, which doesn't carry live voice. Sign in with ChatGPT in Codex, or use an OpenAI key under Voice chat.",
        ),
        Mode::Unknown => Access::no(SIGNED_OUT),
        Mode::Chatgpt => {
            if expired(&auth) {
                return Access::no("Codex's ChatGPT sign-in has expired. Run `codex login` again.");
            }
            Access::ready(account_of(&auth))
        }
    }
}

const SIGNED_OUT: &str = "Codex isn't signed in on this computer. Run `codex login` in a terminal, then try again.";

fn mode_of(auth: &Value) -> Mode {
    match auth["auth_mode"].as_str() {
        Some("chatgpt") => Mode::Chatgpt,
        Some("apikey") | Some("api_key") => Mode::ApiKey,
        // Codex also writes `mode`; a file with a ChatGPT token but no mode is still a ChatGPT login.
        _ if auth["tokens"]["access_token"].is_string() => Mode::Chatgpt,
        _ if auth["OPENAI_API_KEY"].is_string() => Mode::ApiKey,
        _ => Mode::Unknown,
    }
}

/// The account a ChatGPT sign-in names (`tokens.account_id`, else the id token's email), never a token.
fn account_of(auth: &Value) -> Option<String> {
    auth["tokens"]["account_id"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| auth["tokens"]["id_token"].as_str().and_then(|token| jwt_claim(token, "email")))
}

/// A login past its access token's `exp` cannot start a call without Codex refreshing it first, which
/// Codync does not do behind the user's back.
fn expired(auth: &Value) -> bool {
    let Some(exp) = auth["tokens"]["access_token"].as_str().and_then(|token| jwt_claim(token, "exp")) else {
        return false;
    };
    let Ok(exp) = exp.parse::<i64>() else { return false };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX));
    exp <= now
}

/// Claim `name` from a JWT's payload. The token itself is never kept.
fn jwt_claim(token: &str, name: &str) -> Option<String> {
    use base64::Engine as _;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims: Value = serde_json::from_slice(&bytes).ok()?;
    match &claims[name] {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn file(name: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("codync-codex-voice-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    /// A payload only, padded as a JWT's middle part: enough for `jwt_claim`.
    fn jwt(claims: &Value) -> String {
        use base64::Engine as _;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string());
        format!("header.{payload}.signature")
    }

    #[test]
    fn no_sign_in_file_is_a_clear_reason_not_an_error() {
        let dir = std::env::temp_dir().join(format!("codync-codex-missing-{}", uuid::Uuid::new_v4()));
        let access = access_at(Some(dir.join("auth.json").as_path()));
        assert!(!access.available);
        assert!(access.reason.unwrap().contains("codex login"));
        assert!(!access_at(None).available);
    }

    #[test]
    fn a_chatgpt_sign_in_can_carry_live_voice() {
        let path = file(
            "auth.json",
            &serde_json::json!({
                "auth_mode": "chatgpt",
                "tokens": {"access_token": jwt(&serde_json::json!({"exp": 4_102_444_800i64})), "account_id": "acc_1"},
            })
            .to_string(),
        );
        let access = access_at(Some(path.as_path()));
        assert!(access.available, "{access:?}");
        assert_eq!(access.account.as_deref(), Some("acc_1"));
        assert_eq!(access.json()["available"], true);
    }

    #[test]
    fn an_api_key_or_expired_login_says_which_and_never_falls_through() {
        let key = file("auth.json", &serde_json::json!({"auth_mode": "apikey", "OPENAI_API_KEY": "sk-x"}).to_string());
        let access = access_at(Some(key.as_path()));
        assert!(!access.available);
        assert!(access.reason.unwrap().contains("API key"));
        let expired = file(
            "auth.json",
            &serde_json::json!({
                "auth_mode": "chatgpt",
                "tokens": {"access_token": jwt(&serde_json::json!({"exp": 1})), "account_id": "acc_1"},
            })
            .to_string(),
        );
        let access = access_at(Some(expired.as_path()));
        assert!(!access.available);
        assert!(access.reason.unwrap().contains("expired"));
        let broken = file("auth.json", "{not json");
        assert!(!access_at(Some(broken.as_path())).available);
    }

    #[test]
    fn the_account_can_come_from_the_id_token_and_a_token_is_never_reported() {
        let token = jwt(&serde_json::json!({"exp": 4_102_444_800i64, "email": "person@example.com"}));
        let path = file(
            "auth.json",
            &serde_json::json!({"auth_mode": "chatgpt", "tokens": {"access_token": token, "id_token": token}})
                .to_string(),
        );
        let access = access_at(Some(path.as_path()));
        assert_eq!(access.account.as_deref(), Some("person@example.com"));
        assert!(!format!("{access:?}").contains("signature"), "never the token");
    }
}
