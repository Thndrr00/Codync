//! What each voice provider's model list offers: the models of a mode, and the newest of them as
//! the defaults until the user picks one. Split from `voice.rs` so the provider's key handling and
//! the speech routes stay readable.

use super::endpoint::openai_base;
use super::{Defaults, Provider, defaults_key, send};
use crate::store::Store;
use anyhow::Result;
use reqwest::Method;
use serde_json::{Value, json};

/// Models the key can use, from the provider's own list: realtime (conversation),
/// transcribe (speech to text) and speech (text to speech).
pub async fn models(store: &Store, p: Provider) -> Result<Value> {
    let access = super::keys::access(store, p)?;
    let entries: Vec<Model> = match p {
        Provider::OpenAi => {
            let base = openai_base()?;
            let v = send(p, access.request(p, Method::GET, "/v1/models")).await?;
            v["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    let id = m["id"].as_str()?;
                    Some(Model {
                        id: id.to_owned(),
                        live: base.is_none() && id.contains("realtime"),
                        generate: true,
                        created: m["created"].as_i64(),
                    })
                })
                .collect()
        }
        Provider::Gemini => {
            let v = send(p, access.request(p, Method::GET, "/v1beta/models?pageSize=1000")).await?;
            v["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    let methods = m["supportedGenerationMethods"].as_array()?;
                    Some(Model {
                        id: m["name"].as_str()?.trim_start_matches("models/").to_owned(),
                        live: methods.iter().any(|x| x == "bidiGenerateContent"),
                        generate: methods.iter().any(|x| x == "generateContent"),
                        created: None,
                    })
                })
                .collect()
        }
        Provider::ElevenLabs => super::elevenlabs::models(access.secret()?).await?,
        // Neither has a speech model to name: its model list only tells whether the key works.
        Provider::Xai | Provider::Groq => {
            send(p, access.request(p, Method::GET, "/v1/models")).await?;
            Vec::new()
        }
    };
    let (realtime, transcribe, speech) = classify(p, &entries);
    let fallback = p.fallback();
    let defaults = match p {
        // Flash 2.5 is the low-latency model a call needs; Scribe v1 is on every plan.
        Provider::ElevenLabs => Defaults {
            realtime: String::new(),
            transcribe: fallback.transcribe,
            speech: newest(p, &entries, &speech).unwrap_or(fallback.speech),
        },
        // Both keep a fixed list: no model to work out, so the built-in ids stay.
        Provider::Xai | Provider::Groq => fallback,
        Provider::OpenAi | Provider::Gemini => Defaults {
            realtime: newest(p, &entries, &realtime).unwrap_or(fallback.realtime),
            transcribe: newest(p, &entries, &transcribe).unwrap_or(fallback.transcribe),
            speech: newest(p, &entries, &speech).unwrap_or(fallback.speech),
        },
    };
    store.kv_set(&defaults_key(p), &serde_json::to_string(&defaults)?)?;
    Ok(json!({"models": realtime, "transcribe": transcribe, "speech": speech, "defaults": defaults}))
}

/// The provider's current model among `ids`, skipping dated snapshots and previews.
/// `OpenAI` dates every model (`created`): its latest generation is whatever came out within a
/// few months of the newest release, and in it the full model beats `mini` (`gpt-realtime-2.1`
/// over `gpt-realtime-2.1-mini`, yet `gpt-4o-mini-tts` over the older `tts-1`). Gemini's names
/// carry the version (`gemini-3.8-flash`); within one version the full, plain name wins
/// (`-flash` over `-flash-lite`, `-live` over `-live-extended-thinking`).
fn newest(p: Provider, models: &[Model], ids: &[String]) -> Option<String> {
    const GENERATION: i64 = 120 * 24 * 60 * 60;
    let lesser = |id: &str| ["mini", "lite"].iter().any(|w| id.contains(w));
    let pinned = |id: &str| {
        ["preview", "exp", "latest"].iter().any(|w| id.contains(w))
            || id
                .split('-')
                .any(|part| part.len() == 4 && part.starts_with("20") && part.bytes().all(|c| c.is_ascii_digit()))
    };
    let version = |id: &str| -> Vec<u32> {
        id.split('-')
            .find(|part| {
                part.starts_with(|c: char| c.is_ascii_digit()) && part.split('.').all(|n| n.parse::<u32>().is_ok())
            })
            .map(|v| v.split('.').filter_map(|n| n.parse().ok()).collect())
            .unwrap_or_default()
    };
    let created = |id: &str| models.iter().find(|m| m.id == id).and_then(|m| m.created).unwrap_or(0);
    let stable: Vec<&String> = ids.iter().filter(|id| !pinned(id)).collect();
    let pool = if stable.is_empty() { ids.iter().collect() } else { stable };
    match p {
        Provider::OpenAi => {
            let latest = pool.iter().map(|id| created(id)).max()?;
            pool.into_iter()
                .filter(|id| created(id) >= latest - GENERATION)
                .max_by_key(|id| (!lesser(id), created(id)))
                .cloned()
        }
        Provider::Gemini => pool
            .into_iter()
            .max_by(|a, b| (version(a), !lesser(a)).cmp(&(version(b), !lesser(b))).then_with(|| b.len().cmp(&a.len())))
            .cloned(),
        Provider::ElevenLabs => ["eleven_flash_v2_5", "eleven_turbo_v2_5", "eleven_flash_v2", "eleven_turbo_v2"]
            .iter()
            .find(|want| pool.iter().any(|id| id == *want))
            .map(|want| (*want).to_owned())
            .or_else(|| pool.first().map(|id| (*id).clone())),
        // Their lists are fixed (`classify`), so the first is the one to keep.
        Provider::Xai | Provider::Groq => pool.first().map(|id| (*id).clone()),
    }
}

struct Model {
    id: String,
    /// Holds a live conversation (`OpenAI` `realtime`, Gemini `bidiGenerateContent`).
    live: bool,
    generate: bool,
    /// When `OpenAI` released it (Unix seconds); Gemini's list has no date.
    created: Option<i64>,
}

/// Sorts a provider's model list into what each voice mode can use. Translation,
/// diarization and live-transcription models can't serve either mode.
fn classify(p: Provider, models: &[Model]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let has = |id: &str, words: &[&str]| words.iter().any(|w| id.contains(w));
    let pick = |f: &dyn Fn(&Model) -> bool| {
        let mut ids: Vec<String> = models.iter().filter(|m| f(m)).map(|m| m.id.clone()).collect();
        ids.sort();
        ids.dedup();
        ids
    };
    let realtime = pick(&|m| m.live && !has(&m.id, &["translate", "whisper", "transcribe", "tts"]));
    let (transcribe, speech) = match p {
        Provider::OpenAi => (
            pick(&|m| {
                (m.id.contains("transcribe") || m.id == "whisper-1") && !has(&m.id, &["diarize", "live", "realtime"])
            }),
            pick(&|m| m.id.contains("tts")),
        ),
        Provider::Gemini => (
            pick(&|m| {
                m.generate
                    && m.id.starts_with("gemini")
                    && m.id.contains("flash")
                    && !has(&m.id, &["tts", "live", "image", "embedding", "native-audio", "translate"])
            }),
            pick(&|m| m.generate && m.id.contains("tts")),
        ),
        Provider::ElevenLabs => (
            // Batch transcription is Scribe; the model list only covers text to speech.
            vec!["scribe_v2".to_owned(), "scribe_v1".to_owned()],
            pick(&|m| m.generate && !has(&m.id, &["music"])),
        ),
        // xAI reads replies with its one speech route; Groq hears with the Whisper models OpenBot sends.
        Provider::Xai => (Vec::new(), Vec::new(), vec!["xai-tts".to_owned()]),
        Provider::Groq => {
            (Vec::new(), vec!["whisper-large-v3-turbo".to_owned(), "whisper-large-v3".to_owned()], Vec::new())
        }
    };
    (realtime, transcribe, speech)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(p: Provider, ids: &[&str]) -> (Vec<String>, Vec<String>, Vec<String>) {
        let models: Vec<Model> = ids
            .iter()
            .map(|id| Model { id: (*id).to_owned(), live: id.contains("realtime"), generate: true, created: None })
            .collect();
        classify(p, &models)
    }

    #[test]
    fn openai_models_sort_by_mode() {
        let (realtime, transcribe, speech) = ids(
            Provider::OpenAi,
            &[
                "gpt-realtime-2.1",
                "gpt-realtime-translate",
                "gpt-realtime-whisper",
                "gpt-realtime",
                "gpt-4o-transcribe",
                "gpt-4o-transcribe-diarize",
                "gpt-live-transcribe",
                "whisper-1",
                "gpt-4o-mini-tts",
                "gpt-5",
            ],
        );
        assert_eq!(realtime, ["gpt-realtime", "gpt-realtime-2.1"]);
        assert_eq!(transcribe, ["gpt-4o-transcribe", "whisper-1"]);
        assert_eq!(speech, ["gpt-4o-mini-tts"]);
    }

    fn dated(id: &str, created: i64) -> Model {
        Model { id: id.to_owned(), live: id.contains("realtime"), generate: true, created: Some(created) }
    }

    #[test]
    fn newest_openai_models_are_the_defaults() {
        let day = 24 * 60 * 60;
        let models = [
            dated("gpt-realtime", 1_756_000_000),
            dated("gpt-realtime-2", 1_778_000_000),
            dated("gpt-realtime-2.1", 1_786_000_000),
            dated("gpt-realtime-2.1-mini", 1_786_000_000 + 5 * day),
            dated("gpt-realtime-mini-2025-12-15", 1_765_000_000),
            dated("gpt-4o-transcribe", 1_742_000_000),
            dated("gpt-4o-mini-transcribe", 1_742_000_000),
            dated("gpt-transcribe", 1_785_000_000),
            dated("whisper-1", 1_677_000_000),
            dated("tts-1-hd", 1_699_000_000),
            dated("gpt-4o-mini-tts", 1_742_000_000),
            dated("gpt-4o-mini-tts-2025-12-15", 1_765_000_000),
        ];
        let (realtime, transcribe, speech) = classify(Provider::OpenAi, &models);
        let pick = |ids: &[String]| newest(Provider::OpenAi, &models, ids);
        assert_eq!(pick(&realtime).as_deref(), Some("gpt-realtime-2.1"));
        assert_eq!(pick(&transcribe).as_deref(), Some("gpt-transcribe"));
        assert_eq!(pick(&speech).as_deref(), Some("gpt-4o-mini-tts"));
    }

    #[test]
    fn newest_gemini_models_are_the_defaults() {
        let pick = |ids: &[&str]| {
            let ids: Vec<String> = ids.iter().map(|s| (*s).to_owned()).collect();
            newest(Provider::Gemini, &[], &ids)
        };
        assert_eq!(
            pick(&[
                "gemini-3.1-flash-live-preview",
                "gemini-3.8-live",
                "gemini-3.8-live-extended-thinking",
                "gemini-2.5-flash-live"
            ])
            .as_deref(),
            Some("gemini-3.8-live")
        );
        assert_eq!(
            pick(&["gemini-3.5-flash-lite", "gemini-3.8-flash", "gemini-3.9-flash-preview", "gemini-flash-latest"])
                .as_deref(),
            Some("gemini-3.8-flash")
        );
        assert_eq!(
            pick(&["gemini-3.8-flash-lite-tts", "gemini-3.8-flash-tts", "gemini-3.1-flash-tts-preview"]).as_deref(),
            Some("gemini-3.8-flash-tts")
        );
    }

    #[test]
    fn elevenlabs_models_and_defaults() {
        let models = [
            Model { id: "eleven_flash_v2_5".into(), live: false, generate: true, created: None },
            Model { id: "eleven_multilingual_v2".into(), live: false, generate: true, created: None },
            Model { id: "eleven_v3".into(), live: false, generate: true, created: None },
            Model { id: "eleven_music".into(), live: false, generate: true, created: None },
            Model { id: "not_a_voice_model".into(), live: false, generate: false, created: None },
        ];
        let (realtime, transcribe, speech) = classify(Provider::ElevenLabs, &models);
        assert_eq!(realtime, Vec::<String>::new());
        assert_eq!(transcribe, ["scribe_v2", "scribe_v1"]);
        assert_eq!(speech, ["eleven_flash_v2_5", "eleven_multilingual_v2", "eleven_v3"]);
        assert_eq!(newest(Provider::ElevenLabs, &models, &speech).as_deref(), Some("eleven_flash_v2_5"));
        assert!(!Provider::ElevenLabs.live());
        assert!(Provider::OpenAi.live());
    }

    #[test]
    fn xai_reads_and_groq_hears() {
        assert!(Provider::Xai.speaks());
        assert!(!Provider::Xai.transcribes());
        assert!(!Provider::Xai.live());
        assert!(Provider::Groq.transcribes());
        assert!(!Provider::Groq.speaks());
        assert!(!Provider::Groq.live());
        let (realtime, transcribe, speech) = classify(Provider::Groq, &[]);
        assert!(realtime.is_empty() && speech.is_empty());
        assert_eq!(transcribe, ["whisper-large-v3-turbo", "whisper-large-v3"]);
        let (realtime, transcribe, speech) = classify(Provider::Xai, &[]);
        assert!(realtime.is_empty() && transcribe.is_empty());
        assert_eq!(speech, ["xai-tts"]);
        assert_eq!(Provider::Xai.fallback().speech, "xai-tts");
        assert_eq!(Provider::Groq.fallback().transcribe, "whisper-large-v3-turbo");
    }
}
