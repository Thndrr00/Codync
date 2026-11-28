#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/mochi/desktop-logic.ts`: sleep, hit-testing,
//! gaze, alerts, pointer distance and drag geometry. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Desktop panel size (PANEL_SIZE).
pub const PANEL_SIZE: f64 = 120.0;

/// Seconds of inactivity before sleeping (SLEEP_TIMEOUT).
pub const SLEEP_TIMEOUT: f64 = 120.0;

/// Mouse distance that counts as away (SLEEP_MOUSE_DISTANCE).
pub const SLEEP_MOUSE_DISTANCE: f64 = 150.0;

/// Radius of the round body as a fraction of the panel (BODY_RADIUS_FRACTION).
pub const BODY_RADIUS_FRACTION: f64 = 0.24;

/// Delay before retracting after landing (RETRACT_DELAY_MS).
pub const RETRACT_DELAY_MS: u64 = 450;

/// Delay before returning to the island (RETURN_DELAY_MS).
pub const RETURN_DELAY_MS: u64 = 600;

/// Window to treat two clicks as a double click (DOUBLE_CLICK_MS).
pub const DOUBLE_CLICK_MS: u64 = 300;

/// Pixels before a press becomes a drag (DRAG_THRESHOLD).
pub const DRAG_THRESHOLD: f64 = 3.0;

/// Pointer memory on Linux (POINTER_MEMORY_MS).
pub const POINTER_MEMORY_MS: u64 = 4_000;

/// Desktop phases of Mochi.
pub const DESKTOP_PHASES: [&str; 6] = ["home", "flying", "landed", "dragging", "returning", "sleeping"];

/// Whether Mochi should sleep (shouldSleep).
pub fn should_sleep(inactive_seconds: f64, mouse_distance: f64) -> bool {
    inactive_seconds > SLEEP_TIMEOUT && mouse_distance >= SLEEP_MOUSE_DISTANCE
}

/// Hit test of the round body in the square window (isOverBody).
pub fn is_over_body(x: f64, y: f64, panel_size: f64) -> bool {
    let center = panel_size / 2.0;
    let radius = panel_size * BODY_RADIUS_FRACTION;
    let dx = x - center;
    let dy = y - center;
    dx * dx + dy * dy <= radius * radius
}

/// Eye-tracking origin: the window centre (lookOrigin).
pub fn look_origin(panel_min_x: f64, panel_min_y: f64, panel_size: f64) -> Value {
    json!({"x": panel_min_x + panel_size / 2.0, "y": panel_min_y + panel_size / 2.0})
}

/// Gaze offsets from bot to mouse: x right+, y up+ (gaze).
pub fn gaze(bot_x: f64, bot_y: f64, mouse_x: f64, mouse_y: f64) -> Value {
    json!({
        "lookX": ((mouse_x - bot_x) / 260.0).tanh(),
        "lookY": -((mouse_y - bot_y) / 200.0).tanh(),
    })
}

/// Whether to retract on landing after an alert (shouldRetractOnLanding).
pub fn should_retract_on_landing(alert_active: bool) -> bool {
    alert_active
}

/// Whether an agent state keeps Mochi awake (agentActive).
pub fn agent_active(state: &str) -> bool {
    state != "idle" && state != "sleeping"
}

/// Whether a card is waiting for an answer (alertActive).
pub fn alert_active(pending_approval: bool, states: &[String]) -> bool {
    pending_approval || states.iter().any(|state| state == "question")
}

/// Distance from the panel centre to the cursor (pointerDistance).
pub fn pointer_distance(cursor_poll: bool, cursor: Option<(f64, f64)>, since_pointer_ms: u64) -> f64 {
    if cursor_poll {
        cursor.map_or(f64::INFINITY, |(x, y)| {
            let center = PANEL_SIZE / 2.0;
            ((x - center).powi(2) + (y - center).powi(2)).sqrt()
        })
    } else if since_pointer_ms < POINTER_MEMORY_MS {
        0.0
    } else {
        f64::INFINITY
    }
}

/// Top-left of the window during a layer drag (layerDragTopLeft).
pub fn layer_drag_top_left(origin: (f64, f64), client: (f64, f64), grab: (f64, f64), overlay_applied: bool) -> Value {
    let base = if overlay_applied { (0.0, 0.0) } else { origin };
    json!({"x": base.0 + client.0 - grab.0, "y": base.1 + client.1 - grab.1})
}

/// Top-left of the window during an X11 drag (windowDragTopLeft).
pub fn window_drag_top_left(origin: (f64, f64), screen: (f64, f64), screen_at_press: (f64, f64), dpr: f64) -> Value {
    json!({
        "x": origin.0 + (screen.0 - screen_at_press.0) * dpr,
        "y": origin.1 + (screen.1 - screen_at_press.1) * dpr,
    })
}

/// Whether a press became a drag (isDrag).
pub fn is_drag(distance: f64) -> bool {
    distance > DRAG_THRESHOLD
}

/// Whether two clicks fall inside the double-click window (isDoubleClick).
pub fn is_double_click(since_last_ms: u64) -> bool {
    since_last_ms <= DOUBLE_CLICK_MS
}

/// One line describing a desktop phase (desktopLine).
pub fn desktop_line(phase: &str, sleeping: bool, alert: bool) -> String {
    format!("{phase} · {} · {}", if sleeping { "sleeping" } else { "awake" }, if alert { "alert" } else { "quiet" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(panel: u64, distance: u64, pointer_ms: u64, awake: bool) -> Vec<(&'static str, String)> {
    vec![
        ("panel", count_label(panel as usize)),
        ("distance", count_label(distance as usize)),
        ("pointerMs", count_label(pointer_ms as usize)),
        ("awake", if awake { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_and_hits() {
        assert_eq!(PANEL_SIZE, 120.0);
        assert_eq!(SLEEP_TIMEOUT, 120.0);
        assert_eq!(SLEEP_MOUSE_DISTANCE, 150.0);
        assert_eq!(BODY_RADIUS_FRACTION, 0.24);
        assert_eq!(RETRACT_DELAY_MS, 450);
        assert_eq!(RETURN_DELAY_MS, 600);
        assert_eq!(DOUBLE_CLICK_MS, 300);
        assert_eq!(DRAG_THRESHOLD, 3.0);
        assert_eq!(POINTER_MEMORY_MS, 4_000);
        assert_eq!(DESKTOP_PHASES.len(), 6);
        assert!(should_sleep(200.0, 200.0));
        assert!(!should_sleep(60.0, 200.0));
        assert!(!should_sleep(200.0, 100.0));
        assert!(is_over_body(60.0, 60.0, PANEL_SIZE));
        assert!(!is_over_body(10.0, 10.0, PANEL_SIZE));
    }

    #[test]
    fn gaze_and_alerts() {
        assert_eq!(look_origin(10.0, 20.0, 120.0)["x"], 70.0);
        let look = gaze(60.0, 60.0, 320.0, 60.0);
        assert!(look["lookX"].as_f64().unwrap_or(0.0) > 0.0);
        assert_eq!(look["lookY"], 0.0);
        assert!(should_retract_on_landing(true));
        assert!(!should_retract_on_landing(false));
        assert!(agent_active("working"));
        assert!(!agent_active("idle"));
        assert!(!agent_active("sleeping"));
        assert!(alert_active(false, &["question".to_owned()]));
        assert!(alert_active(true, &[]));
        assert!(!alert_active(false, &["working".to_owned()]));
    }

    #[test]
    fn pointer_and_drag() {
        assert_eq!(pointer_distance(false, None, 1_000), 0.0);
        assert_eq!(pointer_distance(false, None, 5_000), f64::INFINITY);
        assert!(pointer_distance(true, Some((60.0, 60.0)), 0) < 1.0);
        assert_eq!(pointer_distance(true, None, 0), f64::INFINITY);
        let layer = layer_drag_top_left((10.0, 20.0), (30.0, 40.0), (5.0, 5.0), false);
        assert_eq!(layer["x"], 35.0);
        assert_eq!(layer["y"], 55.0);
        let window = window_drag_top_left((10.0, 20.0), (30.0, 40.0), (20.0, 30.0), 2.0);
        assert_eq!(window["x"], 30.0);
        assert_eq!(window["y"], 40.0);
        assert!(is_drag(4.0));
        assert!(!is_drag(3.0));
        assert!(is_double_click(200));
        assert!(!is_double_click(400));
    }

    #[test]
    fn presentation() {
        assert!(desktop_line("landed", false, true).contains("alert"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(120, 150, 4000, true).len(), 4);
    }
}
