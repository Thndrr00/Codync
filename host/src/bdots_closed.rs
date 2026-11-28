#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/closed.ts`: the small island that stays at
//! the top of the screen once everything is folded — Mochi, time, date, plan usage or music.
//! Only `botInsights` calls this.

use serde_json::{Value, json};

/// Styles of the closed island (ClosedIsland).
pub const CLOSED_ISLANDS: [&str; 7] = ["mochi", "time", "datetime", "claude", "music", "empty", "hidden"];

/// Height of the closed island (CLOSED_H).
pub const CLOSED_H: u64 = 24;

/// Minimum width (CLOSED_MIN_W).
pub const CLOSED_MIN_W: u64 = 80;

/// Diameter of the resting Mochi (CLOSED_BOT_D).
pub const CLOSED_BOT_D: u64 = 18;

/// Maximum width, so a long song title can't stretch across the screen (CLOSED_MAX_W).
pub const CLOSED_MAX_W: u64 = 280;

/// Room on each side of the text (CLOSED_PAD).
pub const CLOSED_PAD: u64 = 16;

/// A value written by a newer build falls back to Mochi (closedIsland).
pub fn closed_island(value: Option<&str>) -> &'static str {
    value.filter(|style| CLOSED_ISLANDS.contains(style)).unwrap_or("mochi")
}

/// Whether a style shows nothing at all.
pub fn is_blank(style: &str) -> bool {
    matches!(style, "hidden" | "empty" | "mochi")
}

/// A plain text line with no dot and no note (plain in closedContent).
pub fn plain(text: &str) -> Value {
    json!({"text": text, "dot": Value::Null, "note": false})
}

/// The clock line (time).
pub fn time_label(time: &str) -> String {
    time.to_owned()
}

/// "Weekday, day month" plus the clock (datetime).
pub fn datetime_label(date: &str, time: &str) -> String {
    format!("{date}  {time}")
}

/// Claude plan usage line: a percentage and the gauge color (claude).
pub fn claude_content(percent: Option<f64>, time: &str) -> Value {
    match percent {
        Some(percent) => json!({
            "text": format!("Claude {}%", percent.round() as i64),
            "dot": plan_color(percent),
            "note": false,
        }),
        None => plain(time),
    }
}

/// The gauge color of a plan percentage (planColor).
pub fn plan_color(percent: f64) -> &'static str {
    if percent < 25.0 {
        "#F4505E"
    } else if percent < 50.0 {
        "#F5A524"
    } else if percent < 75.0 {
        "#7C5CFC"
    } else {
        "#10B981"
    }
}

/// The biggest remaining limit of a plan (dominantPct).
pub fn dominant_pct(session: Option<f64>, week: Option<f64>) -> Option<f64> {
    match (session, week) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// The playing item's line (music).
pub fn music_content(title: Option<&str>, artist: Option<&str>, time: &str) -> Value {
    match title.filter(|title| !title.is_empty()) {
        None => plain(time),
        Some(title) => {
            let text = artist
                .filter(|artist| !artist.is_empty())
                .map_or_else(|| title.to_owned(), |artist| format!("{title} — {artist}"));
            json!({"text": text, "dot": Value::Null, "note": true})
        }
    }
}

/// What the closed island shows, or null when it shows nothing (closedContent).
pub fn closed_content(
    style: &str,
    time: &str,
    date: &str,
    percent: Option<f64>,
    title: Option<&str>,
    artist: Option<&str>,
) -> Value {
    match style {
        "time" => plain(&time_label(time)),
        "datetime" => plain(&datetime_label(date, time)),
        "claude" => claude_content(percent, time),
        "music" => music_content(title, artist, time),
        _ => Value::Null,
    }
}

/// Text of a content value, or an empty string when there is none.
pub fn content_text(content: &Value) -> &str {
    content["text"].as_str().unwrap_or("")
}

/// Whether a content value has a note before its text.
pub fn has_note(content: &Value) -> bool {
    content["note"] == true
}

/// Dot of a content value, when it has one.
pub fn content_dot(content: &Value) -> Option<&str> {
    content["dot"].as_str()
}

/// Width of the closed island for text of a measured width (closedWidth).
pub fn closed_width(text_width: u64, extras: u64) -> u64 {
    if text_width == 0 {
        return CLOSED_MIN_W;
    }
    text_width.saturating_add(extras).saturating_add(CLOSED_PAD.saturating_mul(2)).max(CLOSED_MIN_W).min(CLOSED_MAX_W)
}

/// Width with no extra items.
pub fn width_for(text_width: u64) -> u64 {
    closed_width(text_width, 0)
}

/// Whether a width is at the maximum and the text must be cut.
pub fn is_cut(width: u64, text_width: u64, extras: u64) -> bool {
    closed_width(text_width, extras) >= CLOSED_MAX_W && text_width + extras + CLOSED_PAD * 2 > CLOSED_MAX_W
}

/// Center of a dot inside the closed island.
pub fn dot_center(width: u64) -> f64 {
    f64::from((width as u32) / 2)
}

/// One line describing the closed island (closedLine).
pub fn closed_line(style: &str, width: u64) -> String {
    format!("{style} · {CLOSED_H}px · {width}px")
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(styles: usize, width: u64, height: u64, blank: bool) -> Vec<(&'static str, String)> {
    vec![
        ("styles", count_label(styles)),
        ("width", format!("{width}px")),
        ("height", format!("{height}px")),
        ("blank", if blank { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_styles() {
        assert_eq!(CLOSED_ISLANDS.len(), 7);
        assert_eq!(CLOSED_H, 24);
        assert_eq!(CLOSED_MIN_W, 80);
        assert_eq!(CLOSED_BOT_D, 18);
        assert_eq!(CLOSED_MAX_W, 280);
        assert_eq!(CLOSED_PAD, 16);
        assert_eq!(closed_island(Some("music")), "music");
        assert_eq!(closed_island(Some("future")), "mochi");
        assert_eq!(closed_island(None), "mochi");
        assert!(is_blank("hidden"));
        assert!(is_blank("empty"));
        assert!(!is_blank("time"));
    }

    #[test]
    fn content() {
        assert_eq!(time_label("10:42"), "10:42");
        assert_eq!(datetime_label("Fri, 27 Nov", "10:42"), "Fri, 27 Nov  10:42");
        assert_eq!(content_text(&plain("x")), "x");
        assert!(!has_note(&plain("x")));
        assert_eq!(content_dot(&plain("x")), None);
        let clock = closed_content("time", "10:42", "Fri, 27 Nov", None, None, None);
        assert_eq!(content_text(&clock), "10:42");
        let date = closed_content("datetime", "10:42", "Fri, 27 Nov", None, None, None);
        assert!(content_text(&date).contains("Fri, 27 Nov"));
        assert_eq!(closed_content("mochi", "10:42", "Fri, 27 Nov", None, None, None), Value::Null);
        assert_eq!(closed_content("hidden", "10:42", "Fri, 27 Nov", None, None, None), Value::Null);
    }

    #[test]
    fn plan_and_music() {
        assert_eq!(dominant_pct(Some(40.0), Some(70.0)), Some(40.0));
        assert_eq!(dominant_pct(None, Some(70.0)), Some(70.0));
        assert_eq!(dominant_pct(None, None), None);
        assert_eq!(plan_color(10.0), "#F4505E");
        assert_eq!(plan_color(40.0), "#F5A524");
        assert_eq!(plan_color(60.0), "#7C5CFC");
        assert_eq!(plan_color(90.0), "#10B981");
        let claude = claude_content(Some(43.4), "10:42");
        assert_eq!(content_text(&claude), "Claude 43%");
        assert_eq!(content_dot(&claude), Some("#F5A524"));
        assert_eq!(content_text(&claude_content(None, "10:42")), "10:42");
        let music = music_content(Some("Song"), Some("Band"), "10:42");
        assert_eq!(content_text(&music), "Song — Band");
        assert!(has_note(&music));
        assert_eq!(content_text(&music_content(None, None, "10:42")), "10:42");
    }

    #[test]
    fn widths_and_presentation() {
        assert_eq!(closed_width(0, 0), CLOSED_MIN_W);
        assert_eq!(closed_width(20, 0), CLOSED_MIN_W);
        assert_eq!(closed_width(200, 20), 252);
        assert_eq!(closed_width(200, 0), 232);
        assert_eq!(closed_width(400, 0), CLOSED_MAX_W);
        assert_eq!(width_for(200), 232);
        assert!(is_cut(400, 400, 0));
        assert!(!is_cut(232, 200, 0));
        assert_eq!(dot_center(80), 40.0);
        assert!(closed_line("music", 232).contains("24px"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(7, 232, 24, false).len(), 4);
    }
}
