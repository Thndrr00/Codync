//! The opt-in OpenAI-compatible path for a `custom` bot.
//!
//! A custom backend normally runs the CLI the user saved. When a bot's
//! `customProviderMode` is on and its saved `OPENAI_BASE_URL` is OpenAI's v1 with a key, this
//! module sends the turn over HTTP instead: one bounded request built by
//! [`crate::agent::config::custom_request`]. Everything else keeps the CLI path, so no existing
//! provider changes.

use crate::agent::auth;
use crate::agent::config::{CustomEndpoint, custom_request, request_path, uses_responses_api};
use crate::egress::Policy;
use crate::store::{BotConfig, CustomProviderMode, Store};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::time::Duration;

/// A turn's response is capped well below anything an agent needs for one answer.
const MAX_ANSWER: usize = 256 * 1024;
/// One turn is a single request, not a long-lived stream.
const TIMEOUT: Duration = Duration::from_secs(60);

/// The saved endpoint's values, kept together so [`CustomEndpoint`] can borrow them. Deliberately
/// has no `Debug`: the key must never reach a log.
pub struct SavedEndpoint {
    base_url: String,
    api_key: Option<String>,
}

impl SavedEndpoint {
    fn endpoint(&self) -> CustomEndpoint<'_> {
        CustomEndpoint { base_url: &self.base_url, api_key: self.api_key.as_deref() }
    }
}

/// The base URL and key saved for a custom bot (its own values over the backend's). Only values
/// the user typed are read; on a managed computer the keeper's grant is not a usable key here, so
/// the mode does not qualify (see [`turn`]).
pub fn saved(store: &Store, bot: &BotConfig) -> Result<Option<SavedEndpoint>> {
    if bot.backend != "custom" {
        return Ok(None);
    }
    let mut base_url = None;
    let mut api_key = None;
    for (name, value) in auth::env(store, &bot.backend, Some(&bot.id))?.0 {
        match name.as_str() {
            "OPENAI_BASE_URL" => base_url = Some(value),
            "OPENAI_API_KEY" => api_key = Some(value),
            _ => {}
        }
    }
    Ok(base_url.map(|base_url| SavedEndpoint { base_url, api_key }))
}

/// The URL a turn is posted to: `request_path` picks the adapter, and a saved trailing `/v1` is
/// trimmed so `https://api.openai.com/v1` becomes `/v1/responses`, never `/v1/v1/responses`.
pub fn request_url(endpoint: &CustomEndpoint<'_>) -> String {
    let base = endpoint.base_url.trim().trim_end_matches('/');
    let base = base.strip_suffix("/v1").unwrap_or(base);
    format!("{base}{}", request_path(endpoint))
}

/// The canonical custom request plus this turn's messages. Streaming is off for the host's single
/// answer; the tools and reasoning effort from [`custom_request`] stay untouched, and the key is
/// not part of this body.
fn body(
    endpoint: &CustomEndpoint<'_>,
    model: &str,
    effort: Option<&str>,
    tools: &[Value],
    messages: &[Value],
) -> Value {
    let mut body = custom_request(endpoint, model, effort, tools);
    body["messages"] = json!(messages);
    body["stream"] = json!(false);
    body
}

/// The text of one answer: chat completions put it in `choices`, the Responses adapter in `output`.
fn answer(body: &Value) -> String {
    if let Some(text) = body["choices"][0]["message"]["content"].as_str() {
        return text.to_owned();
    }
    let mut out = String::new();
    for item in body["output"].as_array().into_iter().flatten() {
        for part in item["content"].as_array().into_iter().flatten() {
            if part["type"] == "output_text"
                && let Some(text) = part["text"].as_str()
            {
                out.push_str(text);
            }
        }
    }
    out
}

/// Posts one turn to the saved endpoint. The key is only ever used as a request header; no URL,
/// key or body is written to a log or returned to the caller. The host's egress policy checks and
/// pins the destination, and redirects stay inside the same origin.
pub async fn send(
    endpoint: &CustomEndpoint<'_>,
    model: &str,
    effort: Option<&str>,
    tools: &[Value],
    messages: &[Value],
) -> Result<Value> {
    let url = request_url(endpoint);
    let client = crate::egress::client(Policy::UserChosen, &url).await?;
    let mut request = client.post(url.as_str()).json(&body(endpoint, model, effort, tools, messages)).timeout(TIMEOUT);
    if let Some(key) = endpoint.api_key.filter(|key| !key.trim().is_empty()) {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        bail!("the endpoint answered {}", status.as_u16());
    }
    let body: Value = serde_json::from_slice(&crate::egress::body(response, MAX_ANSWER).await?)?;
    Ok(json!({"path": request_path(endpoint), "text": answer(&body)}))
}

/// The bounded caller: only an opted-in `custom` bot whose saved URL is OpenAI's v1 with a key
/// takes this path. Every other case returns `None`, leaving the user's CLI as the launch path.
pub async fn turn(
    store: &Store,
    bot: &BotConfig,
    model: Option<&str>,
    effort: Option<&str>,
    tools: &[Value],
    messages: &[Value],
) -> Result<Option<Value>> {
    if bot.custom_provider_mode != CustomProviderMode::OpenaiCompatible {
        return Ok(None);
    }
    let Some(saved) = saved(store, bot)? else { return Ok(None) };
    let endpoint = saved.endpoint();
    if !uses_responses_api(&endpoint) {
        return Ok(None);
    }
    let Some(model) = model.filter(|model| !model.trim().is_empty()) else {
        bail!("the OpenAI-compatible mode needs a model");
    };
    Ok(Some(send(&endpoint, model, effort, tools, messages).await?))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use axum::{Json, Router, extract::Request, extract::State};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    fn bot(mode: CustomProviderMode) -> BotConfig {
        serde_json::from_value(json!({
            "id": "b", "name": "Bot", "backend": "custom", "command": "my-agent --acp",
            "customProviderMode": mode.as_str()
        }))
        .unwrap()
    }

    #[test]
    fn the_mode_switches_the_request_path_and_keeps_tools_with_the_effort() {
        let openai = CustomEndpoint { base_url: "https://api.openai.com/v1", api_key: Some("sk-test") };
        assert_eq!(request_url(&openai), "https://api.openai.com/v1/responses");
        let body = body(&openai, "gpt-6-astra", Some("high"), &[json!({"type": "function"})], &[]);
        assert_eq!(body["tools"], json!([{"type": "function"}]));
        assert_eq!(body["reasoning_effort"], "high");
        assert_eq!(body["store"], false);
        assert!(!body.to_string().contains("sk-test"), "the key is a header, not the body");

        let other = CustomEndpoint { base_url: "https://llm.example.com/v1", api_key: Some("sk-test") };
        assert_eq!(request_url(&other), "https://llm.example.com/v1/chat/completions");
    }

    #[test]
    fn the_saved_endpoint_reads_the_backend_and_bot_values() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        crate::market::vault::write(
            &store,
            "agent-env:custom",
            &json!({"OPENAI_BASE_URL": "https://api.openai.com/v1", "OPENAI_API_KEY": "sk-backend"}).to_string(),
        )
        .unwrap();
        let base = bot(CustomProviderMode::Cli);
        assert_eq!(saved(&store, &base).unwrap().unwrap().endpoint().api_key, Some("sk-backend"));

        crate::market::vault::write(&store, "agent-env:custom:b", &json!({"OPENAI_API_KEY": "sk-bot"}).to_string())
            .unwrap();
        let saved = saved(&store, &base).unwrap().unwrap();
        assert_eq!(saved.endpoint().api_key, Some("sk-bot"), "the bot's own key wins");
        assert_eq!(saved.endpoint().base_url, "https://api.openai.com/v1");

        let claude: BotConfig = serde_json::from_value(json!({"id": "b", "name": "B", "backend": "claude"})).unwrap();
        assert!(saved(&store, &claude).unwrap().is_none(), "only custom bots use this path");
    }

    #[tokio::test]
    async fn the_mode_off_and_a_non_openai_url_keep_the_cli_path() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let off = bot(CustomProviderMode::Cli);
        assert!(turn(&store, &off, Some("gpt-6-astra"), None, &[], &[]).await.unwrap().is_none());

        crate::market::vault::write(
            &store,
            "agent-env:custom",
            &json!({"OPENAI_BASE_URL": "https://llm.example.com/v1", "OPENAI_API_KEY": "sk-test"}).to_string(),
        )
        .unwrap();
        let on = bot(CustomProviderMode::OpenaiCompatible);
        // The saved URL is not OpenAI's v1, so `uses_responses_api` keeps the CLI path.
        assert!(turn(&store, &on, Some("gpt-6-astra"), None, &[], &[]).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn one_turn_posts_tools_and_effort_with_the_key_only_in_the_header() {
        #[derive(Clone)]
        struct Seen {
            path: Arc<Mutex<String>>,
            auth: Arc<Mutex<String>>,
            body: Arc<Mutex<Value>>,
        }
        async fn capture(State(seen): State<Seen>, request: Request) -> Json<Value> {
            *seen.path.lock().unwrap() = request.uri().path().to_owned();
            *seen.auth.lock().unwrap() =
                request.headers().get("authorization").and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned();
            let bytes = axum::body::to_bytes(request.into_body(), 64 * 1024).await.unwrap();
            *seen.body.lock().unwrap() = serde_json::from_slice(&bytes).unwrap();
            Json(json!({"choices": [{"message": {"content": "done"}}]}))
        }

        let seen = Seen {
            path: Arc::new(Mutex::new(String::new())),
            auth: Arc::new(Mutex::new(String::new())),
            body: Arc::new(Mutex::new(Value::Null)),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().fallback(capture).with_state(seen.clone())).await
        });

        let endpoint = CustomEndpoint { base_url: &format!("http://{address}/v1"), api_key: Some("sk-test") };
        let answer = send(
            &endpoint,
            "gpt-6-astra",
            Some("high"),
            &[json!({"type": "function", "function": {"name": "search"}})],
            &[json!({"role": "user", "content": "hello"})],
        )
        .await
        .unwrap();
        server.abort();

        assert_eq!(seen.path.lock().unwrap().as_str(), "/v1/chat/completions");
        assert_eq!(seen.auth.lock().unwrap().as_str(), "Bearer sk-test");
        let sent = seen.body.lock().unwrap().clone();
        assert_eq!(sent["reasoning_effort"], "high");
        assert_eq!(sent["tools"][0]["function"]["name"], "search");
        assert_eq!(sent["messages"][0]["content"], "hello");
        assert!(!sent.to_string().contains("sk-test"), "the key stays out of the body");
        assert_eq!(answer["path"], "/v1/chat/completions");
        assert_eq!(answer["text"], "done");
        assert!(!answer.to_string().contains("sk-test"), "the key stays out of the answer");
    }

    #[tokio::test]
    async fn a_failed_request_does_not_echo_the_key_or_the_url() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        });
        let endpoint = CustomEndpoint { base_url: &format!("http://{address}/v1"), api_key: Some("sk-secret") };
        let error = send(&endpoint, "gpt-6-astra", None, &[], &[]).await.unwrap_err().to_string();
        server.await.unwrap();
        assert!(!error.contains("sk-secret"), "no key in an error or a log line");
        assert!(!error.contains(&address.to_string()), "no URL in an error or a log line");
    }
}
