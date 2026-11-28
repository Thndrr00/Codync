#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/layout.ts`: island sizes, bot placement,
//! glow colors, project colors and washes. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Island modes (IslandMode).
pub const ISLAND_MODES: [&str; 3] = ["hidden", "compact", "expanded"];

/// Bot states (BotStateName).
pub const BOT_STATES: [&str; 10] =
    ["idle", "working", "thinking", "searching", "approval", "error", "sleeping", "dizzy", "love", "proud"];

/// Bot emotes (BotEmoteName).
pub const BOT_EMOTES: [&str; 7] = ["love", "surprised", "proud", "wink", "yawn", "happy", "annoyed"];

/// Agent layout modes (AgentLayoutMode).
pub const AGENT_LAYOUT_MODES: [&str; 4] = ["none", "grid", "pills", "column"];

/// Panel width (PANEL_W).
pub const PANEL_W: u64 = 720;

/// Panel height (PANEL_H).
pub const PANEL_H: u64 = 320;

/// Notch width (NOTCH_W).
pub const NOTCH_W: u64 = 184;

/// Notch height (NOTCH_H).
pub const NOTCH_H: u64 = 32;

/// Compact and expanded widths.
pub const COMPACT_W: u64 = 288;

/// Expanded width (EXPANDED_W).
pub const EXPANDED_W: u64 = 640;

/// Corner radii of hidden/compact and expanded islands.
pub const ROUNDED_CORNER: u64 = 14;

/// Expanded corner radius (EXPANDED_CORNER).
pub const EXPANDED_CORNER: u64 = 22;

/// Wake strip dimensions.
pub const WAKE_STRIP: (u64, u64) = (240, 6);

/// Question picker height (QUESTION_PICKER_H).
pub const QUESTION_PICKER_H: u64 = 200;

/// Named project colors.
pub const PROJECT_COLORS: [(&str, &str); 6] = [
    ("codync", "#7C5CFC"),
    ("agentes", "#10B981"),
    ("rakazo", "#F59E0B"),
    ("hermes", "#3B82F6"),
    ("mercury", "#EC4899"),
    ("coucou", "#06B6D4"),
];

/// Fallback colors for unnamed projects.
pub const FALLBACK_COLORS: [&str; 6] = ["#6366F1", "#8B5CF6", "#EC4899", "#F59E0B", "#10B981", "#3B82F6"];

/// Washes of the island background.
pub const WASHES: [&str; 8] = ["red", "green", "pink", "amber", "cyan", "indigo", "soft", "none"];

/// Height of the chat prompt by message count (chatPromptHeight).
pub fn chat_prompt_height(message_count: u64) -> u64 {
    (240 + message_count * 40).min(300)
}

/// Size of the island in a mode and view (islandSize).
pub fn island_size(mode: &str, view: &str, chat_count: u64) -> Value {
    match mode {
        "hidden" => json!({"w": NOTCH_W, "h": 0}),
        "compact" => json!({"w": COMPACT_W, "h": NOTCH_H}),
        "expanded" => {
            let height = if view == "chat" { chat_prompt_height(chat_count) + 80 } else { PANEL_H };
            json!({"w": EXPANDED_W, "h": height})
        }
        _ => json!({"w": 0, "h": 0}),
    }
}

/// Placement of the bot dot (botPosition).
pub fn bot_position(mode: &str, view: &str, island_h: u64, upload_progress: u64) -> Value {
    match mode {
        "hidden" => json!({"cx": 46, "cy": 16, "diameter": 6, "opacity": 0}),
        "compact" => json!({"cx": 40, "cy": 16, "diameter": 20, "opacity": 1}),
        "expanded" => {
            let diameter = if view == "chat" && upload_progress > 0 { 28 } else { 24 };
            json!({"cx": 36, "cy": (island_h / 2) as i64, "diameter": diameter, "opacity": 1})
        }
        _ => json!({"cx": 0, "cy": 0, "diameter": 0, "opacity": 0}),
    }
}

/// Glow color of a bot state (botGlowColor).
pub fn bot_glow_color(state: &str) -> &'static str {
    match state {
        "working" => "#3B9EFF",
        "thinking" => "#A78BFA",
        "searching" => "#6366F1",
        "approval" => "#F5A524",
        "error" => "#F4505E",
        "love" => "#EC4899",
        _ => "#7C5CFC",
    }
}

/// Glow opacity of a bot state (botGlowOpacity).
pub fn bot_glow_opacity(state: &str) -> f64 {
    match state {
        "idle" | "sleeping" => 0.15,
        "dizzy" => 0.0,
        _ => 0.65,
    }
}

/// Color of a project by name (colorForProject).
pub fn color_for_project(name: &str) -> &'static str {
    let key = name.trim().to_ascii_lowercase();
    if let Some((_, color)) = PROJECT_COLORS.iter().find(|(candidate, _)| *candidate == key) {
        return color;
    }
    for (prefix, color) in PROJECT_COLORS {
        if key.starts_with(prefix) || key.contains(prefix) {
            return color;
        }
    }
    let mut hash: i64 = 0;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(i64::from(byte));
    }
    FALLBACK_COLORS[(hash.unsigned_abs() as usize) % FALLBACK_COLORS.len()]
}

/// RGBA of a wash (washRGBA).
pub fn wash_rgba(wash: &str) -> &'static str {
    match wash {
        "red" => "rgba(244,80,94,0.55)",
        "green" => "rgba(52,211,153,0.5)",
        "pink" => "rgba(244,114,182,0.55)",
        "amber" => "rgba(245,165,36,0.42)",
        "cyan" => "rgba(56,189,248,0.5)",
        "indigo" => "rgba(99,102,241,0.5)",
        "soft" => "rgba(255,255,255,0.08)",
        _ => "rgba(0,0,0,0)",
    }
}

/// Corner radius of an island mode (islandCorner).
pub fn island_corner(mode: &str) -> u64 {
    if mode == "expanded" { EXPANDED_CORNER } else { ROUNDED_CORNER }
}

/// Whether a state is dimmed (isDimmed).
pub fn is_dimmed(state: &str) -> bool {
    bot_glow_opacity(state) < 0.2
}

/// One line describing an island (islandLine).
pub fn island_line(mode: &str, view: &str, state: &str) -> String {
    let size = island_size(mode, view, 0);
    format!("{mode} · {}×{} · {state}", size["w"].as_u64().unwrap_or(0), size["h"].as_u64().unwrap_or(0))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(modes: usize, states: usize, emotes: usize, expanded: bool) -> Vec<(&'static str, String)> {
    vec![
        ("modes", count_label(modes)),
        ("states", count_label(states)),
        ("emotes", count_label(emotes)),
        ("expanded", if expanded { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_sizes() {
        assert_eq!(ISLAND_MODES.len(), 3);
        assert_eq!(BOT_STATES.len(), 10);
        assert_eq!(BOT_EMOTES.len(), 7);
        assert_eq!(AGENT_LAYOUT_MODES.len(), 4);
        assert_eq!(PANEL_W, 720);
        assert_eq!(PANEL_H, 320);
        assert_eq!(NOTCH_W, 184);
        assert_eq!(COMPACT_W, 288);
        assert_eq!(EXPANDED_W, 640);
        assert_eq!(WAKE_STRIP, (240, 6));
        assert_eq!(QUESTION_PICKER_H, 200);
        assert_eq!(chat_prompt_height(0), 240);
        assert_eq!(chat_prompt_height(2), 300);
        assert_eq!(island_size("hidden", "chat", 0)["h"], 0);
        assert_eq!(island_size("compact", "chat", 0)["w"], COMPACT_W);
        assert_eq!(island_size("expanded", "chat", 1)["h"], 360);
    }

    #[test]
    fn bot_and_colors() {
        let position = bot_position("hidden", "chat", 0, 0);
        assert_eq!(position["diameter"], 6);
        assert_eq!(position["opacity"], 0);
        assert_eq!(bot_position("compact", "chat", 0, 0)["diameter"], 20);
        assert_eq!(bot_position("expanded", "chat", 40, 50)["diameter"], 28);
        assert_eq!(bot_glow_color("working"), "#3B9EFF");
        assert_eq!(bot_glow_color("error"), "#F4505E");
        assert_eq!(bot_glow_opacity("idle"), 0.15);
        assert_eq!(bot_glow_opacity("dizzy"), 0.0);
        assert!(is_dimmed("sleeping"));
        assert!(!is_dimmed("working"));
    }

    #[test]
    fn projects_and_washes() {
        assert_eq!(color_for_project("Codync"), "#7C5CFC");
        assert_eq!(color_for_project("codync-desktop"), "#7C5CFC");
        assert!(FALLBACK_COLORS.contains(&color_for_project("random")));
        assert_eq!(wash_rgba("red"), "rgba(244,80,94,0.55)");
        assert_eq!(wash_rgba("none"), "rgba(0,0,0,0)");
        assert_eq!(WASHES.len(), 8);
        assert_eq!(island_corner("expanded"), 22);
        assert_eq!(island_corner("compact"), 14);
    }

    #[test]
    fn presentation() {
        assert!(island_line("compact", "chat", "idle").contains("288"));
        assert!(island_line("hidden", "chat", "idle").contains("hidden"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 10, 7, true).len(), 4);
    }
}
