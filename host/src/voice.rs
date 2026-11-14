//! Voice on the user's own provider keys: `OpenAI` Realtime and Gemini Live for
//! a live call, and their speech models (plus `ElevenLabs` and xAI) for speech mode, where
//! each utterance is transcribed and each reply is read aloud; `Groq` dictates only. The keys
//! stay in this computer's vault; a client starting a live call gets a short-lived
//! credential minted here and then streams audio to the provider directly, so
//! audio never crosses the channel or the relay.

pub(crate) mod decisions;
mod elevenlabs;
mod endpoint;
pub(crate) mod error_log;
mod keys;
mod models;
pub(crate) mod neutts;
pub(crate) mod receipts;
mod server;
mod speech;
mod streaming;
pub(crate) mod synthesis;
mod transcript;
pub(crate) mod transcription;
mod xai;

pub use models::models;
pub use server::{save as set_server, state as server_state, test as test_server};
/// Gemini's generate route serves both speech and transcription.
pub(super) use speech::gemini_generate;
pub use speech::speak;
pub use transcription::transcribe;

use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use base64::engine::general_purpose::STANDARD;
use endpoint::realtime_url;
use keys::Access;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

pub(super) const TIMEOUT: Duration = Duration::from_secs(15);
/// Speech to text: keep the speaker's language and script, and coding terms as written.
const TRANSCRIBE_HINT: &str = "Keep the speaker's language and script. Keep code, file and product names as written.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "gemini")]
    Gemini,
    /// Speech only: `ElevenLabs` holds no live conversation here.
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
    /// Speech only: xAI reads replies aloud and hears nothing (OpenBot 0.60).
    #[serde(rename = "xai")]
    Xai,
    /// Dictation only: Groq's Whisper hears speech and reads nothing (OpenBot 0.60).
    #[serde(rename = "groq")]
    Groq,
}

impl Provider {
    const ALL: [Self; 5] = [Self::OpenAi, Self::Gemini, Self::ElevenLabs, Self::Xai, Self::Groq];

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "openai" => Ok(Self::OpenAi),
            "gemini" => Ok(Self::Gemini),
            "elevenlabs" => Ok(Self::ElevenLabs),
            "xai" => Ok(Self::Xai),
            "groq" => Ok(Self::Groq),
            _ => bail!("unknown voice provider `{s}`"),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Gemini => "gemini",
            Self::ElevenLabs => "elevenlabs",
            Self::Xai => "xai",
            Self::Groq => "groq",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini",
            Self::ElevenLabs => "ElevenLabs",
            Self::Xai => "xAI",
            Self::Groq => "Groq",
        }
    }

    /// The defaults until this computer has seen the provider's own list (`refresh_loop`).
    fn fallback(self) -> Defaults {
        let (realtime, transcribe, speech) = match self {
            Self::OpenAi => ("gpt-realtime-2.1", "gpt-transcribe", "gpt-4o-mini-tts"),
            Self::Gemini => ("gemini-3.8-live", "gemini-3.8-flash", "gemini-3.8-flash-tts"),
            Self::ElevenLabs => ("", "scribe_v1", "eleven_flash_v2_5"),
            // xAI's speech route takes no model: the id names its one text-to-speech choice.
            Self::Xai => ("", "", "xai-tts"),
            // Groq's Whisper is sent to it by name (OpenBot's `GROQ_MODELS`).
            Self::Groq => ("", "whisper-large-v3-turbo", ""),
        };
        Defaults { realtime: realtime.into(), transcribe: transcribe.into(), speech: speech.into() }
    }

    /// Only OpenAI and Gemini hold a live conversation.
    fn live(self) -> bool {
        matches!(self, Self::OpenAi | Self::Gemini)
    }

    /// Whether it can hear speech (xAI reads replies only).
    pub(super) fn transcribes(self) -> bool {
        self != Self::Xai
    }

    /// Whether it can read replies aloud (Groq hears only).
    fn speaks(self) -> bool {
        self != Self::Groq
    }
}

/// The model each voice mode uses until the user picks one: the provider's newest, worked out
/// from its model list once a day, so a new release becomes the default without an app update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Defaults {
    realtime: String,
    transcribe: String,
    speech: String,
}

const REFRESH_EVERY: Duration = Duration::from_hours(24);

pub(super) fn defaults_key(p: Provider) -> String {
    format!("voice-defaults.{}", p.id())
}

fn defaults(store: &Store, p: Provider) -> Defaults {
    store.kv_get(&defaults_key(p)).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| p.fallback())
}

/// Looks at each keyed provider's model list once a day and keeps its newest models as the defaults.
pub async fn refresh_loop(hub: std::sync::Arc<crate::hub::Hub>) {
    loop {
        for p in Provider::ALL {
            if keys::configured(&hub.store, p).unwrap_or(false)
                && let Err(e) = models(&hub.store, p).await
            {
                tracing::info!(
                    provider = p.id(),
                    error = format!("{e:#}"),
                    "voice model refresh failed; keeping the last defaults"
                );
            }
        }
        tokio::time::sleep(REFRESH_EVERY).await;
    }
}

/// Model ids and voices go into provider requests: letters, digits, `-`, `.`, `_`.
fn checked(s: &str) -> Result<&str> {
    let ok = !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || "-._".contains(c));
    if ok { Ok(s) } else { bail!("`{s}` isn't a model or voice name") }
}

/// Which half of a voice a model serves: hearing (`/v1/audio/transcriptions`) or reading
/// (`/v1/audio/speech`). A local voice server saves one model for each.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Transcribe,
    Speech,
}

/// The model a request actually uses: the local voice server's saved model for the mode when
/// there is one (the server serves what it serves), else the model the app picked, which is the
/// provider's own default until the user chooses.
pub(super) fn model_for(store: &Store, p: Provider, kind: Kind, chosen: &str) -> Result<String> {
    if p == Provider::OpenAi && endpoint::is_custom(store)? {
        let saved = server::model(store, kind == Kind::Transcribe);
        if !saved.is_empty() {
            return Ok(saved);
        }
    }
    Ok(chosen.to_owned())
}

/// The provider's own message from an error body ("Incorrect API key provided…"), whatever shape
/// it uses: `error.message` for `OpenAI` and Gemini, `detail` for `ElevenLabs`.
pub(super) fn provider_error(p: Provider, status: reqwest::StatusCode, v: &Value) -> anyhow::Error {
    let message = v["error"]["message"]
        .as_str()
        .or_else(|| v["detail"]["message"].as_str())
        .or_else(|| v["detail"].as_str())
        .map_or_else(|| format!("{} answered {status}.", p.name()), str::to_owned);
    anyhow!(message)
}

async fn send(p: Provider, req: reqwest::RequestBuilder) -> Result<Value> {
    let res = req.timeout(TIMEOUT).send().await.map_err(|_| anyhow!("Can't reach {}.", p.name()))?;
    let status = res.status();
    let v: Value = serde_json::from_str(&res.text().await.unwrap_or_default()).unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(provider_error(p, status, &v));
    }
    Ok(v)
}

/// A credential for one call's audio session: an `OpenAI` client secret (10 minutes)
/// or a one-use Gemini ephemeral token.
async fn mint(p: Provider, access: &Access, model: &str, voice: &str) -> Result<String> {
    let v = match p {
        Provider::OpenAi => {
            let body = json!({
                "expires_after": {"anchor": "created_at", "seconds": 600},
                "session": {"type": "realtime", "model": model, "audio": {"output": {"voice": voice}}},
            });
            // A compatible server on this computer has no Realtime credential minting.
            realtime_url(access.is_local())?;
            send(p, access.request(p, Method::POST, "/v1/realtime/client_secrets").json(&body)).await?
        }
        Provider::Gemini => {
            let now = chrono::Utc::now();
            let body = json!({
                "uses": 1,
                "expireTime": (now + chrono::Duration::minutes(30)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "newSessionExpireTime": (now + chrono::Duration::minutes(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            });
            send(p, access.request(p, Method::POST, "/v1alpha/auth_tokens").json(&body)).await?
        }
        Provider::ElevenLabs => bail!("{} doesn't hold live calls. Choose Speech mode.", p.name()),
        Provider::Xai => {
            bail!("{} reads replies aloud but doesn't hear you. Choose Speech with another provider.", p.name())
        }
        Provider::Groq => bail!("{} hears speech but doesn't read replies. Pick another provider for calls.", p.name()),
    };
    let field = match p {
        Provider::OpenAi => "value",
        Provider::Gemini => "name",
        Provider::ElevenLabs | Provider::Xai | Provider::Groq => unreachable!(),
    };
    v[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{} sent no session credential.", p.name()))
}

/// Which providers have a key here, with their default models.
pub fn status(store: &Store) -> Result<Value> {
    let providers = Provider::ALL
        .iter()
        .map(|&p| {
            let defaults = defaults(store, p);
            Ok(json!({"provider": p.id(), "configured": keys::configured(store, p)?, "defaults": defaults}))
        })
        .collect::<Result<Vec<Value>>>()?;
    Ok(json!({"providers": providers}))
}

/// The voices the key can read with, for the picker. Other providers have a fixed list.
pub async fn voices(store: &Store, p: Provider) -> Result<Value> {
    match p {
        Provider::ElevenLabs => {
            let voices: Vec<Value> = elevenlabs::voices(keys::access(store, p)?.secret()?)
                .await?
                .into_iter()
                .map(|(id, name)| json!({"id": id, "name": name}))
                .collect();
            Ok(json!({"voices": voices}))
        }
        _ => bail!("{} has a fixed voice list.", p.name()),
    }
}

/// Checks the key with the provider (a credential mint, or for `ElevenLabs` its voices and one
/// character read), then keeps it. An empty key removes it.
pub async fn set_key(store: &Store, p: Provider, key: &str) -> Result<Value> {
    let key = key.trim();
    if !key.is_empty() {
        match p {
            Provider::ElevenLabs => elevenlabs::check(key).await?,
            // Both answer a model list on the key; their speech routes take no model.
            Provider::Xai | Provider::Groq => {
                let access = Access::direct(store, p, key)?;
                send(p, access.request(p, Method::GET, "/v1/models")).await?;
            }
            Provider::OpenAi | Provider::Gemini => {
                let access = Access::direct(store, p, key)?;
                if p == Provider::OpenAi && access.is_local() {
                    // Compatible speech servers may not implement Realtime credential minting.
                    send(p, access.request(p, Method::GET, "/v1/models")).await?;
                } else {
                    let voice = if p == Provider::OpenAi { "marin" } else { "Kore" };
                    mint(p, &access, &defaults(store, p).realtime, voice).await?;
                }
            }
        }
    }
    keys::save(store, p, key)?;
    if !key.is_empty() {
        // A new key may see newer models; the daily refresh would only get there tomorrow.
        let _ = models(store, p).await;
    }
    status(store)
}

/// A credential for a call starting now.
pub async fn session(store: &Store, p: Provider, model: &str, voice: &str) -> Result<Value> {
    if !p.live() {
        bail!("{} doesn't hold live calls. Choose Speech with a provider that hears and speaks.", p.name());
    }
    let access = keys::access(store, p)?;
    Ok(json!({"credential": mint(p, &access, checked(model)?, checked(voice)?).await?}))
}

/// Names a speaker is likely to say that speech-to-text would otherwise misspell: the app and
/// the bots on this computer.
fn spoken_names(store: &Store) -> Vec<String> {
    let mut names = vec!["Codync".to_owned()];
    for bot in store.bots().unwrap_or_default() {
        if !bot.deleted && !names.contains(&bot.config.name) {
            names.push(bot.config.name);
        }
    }
    names.truncate(50);
    names
}

fn names_hint(names: &[String]) -> String {
    format!("Names that may come up: {}.", names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_checked() {
        assert!(checked("gpt-realtime-2.1").is_ok());
        assert!(checked("Kore").is_ok());
        assert!(checked("x\"}").is_err());
        assert!(checked("").is_err());
        assert!(Provider::parse("openai").is_ok());
        assert!(Provider::parse("elevenlabs").is_ok());
        assert!(Provider::parse("xai").is_ok());
        assert!(Provider::parse("groq").is_ok());
        assert!(Provider::parse("anthropic").is_err());
    }

    fn memory_store() -> Store {
        Store::open(std::path::Path::new(":memory:")).expect("an in-memory store")
    }

    #[test]
    fn the_local_servers_models_win_over_the_apps_choice() {
        let store = memory_store();
        // Other providers keep what the app picked, whatever a local server saves.
        store.kv_set("voice.baseUrl", "http://127.0.0.1:8890").unwrap();
        store.kv_set("voice.transcribeModel", "parakeet-tdt-0.6b-v3").unwrap();
        store.kv_set("voice.speechModel", "whisper-turbo").unwrap();
        assert_eq!(
            model_for(&store, Provider::Gemini, Kind::Transcribe, "gemini-3.8-flash").unwrap(),
            "gemini-3.8-flash"
        );
        assert_eq!(model_for(&store, Provider::ElevenLabs, Kind::Speech, "scribe_v1").unwrap(), "scribe_v1");
        // OpenAI speech goes to the server, so its saved models are the ones it serves.
        assert_eq!(
            model_for(&store, Provider::OpenAi, Kind::Transcribe, "gpt-transcribe").unwrap(),
            "parakeet-tdt-0.6b-v3"
        );
        assert_eq!(model_for(&store, Provider::OpenAi, Kind::Speech, "gpt-4o-mini-tts").unwrap(), "whisper-turbo");
        // A mode with no model saved keeps the provider's default.
        store.kv_delete("voice.speechModel").unwrap();
        assert_eq!(model_for(&store, Provider::OpenAi, Kind::Speech, "gpt-4o-mini-tts").unwrap(), "gpt-4o-mini-tts");
    }

    #[tokio::test]
    async fn a_local_server_reads_replies_with_its_saved_model_as_wav() {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let store = memory_store();
        store.kv_set("voice.baseUrl", &format!("http://{address}")).unwrap();
        store.kv_set("voice.speechModel", "whisper-turbo").unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut request = String::new();
            stream.read_line(&mut request).await.unwrap();
            let mut size = 0;
            loop {
                let mut line = String::new();
                stream.read_line(&mut line).await.unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    size = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; size];
            stream.read_exact(&mut body).await.unwrap();
            let body = String::from_utf8_lossy(&body);
            assert_eq!(request.trim(), "POST /v1/audio/speech HTTP/1.1");
            assert!(body.contains("\"model\":\"whisper-turbo\""), "{body}");
            assert!(body.contains("\"response_format\":\"wav\""), "{body}");
            let audio = b"RIFFwavfixture";
            stream
                .get_mut()
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        audio.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            stream.get_mut().write_all(audio).await.unwrap();
        });
        let operation = async {
            let clip = speak(&store, Provider::OpenAi, "gpt-4o-mini-tts", "marin", "Hello").await.unwrap();
            assert_eq!(clip["format"], "wav");
            assert_eq!(clip["audio"], STANDARD.encode(b"RIFFwavfixture"));
            server.await.unwrap();
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), operation).await.unwrap();
    }
}
