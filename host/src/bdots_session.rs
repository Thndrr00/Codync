#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's session and sync state: its live session id, revision and read mark.
//! Reuses `bdots_text`, `bdots_more` and `bdots_activity`. Only `botInsights` calls this.

/// One sync state reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncLine {
    pub id: &'static str,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub ok: bool,
}

fn line(id: &'static str, label: &str, detail: String, icon: &'static str, tone: &'static str, ok: bool) -> SyncLine {
    SyncLine { id, label: label.to_owned(), detail, icon, tone, ok }
}

/// Whether a bot has a live session.
pub fn has_session(session: Option<&str>) -> bool {
    session.is_some_and(|s| !s.trim().is_empty())
}

/// Short display for a session id, reusing the shared shortener.
pub fn session_label(session: Option<&str>) -> String {
    if has_session(session) {
        crate::bdots_more::shorten(session.unwrap_or(""), 24)
    } else {
        "No live session".to_owned()
    }
}

/// SF Symbol for a session state.
pub fn session_icon(session: Option<&str>) -> &'static str {
    if has_session(session) { "bolt.circle.fill" } else { "moon.circle" }
}

/// Semantic tone for a session state.
pub fn session_tone(session: Option<&str>) -> &'static str {
    if has_session(session) { "good" } else { "secondary" }
}

/// Number of revisions the reader is behind.
pub fn unread_revisions(rev: i64, read_rev: i64) -> i64 {
    (rev - read_rev).max(0)
}

/// Whether the reader is up to date.
pub fn is_caught_up(rev: i64, read_rev: i64) -> bool {
    unread_revisions(rev, read_rev) == 0
}

/// SF Symbol for the read mark.
pub fn read_icon(rev: i64, read_rev: i64) -> &'static str {
    if is_caught_up(rev, read_rev) { "checkmark.circle.fill" } else { "envelope.badge.fill" }
}

/// Semantic tone for the read mark.
pub fn read_tone(rev: i64, read_rev: i64) -> &'static str {
    if is_caught_up(rev, read_rev) { "good" } else { "attention" }
}

/// Compact human count of unread revisions.
pub fn unread_label(rev: i64, read_rev: i64) -> String {
    let n = unread_revisions(rev, read_rev);
    if n == 0 { "All caught up".to_owned() } else { crate::bdots_desk::compact_count(n as u64) }
}

/// A bar of filled cells for the read mark against the revision.
pub fn read_bar(rev: i64, read_rev: i64, width: usize) -> String {
    if rev <= 0 {
        return String::new();
    }
    let read = read_rev.clamp(0, rev) as usize;
    let total = rev as usize;
    "▮".repeat(crate::bdots_activity::bar_width(read, total, width))
}

/// Whether the bot is deleted.
pub fn is_deleted(deleted: bool) -> bool {
    deleted
}

/// SF Symbol for the deleted state.
pub fn deleted_icon(deleted: bool) -> &'static str {
    if deleted { "trash.circle.fill" } else { "circle.slash" }
}

/// Semantic tone for the deleted state.
pub fn deleted_tone(deleted: bool) -> &'static str {
    if deleted { "danger" } else { "good" }
}

/// 0–100 sync score: caught up and not deleted is best.
pub fn sync_score(rev: i64, read_rev: i64, deleted: bool) -> u32 {
    let mut score = if is_caught_up(rev, read_rev) { 60 } else { 30 };
    if !deleted {
        score += 40;
    }
    score
}

/// Semantic tone for a sync score.
pub fn sync_tone(score: u32) -> &'static str {
    if score >= 80 {
        "good"
    } else if score >= 40 {
        "attention"
    } else {
        "danger"
    }
}

/// Short label for a sync score.
pub fn sync_label(score: u32) -> &'static str {
    if score >= 80 {
        "In sync"
    } else if score >= 40 {
        "Catching up"
    } else {
        "Archived"
    }
}

/// One plain sentence describing the sync state.
pub fn session_headline(rev: i64, read_rev: i64, deleted: bool) -> String {
    let score = sync_score(rev, read_rev, deleted);
    if deleted {
        return "This bot is archived.".to_owned();
    }
    format!("{} · {}", sync_label(score), unread_label(rev, read_rev))
}

/// Compact display for a revision number.
pub fn revision_label(rev: i64) -> String {
    if rev <= 0 { "0".to_owned() } else { crate::bdots_desk::compact_count(rev as u64) }
}

/// Build every sync line for one bot.
pub fn build_lines(session: Option<&str>, rev: i64, read_rev: i64, deleted: bool) -> Vec<SyncLine> {
    vec![
        line(
            "session",
            "Session",
            session_label(session),
            session_icon(session),
            session_tone(session),
            has_session(session),
        ),
        line(
            "read",
            "Read mark",
            unread_label(rev, read_rev),
            read_icon(rev, read_rev),
            read_tone(rev, read_rev),
            is_caught_up(rev, read_rev),
        ),
        line(
            "archive",
            "Archive",
            if deleted { "Archived bot" } else { "Active bot" }.into(),
            deleted_icon(deleted),
            deleted_tone(deleted),
            !deleted,
        ),
    ]
}

/// SF Symbol for an ok state.
pub fn state_icon(ok: bool) -> &'static str {
    if ok { "checkmark.circle.fill" } else { "exclamationmark.circle" }
}

/// Semantic tone for an ok state.
pub fn state_tone(ok: bool) -> &'static str {
    if ok { "good" } else { "attention" }
}

/// Summary rows for the card.
pub fn summary(session: Option<&str>, rev: i64, read_rev: i64, deleted: bool) -> Vec<(&'static str, String)> {
    vec![
        ("revision", revision_label(rev)),
        ("unread", unread_label(rev, read_rev)),
        ("session", if has_session(session) { "Live" } else { "Idle" }.to_owned()),
        ("headline", session_headline(rev, read_rev, deleted)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_detectors() {
        assert!(has_session(Some("s1")));
        assert!(!has_session(Some("  ")));
        assert_eq!(session_label(None), "No live session");
        assert_eq!(session_icon(Some("s")), "bolt.circle.fill");
        assert_eq!(session_tone(None), "secondary");
    }

    #[test]
    fn revisions_and_unread() {
        assert_eq!(unread_revisions(10, 7), 3);
        assert_eq!(unread_revisions(7, 10), 0);
        assert!(is_caught_up(5, 5));
        assert!(!is_caught_up(5, 4));
        assert_eq!(read_icon(5, 5), "checkmark.circle.fill");
        assert_eq!(read_tone(5, 4), "attention");
        assert_eq!(unread_label(5, 5), "All caught up");
        assert_eq!(revision_label(-1), "0");
        assert_eq!(revision_label(1500), "1.5K");
    }

    #[test]
    fn read_bar_and_archive() {
        assert!(!read_bar(10, 5, 8).is_empty());
        assert!(read_bar(0, 0, 8).is_empty());
        assert!(is_deleted(true));
        assert_eq!(deleted_icon(true), "trash.circle.fill");
        assert_eq!(deleted_tone(false), "good");
    }

    #[test]
    fn scores_and_labels() {
        assert_eq!(sync_score(5, 5, false), 100);
        assert_eq!(sync_score(5, 4, false), 70);
        assert_eq!(sync_score(5, 5, true), 60);
        assert_eq!(sync_tone(100), "good");
        assert_eq!(sync_label(50), "Catching up");
        assert_eq!(sync_label(20), "Archived");
        assert!(session_headline(5, 5, true).contains("archived"));
    }

    #[test]
    fn lines_and_summary() {
        let l = build_lines(Some("s1"), 10, 7, false);
        assert_eq!(l.len(), 3);
        assert!(!l[0].ok == false);
        assert_eq!(state_icon(true), "checkmark.circle.fill");
        assert_eq!(state_tone(false), "attention");
        let rows = summary(Some("s1"), 10, 7, false);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].1, "10");
    }
}
