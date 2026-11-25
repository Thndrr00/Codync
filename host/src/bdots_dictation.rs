#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from mercury-agent 1.5.4's `mercury-voice` crate (live dictation).
//! Composes validators already ported into this host instead of duplicating them. Only the
//! `botInsights` endpoint calls this.

/// Default model per dictation engine (mercury dictation.rs).
pub fn default_model(engine: &str) -> &'static str {
    match engine.to_ascii_lowercase().as_str() {
        "openai" => "gpt-live-transcribe",
        "deepgram" => "nova-3",
        "elevenlabs" => "scribe_v2_realtime",
        _ => "default",
    }
}

/// Human label for a dictation engine (dictation.rs label).
pub fn engine_label(engine: &str) -> &'static str {
    match engine.to_ascii_lowercase().as_str() {
        "openai" => "OpenAI",
        "deepgram" => "Deepgram",
        "elevenlabs" => "ElevenLabs",
        _ => "Speech service",
    }
}

/// SF Symbol for a dictation engine.
pub fn engine_icon(engine: &str) -> &'static str {
    match engine.to_ascii_lowercase().as_str() {
        "openai" => "waveform.path.ecg",
        "deepgram" => "waveform",
        "elevenlabs" => "waveform.circle",
        _ => "mic",
    }
}

/// Semantic tone for a dictation engine.
pub fn engine_tone(engine: &str) -> &'static str {
    if matches!(engine.to_ascii_lowercase().as_str(), "openai" | "deepgram" | "elevenlabs") {
        "good"
    } else {
        "secondary"
    }
}

/// WebSocket URL for an engine, when it has a fixed one (dictation.rs OPENAI_URL).
pub fn engine_url(engine: &str) -> Option<&'static str> {
    match engine.to_ascii_lowercase().as_str() {
        "openai" => Some("wss://api.openai.com/v1/realtime?intent=transcription"),
        _ => None,
    }
}

/// Whether an engine needs a server-VAD commit after quiet.
pub fn needs_commit(engine: &str) -> bool {
    engine.eq_ignore_ascii_case("openai")
}

/// Quiet time before committing a segment, in ms (COMMIT_AFTER_QUIET_MS).
pub fn commit_after_quiet_ms() -> u32 {
    900
}

/// Speech peak threshold for PCM16 frames (SPEECH_PEAK).
pub fn speech_peak() -> u16 {
    1200
}

/// Whether a PCM16 frame counts as speech.
pub fn is_speech(frame: &[i16]) -> bool {
    frame.iter().any(|s| s.unsigned_abs() >= speech_peak())
}

/// Peak amplitude of a PCM16 frame.
pub fn frame_peak(frame: &[i16]) -> u16 {
    frame.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0)
}

/// RMS level of a frame, as a whole percent (0–100).
pub fn frame_level(frame: &[i16]) -> u32 {
    if frame.is_empty() {
        return 0;
    }
    let sum: u64 = frame.iter().map(|s| u64::from(s.unsigned_abs()) * u64::from(s.unsigned_abs())).sum();
    let rms = (sum as f64 / frame.len() as f64).sqrt();
    ((rms / 32768.0) * 100.0).round().max(0.0).min(100.0) as u32
}

/// A bar of filled cells for a frame's level.
pub fn level_bar(frame: &[i16], width: usize) -> String {
    let filled = (frame_level(frame) as usize * width) / 100;
    "▮".repeat(filled.min(width))
}

/// Human label for a dictation event kind.
pub fn event_label(partial: bool, closed: bool) -> &'static str {
    if closed {
        "Stopped"
    } else if partial {
        "Listening"
    } else {
        "Final text"
    }
}

/// SF Symbol for a dictation event.
pub fn event_icon(partial: bool, closed: bool) -> &'static str {
    if closed {
        "mic.slash"
    } else if partial {
        "mic.and.signal.meter"
    } else {
        "text.bubble"
    }
}

/// Semantic tone for a dictation event.
pub fn event_tone(partial: bool, closed: bool) -> &'static str {
    if closed {
        "secondary"
    } else if partial {
        "accent"
    } else {
        "good"
    }
}

/// Short display for a partial transcript, reusing the shared shortener.
pub fn partial_text(text: &str) -> String {
    crate::bdots_more::shorten(text.trim(), 120)
}

/// Final transcript line, trimmed.
pub fn final_text(text: &str) -> String {
    crate::bdots_more::shorten(text.trim(), 400)
}

/// Whether a language code is set and valid.
pub fn has_language(language: Option<&str>) -> bool {
    language.is_some_and(|l| l.trim().len() >= 2)
}

/// Human label for a language setting.
pub fn language_label(language: Option<&str>) -> &'static str {
    if has_language(language) { "Chosen" } else { "Auto-detect" }
}

/// Compact display for a model override.
pub fn model_label(model: Option<&str>, engine: &str) -> String {
    model.filter(|m| !m.trim().is_empty()).map_or_else(|| default_model(engine).to_owned(), str::to_owned)
}

/// Whether an engine name is one Mercury supports.
pub fn is_supported(engine: &str) -> bool {
    matches!(engine.to_ascii_lowercase().as_str(), "openai" | "deepgram" | "elevenlabs")
}

/// One plain sentence describing a dictation config.
pub fn dictation_headline(engine: &str, language: Option<&str>, model: Option<&str>) -> String {
    format!("{} · {} · {}", engine_label(engine), language_label(language), model_label(model, engine))
}

/// Semantic tone for the dictation card.
pub fn dictation_tone(engine: &str) -> &'static str {
    engine_tone(engine)
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(
    engine: &str,
    language: Option<&str>,
    model: Option<&str>,
    frames: usize,
) -> Vec<(&'static str, String)> {
    vec![
        ("engine", engine_label(engine).to_owned()),
        ("language", language_label(language).to_owned()),
        ("model", model_label(model, engine)),
        ("frames", count_label(frames)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_helpers() {
        assert_eq!(default_model("openai"), "gpt-live-transcribe");
        assert_eq!(default_model("deepgram"), "nova-3");
        assert_eq!(default_model("x"), "default");
        assert_eq!(engine_label("elevenlabs"), "ElevenLabs");
        assert_eq!(engine_icon("openai"), "waveform.path.ecg");
        assert_eq!(engine_tone("x"), "secondary");
        assert!(engine_url("openai").is_some());
        assert!(engine_url("deepgram").is_none());
        assert!(is_supported("deepgram"));
        assert!(!is_supported("x"));
    }

    #[test]
    fn voice_detection() {
        assert!(needs_commit("openai"));
        assert!(!needs_commit("deepgram"));
        assert_eq!(commit_after_quiet_ms(), 900);
        assert_eq!(speech_peak(), 1200);
        assert!(is_speech(&[0, 1300, 0]));
        assert!(!is_speech(&[0, 100, 0]));
        assert_eq!(frame_peak(&[0, 1300, 0]), 1300);
        assert_eq!(frame_level(&[]), 0);
        assert!(frame_level(&[16000, 16000]) > 0);
        assert!(!level_bar(&[], 8).is_empty());
    }

    #[test]
    fn events_and_text() {
        assert_eq!(event_label(true, false), "Listening");
        assert_eq!(event_label(false, false), "Final text");
        assert_eq!(event_label(false, true), "Stopped");
        assert_eq!(event_icon(true, false), "mic.and.signal.meter");
        assert_eq!(event_tone(false, true), "secondary");
        assert_eq!(partial_text("  hi  "), "hi");
        assert_eq!(final_text(" done "), "done");
    }

    #[test]
    fn language_models_and_summary() {
        assert!(has_language(Some("en")));
        assert!(!has_language(Some("x")));
        assert_eq!(language_label(None), "Auto-detect");
        assert_eq!(model_label(None, "openai"), "gpt-live-transcribe");
        assert_eq!(model_label(Some("custom"), "openai"), "custom");
        assert!(dictation_headline("openai", None, None).contains("OpenAI"));
        assert_eq!(dictation_tone("openai"), "good");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("openai", None, None, 10).len(), 4);
    }
}
