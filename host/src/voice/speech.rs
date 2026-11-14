//! Reading a reply aloud at the user's provider: OpenAI, Gemini, ElevenLabs or xAI, each in its own
//! answer shape, returned as base64 audio with the format the clients play.

use super::keys::Access;
use super::{Kind, Provider, STANDARD, TIMEOUT, checked, elevenlabs, model_for, send, xai};
use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use base64::Engine as _;
use reqwest::Method;
use serde_json::{Value, json};

/// Text to speech for one stretch of a reply: `{audio: base64, format: "aac" | "wav" | "mp3"}`.
pub async fn speak(store: &Store, p: Provider, model: &str, voice: &str, text: &str) -> Result<Value> {
    if !p.speaks() {
        bail!(
            "{} hears speech but doesn't read replies. Pick OpenAI, Gemini, ElevenLabs or xAI for spoken replies.",
            p.name()
        );
    }
    let access = super::keys::access(store, p)?;
    let model = model_for(store, p, Kind::Speech, model)?;
    let (model, voice) = (model.as_str(), checked(voice)?);
    if text.trim().is_empty() || text.len() > 4000 {
        bail!("text must be 1–4000 bytes");
    }
    // An `async` block, so a `?` inside an arm is logged like a provider's refusal.
    let result: Result<Value> = async {
        match p {
            Provider::OpenAi => {
                // A local server answers WAV (no AAC encoder needed); the phone plays both.
                let format = if access.is_local() { "wav" } else { "aac" };
                let body = json!({"model": model, "voice": voice, "input": text, "response_format": format});
                let res = access
                    .request(p, Method::POST, "/v1/audio/speech")
                    .json(&body)
                    .timeout(TIMEOUT * 4)
                    .send()
                    .await
                    .map_err(|_| anyhow!("Can't reach {}.", p.name()))?;
                let status = res.status();
                let bytes = res.bytes().await.map_err(|_| anyhow!("{} sent no audio.", p.name()))?;
                if !status.is_success() {
                    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                    Err(super::provider_error(p, status, &v))
                } else {
                    Ok(json!({"audio": STANDARD.encode(&bytes), "format": format}))
                }
            }
            Provider::Gemini => {
                let body = json!({
                    "contents": [{"parts": [{"text": text}]}],
                    "generationConfig": {
                        "responseModalities": ["AUDIO"],
                        "speechConfig": {"voiceConfig": {"prebuiltVoiceConfig": {"voiceName": voice}}},
                    },
                });
                let v = send(p, gemini_generate(&access, model).json(&body)).await?;
                let pcm = v["candidates"][0]["content"]["parts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find_map(|part| part["inlineData"]["data"].as_str())
                    .ok_or_else(|| anyhow!("{} sent no audio.", p.name()))?;
                let pcm = STANDARD.decode(pcm).map_err(|_| anyhow!("{} sent unreadable audio.", p.name()))?;
                Ok(json!({"audio": STANDARD.encode(wav(&pcm, 24_000)), "format": "wav"}))
            }
            Provider::ElevenLabs => {
                let bytes = elevenlabs::speech(access.secret()?, model, voice, text).await?;
                Ok(json!({"audio": STANDARD.encode(bytes), "format": "mp3"}))
            }
            Provider::Xai => {
                let pcm = xai::speak(&access, voice, text).await?;
                Ok(json!({"audio": STANDARD.encode(wav(&pcm, 24_000)), "format": "wav"}))
            }
            Provider::Groq => unreachable!("checked above"),
        }
    }
    .await;
    if let Err(error) = &result {
        super::error_log::record(super::error_log::Failure {
            operation: "speak",
            provider: p,
            model,
            turn: None,
            reason: &format!("{error:#}"),
        })
        .await;
    }
    result
}

pub(super) fn gemini_generate(access: &Access, model: &str) -> reqwest::RequestBuilder {
    access
        .request(Provider::Gemini, Method::POST, &format!("/v1beta/models/{model}:generateContent"))
        .timeout(TIMEOUT * 4)
}

/// 16-bit mono PCM wrapped in a WAV header (Gemini's and xAI's speech are raw PCM).
fn wav(pcm: &[u8], rate: u32) -> Vec<u8> {
    let len = u32::try_from(pcm.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(pcm.len() + 44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_pcm() {
        let out = wav(&[0, 0, 1, 0], 24_000);
        assert_eq!(out.len(), 48);
        assert_eq!(&out[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([out[40], out[41], out[42], out[43]]), 4);
    }
}
