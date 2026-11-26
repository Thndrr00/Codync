#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from OpenDots' shared models (learning, scheduled messages, voice
//! receipts, page review). Composes validators already ported into this host. Only `botInsights`
//! calls this.

/// Prefix marking an internal scheduled-task message (scheduled-message.ts).
pub const SCHEDULED_PREFIX: &str = "opendots:scheduled_task:";

/// Prefix marking an internal voice-receipt message (voice-receipt.ts).
pub const VOICE_PREFIX: &str = "opendots:voice_receipt:";

/// Whether a Learning container id is valid (learningContainerIdSchema).
pub fn valid_container(id: Option<&str>) -> bool {
    id.is_none_or(|s| {
        let len = s.len();
        (1..=64).contains(&len)
            && s.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
            && s.ends_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
            && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !s.contains("--")
    })
}

/// Error line for an invalid Learning container id.
pub fn container_error(id: Option<&str>) -> String {
    if valid_container(id) {
        String::new()
    } else {
        "Learning container ID must use 1–64 lowercase letters, numbers, and single hyphens.".to_owned()
    }
}

/// Whether skill delivery is configured (validateLearningSettings).
pub fn delivery_configured(container: Option<&str>, delivery: bool) -> bool {
    if !delivery {
        return true;
    }
    valid_container(container)
}

/// Error line when delivery is on but no container is set.
pub fn delivery_error(container: Option<&str>, delivery: bool) -> String {
    if delivery && !valid_container(container) {
        "Skill delivery requires a Learning container.".to_owned()
    } else {
        String::new()
    }
}

/// SF Symbol for a Learning state.
pub fn learning_icon(on: bool) -> &'static str {
    if on { "graduationcap.fill" } else { "graduationcap" }
}

/// Semantic tone for a Learning state.
pub fn learning_tone(on: bool) -> &'static str {
    if on { "good" } else { "secondary" }
}

/// Whether a message is an internal scheduled-task message (isScheduledTaskMessage).
pub fn is_scheduled(text: &str) -> bool {
    text.starts_with(SCHEDULED_PREFIX)
}

/// Human label for a scheduled task message.
pub fn scheduled_label(text: &str) -> &'static str {
    if is_scheduled(text) { "Scheduled task" } else { "Chat message" }
}

/// Task id from a scheduled message, when present.
pub fn scheduled_task_id(text: &str) -> Option<String> {
    text.strip_prefix(SCHEDULED_PREFIX).map(|rest| crate::bdots_more::shorten(rest.trim(), 64))
}

/// Whether a message is an internal voice receipt.
pub fn is_voice_receipt(text: &str) -> bool {
    text.starts_with(VOICE_PREFIX)
}

/// Human label for a voice receipt message.
pub fn voice_label(text: &str) -> &'static str {
    if is_voice_receipt(text) { "Voice receipt" } else { "Chat message" }
}

/// Receipt id from a voice receipt message.
pub fn voice_receipt_id(text: &str) -> Option<String> {
    text.strip_prefix(VOICE_PREFIX).map(|rest| crate::bdots_more::shorten(rest.trim(), 64))
}

/// Whether a message is internal (scheduled or voice receipt).
pub fn is_internal(text: &str) -> bool {
    is_scheduled(text) || is_voice_receipt(text)
}

/// SF Symbol for a message's origin.
pub fn source_icon(text: &str) -> &'static str {
    if is_scheduled(text) {
        "calendar.badge.clock"
    } else if is_voice_receipt(text) {
        "mic.badge"
    } else {
        "bubble.left"
    }
}

/// Semantic tone for a message's origin.
pub fn source_tone(text: &str) -> &'static str {
    if is_internal(text) { "accent" } else { "secondary" }
}

/// Whether a page review draft has a rating and a note (pageReviewSchema).
pub fn valid_review(rating: Option<u8>, note: &str) -> bool {
    rating.is_some_and(|r| (1..=5).contains(&r)) && !note.trim().is_empty()
}

/// Star glyphs for a review rating.
pub fn review_stars(rating: u8) -> String {
    "★".repeat(rating.min(5) as usize)
}

/// Human label for a review rating.
pub fn review_label(rating: u8) -> &'static str {
    match rating {
        5 => "Excellent",
        4 => "Good",
        3 => "Okay",
        2 => "Poor",
        1 => "Bad",
        _ => "Unrated",
    }
}

/// SF Symbol for a review.
pub fn review_icon(rating: u8) -> &'static str {
    if rating >= 4 {
        "hand.thumbsup.fill"
    } else if rating > 0 {
        "hand.thumbsdown"
    } else {
        "questionmark.circle"
    }
}

/// Semantic tone for a review.
pub fn review_tone(rating: u8) -> &'static str {
    if rating >= 4 {
        "good"
    } else if rating > 0 {
        "attention"
    } else {
        "secondary"
    }
}

/// Compact display for a review note.
pub fn review_note(note: &str) -> String {
    crate::bdots_more::shorten(note.trim(), 200)
}

/// Summary rows for the card.
pub fn summary(container: Option<&str>, delivery: bool, internal: usize) -> Vec<(&'static str, String)> {
    vec![
        ("container", container.unwrap_or("none").to_owned()),
        ("delivery", if delivery { "on" } else { "off" }.to_owned()),
        ("internal", crate::bdots_desk::compact_count(internal as u64)),
        ("learning", if valid_container(container) { "ready" } else { "not set" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_and_detection() {
        assert!(is_scheduled("opendots:scheduled_task:abc"));
        assert!(!is_scheduled("hello"));
        assert_eq!(scheduled_label("opendots:scheduled_task:x"), "Scheduled task");
        assert_eq!(scheduled_task_id("opendots:scheduled_task:abc").as_deref(), Some("abc"));
        assert!(is_voice_receipt("opendots:voice_receipt:r1"));
        assert_eq!(voice_label("opendots:voice_receipt:r1"), "Voice receipt");
        assert_eq!(voice_receipt_id("opendots:voice_receipt:r1").as_deref(), Some("r1"));
        assert!(is_internal("opendots:scheduled_task:x"));
    }

    #[test]
    fn container_validation() {
        assert!(valid_container(Some("learning-1")));
        assert!(valid_container(None));
        assert!(!valid_container(Some("Bad Name")));
        assert!(!valid_container(Some("a--b")));
        assert!(!valid_container(Some("-a")));
        assert!(container_error(Some("bad name")).contains("lowercase"));
        assert!(container_error(Some("ok")).is_empty());
    }

    #[test]
    fn delivery_and_icons() {
        assert!(delivery_configured(None, false));
        assert!(!delivery_configured(None, true));
        assert!(delivery_error(None, true).contains("Learning container"));
        assert!(delivery_error(Some("ok"), true).is_empty());
        assert_eq!(learning_icon(true), "graduationcap.fill");
        assert_eq!(learning_tone(false), "secondary");
        assert_eq!(source_icon("opendots:scheduled_task:x"), "calendar.badge.clock");
        assert_eq!(source_tone("hello"), "secondary");
    }

    #[test]
    fn reviews() {
        assert!(valid_review(Some(4), "nice"));
        assert!(!valid_review(Some(0), "nice"));
        assert!(!valid_review(Some(4), ""));
        assert_eq!(review_stars(3), "★★★");
        assert_eq!(review_label(5), "Excellent");
        assert_eq!(review_label(0), "Unrated");
        assert_eq!(review_icon(4), "hand.thumbsup.fill");
        assert_eq!(review_tone(1), "attention");
        assert_eq!(review_note("  hi  "), "hi");
    }

    #[test]
    fn summary_rows() {
        let rows = summary(Some("learning"), true, 3);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].1, "learning");
        assert_eq!(rows[3].1, "ready");
    }
}
