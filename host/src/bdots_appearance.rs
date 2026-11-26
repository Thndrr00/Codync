#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo v0.14.0's `packages/ui-tokens`: appearance preferences, type
//! scale, palette selection and colour utilities. Composes helpers already ported into this host.
//! Only `botInsights` calls this.

/// The three appearance preferences a person can store.
pub const APPEARANCE_PREFERENCES: [&str; 3] = ["system", "light", "dark"];

/// Border radius shared by every control (`RADIUS`).
pub const RADIUS: &str = "0.75rem";

/// Bot avatar colours (`botColors`).
pub const BOT_COLORS: [&str; 7] = ["#3EC5A8", "#F5A03C", "#6A6BF5", "#9B5CF6", "#3B82F6", "#F2622A", "#D9508A"];

/// Page icon bar colours (`pageIconBars`).
pub const PAGE_BARS: [&str; 4] = ["#489D61", "#E7712E", "#BC3B3D", "#3E82F3"];

/// The eye colour of the bot mark (`botEyeColor`).
pub const BOT_EYE_COLOR: &str = "#101014";

/// Maximum font scale before body text stops growing (`MAX_FONT_SCALE`).
pub fn max_font_scale() -> f64 {
    23.0 / 17.0
}

/// Whether a stored value is a supported appearance preference (isAppearancePreference).
pub fn is_appearance_preference(value: Option<&str>) -> bool {
    value.is_some_and(|v| APPEARANCE_PREFERENCES.contains(&v.to_ascii_lowercase().as_str()))
}

/// Normalise an unknown stored value to a preference (normalizeAppearancePreference).
pub fn normalize_appearance(raw: Option<&str>) -> &'static str {
    match raw.map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "light" => "light",
        Some(v) if v == "dark" => "dark",
        _ => "system",
    }
}

/// Resolve a preference against the system appearance (resolveAppearance).
pub fn resolve_appearance(preference: &str, system: &str) -> &'static str {
    match preference.to_ascii_lowercase().as_str() {
        "light" => "light",
        "dark" => "dark",
        _ => {
            if system.eq_ignore_ascii_case("light") {
                "light"
            } else {
                "dark"
            }
        }
    }
}

/// Whether a resolved appearance is dark.
pub fn is_dark(appearance: &str) -> bool {
    resolve_appearance(appearance, "dark") == "dark"
}

/// Mobile palette for an appearance: background, text and accent (tokensForAppearance).
pub fn tokens_for_appearance(appearance: &str) -> (&'static str, &'static str, &'static str) {
    if is_dark(appearance) { ("#101014", "#F5F5F7", "#E7712E") } else { ("#FFFFFF", "#1C1C1E", "#C05621") }
}

/// Web/electron palette for an appearance, with its brighter accent (webTokensForAppearance).
pub fn web_tokens_for_appearance(appearance: &str) -> (&'static str, &'static str, &'static str) {
    if is_dark(appearance) { ("#0B0B0D", "#F5F5F7", "#F5A03C") } else { ("#FFFFFF", "#1C1C1E", "#B7791F") }
}

/// CSS custom-property name for a token (`cssVariableName`).
pub fn css_variable_name(token: &str) -> String {
    format!("--color-{}", crate::bdots_textfmt::slugify(token))
}

/// Font size in points for a type-scale role (typeScale).
pub fn type_size(role: &str) -> u32 {
    match role.to_ascii_lowercase().as_str() {
        "title" => 22,
        "headline" => 19,
        "cardtitle" | "card_title" => 17,
        "body" => 17,
        "secondary" => 15,
        "caption" => 14,
        "footnote" => 12,
        _ => 17,
    }
}

/// Line height in points for a type-scale role (typeScale).
pub fn type_line_height(role: &str) -> u32 {
    match role.to_ascii_lowercase().as_str() {
        "title" => 26,
        "headline" => 23,
        "cardtitle" | "card_title" | "body" => 22,
        "secondary" => 19,
        "caption" => 18,
        "footnote" => 16,
        _ => 22,
    }
}

/// CSS font weight for a role (fontWeight).
pub fn font_weight(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "medium" => "500",
        "semibold" => "600",
        _ => "400",
    }
}

/// Clamp a user font scale into the supported range (MAX_FONT_SCALE).
pub fn clamp_font_scale(scale: f64) -> f64 {
    if !scale.is_finite() { 1.0 } else { scale.clamp(1.0, max_font_scale()) }
}

/// Scaled type size for a role, rounded to points.
pub fn scaled_type_size(role: &str, scale: f64) -> u32 {
    (type_size(role) as f64 * clamp_font_scale(scale)).round() as u32
}

/// A bot colour by index, wrapping around (`botColors`).
pub fn bot_color(index: usize) -> &'static str {
    BOT_COLORS[index % BOT_COLORS.len()]
}

/// A page bar colour by index, wrapping around (`pageIconBars`).
pub fn page_bar_color(index: usize) -> &'static str {
    PAGE_BARS[index % PAGE_BARS.len()]
}

/// Human label for an appearance preference.
pub fn appearance_label(preference: &str) -> &'static str {
    match preference.to_ascii_lowercase().as_str() {
        "light" => "Light",
        "dark" => "Dark",
        _ => "System",
    }
}

/// SF Symbol for an appearance preference.
pub fn appearance_icon(preference: &str) -> &'static str {
    match preference.to_ascii_lowercase().as_str() {
        "light" => "sun.max.fill",
        "dark" => "moon.stars.fill",
        _ => "gearshape.2.fill",
    }
}

/// Parse a `#RRGGBB` colour into a relative luminance (0–1).
pub fn hex_luminance(hex: &str) -> f64 {
    let value = hex.trim().trim_start_matches('#');
    if value.len() != 6 {
        return 0.0;
    }
    let Ok(r) = u8::from_str_radix(&value[0..2], 16) else {
        return 0.0;
    };
    let Ok(g) = u8::from_str_radix(&value[2..4], 16) else {
        return 0.0;
    };
    let Ok(b) = u8::from_str_radix(&value[4..6], 16) else {
        return 0.0;
    };
    (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0
}

/// Foreground colour with readable contrast on a background.
pub fn contrast_foreground(background: &str) -> &'static str {
    if hex_luminance(background) > 0.55 { "#101014" } else { "#FFFFFF" }
}

/// Blend two `#RRGGBB` colours by `t` (0 = a, 1 = b).
pub fn blend_hex(a: &str, b: &str, t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let left = hex_channels(a);
    let right = hex_channels(b);
    let mix: Vec<String> = left
        .iter()
        .zip(right)
        .map(|(x, y)| format!("{:02X}", (f64::from(*x) * (1.0 - t) + f64::from(y) * t).round() as u8))
        .collect();
    format!("#{}", mix.concat())
}

/// Channels of a `#RRGGBB` colour, or black when malformed.
fn hex_channels(hex: &str) -> [u8; 3] {
    let value = hex.trim().trim_start_matches('#');
    if value.len() != 6 {
        return [0, 0, 0];
    }
    let channel = |from: usize| u8::from_str_radix(&value[from..from + 2], 16).unwrap_or(0);
    [channel(0), channel(2), channel(4)]
}

/// A colour as an `rgba(...)` string with the given opacity.
pub fn with_alpha(hex: &str, alpha: f64) -> String {
    let [r, g, b] = hex_channels(hex);
    format!("rgba({r}, {g}, {b}, {:.2})", alpha.clamp(0.0, 1.0))
}

/// Count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(preference: &str, appearance: &str, scale: f64) -> Vec<(&'static str, String)> {
    vec![
        ("preference", appearance_label(preference).to_owned()),
        ("appearance", crate::bdots_textfmt::upper_first(appearance)),
        ("scale", format!("{:.2}×", clamp_font_scale(scale))),
        ("body", format!("{} pt", scaled_type_size("body", scale))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences() {
        assert!(is_appearance_preference(Some("DARK")));
        assert!(!is_appearance_preference(Some("auto")));
        assert_eq!(normalize_appearance(Some("light")), "light");
        assert_eq!(normalize_appearance(Some("weird")), "system");
        assert_eq!(resolve_appearance("system", "light"), "light");
        assert_eq!(resolve_appearance("dark", "light"), "dark");
        assert!(is_dark("system"));
        assert!(!is_dark("light"));
    }

    #[test]
    fn palettes_and_css() {
        assert_eq!(tokens_for_appearance("dark").0, "#101014");
        assert_eq!(tokens_for_appearance("light").0, "#FFFFFF");
        assert_eq!(web_tokens_for_appearance("dark").2, "#F5A03C");
        assert_eq!(css_variable_name("Panel BG"), "--color-panel-bg");
        assert_eq!(appearance_label("system"), "System");
        assert_eq!(appearance_icon("dark"), "moon.stars.fill");
    }

    #[test]
    fn type_scale() {
        assert_eq!(type_size("title"), 22);
        assert_eq!(type_line_height("footnote"), 16);
        assert_eq!(font_weight("semibold"), "600");
        assert_eq!(font_weight("regular"), "400");
        assert_eq!(max_font_scale(), 23.0 / 17.0);
        assert_eq!(clamp_font_scale(99.0), max_font_scale());
        assert_eq!(clamp_font_scale(f64::NAN), 1.0);
        assert_eq!(scaled_type_size("body", 1.0), 17);
    }

    #[test]
    fn colours() {
        assert_eq!(bot_color(7), BOT_COLORS[0]);
        assert_eq!(page_bar_color(5), PAGE_BARS[1]);
        assert_eq!(BOT_EYE_COLOR, "#101014");
        assert!(hex_luminance("#FFFFFF") > hex_luminance("#000000"));
        assert_eq!(contrast_foreground("#FFFFFF"), "#101014");
        assert_eq!(contrast_foreground("#000000"), "#FFFFFF");
        assert_eq!(blend_hex("#000000", "#FFFFFF", 0.5), "#808080");
        assert_eq!(with_alpha("#3EC5A8", 0.5), "rgba(62, 197, 168, 0.50)");
    }

    #[test]
    fn summary_rows() {
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("system", "dark", 1.2).len(), 4);
    }
}
