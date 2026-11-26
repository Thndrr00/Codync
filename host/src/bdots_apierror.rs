#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `api_error_summary.py`: provider stream-parse and
//! network-resolution detection, entitlement failures, structured-detail coercion and log-safe
//! redaction. Composes helpers already ported into this host. Only `botInsights` calls this.

/// Serde-style markers of a truncated provider SSE frame (PROVIDER_STREAM_PARSE_MARKERS).
pub const STREAM_PARSE_MARKERS: [&str; 14] = [
    "expected ident at line",
    "expected value at line",
    "eof while parsing a value at line",
    "eof while parsing a string at line",
    "eof while parsing a list at line",
    "eof while parsing an object at line",
    "key must be a string at line",
    "trailing characters at line",
    "trailing comma at line",
    "expected `,` or `}` at line",
    "expected `,` or `]` at line",
    "expected `:` at line",
    "invalid escape at line",
    "invalid number at line",
];

/// Markers of an offline DNS failure wrapped as a generic connection error.
pub const NETWORK_MARKERS: [&str; 6] = [
    "temporary failure in name resolution",
    "name or service not known",
    "nodename nor servname provided, or not known",
    "getaddrinfo failed",
    "no address associated with hostname",
    "network is unreachable",
];

/// Keys tried when coercing a structured error's detail (_ERROR_DETAIL_KEYS).
pub const DETAIL_KEYS: [&str; 5] = ["message", "detail", "error", "code", "type"];

/// Hint shown for an xAI entitlement failure.
pub fn xai_entitlement_hint() -> &'static str {
    "xAI rejected this OAuth account. SuperGrok is required for this provider; use /model to switch."
}

/// Whether an error text is a provider stream-parse failure, not local validation.
pub fn is_stream_parse_error(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    STREAM_PARSE_MARKERS.iter().any(|marker| lower.contains(marker))
}

/// Whether an error text is an offline DNS/network resolution failure.
pub fn is_network_error(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    NETWORK_MARKERS.iter().any(|marker| lower.contains(marker))
}

/// xAI's permission-denied body for an unsubscribed or under-tiered account.
pub fn is_xai_entitlement(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("do not have an active grok subscription")
        || (lower.contains("out of available resources") && lower.contains("grok"))
        || (lower.contains("does not have permission") && lower.contains("grok"))
}

/// HTTP prefix for an error with a status code (_httpPrefix).
pub fn http_prefix(status: Option<u16>) -> String {
    status.map_or_else(String::new, |code| format!("HTTP {code}: "))
}

/// First string detail from a structured error payload.
pub fn detail_text(value: &Value) -> String {
    for key in DETAIL_KEYS {
        if let Some(text) = value[key].as_str() {
            if !text.trim().is_empty() {
                return text.to_owned();
            }
        }
    }
    if let Some(nested) = value.get("error").filter(|v| v.is_object()) {
        return detail_text(nested);
    }
    String::new()
}

/// User-facing summary of a structured error (summarizeApiError).
pub fn error_summary(value: &Value, status: Option<u16>) -> String {
    let detail = detail_text(value);
    format!("{}{}", http_prefix(status), crate::bdots_more::shorten(detail.trim(), 200))
}

/// Whether an error is an authentication/entitlement failure.
pub fn is_auth_error(status: Option<u16>, text: &str) -> bool {
    matches!(status, Some(401) | Some(403))
        || is_xai_entitlement(text)
        || text.to_ascii_lowercase().contains("invalid api key")
}

/// Whether an error is a rate limit.
pub fn is_rate_limit(status: Option<u16>, text: &str) -> bool {
    status == Some(429) || text.to_ascii_lowercase().contains("rate limit")
}

/// Whether an error is a context-length overflow.
pub fn is_context_length(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("context length") || lower.contains("maximum context") || lower.contains("too many tokens")
}

/// Whether a model id was not found.
pub fn is_model_not_found(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("model not found") || lower.contains("unknown model") || lower.contains("does not exist")
}

/// Whether a status is a server error.
pub fn is_server_error(status: Option<u16>) -> bool {
    status.is_some_and(|code| (500..600).contains(&code))
}

/// Main class of an error, for presentation.
pub fn error_class(status: Option<u16>, text: &str) -> &'static str {
    if is_auth_error(status, text) {
        "auth"
    } else if is_rate_limit(status, text) {
        "rate"
    } else if is_context_length(text) {
        "context"
    } else if is_model_not_found(text) {
        "model"
    } else if is_stream_parse_error(text) || is_network_error(text) || is_server_error(status) {
        "transient"
    } else {
        "other"
    }
}

/// Human label for an error class.
pub fn class_label(class: &str) -> &'static str {
    match class {
        "auth" => "Access",
        "rate" => "Rate limit",
        "context" => "Context full",
        "model" => "Model",
        "transient" => "Temporary",
        _ => "Error",
    }
}

/// SF Symbol for an error class.
pub fn class_icon(class: &str) -> &'static str {
    match class {
        "auth" => "lock.circle.fill",
        "rate" => "timer.circle.fill",
        "context" => "text.badge.xmark",
        "model" => "cpu.circle.fill",
        "transient" => "arrow.triangle.2.circlepath.circle.fill",
        _ => "exclamationmark.circle.fill",
    }
}

/// Semantic tone for an error class.
pub fn class_tone(class: &str) -> &'static str {
    match class {
        "auth" | "context" => "danger",
        "rate" | "model" => "attention",
        "transient" => "accent",
        _ => "secondary",
    }
}

/// Whether a provider error is worth retrying.
pub fn is_retryable(status: Option<u16>, text: &str) -> bool {
    matches!(error_class(status, text), "rate" | "transient")
}

/// Log-safe error text: keys removed and length bounded.
pub fn redact_error(text: &str) -> String {
    crate::bdots_more::shorten(crate::bdots_mailtext::redact_keys(text).trim(), 200)
}

/// Status label for the card.
pub fn status_label(status: Option<u16>) -> String {
    status.map_or_else(|| "No status".to_owned(), |code| format!("HTTP {code}"))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(status: Option<u16>, class: &str, retryable: bool, samples: usize) -> Vec<(&'static str, String)> {
    vec![
        ("status", status_label(status)),
        ("class", class_label(class).to_owned()),
        ("retry", if retryable { "yes" } else { "no" }.to_owned()),
        ("samples", count_label(samples)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stream_and_network() {
        assert!(is_stream_parse_error("Expected value at line 1 column 2"));
        assert!(!is_stream_parse_error("plain error"));
        assert!(is_network_error("getaddrinfo failed"));
        assert!(!is_network_error("HTTP 500"));
        assert_eq!(STREAM_PARSE_MARKERS.len(), 14);
        assert_eq!(NETWORK_MARKERS.len(), 6);
    }

    #[test]
    fn entitlement_and_details() {
        assert!(is_xai_entitlement("You do not have an active Grok subscription"));
        assert!(!is_xai_entitlement("all good"));
        assert!(xai_entitlement_hint().contains("SuperGrok"));
        assert_eq!(http_prefix(Some(429)), "HTTP 429: ");
        assert_eq!(http_prefix(None), "");
        assert_eq!(detail_text(&json!({"message":"bad","code":"x"})), "bad");
        assert_eq!(detail_text(&json!({"error":{"detail":"nested"}})), "nested");
        assert!(error_summary(&json!({"message":"oops"}), Some(400)).starts_with("HTTP 400"));
    }

    #[test]
    fn classification() {
        assert!(is_auth_error(Some(401), ""));
        assert!(is_rate_limit(Some(429), ""));
        assert!(is_context_length("maximum context length exceeded"));
        assert!(is_model_not_found("unknown model x"));
        assert!(is_server_error(Some(503)));
        assert_eq!(error_class(None, "invalid api key"), "auth");
        assert_eq!(error_class(Some(429), ""), "rate");
        assert_eq!(error_class(None, "context length"), "context");
        assert_eq!(error_class(None, "model not found"), "model");
        assert_eq!(error_class(None, "getaddrinfo failed"), "transient");
        assert_eq!(error_class(None, "something"), "other");
    }

    #[test]
    fn presentation() {
        assert_eq!(class_label("rate"), "Rate limit");
        assert_eq!(class_icon("auth"), "lock.circle.fill");
        assert_eq!(class_tone("context"), "danger");
        assert!(is_retryable(Some(429), ""));
        assert!(!is_retryable(Some(401), ""));
        assert_eq!(redact_error("hello"), "hello");
        assert_eq!(status_label(None), "No status");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(Some(500), "transient", true, 2).len(), 4);
    }
}
