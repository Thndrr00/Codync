//! Pure decisions behind the browser tools, mirrored in `page.js` where the page is read.
//!
//! Kept off the Windows CDP path so the rules and their tests run everywhere. Every rule here is a
//! short translation of an OpenBot change (0.68–0.70) into a decision Codync can test on its own.

#![allow(dead_code, reason = "some rules are read only by the Windows CDP bridge")]

use serde_json::Value;
use serde_json::json;

/// The largest coordinate a drag may use, matching the live input bound in `cdp.rs`.
const MAX_DRAG_COORD: u64 = 100_000;
/// Shorter than this is a click that twitched, not a drag (OpenBot 0.70).
const MIN_DRAG: i64 = 8;
/// The CDP event Chromium raises when a page renderer dies (OpenBot 0.78.0).
pub(crate) const TARGET_CRASHED: &str = "Inspector.targetCrashed";

/// A page dialog's own text is handed to the agent bounded to this many characters (triage-15938).
pub(crate) const DIALOG_TEXT_LIMIT: usize = 2_000;

/// One page dialog the agent is told about (OpenBot triage-15938, `changelog.d/triage-15938.md`):
/// a fixed `label` that is safe to count in analytics, and the dialog's own `text`, bounded for the
/// tool result. The text is data for the agent only; no page content ever reaches analytics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DialogNotice {
    pub(crate) label: &'static str,
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

/// Turns a CDP dialog `type` and message into the label and bounded text the tool result carries.
/// Whitespace is collapsed so a message cannot smuggle a fake snapshot line; an unknown kind still
/// gets a fixed label. Pure, so the rule and its limits are tested off the Windows CDP path.
pub(crate) fn dialog_notice(kind: &str, text: &str, limit: usize) -> DialogNotice {
    let label = match kind.trim().to_ascii_lowercase().as_str() {
        "alert" => "alert",
        "confirm" => "confirm",
        "prompt" => "prompt",
        _ => "dialog",
    };
    let clean = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let (text, truncated) = super::cut_at(&clean, limit);
    DialogNotice { label, text, truncated }
}

/// One end of a drag, in viewport pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) x: i64,
    pub(crate) y: i64,
}

/// The display name of a saved browser screenshot (OpenBot 0.68): the page's host and a timestamp,
/// so several shots in one chat stay apart. Safe as a file name and bounded for `files::name`.
pub(crate) fn screenshot_name(url: &str, now_secs: u64) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default().split('@').last().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    let host: String = host
        .chars()
        .take(40)
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '-' })
        .collect();
    let host = if host.is_empty() { "page".to_owned() } else { host };
    format!("{host}-{now_secs}.png")
}

/// Whether an empty snapshot needs a picture instead (OpenBot 0.70): no page text and no controls
/// means the agent has nothing to read, so a screenshot is attached.
pub(crate) fn needs_visual(text: &str, controls: usize) -> bool {
    controls == 0 && text.trim().is_empty()
}

/// The two ends of a drag after checking them (OpenBot 0.70): both inside the page's coordinate
/// bound and far enough apart to be a drag. `None` means the press would be a stray click or land
/// outside the page, so no mouse sequence is sent.
pub(crate) fn drag_points(from: Point, to: Point) -> Option<(Point, Point)> {
    let bounded = |point: Point| point.x.unsigned_abs() <= MAX_DRAG_COORD && point.y.unsigned_abs() <= MAX_DRAG_COORD;
    if !bounded(from) || !bounded(to) {
        return None;
    }
    let far = (to.x - from.x).abs() >= MIN_DRAG || (to.y - from.y).abs() >= MIN_DRAG;
    far.then_some((from, to))
}

/// Which snapshot pass `page.js` runs. The drawn pass includes content that is rendered on screen
/// but hidden from assistive tech, for pages that hide everything from a plain snapshot (OpenBot
/// 0.70); the first pass keeps the accessible tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotPass {
    Accessible,
    Drawn,
}

impl SnapshotPass {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Accessible => "accessible",
            Self::Drawn => "drawn",
        }
    }
}

/// The arguments passed to `page.js::snapshot` for a pass. The accessible pass sends no flag; the
/// drawn pass asks the page to keep `aria-hidden` and opacity-zero content.
pub(crate) fn snapshot_pass(kind: SnapshotPass) -> Value {
    match kind {
        SnapshotPass::Accessible => json!({}),
        SnapshotPass::Drawn => json!({"drawn": true}),
    }
}

/// What the agent is told when a tab crashes (OpenBot 0.78.0). `reason` is a CDP label such as
/// `Inspector.targetCrashed` or `oom`, never the page's address or text, and is not echoed back:
/// only one of these fixed sentences reaches the tool result or analytics.
pub(crate) fn crash_notice(reason: &str) -> &'static str {
    if reason.eq_ignore_ascii_case("oom") || reason.eq_ignore_ascii_case("out-of-memory") {
        "The page ran out of memory and crashed. Reload it with navigate direction=reload, then take a new snapshot."
    } else {
        "The page crashed (Inspector.targetCrashed). Reload it with navigate direction=reload, then take a new snapshot."
    }
}

/// Whether an element is shown in a snapshot. The `hidden` and `inert` attributes always hide it;
/// `page.js` reads them with `Element.prototype.hasAttribute.call` because a form control named
/// `hidden` or `inert` shadows the element's own property and would otherwise hide a visible form
/// (OpenBot 0.69). `aria_hidden` and `transparent` content is dropped only by the accessible pass;
/// the drawn pass keeps it so the agent sees what is painted (OpenBot 0.70).
pub(crate) fn element_shown(hidden: bool, inert: bool, aria_hidden: bool, transparent: bool, drawn: bool) -> bool {
    if hidden || inert {
        return false;
    }
    if drawn {
        return true;
    }
    !aria_hidden && !transparent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screenshot_name_uses_the_host_and_a_timestamp() {
        assert_eq!(screenshot_name("https://example.com/login", 1_700_000_000), "example.com-1700000000.png");
        assert_eq!(screenshot_name("http://shop.example.com:8080/a?b=1", 42), "shop.example.com-42.png");
        assert_eq!(screenshot_name("about:blank", 7), "about-7.png");
        let long = format!("https://{}.example.com/", "a".repeat(60));
        let name = screenshot_name(&long, 1);
        assert!(name.len() <= 200 && name.ends_with("-1.png"), "{name}");
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || "-.".contains(c)), "{name}");
    }

    #[test]
    fn a_snapshot_with_no_text_or_controls_needs_a_picture() {
        assert!(needs_visual("", 0));
        assert!(needs_visual("  \n ", 0), "whitespace is no text");
        assert!(!needs_visual("Welcome", 0));
        assert!(!needs_visual("", 1));
        assert!(!needs_visual("Welcome", 3));
    }

    #[test]
    fn a_drag_lands_in_bounds_and_far_enough_apart() {
        let (from, to) = (Point { x: 10, y: 10 }, Point { x: 200, y: 30 });
        assert_eq!(drag_points(from, to), Some((from, to)));
        let (from, to) = (Point { x: 10, y: 10 }, Point { x: 12, y: 11 });
        assert_eq!(drag_points(from, to), None, "a two-pixel move is a click, not a drag");
        assert_eq!(drag_points(from, Point { x: 10, y: 20 }), Some((from, Point { x: 10, y: 20 })));
        assert_eq!(drag_points(Point { x: -1, y: 10 }, to), None, "outside the page");
        assert_eq!(drag_points(from, Point { x: 200_000, y: 10 }), None, "out of the coordinate bound");
    }

    #[test]
    fn the_drawn_pass_asks_the_page_to_keep_hidden_content() {
        assert_eq!(snapshot_pass(SnapshotPass::Accessible), json!({}));
        assert_eq!(snapshot_pass(SnapshotPass::Drawn), json!({"drawn": true}));
        assert_eq!(SnapshotPass::Drawn.name(), "drawn");
        assert_ne!(SnapshotPass::Accessible.name(), SnapshotPass::Drawn.name());
    }

    #[test]
    fn a_crashed_page_is_named_and_a_reload_is_offered() {
        let notice = crash_notice(TARGET_CRASHED);
        assert!(notice.contains("page crashed"), "{notice}");
        assert!(notice.contains("navigate direction=reload"), "{notice}");
        assert!(notice.contains("new snapshot"), "{notice}");
        let oom = crash_notice("oom");
        assert!(oom.contains("out of memory"), "{oom}");
        assert!(oom.contains("navigate direction=reload"), "{oom}");
        assert_eq!(crash_notice("OOM"), oom);
        assert_eq!(crash_notice("out-of-memory"), oom);
        assert_eq!(crash_notice(""), notice, "an unknown reason still names the crash");
        assert_eq!(crash_notice("https://secret.example/page?token=1"), notice, "no address is echoed");
    }

    #[test]
    fn a_page_dialog_has_a_fixed_label_and_bounded_text_for_the_agent() {
        let notice = dialog_notice("alert", "Your session expired", DIALOG_TEXT_LIMIT);
        assert_eq!(notice.label, "alert");
        assert_eq!(notice.text, "Your session expired");
        assert!(!notice.truncated);

        // A long message is cut in characters, not bytes, and says that it was cut.
        let long = dialog_notice("confirm", &"é".repeat(DIALOG_TEXT_LIMIT + 5), DIALOG_TEXT_LIMIT);
        assert_eq!(long.label, "confirm");
        assert_eq!(long.text.chars().count(), DIALOG_TEXT_LIMIT);
        assert!(long.truncated);
        assert_eq!(dialog_notice("prompt", "short", 3).text, "sho");

        // Newlines cannot forge a snapshot line, and an unknown kind gets a fixed label.
        let lines = dialog_notice("prompt", "1 button \"OK\"\nStore this", DIALOG_TEXT_LIMIT);
        assert_eq!(lines.text, "1 button \"OK\" Store this");
        for kind in ["", "ALERT", " beforeunload ", "https://secret.example/?token=1"] {
            let label = dialog_notice(kind, "anything", DIALOG_TEXT_LIMIT).label;
            assert!(["alert", "confirm", "prompt", "dialog"].contains(&label), "{kind} -> {label}");
        }
        assert_eq!(dialog_notice("beforeunload", "x", 10).label, "dialog");

        // The label is content-free: the same text under any kind never reaches it.
        for kind in ["alert", "confirm", "prompt", "other"] {
            let notice = dialog_notice(kind, "a password for bank.example", DIALOG_TEXT_LIMIT);
            assert!(!notice.label.contains("password") && !notice.label.contains("example"));
        }
        assert_eq!(dialog_notice("alert", "", 10).text, "");
    }

    #[test]
    fn hidden_and_inert_attr_always_hide_but_the_drawn_pass_keeps_painted_content() {
        assert!(!element_shown(true, false, false, false, false));
        assert!(!element_shown(false, true, false, false, false));
        assert!(!element_shown(true, false, false, false, true), "the drawn pass still respects hidden");
        assert!(!element_shown(false, true, false, false, true), "and inert");
        assert!(!element_shown(false, false, true, false, false), "the first pass drops aria-hidden");
        assert!(!element_shown(false, false, false, true, false), "and transparent content");
        assert!(element_shown(false, false, true, true, true), "the drawn pass keeps both");
        assert!(element_shown(false, false, false, false, false));
    }
}
