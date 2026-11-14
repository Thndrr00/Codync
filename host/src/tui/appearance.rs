//! Appearance belongs to this terminal client, never the connected host.

use anyhow::Result;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::LockExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Appearance {
    System,
    Light,
    Dark,
}

impl Appearance {
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

const FILE: &str = "tui-appearance";

fn load() -> Appearance {
    match super::choice::read(FILE) {
        Some(value) => Appearance::parse(&value),
        None => std::env::var("CODYNC_THEME").ok().and_then(|value| Appearance::parse(&value)),
    }
    .unwrap_or(Appearance::System)
}

static SELECTED: LazyLock<AtomicU8> = LazyLock::new(|| AtomicU8::new(load() as u8));

pub fn current() -> Appearance {
    match SELECTED.load(Ordering::Relaxed) {
        1 => Appearance::Light,
        2 => Appearance::Dark,
        _ => Appearance::System,
    }
}

pub fn select(mode: Appearance) -> Result<()> {
    super::choice::write(FILE, mode.name())?;
    SELECTED.store(mode as u8, Ordering::Relaxed);
    Ok(())
}

pub fn system_is_light() -> bool {
    std::env::var("COLORFGBG")
        .ok()
        .and_then(|value| value.rsplit(';').next().and_then(|background| background.parse::<u8>().ok()))
        .is_some_and(|background| background == 7 || background == 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_choices_round_trip_and_unknown_values_are_rejected() {
        for mode in [Appearance::System, Appearance::Light, Appearance::Dark] {
            assert_eq!(Appearance::parse(mode.name()), Some(mode));
        }
        assert_eq!(Appearance::parse("unknown"), None);
    }
}

/// One of the 11 chat colours OpenBot offers, with the shade that reads on each theme.
pub struct ChatColour {
    pub id: &'static str,
    pub light: u32,
    pub dark: u32,
}

pub const CHAT_COLOURS: [ChatColour; 11] = [
    ChatColour { id: "red", light: 0xE35652, dark: 0xF8706D },
    ChatColour { id: "orange", light: 0xD86706, dark: 0xED8125 },
    ChatColour { id: "yellow", light: 0xD39809, dark: 0xE8B027 },
    ChatColour { id: "green", light: 0x06A74E, dark: 0x25BF69 },
    ChatColour { id: "cyan", light: 0x0DA195, dark: 0x2BB9AD },
    ChatColour { id: "sky", light: 0x069BC2, dark: 0x24B2D8 },
    ChatColour { id: "blue", light: 0x408DEA, dark: 0x5CA5FE },
    ChatColour { id: "indigo", light: 0x7B7DEB, dark: 0x9596FF },
    ChatColour { id: "violet", light: 0xA66BE0, dark: 0xBD85F5 },
    ChatColour { id: "magenta", light: 0xCA5BB6, dark: 0xE076CD },
    ChatColour { id: "pink", light: 0xDD5487, dark: 0xF26FA0 },
];

/// The calm tint behind the chat for each scheme, in the same order as [`CHAT_COLOURS`].
pub struct ChatBackground {
    pub id: &'static str,
    pub light: u32,
    pub dark: u32,
}

pub const CHAT_BACKGROUNDS: [ChatBackground; 11] = [
    ChatBackground { id: "red", light: 0xFFEFED, dark: 0x311B19 },
    ChatBackground { id: "orange", light: 0xFEF0E9, dark: 0x2F1D12 },
    ChatBackground { id: "yellow", light: 0xFAF2E6, dark: 0x2A200D },
    ChatBackground { id: "green", light: 0xEBF7ED, dark: 0x152619 },
    ChatBackground { id: "cyan", light: 0xE7F8F5, dark: 0x0A2724 },
    ChatBackground { id: "sky", light: 0xE7F6FD, dark: 0x0C252E },
    ChatBackground { id: "blue", light: 0xECF4FF, dark: 0x162232 },
    ChatBackground { id: "indigo", light: 0xF1F2FF, dark: 0x1E2032 },
    ChatBackground { id: "violet", light: 0xF6F1FE, dark: 0x251D2F },
    ChatBackground { id: "magenta", light: 0xFCEFF8, dark: 0x2C1B28 },
    ChatBackground { id: "pink", light: 0xFFEFF3, dark: 0x2F1A21 },
];

/// A saved chat colour or background: one choice per appearance, `None` for the theme's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Choice {
    pub light: Option<&'static str>,
    pub dark: Option<&'static str>,
}

impl Choice {
    /// "light/dark", `-` for none. A single saved id from an older build means both themes, so an
    /// existing colour survives this change.
    pub fn decode(stored: &str, known: &[&str]) -> Self {
        let value = stored.trim();
        let (light, dark) = match value.split_once('/') {
            Some((light, dark)) => (light, dark),
            None => (value, value),
        };
        // A `&str` from the file is not `'static`; keep the canonical id from `known`.
        let pick = |id: &str| {
            let id = id.trim();
            known.iter().find(|candidate| **candidate == id).copied()
        };
        Self { light: pick(light), dark: pick(dark) }
    }

    pub fn encode(self) -> String {
        let part = |id: Option<&str>| id.unwrap_or("-");
        format!("{}/{}", part(self.light), part(self.dark))
    }

    pub fn for_scheme(self, dark: bool) -> Option<&'static str> {
        if dark { self.dark } else { self.light }
    }

    /// The next choice in the list, with "none" first, then wraps back to it.
    pub fn cycle(self, dark: bool, ids: &[&'static str]) -> Self {
        let current = self.for_scheme(dark);
        let next = match current {
            None => ids.first().copied(),
            Some(id) => {
                let at = ids.iter().position(|candidate| *candidate == id);
                at.and_then(|at| ids.get(at + 1)).copied()
            }
        };
        if dark { Self { dark: next, ..self } } else { Self { light: next, ..self } }
    }
}

pub fn chat_colour(id: &str) -> Option<&'static ChatColour> {
    CHAT_COLOURS.iter().find(|colour| colour.id == id)
}

pub fn chat_background(id: &str) -> Option<&'static ChatBackground> {
    CHAT_BACKGROUNDS.iter().find(|background| background.id == id)
}

pub fn chat_colour_ids() -> Vec<&'static str> {
    CHAT_COLOURS.iter().map(|colour| colour.id).collect()
}

pub fn chat_background_ids() -> Vec<&'static str> {
    CHAT_BACKGROUNDS.iter().map(|background| background.id).collect()
}

const COLOUR_FILE: &str = "tui-chat-colour";
const BACKGROUND_FILE: &str = "tui-chat-background";

static COLOUR: LazyLock<Mutex<Choice>> = LazyLock::new(|| {
    let ids = chat_colour_ids();
    Mutex::new(super::choice::read(COLOUR_FILE).map_or(Choice::default(), |text| Choice::decode(&text, &ids)))
});

static BACKGROUND: LazyLock<Mutex<Choice>> = LazyLock::new(|| {
    let ids = chat_background_ids();
    Mutex::new(super::choice::read(BACKGROUND_FILE).map_or(Choice::default(), |text| Choice::decode(&text, &ids)))
});

pub fn dark_now() -> bool {
    match current() {
        Appearance::Light => false,
        Appearance::Dark => true,
        Appearance::System => !system_is_light(),
    }
}

/// The saved chat colour for the scheme the terminal shows now.
pub fn chat_colour_now() -> Option<&'static ChatColour> {
    let id = COLOUR.locked().for_scheme(dark_now())?;
    chat_colour(id)
}

/// The saved chat background for the scheme the terminal shows now.
pub fn chat_background_now() -> Option<&'static ChatBackground> {
    let id = BACKGROUND.locked().for_scheme(dark_now())?;
    chat_background(id)
}

/// Cycles the chat colour of the scheme the terminal shows now; the label names the new choice.
pub fn cycle_chat_colour() -> Result<String> {
    let ids = chat_colour_ids();
    let dark = dark_now();
    let next = {
        let mut choice = COLOUR.locked();
        let next = choice.cycle(dark, &ids);
        *choice = next;
        next
    };
    super::choice::write(COLOUR_FILE, &next.encode())?;
    Ok(next.for_scheme(dark).unwrap_or("none").to_owned())
}

/// Cycles the chat background of the scheme the terminal shows now; the label names the new choice.
pub fn cycle_chat_background() -> Result<String> {
    let ids = chat_background_ids();
    let dark = dark_now();
    let next = {
        let mut choice = BACKGROUND.locked();
        let next = choice.cycle(dark, &ids);
        *choice = next;
        next
    };
    super::choice::write(BACKGROUND_FILE, &next.encode())?;
    Ok(next.for_scheme(dark).unwrap_or("none").to_owned())
}

#[cfg(test)]
mod choice_tests {
    use super::*;

    const IDS: [&str; 3] = ["red", "green", "blue"];

    #[test]
    fn a_single_saved_colour_becomes_both_themes() {
        let choice = Choice::decode("green", &IDS);
        assert_eq!(choice.light, Some("green"));
        assert_eq!(choice.dark, Some("green"));
    }

    #[test]
    fn each_theme_keeps_its_own_choice_and_none_is_a_dash() {
        let choice = Choice::decode("red/-", &IDS);
        assert_eq!(choice.light, Some("red"));
        assert_eq!(choice.dark, None);
        assert_eq!(choice.encode(), "red/-");
        assert_eq!(Choice::decode("-/blue", &IDS).dark, Some("blue"));
        assert_eq!(Choice::decode("", &IDS), Choice::default());
        assert_eq!(Choice::decode("purple", &IDS), Choice::default());
    }

    #[test]
    fn cycling_walks_the_list_then_returns_to_none() {
        let mut choice = Choice::default();
        for expected in [Some("red"), Some("green"), Some("blue"), None] {
            choice = choice.cycle(false, &IDS);
            assert_eq!(choice.light, expected);
        }
        // The other theme is untouched.
        assert_eq!(choice.dark, None);
    }

    #[test]
    fn every_preset_has_a_light_and_a_dark_shade() {
        assert_eq!(CHAT_COLOURS.len(), 11);
        assert_eq!(CHAT_BACKGROUNDS.len(), 11);
        for colour in &CHAT_COLOURS {
            assert_ne!(colour.light, colour.dark, "{}", colour.id);
            assert!(CHAT_BACKGROUNDS.iter().any(|background| background.id == colour.id));
        }
        assert_eq!(chat_colour("red").map(|colour| colour.light), Some(0xE35652));
    }
}
