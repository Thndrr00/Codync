#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `context_pin.py`: explicit window pins, fallback
//! detection, display markers and local advertised-window metadata. Composes helpers already
//! ported into this host. Only `botInsights` calls this.

/// Suffix appended to a display window for each marker (MARKER_SUFFIX).
pub const MARKER_SUFFIX: [(&str, &str); 2] = [("pinned", " (pinned)"), ("fallback", " (fallback)")];

/// Known fallback window sizes from provider metadata (DEFAULT_CONTEXT_LENGTHS subset).
pub const DEFAULT_WINDOWS: [(&str, u32); 6] = [
    ("sonnet", 200_000),
    ("opus", 200_000),
    ("gpt-4o", 128_000),
    ("gpt-5", 400_000),
    ("gemini", 1_000_000),
    ("llama", 128_000),
];

/// Whether a window is an explicit pin matching the config (isContextPinned).
pub fn is_context_pinned(window: u32, pin: Option<u32>) -> bool {
    pin.is_some_and(|value| value > 0 && window == value)
}

/// Suffix for a pinned window (contextPinSuffix).
pub fn pin_suffix(window: u32, pin: Option<u32>) -> &'static str {
    if is_context_pinned(window, pin) { " (pinned)" } else { "" }
}

/// Whether a window is a known fallback default (isFallbackContextLength).
pub fn is_fallback_window(window: u32, model: &str, override_window: Option<u32>) -> bool {
    if window == 0 || model.trim().is_empty() {
        return false;
    }
    override_window == Some(window) || advertised_window(model).contains(&window)
}

/// Marker for a window: pinned, fallback or empty (contextWindowMarker).
pub fn context_marker(window: u32, pin: Option<u32>, model: &str, fallback: Option<u32>) -> &'static str {
    if is_context_pinned(window, pin) {
        "pinned"
    } else if is_fallback_window(window, model, fallback) {
        "fallback"
    } else {
        ""
    }
}

/// Suffix for a rendered window (contextWindowSuffix).
pub fn context_suffix(marker: &str) -> &'static str {
    MARKER_SUFFIX.iter().find(|(name, _)| *name == marker).map_or("", |(_, suffix)| *suffix)
}

/// Label for a marker, reusing the shared text formatter.
pub fn marker_label(marker: &str) -> String {
    if marker.is_empty() { "Detected".to_owned() } else { crate::bdots_textfmt::upper_first(marker) }
}

/// SF Symbol for a window marker.
pub fn marker_icon(marker: &str) -> &'static str {
    match marker {
        "pinned" => "pin.fill",
        "fallback" => "arrow.uturn.down.circle",
        _ => "checkmark.circle",
    }
}

/// Semantic tone for a window marker.
pub fn marker_tone(marker: &str) -> &'static str {
    match marker {
        "pinned" => "good",
        "fallback" => "attention",
        _ => "secondary",
    }
}

/// Advertised window from local metadata only (advertisedContextLength).
pub fn advertised_window(model: &str) -> Option<u32> {
    let lower = model.to_ascii_lowercase();
    DEFAULT_WINDOWS.iter().find(|(prefix, _)| lower.contains(prefix)).map(|(_, window)| *window)
}

/// Longest matching metadata key for a model, by prefix length.
pub fn longest_key(model: &str) -> Option<(&'static str, u32)> {
    let lower = model.to_ascii_lowercase();
    DEFAULT_WINDOWS
        .iter()
        .filter(|(prefix, _)| lower.contains(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(prefix, window)| (*prefix, *window))
}

/// Strip a provider prefix from a model id (_stripProviderPrefix).
pub fn strip_provider_prefix(model: &str) -> String {
    model
        .split_once('/')
        .map_or(model, |(_, rest)| rest)
        .split_once(':')
        .map_or_else(|| model.split('/').last().unwrap_or(model).to_owned(), |(rest, _)| rest.to_owned())
}

/// Whether a pin disagrees with the advertised window.
pub fn pin_disagrees(model: &str, pin: u32) -> bool {
    advertised_window(model).is_some_and(|advertised| advertised != pin)
}

/// Warning text for a disagreeing pin, reusing the shared formatter.
pub fn disagreement_warning(model: &str, pin: u32) -> String {
    if !pin_disagrees(model, pin) {
        return String::new();
    }
    crate::bdots_more::shorten(&format!("Pin {pin} for {model} differs from the advertised window"), 120)
}

/// Human label for a window size, reusing the locale formatting helper.
pub fn window_label(window: u32) -> String {
    format!("{}{}", crate::bdots_localfmt::format_tokens_short("en", f64::from(window)), " context")
}

/// Whether a window is in the known set (including pins).
pub fn is_known_window(window: u32, pin: Option<u32>) -> bool {
    is_context_pinned(window, pin) || DEFAULT_WINDOWS.iter().any(|(_, value)| *value == window)
}

/// Ratio of a window to the largest known window, as a whole percent.
pub fn window_ratio(window: u32) -> u32 {
    let max = DEFAULT_WINDOWS.iter().map(|(_, value)| *value).max().unwrap_or(1);
    crate::bdots_activity::percent(window as usize, max as usize)
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(window: u32, marker: &str, known: bool, ratio: u32) -> Vec<(&'static str, String)> {
    vec![
        ("window", window.to_string()),
        ("marker", marker_label(marker)),
        ("known", if known { "yes" } else { "no" }.to_owned()),
        ("ratio", format!("{ratio}%")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_and_fallbacks() {
        assert!(is_context_pinned(200_000, Some(200_000)));
        assert!(!is_context_pinned(200_000, Some(128_000)));
        assert_eq!(pin_suffix(200_000, Some(200_000)), " (pinned)");
        assert_eq!(pin_suffix(200_000, None), "");
        assert!(is_fallback_window(200_000, "claude-sonnet", None));
        assert!(!is_fallback_window(7, "claude-sonnet", None));
    }

    #[test]
    fn markers_and_icons() {
        assert_eq!(context_marker(200_000, Some(200_000), "x", None), "pinned");
        assert_eq!(context_marker(200_000, None, "claude-opus", None), "fallback");
        assert_eq!(context_marker(7, None, "x", None), "");
        assert_eq!(context_suffix("pinned"), " (pinned)");
        assert_eq!(context_suffix(""), "");
        assert_eq!(marker_label("pinned"), "Pinned");
        assert_eq!(marker_label(""), "Detected");
        assert_eq!(marker_icon("fallback"), "arrow.uturn.down.circle");
        assert_eq!(marker_tone("pinned"), "good");
    }

    #[test]
    fn metadata() {
        assert_eq!(advertised_window("gpt-4o"), Some(128_000));
        assert_eq!(advertised_window("unknown"), None);
        assert_eq!(longest_key("openai/gpt-5"), Some(("gpt-5", 400_000)));
        assert_eq!(strip_provider_prefix("openai/gpt-5"), "gpt-5");
        assert!(pin_disagrees("gpt-4o", 200_000));
        assert!(disagreement_warning("gpt-4o", 200_000).contains("differs"));
        assert!(disagreement_warning("unknown", 1).is_empty());
    }

    #[test]
    fn presentation() {
        assert!(window_label(200_000).contains("200K"));
        assert!(is_known_window(200_000, None));
        assert!(!is_known_window(7, None));
        assert!(window_ratio(1_000_000) > window_ratio(128_000));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(200_000, "pinned", true, 20).len(), 4);
    }
}
