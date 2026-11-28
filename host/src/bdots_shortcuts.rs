#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `shell-shortcuts.ts`: modifier resolution, shortcut chords,
//! key labels and shortcut descriptions for the host-computed Insights card. Composes helpers
//! already ported into this host. Only `botInsights` calls this.

/// Capabilities of the host handling a key press (ShortcutHost).
#[derive(Clone, Copy, Debug, Default)]
pub struct ShortcutHost {
    /// Whether the host is the desktop app rather than a browser.
    pub desktop: bool,
    /// Whether the host uses Apple modifiers.
    pub apple: bool,
}

/// Modifiers carried by a key event.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyMods {
    pub meta: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub repeat: bool,
    pub composing: bool,
}

/// Whether a key target is editable, so shortcuts must not steal it (isEditableTarget).
pub fn is_editable(tag: &str, content_editable: bool) -> bool {
    content_editable || matches!(tag.to_ascii_lowercase().as_str(), "input" | "textarea" | "select")
}

/// Cmd on Apple keyboards and Ctrl elsewhere, held without the other (primaryModifier).
pub fn primary_modifier(keys: KeyMods, apple: bool) -> bool {
    if apple { keys.meta && !keys.ctrl } else { keys.ctrl && !keys.meta }
}

/// Whether a press is a plain primary chord with no extra modifiers.
pub fn is_plain_chord(keys: KeyMods, apple: bool) -> bool {
    primary_modifier(keys, apple) && !keys.alt && !keys.shift && !keys.repeat
}

/// Whether a press is a primary+shift chord with no extra modifiers.
pub fn is_shift_chord(keys: KeyMods, apple: bool) -> bool {
    primary_modifier(keys, apple) && keys.shift && !keys.alt && !keys.repeat
}

/// The key of the quote chord: web needs Shift, desktop does not (quoteChord).
pub fn quote_shift(host: ShortcutHost) -> bool {
    !host.desktop
}

/// Whether a press is the quote-selection shortcut (isQuoteShortcut).
pub fn is_quote_shortcut(key: &str, keys: KeyMods, host: ShortcutHost) -> bool {
    if keys.repeat || keys.alt || !primary_modifier(keys, host.apple) {
        return false;
    }
    key.eq_ignore_ascii_case("l") && keys.shift == quote_shift(host)
}

/// Human label of the quote chord, as shown in the shortcut pill (quoteShortcutLabel).
pub fn quote_shortcut_label(host: ShortcutHost) -> String {
    let mut parts = vec![if host.apple { "⌘" } else { "Ctrl" }];
    if quote_shift(host) {
        parts.push(if host.apple { "⇧" } else { "Shift" });
    }
    parts.push("L");
    if host.apple { parts.concat() } else { parts.join("+") }
}

/// The shell command a key press asks for, as a stable name (shellShortcut).
pub fn shell_shortcut(key: &str, keys: KeyMods, host: ShortcutHost) -> Option<&'static str> {
    if keys.repeat || key.is_empty() {
        return None;
    }
    let lower = key.to_ascii_lowercase();
    if is_quote_shortcut(key, keys, host) {
        return Some("quote-selection");
    }
    if (keys.meta || keys.ctrl) && !keys.alt && !keys.shift {
        if lower == "k" {
            return Some("command-menu");
        }
        if lower == "n" {
            return Some("new");
        }
    }
    if primary_modifier(keys, host.apple) && !keys.alt && key == "/" {
        return Some("shortcuts");
    }
    if is_plain_chord(keys, host.apple) {
        return match lower.as_str() {
            "b" => Some("toggle-sidebar"),
            "i" => Some("focus-composer"),
            "f" if host.desktop => Some("find"),
            n if host.desktop && n.len() == 1 && n.as_bytes()[0].is_ascii_digit() && n != "0" => Some("bot"),
            "[" if host.desktop && host.apple => Some("back"),
            "]" if host.desktop && host.apple => Some("forward"),
            _ => None,
        };
    }
    if is_shift_chord(keys, host.apple) {
        return match lower.as_str() {
            "n" if host.desktop => Some("new-group"),
            "f" if host.desktop => Some("search"),
            "m" => Some("integrations"),
            _ => None,
        };
    }
    if key.eq_ignore_ascii_case("tab") && keys.ctrl && !keys.meta && !keys.alt {
        return Some(if keys.shift { "previous-bot" } else { "next-bot" });
    }
    if matches!(key, "ArrowDown" | "ArrowUp") && (keys.meta || keys.ctrl) && keys.shift && !keys.alt {
        return Some(if key == "ArrowDown" { "next-bot" } else { "previous-bot" });
    }
    None
}

/// Whether a browser owns the plain chord and the page must use Shift instead.
pub fn browser_needs_shift(command: &str, host: ShortcutHost) -> bool {
    !host.desktop && matches!(command, "quote-selection" | "find" | "bot")
}

/// Whether a shortcut is available on the web as well as desktop.
pub fn works_in_browser(command: &str) -> bool {
    !matches!(command, "find" | "bot" | "new-group" | "search" | "back" | "forward")
}

/// Display label for a shell command, reusing the shared upperFirst.
pub fn command_label(command: &str) -> String {
    let spaced = command.replace('-', " ");
    crate::bdots_textfmt::upper_first(&spaced)
}

/// SF Symbol for a shell command.
pub fn command_icon(command: &str) -> &'static str {
    match command {
        "command-menu" => "command",
        "new" | "new-group" => "plus.circle",
        "focus-composer" => "pencil.circle",
        "quote-selection" => "quote.bubble",
        "toggle-sidebar" => "sidebar.left",
        "find" | "search" => "magnifyingglass",
        "next-bot" | "previous-bot" | "bot" => "arrow.left.arrow.right",
        "back" | "forward" => "arrow.uturn.left",
        "shortcuts" => "keyboard",
        _ => "questionmark.circle",
    }
}

/// Semantic tone for a command's availability.
pub fn command_tone(command: &str, host: ShortcutHost) -> &'static str {
    if works_in_browser(command) || host.desktop { "good" } else { "attention" }
}

/// Description of a command for the card, reusing the locale label helper.
pub fn command_detail(command: &str, host: ShortcutHost) -> String {
    let place = if host.desktop { "desktop" } else { "web" };
    format!("{} · {}", command_label(command), crate::bdots_i18n::locale_intl("en").replace("en-US", place))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(commands: usize, desktop_only: usize, apple: bool) -> Vec<(&'static str, String)> {
    vec![
        ("commands", count_label(commands)),
        ("desktop", count_label(desktop_only)),
        ("modifier", if apple { "⌘" } else { "Ctrl" }.to_owned()),
        ("mode", if apple { "Apple" } else { "Other" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> ShortcutHost {
        ShortcutHost { desktop: true, apple: false }
    }

    #[test]
    fn targets_and_modifiers() {
        assert!(is_editable("INPUT", false));
        assert!(is_editable("div", true));
        assert!(!is_editable("DIV", false));
        let keys = KeyMods { ctrl: true, ..KeyMods::default() };
        assert!(primary_modifier(keys, false));
        assert!(!primary_modifier(keys, true));
        assert!(is_plain_chord(keys, false));
        assert!(!is_shift_chord(keys, false));
    }

    #[test]
    fn quote_chords() {
        let keys = KeyMods { meta: true, ..KeyMods::default() };
        let apple = ShortcutHost { desktop: true, apple: true };
        assert!(is_quote_shortcut("L", keys, apple));
        assert_eq!(quote_shortcut_label(apple), "⌘L");
        assert_eq!(quote_shortcut_label(host()), "Ctrl+L");
        assert_eq!(quote_shortcut_label(ShortcutHost::default()), "Ctrl+Shift+L");
        assert!(browser_needs_shift("quote-selection", ShortcutHost::default()));
    }

    #[test]
    fn command_resolution() {
        let keys = KeyMods { ctrl: true, ..KeyMods::default() };
        assert_eq!(shell_shortcut("k", keys, host()), Some("command-menu"));
        assert_eq!(shell_shortcut("b", keys, host()), Some("toggle-sidebar"));
        assert_eq!(shell_shortcut("1", keys, host()), Some("bot"));
        assert_eq!(shell_shortcut("x", keys, host()), None);
        let shifted = KeyMods { ctrl: true, shift: true, ..KeyMods::default() };
        assert_eq!(shell_shortcut("m", shifted, host()), Some("integrations"));
    }

    #[test]
    fn navigation() {
        let tab = KeyMods { ctrl: true, shift: true, ..KeyMods::default() };
        assert_eq!(shell_shortcut("Tab", tab, host()), Some("previous-bot"));
        let down = KeyMods { meta: true, shift: true, ..KeyMods::default() };
        assert_eq!(shell_shortcut("ArrowDown", down, host()), Some("next-bot"));
    }

    #[test]
    fn presentation() {
        assert!(works_in_browser("command-menu"));
        assert!(!works_in_browser("find"));
        assert_eq!(command_label("toggle-sidebar"), "Toggle sidebar");
        assert_eq!(command_icon("find"), "magnifyingglass");
        assert_eq!(command_tone("find", ShortcutHost::default()), "attention");
        assert!(command_detail("new", host()).contains("New"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(10, 3, false).len(), 4);
    }
}
