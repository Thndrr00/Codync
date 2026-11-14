//! The local OpenAI-compatible speech server set in Voice chat settings: its URL, the models it
//! serves for dictation and for spoken replies, and an optional key it takes. Codync runs no
//! model itself: a server the person runs on this computer (sherpa-onnx, whisper.cpp, a
//! Parakeet or Cohere Transcribe wrapper) does the work, and Codync only speaks its protocol.

use super::endpoint;
use crate::market::vault;
use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use reqwest::Method;
use serde_json::{Value, json};

const BASE: &str = "voice.baseUrl";
const TRANSCRIBE: &str = "voice.transcribeModel";
const SPEECH: &str = "voice.speechModel";
/// The optional key this server takes, in this computer's vault: never logged, never shown.
const SLOT: &str = "voice:server";

/// Model ids the common local servers serve, offered as suggestions; any name can be typed.
pub const SUGGESTIONS: [&str; 4] =
    ["parakeet-tdt-0.6b-v3", "parakeet-unified-en-0.6b", "whisper-turbo", "cohere-transcribe-03-2026"];

/// The saved URL, as typed (a trailing `/v1` was dropped when it was saved); empty: none.
pub(super) fn base_url(store: &Store) -> Option<String> {
    store.kv_get(BASE).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// The model saved for one mode; empty keeps the provider's own default.
pub(super) fn model(store: &Store, transcribe: bool) -> String {
    store.kv_get(if transcribe { TRANSCRIBE } else { SPEECH }).unwrap_or_default().trim().to_owned()
}

/// The optional key this server takes, when one is saved.
pub(super) fn key(store: &Store) -> Result<Option<String>> {
    Ok(vault::read(store, SLOT)?.filter(|k| !k.is_empty()))
}

/// What Voice chat settings shows: the saved server, the URL requests actually go to (the
/// saved one, else `CODYNC_OPENAI_BASE_URL`), and the models to suggest.
pub fn state(store: &Store) -> Result<Value> {
    let effective = endpoint::openai_base(store)?
        .map(|url| url.as_str().trim_end_matches('/').to_owned())
        .unwrap_or_else(|| "https://api.openai.com".to_owned());
    Ok(json!({
        "baseUrl": base_url(store).unwrap_or_default(),
        "transcribeModel": model(store, true),
        "speechModel": model(store, false),
        "keySaved": key(store)?.is_some(),
        "effective": effective,
        // Requests go somewhere other than api.openai.com: Realtime is unavailable meanwhile.
        "custom": effective != "https://api.openai.com",
        // A developer's variable applies while no server is saved here.
        "environment": std::env::var("CODYNC_OPENAI_BASE_URL").ok().filter(|v| !v.trim().is_empty()),
        "suggestions": SUGGESTIONS,
    }))
}

/// Saves what the form sent. `baseUrl` and the models are kept when the field is absent, so a
/// partial answer can't wipe the server; an empty one removes it. `key` absent keeps the saved
/// key, empty removes it. The whole form is checked first, so a refused field changes nothing.
pub fn save(store: &Store, b: &Value) -> Result<Value> {
    let base = match b.get("baseUrl").filter(|v| !v.is_null()) {
        None => None,
        Some(value) => {
            let typed = value.as_str().ok_or_else(|| anyhow!("baseUrl must be a string"))?.trim();
            if typed.is_empty() {
                Some(None)
            } else {
                let url = endpoint::parse_base(typed, endpoint::SETTING)?;
                Some(Some(url.as_str().trim_end_matches('/').to_owned()))
            }
        }
    };
    let mut models = Vec::new();
    for (field, slot) in [("transcribeModel", TRANSCRIBE), ("speechModel", SPEECH)] {
        let Some(value) = b.get(field).filter(|v| !v.is_null()) else { continue };
        let typed = value.as_str().ok_or_else(|| anyhow!("{field} must be a string"))?.trim();
        if !typed.is_empty() {
            super::checked(typed).map_err(|_| anyhow!("`{typed}` isn't a model name"))?;
        }
        models.push((slot, typed.to_owned()));
    }
    let key = match b.get("key").filter(|v| !v.is_null()) {
        None => None,
        Some(value) => {
            let key = value.as_str().ok_or_else(|| anyhow!("key must be a string"))?.trim();
            if key.len() > 4096 || key.chars().any(char::is_control) {
                bail!("That key can't be saved: it must be one line of at most 4096 characters.");
            }
            Some(key.to_owned())
        }
    };
    match base {
        Some(Some(url)) => store.kv_set(BASE, &url)?,
        Some(None) => store.kv_delete(BASE)?,
        None => {}
    }
    for (slot, model) in models {
        if model.is_empty() {
            store.kv_delete(slot)?;
        } else {
            store.kv_set(slot, &model)?;
        }
    }
    if let Some(key) = key {
        if key.is_empty() {
            store.kv_delete(SLOT)?;
        } else {
            vault::write(store, SLOT, &key)?;
        }
    }
    state(store)
}

/// Tries the saved server and says why when it doesn't answer: a tiny dictation when a model is
/// set (the path that has to work), else its model list. A failure is an answer, not an error.
pub async fn test(store: &Store) -> Result<Value> {
    let Some(base) = endpoint::openai_base(store)? else {
        return Ok(json!({
            "ok": false,
            "message": "No local voice server is set. Add the URL of the server on this computer.",
        }));
    };
    let key = key(store)?;
    let model = model(store, true);
    // Both at once: the model list is a bonus, so a slow server doesn't wait twice.
    let (served, heard) = tokio::join!(list(&base, key.as_deref()), hear(&base, key.as_deref(), &model));
    let models = served.unwrap_or_default();
    let reason = if model.is_empty() {
        // Nothing to dictate with: the model list is all this setting can be checked against.
        if models.is_empty() { Err(anyhow!("The server answered but listed no models.")) } else { Ok(()) }
    } else {
        heard.map(|text| {
            tracing::info!(model, chars = text.len(), "local voice server transcribed the test recording");
        })
    };
    let answer = match reason {
        Ok(()) => json!({
            "ok": true,
            "effective": base.as_str(),
            "models": models,
            "message": if model.is_empty() {
                "The server answered its model list.".to_owned()
            } else {
                format!("The server transcribed a test recording with {model}.")
            },
        }),
        Err(error) => json!({
            "ok": false,
            "effective": base.as_str(),
            "models": models,
            "message": format!("{error:#}"),
        }),
    };
    Ok(answer)
}

/// The model ids the server serves (`GET /v1/models`), best-effort.
async fn list(base: &reqwest::Url, key: Option<&str>) -> Result<Vec<String>> {
    let url = endpoint::openai_url(Some(base), "/v1/models");
    let (status, v) = request(endpoint::openai_http()?.get(url), key).await?;
    if !status.is_success() {
        return Err(server_error(status, &v));
    }
    Ok(v["data"].as_array().into_iter().flatten().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect())
}

/// A quarter second of silence through the dictation path, with the saved model.
async fn hear(base: &reqwest::Url, key: Option<&str>, model: &str) -> Result<String> {
    if model.is_empty() {
        bail!("Set a dictation model first.");
    }
    let boundary = format!("codync-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::with_capacity(4096);
    let mut field = |name: &str, value: &str| {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes(),
        );
    };
    field("model", model);
    field("response_format", "json");
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"speech.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(&silence());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let url = endpoint::openai_url(Some(base), "/v1/audio/transcriptions");
    let req = endpoint::openai_http()?
        .request(Method::POST, url)
        .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
        .body(body);
    let (status, v) = request(req, key).await?;
    if !status.is_success() {
        return Err(server_error(status, &v));
    }
    Ok(v["text"].as_str().unwrap_or_default().trim().to_owned())
}

async fn request(req: reqwest::RequestBuilder, key: Option<&str>) -> Result<(reqwest::StatusCode, Value)> {
    let req = match key {
        Some(key) => req.bearer_auth(key),
        None => req,
    };
    let res = req
        .timeout(super::TIMEOUT)
        .send()
        .await
        .map_err(|_| anyhow!("Can't reach the voice server. Check that it is running and the URL is right."))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    Ok((status, serde_json::from_str(&body).unwrap_or(Value::Null)))
}

/// The server's own message, in the shapes compatible servers use.
fn server_error(status: reqwest::StatusCode, v: &Value) -> anyhow::Error {
    v["error"]["message"]
        .as_str()
        .or_else(|| v["detail"]["message"].as_str())
        .or_else(|| v["detail"].as_str())
        .map_or_else(|| anyhow!("The voice server answered {status}."), |message| anyhow!("{message}"))
}

/// 16 kHz mono 16-bit WAV of 0.25 s of silence: enough for any server to answer.
fn silence() -> Vec<u8> {
    let data = vec![0u8; 8_000];
    let mut out = Vec::with_capacity(data.len() + 44);
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&16_000u32.to_le_bytes());
    out.extend_from_slice(&32_000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn store() -> Store {
        // An in-memory store makes its own vault key, so the optional key round-trips.
        Store::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn saving_keeps_what_the_form_did_not_send() {
        let store = store();
        let saved = save(
            &store,
            &json!({
                "baseUrl": "http://127.0.0.1:8890/v1/",
                "transcribeModel": "parakeet-tdt-0.6b-v3",
                "speechModel": "whisper-turbo",
            }),
        )
        .unwrap();
        assert_eq!(saved["baseUrl"], "http://127.0.0.1:8890");
        assert_eq!(saved["transcribeModel"], "parakeet-tdt-0.6b-v3");
        assert_eq!(saved["effective"], "http://127.0.0.1:8890");
        assert_eq!(saved["custom"], true);
        // A partial answer (a key only) leaves the URL and models where they were.
        let again = save(&store, &json!({"key": "local-token"})).unwrap();
        assert_eq!(again["baseUrl"], "http://127.0.0.1:8890");
        assert_eq!(again["speechModel"], "whisper-turbo");
        assert_eq!(again["keySaved"], true);
        // An empty key removes it; an empty URL turns the setting off and keeps the rest.
        assert_eq!(save(&store, &json!({"key": ""})).unwrap()["keySaved"], false);
        let off = save(&store, &json!({"baseUrl": ""})).unwrap();
        assert_eq!(off["baseUrl"], "");
        assert_eq!(off["custom"], false);
        assert_eq!(off["effective"], "https://api.openai.com");
        assert_eq!(off["transcribeModel"], "parakeet-tdt-0.6b-v3");
    }

    #[test]
    fn an_invalid_url_or_model_is_refused_with_its_reason() {
        let store = store();
        for base in ["http://speech.example", "ftp://127.0.0.1:8890", "http://127.0.0.1:8890?key=x"] {
            let error = save(&store, &json!({"baseUrl": base})).unwrap_err().to_string();
            assert!(error.contains("local voice server URL"), "{error}");
            assert_eq!(base_url(&store), None, "nothing invalid is kept");
        }
        let error = save(&store, &json!({"transcribeModel": "bad model"})).unwrap_err().to_string();
        assert!(error.contains("isn't a model name"), "{error}");
    }

    #[tokio::test]
    async fn a_server_that_answers_its_list_is_reported_as_working() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let store = store();
        save(&store, &json!({"baseUrl": format!("http://{address}")})).unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert!(line.starts_with("GET /v1/models"), "{line}");
            loop {
                let mut header = String::new();
                stream.read_line(&mut header).await.unwrap();
                if header == "\r\n" {
                    break;
                }
            }
            let answer = r#"{"data":[{"id":"parakeet-tdt-0.6b-v3"},{"id":"whisper-turbo"}]}"#;
            stream
                .get_mut()
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                        answer.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let operation = async {
            let answer = test(&store).await.unwrap();
            assert_eq!(answer["ok"], true, "{answer}");
            assert_eq!(answer["models"], json!(["parakeet-tdt-0.6b-v3", "whisper-turbo"]));
            server.await.unwrap();
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), operation).await.unwrap();
    }

    #[tokio::test]
    async fn a_refused_server_reports_the_servers_own_reason() {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let store = store();
        save(&store, &json!({"baseUrl": format!("http://{address}"), "transcribeModel": "parakeet-tdt-0.6b-v3"}))
            .unwrap();
        let server = tokio::spawn(async move {
            // The model list and the test recording arrive at once: two requests.
            for _ in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = BufReader::new(stream);
                let mut size = 0;
                loop {
                    let mut line = String::new();
                    stream.read_line(&mut line).await.unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':')
                        && name.eq_ignore_ascii_case("content-length")
                    {
                        size = value.trim().parse::<usize>().unwrap();
                    }
                }
                // Read the body before answering, so the client isn't cut off mid-send.
                let mut body = vec![0; size];
                if size > 0 {
                    stream.read_exact(&mut body).await.unwrap();
                }
                let answer = r#"{"error":{"message":"model 'parakeet-tdt-0.6b-v3' is not loaded"}}"#;
                stream
                    .get_mut()
                    .write_all(
                        format!(
                            "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                            answer.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        });
        let operation = async {
            let answer = test(&store).await.unwrap();
            assert_eq!(answer["ok"], false, "{answer}");
            assert!(answer["message"].as_str().unwrap().contains("is not loaded"), "{answer}");
            server.await.unwrap();
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), operation).await.unwrap();
    }

    #[test]
    fn the_test_recording_is_a_valid_wav() {
        let wav = silence();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]), 16_000);
        assert_eq!(wav.len(), 8_044);
    }
}
