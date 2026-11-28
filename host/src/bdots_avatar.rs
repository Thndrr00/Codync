#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `bot-avatar-colors.ts`: persona colors, shipped hashing,
//! hex/contrast utilities and eye tones. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

/// Default persona color id.
pub const DEFAULT_COLOR_ID: &str = "violet";

/// Order of invite artwork colors.
pub const INVITE_COLOR_IDS: [&str; 5] = ["violet", "green", "orange", "cyan", "blue"];

/// Palette of shipped persona colors (id, name, hex).
pub const BOT_COLORS: [(&str, &str, &str); 8] = [
    ("violet", "Violet", "#7C5CFC"),
    ("green", "Green", "#3EC5A8"),
    ("orange", "Orange", "#F5A03C"),
    ("cyan", "Cyan", "#4FC3F7"),
    ("blue", "Blue", "#3B82F6"),
    ("red", "Red", "#EF5350"),
    ("yellow", "Yellow", "#FDD835"),
    ("white", "White", "#FFFFFF"),
];

/// Minimum white-text contrast for a chat bubble (BUBBLE_TEXT_CONTRAST).
pub const BUBBLE_CONTRAST: f64 = 3.0;

/// White text on colored bubbles.
pub const LIGHT_TEXT: &str = "#FFFFFF";

/// FNV-1a hash used for shipped persona colors (shippedHash).
pub fn shipped_hash(value: &str) -> u32 {
    let mut hash = 2_166_136_261_u32;
    for byte in value.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

/// Deterministic pseudo-random generator from a seed (shippedRandom).
pub fn shipped_random(seed: u32) -> impl FnMut() -> f64 {
    let mut value = seed;
    move || {
        value = value.wrapping_add(1_831_565_813);
        let mut next = (value ^ (value >> 15)).wrapping_mul(1 | value);
        next = (next.wrapping_add((next ^ (next >> 7)).wrapping_mul(61 | next))) ^ next;
        f64::from(next ^ (next >> 14)) / 4_294_967_296.0
    }
}

/// Expand a 3-digit hex color to 6 digits (expandHex).
pub fn expand_hex(hex: &str) -> String {
    let value = hex.trim().trim_start_matches('#');
    if value.len() == 3 { value.chars().map(|c| format!("{c}{c}")).collect() } else { value.to_ascii_lowercase() }
}

/// RGB channels of a hex color.
pub fn hex_channels(hex: &str) -> Option<[u8; 3]> {
    let value = expand_hex(hex);
    if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |index: usize| u8::from_str_radix(&value[index..index + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Relative luminance of a hex color.
pub fn luminance(hex: &str) -> f64 {
    let Some([r, g, b]) = hex_channels(hex) else {
        return 0.0;
    };
    let linear = |value: u8| {
        let channel = f64::from(value) / 255.0;
        if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/// WCAG contrast ratio between two hex colors (contrastRatio).
pub fn contrast_ratio(a: &str, b: &str) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (high, low) = if la > lb { (la, lb) } else { (lb, la) };
    (high + 0.05) / (low + 0.05)
}

/// Resolve a persona color by id, name, hex or deterministic hash (resolvePersonaColorDef).
pub fn resolve_persona_color(identity: &str, explicit: Option<&str>) -> (&'static str, &'static str) {
    if let Some(clean) = explicit {
        let lower = clean.to_ascii_lowercase();
        if let Some(found) = BOT_COLORS.iter().find(|(id, name, _)| *id == lower || name.to_ascii_lowercase() == lower)
        {
            return (found.0, found.2);
        }
        if let Some(found) = BOT_COLORS.iter().find(|(_, _, hex)| hex.to_ascii_lowercase() == lower) {
            return (found.0, found.2);
        }
        let value = lower.trim_start_matches('#');
        if (value.len() == 3 || value.len() == 6) && value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return ("custom", explicit.unwrap_or("#000000"));
        }
    }
    let seed = shipped_hash(identity) ^ 2_654_435_769;
    let mut random = shipped_random(seed);
    let index = (random() * BOT_COLORS.len() as f64) as usize % BOT_COLORS.len();
    (BOT_COLORS[index].0, BOT_COLORS[index].2)
}

/// Tone of the bot eye on a body color (botEyeTone).
pub fn eye_tone(body_hex: &str) -> &'static str {
    if luminance(body_hex) < 0.25 { "light" } else { "page" }
}

/// Ink color for the eye on a body color and page (botEyeInk).
pub fn eye_ink(body_hex: &str, page_foreground: &str) -> &'static str {
    if contrast_ratio(body_hex, LIGHT_TEXT) >= BUBBLE_CONTRAST {
        LIGHT_TEXT
    } else {
        let _ = page_foreground;
        "#101014"
    }
}

/// Whether a color is usable for invite artwork (no white).
pub fn invite_color(hex: &str) -> bool {
    hex.to_ascii_lowercase() != "#ffffff"
}

/// Darken a color until white text reaches the required contrast.
pub fn bubble_background(hex: &str) -> String {
    let mut current = expand_hex(hex).to_ascii_uppercase();
    let mut step = 0;
    while contrast_ratio(&current, LIGHT_TEXT) < BUBBLE_CONTRAST && step < 100 {
        let Some([r, g, b]) = hex_channels(&current) else {
            break;
        };
        let darken = |value: u8| value.saturating_sub(4);
        current = format!("#{:02X}{:02X}{:02X}", darken(r), darken(g), darken(b));
        step += 1;
    }
    current
}

/// Whether a value is a 3- or 6-digit hex color.
pub fn is_hex_color(value: &str) -> bool {
    let clean = value.trim().trim_start_matches('#');
    (clean.len() == 3 || clean.len() == 6) && clean.bytes().all(|b| b.is_ascii_hexdigit())
}

/// RGB display string for a hex color.
pub fn rgb_string(hex: &str) -> String {
    hex_channels(hex).map_or_else(String::new, |[r, g, b]| format!("rgb({r}, {g}, {b})"))
}

/// Avatar initials from a name, reusing the shared upper-first helper.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().filter(|word| !word.is_empty()).collect();
    match words.len() {
        0 => String::new(),
        1 => words[0].chars().take(2).collect::<String>().to_ascii_uppercase(),
        _ => format!(
            "{}{}",
            words[0].chars().next().unwrap_or(' ').to_ascii_uppercase(),
            words[words.len() - 1].chars().next().unwrap_or(' ').to_ascii_uppercase()
        ),
    }
}

/// Brightness bucket of a color for theming.
pub fn brightness_bucket(hex: &str) -> &'static str {
    let l = luminance(hex);
    if l < 0.2 {
        "dark"
    } else if l > 0.7 {
        "light"
    } else {
        "medium"
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(color: &str, tone: &str, ratio: f64, invites: usize) -> Vec<(&'static str, String)> {
    vec![
        ("color", color.to_owned()),
        ("eye", tone.to_owned()),
        ("contrast", format!("{ratio:.1}")),
        ("invites", count_label(invites)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_and_random() {
        assert_eq!(shipped_hash("abc"), shipped_hash("abc"));
        assert_ne!(shipped_hash("abc"), shipped_hash("abd"));
        let mut random = shipped_random(42);
        let first = random();
        assert!(first >= 0.0 && first < 1.0);
        assert_eq!(first, shipped_random(42)());
    }

    #[test]
    fn hex_and_contrast() {
        assert_eq!(expand_hex("#abc"), "aabbcc");
        assert_eq!(hex_channels("#7C5CFC"), Some([124, 92, 252]));
        assert_eq!(hex_channels("zzz"), None);
        assert!(luminance("#FFFFFF") > luminance("#000000"));
        assert!(contrast_ratio("#FFFFFF", "#000000") > 20.0);
    }

    #[test]
    fn persona_and_eyes() {
        assert_eq!(resolve_persona_color("id", Some("violet")).0, "violet");
        assert_eq!(resolve_persona_color("id", Some("#123456")).0, "custom");
        let (_, color) = resolve_persona_color("id", None);
        assert!(color.starts_with('#'));
        assert_eq!(eye_tone("#101014"), "light");
        assert_eq!(eye_tone("#FFFFFF"), "page");
        assert_eq!(eye_ink("#101014", "#fff"), LIGHT_TEXT);
        assert!(!invite_color("#FFFFFF"));
    }

    #[test]
    fn bubbles_and_summary() {
        let bg = bubble_background("#7C5CFC");
        assert!(contrast_ratio(&bg, LIGHT_TEXT) >= BUBBLE_CONTRAST || bg.to_ascii_lowercase() == "#000000");
        assert!(is_hex_color("#abc"));
        assert!(!is_hex_color("#abcd"));
        assert_eq!(rgb_string("#7C5CFC"), "rgb(124, 92, 252)");
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials(""), "");
        assert_eq!(brightness_bucket("#000000"), "dark");
        assert_eq!(brightness_bucket("#FFFFFF"), "light");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("#fff", "page", 4.5, INVITE_COLOR_IDS.len()).len(), 4);
    }
}
