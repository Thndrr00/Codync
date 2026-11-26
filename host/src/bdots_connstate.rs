#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo v0.14.0's connection context (`selectRunConnections`,
//! `connectedContext`): choosing the connections a run may use and describing their state.
//! Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Maximum retry backoff, in milliseconds.
pub const MAX_BACKOFF_MS: u64 = 60_000;

/// Whether a connection state counts as live.
pub fn is_connected(state: &str) -> bool {
    matches!(state.to_ascii_lowercase().as_str(), "connected" | "online" | "ready" | "active")
}

/// Human label for a connection state.
pub fn state_label(state: &str) -> &'static str {
    match state.to_ascii_lowercase().as_str() {
        "connected" | "online" | "ready" | "active" => "Connected",
        "connecting" | "pending" => "Connecting",
        "disconnected" | "offline" => "Disconnected",
        "error" | "failed" => "Failed",
        _ => "Unknown",
    }
}

/// SF Symbol for a connection state.
pub fn state_icon(state: &str) -> &'static str {
    match state.to_ascii_lowercase().as_str() {
        "connected" | "online" | "ready" | "active" => "wifi",
        "connecting" | "pending" => "arrow.triangle.2.circlepath",
        "error" | "failed" => "exclamationmark.triangle.fill",
        _ => "wifi.slash",
    }
}

/// Semantic tone for a connection state.
pub fn state_tone(state: &str) -> &'static str {
    if is_connected(state) {
        "good"
    } else if matches!(state.to_ascii_lowercase().as_str(), "connecting" | "pending") {
        "accent"
    } else {
        "secondary"
    }
}

/// Whether a stamp is stale against now (a heartbeat older than `within_ms`).
pub fn is_stale(stamp_ms: i64, now_ms: i64, within_ms: i64) -> bool {
    stamp_ms <= 0 || now_ms.saturating_sub(stamp_ms) > within_ms.max(0)
}

/// Exponential retry backoff with a cap, in milliseconds.
pub fn backoff_ms(attempts: u32) -> u64 {
    let shift = attempts.min(6);
    (500u64 << shift).min(MAX_BACKOFF_MS)
}

/// Next retry timestamp after `attempts` failures.
pub fn next_retry_ms(attempts: u32, now_ms: i64) -> i64 {
    now_ms.saturating_add(backoff_ms(attempts) as i64)
}

/// Connections that belong to a bot, in their original order (selectRunConnections).
pub fn select_connections<'a>(connections: &'a [Value], bot: &str) -> Vec<&'a Value> {
    connections
        .iter()
        .filter(|c| {
            let owner = c["bot"].as_str().unwrap_or("");
            owner.is_empty() || owner == bot
        })
        .collect()
}

/// Number of live connections in a set (connectedContext).
pub fn connected_count(connections: &[Value]) -> usize {
    connections.iter().filter(|c| is_connected(c["state"].as_str().unwrap_or(""))).count()
}

/// Number of stale connections in a set.
pub fn stale_count(connections: &[Value], now_ms: i64, within_ms: i64) -> usize {
    connections.iter().filter(|c| is_stale(c["updatedAt"].as_i64().unwrap_or(0), now_ms, within_ms)).count()
}

/// Human label for a channel kind.
pub fn channel_label(kind: &str) -> &'static str {
    match kind.to_ascii_lowercase().as_str() {
        "desktop" => "Desktop",
        "ios" => "iPhone",
        "android" => "Android",
        "web" => "Web",
        "cli" => "Terminal",
        _ => "Other",
    }
}

/// Whether a connection row is enabled for a run.
pub fn is_channel_enabled(row: &Value) -> bool {
    row["enabled"] != false && !matches!(row["state"].as_str().unwrap_or(""), "disabled" | "blocked")
}

/// Summary rows for the card.
pub fn summary(connected: usize, total: usize, stale: usize, backoff: u64) -> Vec<(&'static str, String)> {
    vec![
        ("connected", crate::bdots_desk::compact_count(connected as u64)),
        ("total", crate::bdots_desk::compact_count(total as u64)),
        ("stale", crate::bdots_desk::compact_count(stale as u64)),
        ("backoff", format!("{backoff} ms")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn connections() -> Vec<Value> {
        vec![
            json!({"bot":"a","state":"connected","updatedAt":900,"enabled":true}),
            json!({"bot":"b","state":"offline","updatedAt":100,"enabled":true}),
            json!({"bot":"a","state":"connected","updatedAt":100,"enabled":false}),
            json!({"bot":"a","state":"connected","updatedAt":950,"enabled":true}),
        ]
    }

    #[test]
    fn states() {
        assert!(is_connected("Connected"));
        assert!(!is_connected("offline"));
        assert_eq!(state_label("connecting"), "Connecting");
        assert_eq!(state_icon("error"), "exclamationmark.triangle.fill");
        assert_eq!(state_tone("ready"), "good");
        assert_eq!(state_label("weird"), "Unknown");
    }

    #[test]
    fn freshness_and_backoff() {
        assert!(!is_stale(900, 1000, 500));
        assert!(is_stale(100, 1000, 500));
        assert!(is_stale(0, 1000, 500));
        assert_eq!(backoff_ms(0), 500);
        assert_eq!(backoff_ms(10), MAX_BACKOFF_MS);
        assert_eq!(next_retry_ms(2, 1000), 3000);
    }

    #[test]
    fn selecting_and_counting() {
        let all = connections();
        let mine = select_connections(&all, "a");
        assert_eq!(mine.len(), 3);
        assert_eq!(connected_count(&all), 3);
        assert_eq!(stale_count(&all, 1000, 500), 2);
        assert!(is_channel_enabled(&all[0]));
        assert!(!is_channel_enabled(&all[2]));
    }

    #[test]
    fn channels_and_summary() {
        assert_eq!(channel_label("ios"), "iPhone");
        assert_eq!(channel_label("x"), "Other");
        assert_eq!(crate::bdots_desk::compact_count(1500), "1.5K");
        assert_eq!(summary(2, 4, 1, 500).len(), 4);
    }
}
