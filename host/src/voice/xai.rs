//! xAI text to speech, ported from OpenBot 0.65.0 (`src/main/voice-cloud-speech.ts`):
//! `POST https://api.x.ai/v1/tts` with `Authorization: Bearer <key>` and
//! `{text, voice_id, language, output_format: {codec: "pcm", sample_rate: 24000}}`.
//! The answer is 24 kHz 16-bit mono PCM, raw or as JSON `{"audio": "<base64>"}`.

use super::keys::Access;
use super::{Provider, STANDARD, TIMEOUT, provider_error};
use anyhow::{Result, anyhow, bail};
use base64::Engine as _;
use reqwest::Method;
use serde_json::{Value, json};

/// xAI names a few languages with a region (`ar-SA`, `es-ES`, `pt-BR`) and finds the rest itself.
/// A reply's language isn't known here, so every request lets xAI find it, as OpenBot does when its
/// speech language is automatic.
const LANGUAGE: &str = "auto";

/// The request body of xAI text to speech (OpenBot's `XaiSpeechBody`).
pub(super) fn body(text: &str, voice: &str) -> Value {
    json!({
        "text": text,
        "voice_id": voice,
        "language": LANGUAGE,
        "output_format": {"codec": "pcm", "sample_rate": 24_000},
    })
}

/// Reads `text` in `voice` as 24 kHz 16-bit mono PCM.
pub(super) async fn speak(access: &Access, voice: &str, text: &str) -> Result<Vec<u8>> {
    let req = access
        .request(Provider::Xai, Method::POST, "/v1/tts")
        .header("Accept", "*/*")
        .json(&body(text, voice))
        .timeout(TIMEOUT * 4);
    let res = req.send().await.map_err(|_| anyhow!("Can't reach {}.", Provider::Xai.name()))?;
    let status = res.status();
    let json_answer = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("json"));
    let bytes = res.bytes().await.map_err(|_| anyhow!("{} sent no audio.", Provider::Xai.name()))?;
    if !status.is_success() {
        let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        return Err(provider_error(Provider::Xai, status, &v));
    }
    let pcm = if json_answer {
        let v: Value =
            serde_json::from_slice(&bytes).map_err(|_| anyhow!("{} sent no audio.", Provider::Xai.name()))?;
        let audio = v["audio"].as_str().ok_or_else(|| anyhow!("{} sent no audio.", Provider::Xai.name()))?;
        STANDARD.decode(audio).map_err(|_| anyhow!("{} sent unreadable audio.", Provider::Xai.name()))?
    } else {
        bytes.to_vec()
    };
    // A 16-bit sample is two bytes; an odd last byte is not a sample.
    let pcm = &pcm[..pcm.len() & !1];
    if pcm.len() < 2 {
        bail!("{} sent no audio.", Provider::Xai.name());
    }
    Ok(pcm.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_matches_openbots_xai_speech_body() {
        let body = body("Hello there.", "eve");
        assert_eq!(body["text"], "Hello there.");
        assert_eq!(body["voice_id"], "eve");
        assert_eq!(body["language"], "auto");
        assert_eq!(body["output_format"]["codec"], "pcm");
        assert_eq!(body["output_format"]["sample_rate"], 24_000);
        // Only these four fields go to xAI: a model name is not part of its speech route.
        assert_eq!(body.as_object().map(serde_json::Map::len), Some(4));
    }
}
