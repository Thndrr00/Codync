#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from OpenMausBot's `src/lib/aurora-field.ts`: the dark aurora background with
//! one bright band, Bayer dithering and a seeded star field. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Spacing between sampled dots (PITCH).
pub const PITCH: u64 = 3;

/// Size of a painted dot (DOT).
pub const DOT: u64 = 2;

/// Number of quantization levels (LEVELS).
pub const LEVELS: u64 = 5;

/// 4×4 Bayer dithering thresholds.
pub const BAYER: [f64; 16] = [
    0.5 / 16.0,
    8.5 / 16.0,
    2.5 / 16.0,
    10.5 / 16.0,
    12.5 / 16.0,
    4.5 / 16.0,
    14.5 / 16.0,
    6.5 / 16.0,
    3.5 / 16.0,
    11.5 / 16.0,
    1.5 / 16.0,
    9.5 / 16.0,
    15.5 / 16.0,
    7.5 / 16.0,
    13.5 / 16.0,
    5.5 / 16.0,
];

/// Curve of the band's centre line (CURVE).
pub const CURVE: (f64, f64, f64) = (-0.087, 1.157, -0.262);

/// Reference height the field was tuned at (REFERENCE_HEIGHT).
pub const REFERENCE_HEIGHT: f64 = 1_162.0;

/// Seed of the star field (the original uses 7).
pub const FIELD_SEED: u32 = 7;

/// Ground color of the field.
pub const GROUND: &str = "#080908";

/// Palette corners of the band (DEEP, GREEN, TEAL, CORE, LIME, VIOLET, PLUM).
pub const PALETTE: [&str; 7] = ["#052b20", "#0d8c43", "#1ac79e", "#eaffcc", "#ccf240", "#8c5cff", "#401a73"];

/// Mix two channels by t.
fn mix(from: [f64; 3], to: [f64; 3], t: f64) -> [f64; 3] {
    [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, from[2] + (to[2] - from[2]) * t]
}

/// Scale channels by k.
fn scale(channels: [f64; 3], k: f64) -> [f64; 3] {
    [channels[0] * k, channels[1] * k, channels[2] * k]
}

/// Deterministic pseudo-random generator (seeded).
pub fn seeded(seed: u32) -> impl FnMut() -> f64 {
    let mut a = seed as i64;
    move || {
        a = a.wrapping_add(0x6d2b79f5);
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as u64 as f64) / 4_294_967_296.0
    }
}

/// y/h of the band's centre line at a given x.
pub fn band_center(u: f64) -> f64 {
    CURVE.0 + CURVE.1 * u + CURVE.2 * u * u
}

/// Light of the band at a point, as three channels (bandLight).
pub fn band_light(x: f64, y: f64, w: f64, h: f64) -> [f64; 3] {
    let u = x / w;
    let scale_h = h / REFERENCE_HEIGHT;
    let center = h * band_center(u);
    let slope = (h / w) * (CURVE.1 + 2.0 * CURVE.2 * u);
    let d = (y - center) / (1.0 + slope * slope).sqrt() / scale_h;
    let along = 0.7 + 0.3 * (-(((x / scale_h - 820.0) / 800.0).powi(2))).exp();
    let rays = 0.82 + 0.18 * (d * 0.11 + (x * 0.004).sin() * 3.0).sin() + 0.08 * (d * 0.37 + x * 0.002).sin();
    let deep = [0.02, 0.22, 0.12];
    let green = [0.05, 0.55, 0.26];
    let teal = [0.10, 0.78, 0.62];
    let core = [0.92, 1.0, 0.80];
    let lime = [0.80, 0.95, 0.25];
    let violet = [0.55, 0.36, 1.0];
    let plum = [0.25, 0.10, 0.45];
    let light = if d < 0.0 {
        let t = (d / 250.0).exp();
        scale(mix(deep, mix(green, teal, (d / 80.0).exp()), (t * 1.5).min(1.0)), t * 1.55)
    } else {
        let t = (-d / 130.0).exp();
        scale(mix(plum, mix(violet, lime, (-d / 14.0).exp()), ((-d / 40.0).exp() * 1.1 + t * 0.5).min(1.0)), t * 1.5)
    };
    scale(mix(light, core, (-(d / 24.0).powi(2)).exp() * 0.85), along * rays)
}

/// Bayer threshold for a dot's row and column.
pub fn bayer_threshold(row: usize, col: usize) -> f64 {
    BAYER[(row & 3) * 4 + (col & 3)]
}

/// Quantize a channel to a display level (level).
pub fn quantize(value: f64, threshold: f64) -> u8 {
    let clamped = value.clamp(0.0, 1.0);
    ((((clamped * LEVELS as f64 + threshold).floor() / LEVELS as f64) * 255.0).round() as u8)
}

/// Whether a dot is dark enough to skip.
pub fn is_dark(r: u8, g: u8, b: u8) -> bool {
    u16::from(r) + u16::from(g) + u16::from(b) < 6
}

/// Brightness of a color (sum of channels).
pub fn brightness(channels: [f64; 3]) -> f64 {
    channels[0] + channels[1] + channels[2]
}

/// The band slopes from top-left to lower-right (fallsRight).
pub fn falls_right() -> bool {
    band_center(1.0) > band_center(0.0)
}

/// CSS size string of the painted field.
pub fn field_size(w: u64, h: u64) -> String {
    format!("{w}px {h}px")
}

/// CSS variable that holds the field (fieldVar).
pub fn field_var() -> &'static str {
    "--aurora-field"
}

/// Brightest row of a column, sampled every 2 pixels (brightestRow).
pub fn brightest_row(x: f64, w: f64, h: f64) -> (u64, f64) {
    let mut best = (0_u64, 0.0_f64);
    let mut y = 0_u64;
    while y < h as u64 {
        let light = brightness(band_light(x, y as f64, w, h));
        if light > best.1 {
            best = (y, light);
        }
        y += 2;
    }
    best
}

/// Whether the background is worth painting at this size.
pub fn should_paint(w: u64, h: u64) -> bool {
    w > 0 && h > 0
}

/// One line describing the field (fieldLine).
pub fn field_line(w: u64, h: u64, painted: bool) -> String {
    format!("{w}×{h} · {} · {}", GROUND, if painted { "painted" } else { "idle" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(dots: usize, levels: usize, seeded: bool, painted: bool) -> Vec<(&'static str, String)> {
    vec![
        ("dots", count_label(dots)),
        ("levels", count_label(levels)),
        ("seeded", if seeded { "yes" } else { "no" }.to_owned()),
        ("painted", if painted { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_constants() {
        assert_eq!(PITCH, 3);
        assert_eq!(DOT, 2);
        assert_eq!(LEVELS, 5);
        assert_eq!(BAYER.len(), 16);
        assert_eq!(PALETTE.len(), 7);
        assert_eq!(GROUND, "#080908");
        assert!(falls_right());
        assert_eq!(field_var(), "--aurora-field");
        assert_eq!(field_size(100, 200), "100px 200px");
    }

    #[test]
    fn band_shape() {
        let w = 1874.0;
        let h = 1162.0;
        let (y, light) = brightest_row(900.0, w, h);
        assert!(light > 1.5);
        assert!(brightness(band_light(900.0, (y + 400) as f64, w, h)) < light / 4.0);
        assert!(brightness(band_light(200.0, 1120.0, w, h)) < 0.1);
        assert!(brightest_row(1600.0, w, h).0 > brightest_row(500.0, w, h).0);
        let full = brightest_row(900.0, w, h).0 as f64 / h;
        let half = brightest_row(450.0, w / 2.0, h / 2.0).0 as f64 / (h / 2.0);
        assert!((full - half).abs() < 0.05);
    }

    #[test]
    fn dither_and_random() {
        assert_eq!(bayer_threshold(0, 0), 0.5 / 16.0);
        assert_eq!(bayer_threshold(1, 1), 4.5 / 16.0);
        assert_eq!(quantize(0.0, 0.0), 0);
        assert_eq!(quantize(1.0, 0.0), 255);
        assert!(quantize(0.5, 0.5) > 0);
        assert!(is_dark(1, 2, 2));
        assert!(!is_dark(100, 100, 100));
        let mut random = seeded(FIELD_SEED);
        let first = random();
        assert!(first >= 0.0 && first < 1.0);
        assert_eq!(first, seeded(FIELD_SEED)());
    }

    #[test]
    fn presentation() {
        assert!(should_paint(100, 100));
        assert!(!should_paint(0, 100));
        assert!(field_line(100, 200, true).contains("painted"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(100, 5, true, true).len(), 4);
    }
}
