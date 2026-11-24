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
