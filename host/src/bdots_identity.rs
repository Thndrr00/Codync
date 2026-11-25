#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for how a bot appears in the roster: avatar, section, pin/lead position and
//! notifications. Reuses the real avatar constants from `store::model` and the shared helpers
//! in `bdots_text` / `bdots_more`. Only `botInsights` calls this.

/// One roster attribute reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Trait {
    pub id: &'static str,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub on: bool,
}

fn tr(id: &'static str, label: &str, detail: String, icon: &'static str, tone: &'static str, on: bool) -> Trait {
    Trait { id, label: label.to_owned(), detail, icon, tone, on }
}

/// Human label for an avatar colour, straight from the repo's palette.
pub fn color_label(color: &str) -> String {
    let lower = color.to_ascii_lowercase();
    if crate::store::model::AVATAR_COLORS.contains(&lower.as_str()) { lower } else { "blue".to_owned() }
}

/// Human label for an avatar shape, from the repo's shapes.
pub fn shape_label(shape: &str) -> String {
    let lower = shape.to_ascii_lowercase();
    if crate::store::model::AVATAR_SHAPES.contains(&lower.as_str()) { lower } else { "blob".to_owned() }
}

/// Human label for a geometric robot look, from the repo's looks.
pub fn look_label(look: Option<&str>) -> String {
    look.filter(|l| crate::store::model::BOT_AVATAR_LOOKS.contains(&l.to_ascii_lowercase().as_str()))
        .map_or_else(|| "default".to_owned(), str::to_owned)
}

/// SF Symbol for a colour, shared by the four clients.
pub fn color_icon(color: &str) -> &'static str {
    match color.to_ascii_lowercase().as_str() {
        "red" | "orange" => "flame.fill",
        "green" => "leaf.fill",
        "blue" | "cyan" => "drop.fill",
        "violet" | "magenta" => "sparkles",
        "yellow" => "sun.fill",
        _ => "circle.fill",
    }
}

/// SF Symbol for a trait's on/off state.
pub fn state_icon(on: bool) -> &'static str {
    if on { "checkmark.circle.fill" } else { "circle" }
}

/// Semantic tone for a trait's on/off state.
pub fn state_tone(on: bool) -> &'static str {
    if on { "good" } else { "secondary" }
}

/// Band in the roster: lead, pinned, or regular.
pub fn band(lead: bool, pinned: bool) -> &'static str {
    if lead {
        "lead"
    } else if pinned {
        "pinned"
    } else {
        "regular"
    }
}

/// SF Symbol for a roster band.
pub fn band_icon(band_name: &str) -> &'static str {
    match band_name {
        "lead" => "crown.fill",
        "pinned" => "pin.fill",
        _ => "person.circle",
    }
}

/// Semantic tone for a roster band.
pub fn band_tone(band_name: &str) -> &'static str {
    match band_name {
        "lead" => "warn",
        "pinned" => "accent",
        _ => "secondary",
    }
}

/// Human label for a section, or the default.
pub fn section_label(section: Option<&str>) -> String {
    section.filter(|s| !s.trim().is_empty()).map_or_else(|| "Bots".to_owned(), str::to_owned)
}

/// Human label for an agent mode, or the default.
pub fn mode_label(mode: Option<&str>) -> String {
    mode.filter(|m| !m.trim().is_empty()).map_or_else(|| "Default".to_owned(), str::to_owned)
}

/// Whether the bot has a custom avatar photo.
pub fn has_photo(photo: Option<&str>) -> bool {
    photo.is_some_and(|p| p.starts_with("data:image/"))
}

/// Whether the bot was created from the marketplace catalogue.
pub fn from_catalog(catalog: Option<&str>) -> bool {
    catalog.is_some_and(|c| !c.trim().is_empty())
}

/// Compact display label for a catalogue entry, reusing the shortener.
pub fn catalog_label(catalog: Option<&str>) -> String {
    catalog
        .filter(|c| !c.trim().is_empty())
        .map(|c| crate::bdots_more::shorten(c, 48))
        .unwrap_or_else(|| "Made here".to_owned())
}

/// Normalise a position into a 1-based roster rank.
pub fn rank(position: i64) -> u32 {
    (position.max(0) as u64).saturating_add(1).min(u32::MAX as u64) as u32
}

/// Whether the bot is visible in the roster.
pub fn is_visible(hidden: bool) -> bool {
    !hidden
}

/// Notification label for a bot.
pub fn notify_label(notify: Option<bool>) -> &'static str {
    match notify {
        Some(true) => "On",
        Some(false) => "Off",
        None => "Default",
    }
}

/// One-line identity headline.
pub fn identity_headline(name: &str, band_name: &str, section: &str) -> String {
    let display = if name.trim().is_empty() { "Unnamed bot" } else { name };
    format!("{display} · {band_name} in {section}")
}

/// Semantic tone for the identity card.
pub fn identity_tone(name: &str, hidden: bool) -> &'static str {
    if name.trim().is_empty() || hidden { "warn" } else { "good" }
}

/// Build every roster trait for one bot.
pub fn build_traits(
    color: &str,
    shape: &str,
    look: Option<&str>,
    photo: Option<&str>,
    pinned: bool,
    lead: bool,
    hidden: bool,
    notify: Option<bool>,
    section: Option<&str>,
    mode: Option<&str>,
    catalog: Option<&str>,
) -> Vec<Trait> {
    let band_name = band(lead, pinned);
    vec![
        tr("color", "Colour", color_label(color), color_icon(color), "accent", true),
        tr("shape", "Shape", shape_label(shape), "square.on.circle", "secondary", true),
        tr("look", "Look", look_label(look), "face.smiling", "good", look.is_some()),
        tr(
            "photo",
            "Photo",
            if has_photo(photo) { "Custom image".into() } else { "Generated".into() },
            "photo",
            "good",
            has_photo(photo),
        ),
        tr("band", "Roster", band_name.to_owned(), band_icon(band_name), band_tone(band_name), lead || pinned),
        tr("section", "Section", section_label(section), "folder.fill", "secondary", section.is_some()),
        tr("mode", "Mode", mode_label(mode), "gearshape", "accent", mode.is_some()),
        tr("catalog", "Source", catalog_label(catalog), "bag.fill", "good", from_catalog(catalog)),
        tr(
            "visible",
            "Visible",
            if hidden { "Hidden from roster" } else { "Shown in roster" }.into(),
            "eye.fill",
            "secondary",
            is_visible(hidden),
        ),
        tr("notify", "Notifications", notify_label(notify).to_owned(), "bell.fill", "good", notify == Some(true)),
    ]
}

/// Count traits that are on.
pub fn enabled_count(traits: &[Trait]) -> usize {
    traits.iter().filter(|t| t.on).count()
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(traits: &[Trait]) -> String {
    crate::bdots_desk::compact_count(traits.len() as u64)
}

/// Assemble the identity summary.
pub fn summary(traits: &[Trait]) -> Vec<(&'static str, String)> {
    vec![
        ("traits", count_label(traits)),
        ("on", enabled_count(traits).to_string()),
        ("off", (traits.len() - enabled_count(traits)).to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traits() -> Vec<Trait> {
        build_traits(
            "blue",
            "blob",
            Some("round"),
            None,
            true,
            false,
            false,
            Some(true),
            Some("Work"),
            Some("auto"),
            Some("bot"),
        )
    }

    #[test]
    fn palette_and_shapes_use_repo_constants() {
        assert_eq!(color_label("BLUE"), "blue");
        assert_eq!(color_label("unknown"), "blue");
        assert_eq!(shape_label("squircle"), "squircle");
        assert_eq!(shape_label("bad"), "blob");
        assert_eq!(look_label(Some("owl")), "owl");
        assert_eq!(look_label(Some("bad")), "default");
        assert_eq!(color_icon("red"), "flame.fill");
        assert_eq!(color_icon("gray"), "circle.fill");
    }

    #[test]
    fn bands_sections_and_modes() {
        assert_eq!(band(true, true), "lead");
        assert_eq!(band(false, true), "pinned");
        assert_eq!(band(false, false), "regular");
        assert_eq!(band_icon("lead"), "crown.fill");
        assert_eq!(band_tone("pinned"), "accent");
        assert_eq!(section_label(None), "Bots");
        assert_eq!(section_label(Some(" Work ")), " Work ");
        assert_eq!(mode_label(None), "Default");
        assert_eq!(notify_label(None), "Default");
        assert_eq!(notify_label(Some(true)), "On");
    }

    #[test]
    fn photos_catalog_and_rank() {
        assert!(has_photo(Some("data:image/png;base64,aa")));
        assert!(!has_photo(Some("http://x")));
        assert!(from_catalog(Some("bot")));
        assert!(!from_catalog(None));
        assert_eq!(catalog_label(None), "Made here");
        assert_eq!(rank(-5), 1);
        assert_eq!(rank(3), 4);
        assert!(is_visible(false));
        assert!(!is_visible(true));
    }

    #[test]
    fn headline_tone_and_state() {
        assert!(identity_headline("Ada", "lead", "Work").contains("Ada"));
        assert_eq!(identity_tone("", false), "warn");
        assert_eq!(identity_tone("Ada", false), "good");
        assert_eq!(state_icon(true), "checkmark.circle.fill");
        assert_eq!(state_tone(false), "secondary");
    }

    #[test]
    fn build_summary_and_counts() {
        let t = traits();
        assert_eq!(t.len(), 10);
        assert!(t.iter().any(|x| x.id == "band" && x.on));
        assert!(enabled_count(&t) > 0);
        assert_eq!(count_label(&t), "10");
        assert_eq!(summary(&t).len(), 3);
    }
}
