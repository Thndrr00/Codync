#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/anim.ts`: easing curves, cubic-bezier,
//! spring integration and tracked-value timings. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Easing names (Ease).
pub const EASE_NAMES: [&str; 5] = ["out", "inOut", "back", "lin", "easeIn"];

/// Sub-steps per second of a spring (1/240).
pub const SPRING_SUBSTEPS: f64 = 240.0;

/// Duration of the island close curve in milliseconds.
pub const CLOSE_DURATION_MS: u64 = 340;

/// Default spring response in seconds.
pub const DEFAULT_RESPONSE: f64 = 0.5;

/// Default spring damping fraction.
pub const DEFAULT_DAMPING: f64 = 0.72;

/// Modes of a tracked value (spring / curve / idle).
pub const TRACKED_MODES: [&str; 3] = ["spring", "curve", "idle"];

/// Linear interpolation (lerp).
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Clamp to a range (clamp).
pub fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    value.max(lo).min(hi)
}

/// Normalized position of `t` inside the segment `[a, b]` (seg).
pub fn seg(t: f64, a: f64, b: f64) -> f64 {
    if b == a {
        return 0.0;
    }
    clamp((t - a) / (b - a), 0.0, 1.0)
}

/// Cubic ease-out (Ease.out).
pub fn ease_out(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

/// Cubic ease-in (Ease.easeIn).
pub fn ease_in(t: f64) -> f64 {
    t * t * t
}

/// Cubic ease-in-out (Ease.inOut).
pub fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// Back overshoot (Ease.back).
pub fn ease_back(t: f64) -> f64 {
    let c1 = 1.7;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Linear easing and identity (Ease.lin).
pub fn ease_linear(t: f64) -> f64 {
    t
}

/// Easing by name, defaulting to linear (Ease[name]).
pub fn ease_named(name: &str, t: f64) -> f64 {
    match name {
        "out" => ease_out(t),
        "inOut" => ease_in_out(t),
        "back" => ease_back(t),
        "easeIn" => ease_in(t),
        _ => ease_linear(t),
    }
}

/// Cubic-bezier coordinate for x (cubicBezier).
fn bezier_coordinate(t: f64, p1: f64, p2: f64) -> f64 {
    let u = 1.0 - t;
    3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t.powi(3)
}

/// Solve a cubic-bezier curve at x with 12 bisection steps (cubicBezier).
pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    let mut lo = 0.0;
    let mut hi = 1.0;
    let mut t = clamp(x, 0.0, 1.0);
    for _ in 0..12 {
        let value = bezier_coordinate(t, x1, x2);
        if value < x {
            lo = t;
        } else {
            hi = t;
        }
        t = (lo + hi) / 2.0;
    }
    bezier_coordinate(t, y1, y2)
}

/// The island close curve (closeCurve).
pub fn close_curve(x: f64) -> f64 {
    cubic_bezier(0.45, 0.0, 0.2, 1.0, x)
}

/// Spring angular frequency from a response in seconds.
pub fn spring_omega(response: f64) -> f64 {
    if response <= 0.0 { 0.0 } else { 2.0 * std::f64::consts::PI / response }
}

/// Spring acceleration at a point (Spring.step).
pub fn spring_accel(value: f64, target: f64, velocity: f64, omega: f64, zeta: f64) -> f64 {
    omega * omega * (target - value) - 2.0 * zeta * omega * velocity
}

/// Integrate a spring one frame, sub-stepped at 240 Hz (Spring.step).
pub fn spring_step(value: f64, target: f64, velocity: f64, omega: f64, zeta: f64, dt: f64) -> (f64, f64) {
    let steps = ((dt * SPRING_SUBSTEPS).ceil() as usize).max(1);
    let h = dt / steps as f64;
    let mut value = value;
    let mut velocity = velocity;
    for _ in 0..steps {
        let accel = spring_accel(value, target, velocity, omega, zeta);
        velocity += accel * h;
        value += velocity * h;
    }
    (value, velocity)
}

/// Whether a spring has settled (Spring.settled).
pub fn spring_settled(value: f64, target: f64, velocity: f64) -> bool {
    (target - value).abs() < 0.01 && velocity.abs() < 0.05
}

/// Timed curve value of a tracked value (Tracked.step).
pub fn tracked_curve(from: f64, to: f64, elapsed_ms: u64, duration_ms: u64) -> f64 {
    let p = seg(elapsed_ms as f64, 0.0, duration_ms.max(1) as f64);
    lerp(from, to, close_curve(p))
}

/// Blend two colors' weights with an eased factor (blend).
pub fn blend(from: f64, to: f64, t: f64, ease: &str) -> f64 {
    lerp(from, to, ease_named(ease, t))
}

/// Frames a duration takes at a refresh rate.
pub fn frame_count(duration_ms: u64, fps: u64) -> u64 {
    duration_ms.saturating_mul(fps.max(1)) / 1_000
}

/// Human label of a duration in milliseconds.
pub fn duration_label(ms: u64) -> String {
    if ms < 1_000 { format!("{ms}ms") } else { format!("{:.2}s", ms as f64 / 1_000.0) }
}

/// One line describing the easing state (animLine).
pub fn anim_line(name: &str, ms: u64, settled: bool) -> String {
    format!("{} · {} · {}", name, duration_label(ms), if settled { "settled" } else { "moving" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(eases: usize, modes: usize, steps: usize, moving: bool) -> Vec<(&'static str, String)> {
    vec![
        ("eases", count_label(eases)),
        ("modes", count_label(modes)),
        ("steps", count_label(steps)),
        ("moving", if moving { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_eases() {
        assert_eq!(EASE_NAMES.len(), 5);
        assert_eq!(SPRING_SUBSTEPS, 240.0);
        assert_eq!(CLOSE_DURATION_MS, 340);
        assert_eq!(DEFAULT_RESPONSE, 0.5);
        assert_eq!(DEFAULT_DAMPING, 0.72);
        assert_eq!(TRACKED_MODES.len(), 3);
        assert_eq!(lerp(0.0, 10.0, 0.25), 2.5);
        assert_eq!(clamp(12.0, 0.0, 10.0), 10.0);
        assert_eq!(seg(5.0, 2.0, 6.0), 0.75);
        assert_eq!(seg(1.0, 1.0, 1.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert_eq!(ease_in(2.0), 8.0);
        assert_eq!(ease_in_out(0.5), 0.5);
        assert_eq!(ease_back(1.0), 1.0);
        assert_eq!(ease_linear(0.3), 0.3);
        assert_eq!(ease_named("back", 1.0), 1.0);
        assert_eq!(ease_named("unknown", 0.4), 0.4);
    }

    #[test]
    fn curves() {
        assert!((cubic_bezier(0.0, 0.0, 1.0, 1.0, 0.5) - 0.5).abs() < 0.01);
        assert!(close_curve(0.0).abs() < 0.01);
        assert!((close_curve(1.0) - 1.0).abs() < 0.01);
        assert_eq!(tracked_curve(0.0, 10.0, 0, 340), 0.0);
        assert_eq!(tracked_curve(0.0, 10.0, 340, 340), 10.0);
        assert_eq!(blend(0.0, 8.0, 0.5, "lin"), 4.0);
    }

    #[test]
    fn springs() {
        let omega = spring_omega(DEFAULT_RESPONSE);
        assert!((omega - 2.0 * std::f64::consts::PI / 0.5).abs() < 1e-9);
        assert_eq!(spring_omega(0.0), 0.0);
        let accel = spring_accel(0.0, 1.0, 0.0, omega, DEFAULT_DAMPING);
        assert!(accel > 0.0);
        let (value, velocity) = spring_step(0.0, 1.0, 0.0, omega, DEFAULT_DAMPING, 1.0 / 60.0);
        assert!(value > 0.0 && velocity > 0.0);
        assert!(spring_settled(1.0, 1.0, 0.0));
        assert!(!spring_settled(0.9, 1.0, 0.0));
    }

    #[test]
    fn presentation() {
        assert_eq!(frame_count(340, 60), 20);
        assert_eq!(duration_label(340), "340ms");
        assert_eq!(duration_label(1_500), "1.50s");
        assert!(anim_line("close", 340, true).contains("settled"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(5, 3, 240, false).len(), 4);
    }
}
