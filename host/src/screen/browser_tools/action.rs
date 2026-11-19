//! What one browser tool call asks for: the arguments parsed into the one action it runs. Ported
//! from `OpenBot` nightly's `src/backend/browser-tools.ts` (the same field names, defaults and
//! limits), without what Codync does not take: a tab id, the vault, card fills, takeovers, images,
//! recording, uploads, environments and dialogs.

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::time::Duration;

/// How long an action waits for the page when the call names no `timeoutMs`.
pub(super) const DEFAULT_TIMEOUT: Duration = Duration::from_millis(DEFAULT_TIMEOUT_MS);
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
/// The longest one action may wait. The MCP bridge gives a chat tool call 30 s, so a longer wait
/// would answer the agent with a broken call while the page work went on.
const MAX_TIMEOUT_MS: u64 = 20_000;
/// The most text one `type` call enters, in characters.
const MAX_TEXT_CHARS: usize = 20_000;
/// The most JavaScript one `evaluate` call takes, in characters.
const MAX_EXPRESSION_CHARS: usize = 64_000;
/// The most steps of one `run`.
const MAX_STEPS: usize = 200;
/// The longest one `run` step may sleep, in milliseconds.
const MAX_SLEEP_MS: u64 = 25_000;
/// The default and longest time a whole `run` may take, in milliseconds.
const DEFAULT_RUN_MS: u64 = 25_000;
const MAX_RUN_MS: u64 = 25_000;
/// The most options one `select_option` call selects.
const MAX_VALUES: usize = 100;
const MAX_VALUE_CHARS: usize = 1_000;
/// The longest address a `navigate` call takes, in characters.
const MAX_URL_CHARS: usize = 2_048;
/// The longest text a `wait_for` call looks for, in characters.
const MAX_WAIT_TEXT_CHARS: usize = 2_000;
/// The largest ref a snapshot can hand out.
const MAX_REF: u32 = 100_000;
/// The CDP modifier bits.
const ALT: u8 = 1;
const CONTROL: u8 = 2;
const META: u8 = 4;
const SHIFT: u8 = 8;

/// One tool call: which action, how long it may wait, and whether a snapshot follows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Action {
    pub(crate) timeout: Duration,
    /// Take a snapshot after the action, as the tools' `snapshot: true` asks.
    pub(crate) snapshot: bool,
    pub(crate) kind: Kind,
}

/// The action itself. `Sleep` and `Run` run in the host; the rest go to the browser.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    Navigate { url: Option<String>, direction: Option<Direction> },
    Snapshot,
    Click { target: Target, button: Button, count: u8, modifiers: u8 },
    Type { target: Option<Target>, text: String, replace: bool, submit: bool },
    Press { key: Key, target: Option<Target> },
    Hover { target: Target },
    Scroll { target: Option<Target>, x: f64, y: f64 },
    Select { target: Target, values: Vec<String> },
    Check { target: Target, checked: bool },
    Wait { text: Option<String>, url: Option<String>, state: Option<Load> },
    Evaluate { expression: String, await_promise: bool },
    Run { steps: Vec<Action> },
    Sleep { ms: u64 },
}

impl Action {
    /// One tool call, or one step of a `run`, from the tool's name and arguments.
    pub(crate) fn parse(name: &str, args: &Value) -> Result<Self> {
        let timeout = if name == "run" {
            Duration::from_millis(milliseconds(args.get("timeoutMs"), DEFAULT_RUN_MS, MAX_RUN_MS)?)
        } else {
            Duration::from_millis(milliseconds(args.get("timeoutMs"), DEFAULT_TIMEOUT_MS, MAX_TIMEOUT_MS)?)
        };
        Ok(Self { timeout, snapshot: args["snapshot"] == true, kind: kind_of(name, args)? })
    }
}

impl Kind {
    /// The tool name of this action, for a run's answers.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Navigate { .. } => "navigate",
            Self::Snapshot => "snapshot",
            Self::Click { .. } => "click",
            Self::Type { .. } => "type",
            Self::Press { .. } => "press",
            Self::Hover { .. } => "hover",
            Self::Scroll { .. } => "scroll",
            Self::Select { .. } => "select_option",
            Self::Check { .. } => "set_checked",
            Self::Wait { .. } => "wait_for",
            Self::Evaluate { .. } => "evaluate",
            Self::Run { .. } => "run",
            Self::Sleep { .. } => "sleep",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Back,
    Forward,
    Reload,
}

impl Direction {
    fn parse(value: &Value) -> Result<Self> {
        match value.as_str().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("back") => Ok(Self::Back),
            Some("forward") => Ok(Self::Forward),
            Some("reload") => Ok(Self::Reload),
            _ => bail!("A direction is back, forward or reload."),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Button {
    Left,
    Middle,
    Right,
}

impl Button {
    fn parse(value: &Value) -> Result<Self> {
        match value.as_str().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("left") => Ok(Self::Left),
            Some("middle") => Ok(Self::Middle),
            Some("right") => Ok(Self::Right),
            _ => bail!("A button is left, middle or right."),
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Middle => "middle",
            Self::Right => "right",
        }
    }

    /// The CDP `buttons` mask while the button is held.
    pub(crate) fn buttons(self) -> u8 {
        match self {
            Self::Left => 1,
            Self::Middle => 4,
            Self::Right => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Load {
    Load,
    DomContentLoaded,
    Quiet,
}

impl Load {
    fn parse(value: &Value) -> Result<Self> {
        match value.as_str().map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("load") => Ok(Self::Load),
            Some("domcontentloaded") => Ok(Self::DomContentLoaded),
            Some("dom-quiet") => Ok(Self::Quiet),
            _ => bail!("A load state is load, domcontentloaded or dom-quiet."),
        }
    }

    /// What the wait asks the page for, for an error that says what was missing.
    pub(crate) fn waiting(self) -> &'static str {
        match self {
            Self::Load => "finish loading",
            Self::DomContentLoaded => "reach DOMContentLoaded",
            Self::Quiet => "stop changing",
        }
    }
}

/// One element of the page's last snapshot: the number the snapshot printed before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Target(u32);

impl Target {
    /// `7`, `"7"`, `"#7"`, `"[7]"`, `{"ref": 7}` or `{"kind": "ref", "ref": 7}`, as OpenBot's
    /// target union allows but with a ref as the only kind.
    pub(crate) fn parse(value: &Value) -> Result<Self> {
        if let Value::Object(fields) = value {
            if let Some(kind) = fields.get("kind").and_then(Value::as_str)
                && kind != "ref"
            {
                bail!(
                    "This browser tool acts on an element ref from the last snapshot; `{kind}` targets are not supported."
                );
            }
            let number = fields.get("ref").context("Give the element's `ref` number from the last snapshot.")?;
            return Self::parse(number);
        }
        let number = match value {
            Value::Number(number) => number.as_u64(),
            Value::String(text) => {
                text.trim().trim_start_matches('#').trim_start_matches('[').trim_end_matches(']').parse().ok()
            }
            _ => None,
        };
        let number = number.context("Give the element's ref number from the last snapshot, such as 7.")?;
        let number = u32::try_from(number)
            .ok()
            .filter(|number| (1..=MAX_REF).contains(number))
            .context("A ref is a number from 1 to 100000; take a new snapshot.")?;
        Ok(Self(number))
    }

    pub(crate) fn number(self) -> u32 {
        self.0
    }
}

/// One key press, as `Input.dispatchKeyEvent` describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Key {
    /// The CDP `key`: `Enter`, `A`, `ArrowDown`, or the character itself.
    pub(crate) key: String,
    /// The CDP `code` when the key has one: `KeyA`, `Digit1`, `Enter`.
    pub(crate) code: Option<String>,
    pub(crate) vk: u32,
    /// The character the key inserts, unless a shortcut turns it into a command.
    text: Option<String>,
    pub(crate) modifiers: u8,
    /// The key as the call wrote it, for the answer.
    pub(crate) label: String,
}

impl Key {
    /// `Enter`, `Control+A`, `Meta+Shift+P` or one character.
    pub(crate) fn parse(value: &Value) -> Result<Self> {
        let label = value
            .as_str()
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .context("`key` names the key to press, such as Enter or Control+A.")?;
        ensure!(label.chars().count() <= 64, "A key is at most 64 characters.");
        let parts: Vec<&str> = label.split('+').collect();
        let mut modifiers = 0_u8;
        for part in &parts[..parts.len().saturating_sub(1)] {
            let bit = modifier_bit(part)?;
            ensure!(modifiers & bit == 0, "`{}` appears twice in `{label}`.", part.trim());
            modifiers |= bit;
        }
        let name = parts.last().copied().unwrap_or_default().trim();
        ensure!(!name.is_empty(), "`{label}` names no key after its modifiers.");
        if let Some((key, code, vk, text)) = named_key(name) {
            return Ok(Self {
                key: key.to_owned(),
                code: code.map(str::to_owned),
                vk,
                text: text.map(str::to_owned),
                modifiers,
                label: label.to_owned(),
            });
        }
        let mut characters = name.chars();
        let (Some(character), None) = (characters.next(), characters.next()) else {
            bail!(
                "`{name}` is not a key Codync knows: use one character, Enter, Tab, Escape, Backspace, Delete, Space, Home, End, PageUp, PageDown, Insert, an Arrow key, or F1 to F12."
            );
        };
        ensure!(character.is_ascii_graphic() || character == ' ', "`{name}` is not a key Codync can press.");
        let text = if character.is_ascii_alphabetic() && modifiers & SHIFT != 0 {
            character.to_ascii_uppercase()
        } else {
            character
        };
        let code = if character.is_ascii_alphabetic() {
            Some(format!("Key{}", character.to_ascii_uppercase()))
        } else if character.is_ascii_digit() {
            Some(format!("Digit{character}"))
        } else if character == ' ' {
            Some("Space".to_owned())
        } else {
            None
        };
        Ok(Self {
            key: text.to_string(),
            code,
            vk: u32::from(text.to_ascii_uppercase()),
            text: Some(text.to_string()),
            modifiers,
            label: label.to_owned(),
        })
    }

    /// The Enter key, for a `type` that submits.
    #[must_use]
    pub(crate) fn enter() -> Self {
        Self {
            key: "Enter".to_owned(),
            code: Some("Enter".to_owned()),
            vk: 13,
            text: Some("\r".to_owned()),
            modifiers: 0,
            label: "Enter".to_owned(),
        }
    }

    /// The character this key inserts: none for a shortcut, or for a key that only moves.
    pub(crate) fn insertable(&self) -> Option<&str> {
        if self.modifiers & (ALT | CONTROL | META) == 0 { self.text.as_deref() } else { None }
    }
}

fn modifier_bit(name: &str) -> Result<u8> {
    Ok(match name.trim().to_ascii_lowercase().as_str() {
        "alt" => ALT,
        "control" | "ctrl" => CONTROL,
        "meta" | "cmd" | "command" => META,
        "shift" => SHIFT,
        other => bail!("`{other}` is not a modifier: Alt, Control, Meta or Shift."),
    })
}

type NamedKey = (&'static str, Option<&'static str>, u32, Option<&'static str>);

fn named_key(name: &str) -> Option<NamedKey> {
    Some(match name.to_ascii_lowercase().as_str() {
        "enter" | "return" => ("Enter", Some("Enter"), 13, Some("\r")),
        "tab" => ("Tab", Some("Tab"), 9, Some("\t")),
        "escape" | "esc" => ("Escape", Some("Escape"), 27, None),
        "backspace" => ("Backspace", Some("Backspace"), 8, None),
        "delete" | "del" => ("Delete", Some("Delete"), 46, None),
        "space" => (" ", Some("Space"), 32, Some(" ")),
        "arrowup" | "up" => ("ArrowUp", Some("ArrowUp"), 38, None),
        "arrowdown" | "down" => ("ArrowDown", Some("ArrowDown"), 40, None),
        "arrowleft" | "left" => ("ArrowLeft", Some("ArrowLeft"), 37, None),
        "arrowright" | "right" => ("ArrowRight", Some("ArrowRight"), 39, None),
        "home" => ("Home", Some("Home"), 36, None),
        "end" => ("End", Some("End"), 35, None),
        "pageup" => ("PageUp", Some("PageUp"), 33, None),
        "pagedown" => ("PageDown", Some("PageDown"), 34, None),
        "insert" => ("Insert", Some("Insert"), 45, None),
        "f1" => ("F1", Some("F1"), 112, None),
        "f2" => ("F2", Some("F2"), 113, None),
        "f3" => ("F3", Some("F3"), 114, None),
        "f4" => ("F4", Some("F4"), 115, None),
        "f5" => ("F5", Some("F5"), 116, None),
        "f6" => ("F6", Some("F6"), 117, None),
        "f7" => ("F7", Some("F7"), 118, None),
        "f8" => ("F8", Some("F8"), 119, None),
        "f9" => ("F9", Some("F9"), 120, None),
        "f10" => ("F10", Some("F10"), 121, None),
        "f11" => ("F11", Some("F11"), 122, None),
        "f12" => ("F12", Some("F12"), 123, None),
        _ => return None,
    })
}

/// The action a tool name and its arguments name.
fn kind_of(name: &str, args: &Value) -> Result<Kind> {
    Ok(match name {
        "navigate" => {
            let url = optional(args.get("url"), |value| {
                normalize_url(value.as_str().context("`url` is the address to open, as a string.")?)
            })?;
            let direction = optional(args.get("direction"), Direction::parse)?;
            ensure!(
                url.is_some() || direction.is_some(),
                "Name a url to open, or a direction: back, forward or reload."
            );
            ensure!(!(url.is_some() && direction.is_some()), "Name a url or a direction, not both.");
            Kind::Navigate { url, direction }
        }
        "snapshot" => Kind::Snapshot,
        "click" => Kind::Click {
            target: target_arg(args)?,
            button: optional(args.get("button"), Button::parse)?.unwrap_or(Button::Left),
            count: optional(args.get("clickCount"), |value| {
                u8::try_from(value.as_u64().context("`clickCount` is 1 or 2.")?)
                    .ok()
                    .filter(|count| (1..=2).contains(count))
                    .context("`clickCount` is 1 or 2.")
            })?
            .unwrap_or(1),
            modifiers: modifiers(args.get("modifiers"))?,
        },
        "type" => {
            let text = args.get("text").and_then(Value::as_str).context("`text` is the text to enter.")?;
            ensure!(text.chars().count() <= MAX_TEXT_CHARS, "`text` is at most {MAX_TEXT_CHARS} characters.");
            Kind::Type {
                target: optional_target(args)?,
                text: text.to_owned(),
                replace: match args.get("mode").and_then(Value::as_str).map(str::trim) {
                    None | Some("" | "replace") => true,
                    Some("append") => false,
                    Some(other) => bail!("A mode is replace or append, not `{other}`."),
                },
                submit: args["submit"] == true,
            }
        }
        "press" => Kind::Press {
            key: Key::parse(args.get("key").context("`key` names the key to press, such as Enter.")?)?,
            target: optional_target(args)?,
        },
        "hover" => Kind::Hover { target: target_arg(args)? },
        "scroll" => {
            let delta = |key: &str| -> Result<f64> {
                let Some(value) = args.get(key).filter(|value| !value.is_null()) else { return Ok(0.0) };
                let pixels = value.as_f64().context(format!("`{key}` is a number of pixels, such as 600."))?;
                ensure!(pixels.abs() <= 100_000.0, "`{key}` is at most 100000 pixels either way.");
                Ok(pixels)
            };
            let (x, y) = (delta("deltaX")?, delta("deltaY")?);
            ensure!(x.abs() > f64::EPSILON || y.abs() > f64::EPSILON, "Give deltaX or deltaY, in pixels.");
            Kind::Scroll { target: optional_target(args)?, x, y }
        }
        "select_option" => {
            let values = args
                .get("values")
                .and_then(Value::as_array)
                .context("`values` lists the option values or labels to select.")?;
            ensure!(!values.is_empty() && values.len() <= MAX_VALUES, "Name 1 to {MAX_VALUES} options.");
            let values = values
                .iter()
                .map(|value| -> Result<String> {
                    let text = value.as_str().context("An option is its value or label, as a string.")?;
                    ensure!(
                        text.chars().count() <= MAX_VALUE_CHARS,
                        "An option is at most {MAX_VALUE_CHARS} characters."
                    );
                    Ok(text.to_owned())
                })
                .collect::<Result<Vec<_>>>()?;
            Kind::Select { target: target_arg(args)?, values }
        }
        "set_checked" => Kind::Check {
            target: target_arg(args)?,
            checked: args.get("checked").and_then(Value::as_bool).context("`checked` is true or false.")?,
        },
        "wait_for" => {
            let text = optional(args.get("text"), |value| {
                non_blank(value, "`text` is the text to wait for.", MAX_WAIT_TEXT_CHARS)
            })?;
            let url = optional(args.get("url"), |value| {
                non_blank(value, "`url` is the part of the address to wait for.", MAX_URL_CHARS)
            })?;
            let state = optional(args.get("state"), Load::parse)?;
            ensure!(
                text.is_some() || url.is_some() || state.is_some(),
                "wait_for waits for text, an address or a load state; name one."
            );
            Kind::Wait { text, url, state }
        }
        "evaluate" => {
            let expression = args
                .get("expression")
                .and_then(Value::as_str)
                .context("`expression` is the JavaScript to evaluate.")?;
            let expression = expression.trim();
            ensure!(!expression.is_empty(), "`expression` is empty.");
            ensure!(
                expression.chars().count() <= MAX_EXPRESSION_CHARS,
                "`expression` is at most {MAX_EXPRESSION_CHARS} characters."
            );
            Kind::Evaluate { expression: expression.to_owned(), await_promise: args["awaitPromise"] == true }
        }
        "run" => Kind::Run { steps: steps_of(args)? },
        // `sleep` is a step of a run, never a tool of its own.
        "sleep" => Kind::Sleep { ms: milliseconds(args.get("ms"), 0, MAX_SLEEP_MS)? },
        other => bail!(
            "`{other}` is not a browser tool. Use navigate, snapshot, click, type, press, hover, scroll, select_option, set_checked, wait_for, evaluate or run."
        ),
    })
}

/// The steps of a `run`, each parsed exactly as the tool it names.
fn steps_of(args: &Value) -> Result<Vec<Action>> {
    let steps = args.get("steps").and_then(Value::as_array).context("`steps` lists the steps of the run.")?;
    ensure!(!steps.is_empty(), "A run needs at least one step.");
    ensure!(steps.len() <= MAX_STEPS, "A run has at most {MAX_STEPS} steps; split the work into more calls.");
    steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let number = index + 1;
            let action = step
                .get("action")
                .and_then(Value::as_str)
                .with_context(|| format!("Step {number} names its action, such as click."))?;
            ensure!(action != "run", "Step {number} is a run; a run cannot hold another run.");
            Action::parse(action, step).with_context(|| format!("Step {number} ({action}) is not valid"))
        })
        .collect()
}

/// A field the call left out, or parsed when it named one.
fn optional<T>(value: Option<&Value>, parse: impl FnOnce(&Value) -> Result<T>) -> Result<Option<T>> {
    value.filter(|value| !value.is_null()).map(parse).transpose()
}

/// The ref of the element a tool acts on.
fn target_arg(args: &Value) -> Result<Target> {
    let value = args
        .get("ref")
        .or_else(|| args.get("target"))
        .context("Give the element's ref number from the last snapshot, such as 7.")?;
    Target::parse(value)
}

/// The ref of the element a tool acts on, when naming one is optional.
fn optional_target(args: &Value) -> Result<Option<Target>> {
    match args.get("ref").or_else(|| args.get("target")) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(Target::parse(value)?)),
    }
}

/// The CDP modifier mask of `modifiers`, such as `["Control", "Shift"]`.
fn modifiers(value: Option<&Value>) -> Result<u8> {
    let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(0) };
    let listed = value.as_array().context("`modifiers` is a list: Alt, Control, Meta, Shift.")?;
    ensure!(listed.len() <= 4, "Name each modifier once, at most four.");
    let mut bits = 0_u8;
    for item in listed {
        let name = item.as_str().context("A modifier is a string: Alt, Control, Meta or Shift.")?;
        let bit = match name {
            "Alt" => ALT,
            "Control" => CONTROL,
            "Meta" => META,
            "Shift" => SHIFT,
            other => bail!("`{other}` is not a modifier: Alt, Control, Meta or Shift."),
        };
        ensure!(bits & bit == 0, "`{name}` appears twice.");
        bits |= bit;
    }
    Ok(bits)
}

/// An http(s) address (or `about:blank`), with `https://` added to a bare host as a browser does.
pub(super) fn normalize_url(raw: &str) -> Result<String> {
    let address = raw.trim();
    ensure!(!address.is_empty(), "Name the address to open, such as https://example.com.");
    ensure!(address.chars().count() <= MAX_URL_CHARS, "An address is at most {MAX_URL_CHARS} characters.");
    ensure!(!address.chars().any(char::is_whitespace), "The address has a space in it.");
    let lower = address.to_ascii_lowercase();
    let refused = [
        "javascript:",
        "data:",
        "file:",
        "chrome:",
        "chrome-extension:",
        "devtools:",
        "view-source:",
        "mailto:",
        "ftp:",
    ];
    ensure!(!refused.iter().any(|scheme| lower.starts_with(scheme)), "Enter an http or https address.");
    let address = if lower.starts_with("http://") || lower.starts_with("https://") || lower == "about:blank" {
        address.to_owned()
    } else {
        ensure!(!lower.contains("://"), "Enter an http or https address.");
        format!("https://{address}")
    };
    ensure!(
        address.to_ascii_lowercase() == "about:blank"
            || !address.split_once("://").map_or("", |(_, rest)| rest).trim_matches('/').is_empty(),
        "Enter a whole address, such as https://example.com."
    );
    Ok(address)
}

fn non_blank(value: &Value, missing: &str, max: usize) -> Result<String> {
    let text = value.as_str().context(missing.to_owned())?.trim();
    ensure!(!text.is_empty(), missing.to_owned());
    ensure!(text.chars().count() <= max, "At most {max} characters.");
    Ok(text.to_owned())
}

/// A whole number of milliseconds, from 0 to `max`, or `default` when the call names none.
fn milliseconds(value: Option<&Value>, default: u64, max: u64) -> Result<u64> {
    let Some(value) = value.filter(|value| !value.is_null()) else { return Ok(default) };
    let ms = value.as_u64().context(format!("The time is a whole number of milliseconds, from 0 to {max}."))?;
    ensure!(ms <= max, "The time is at most {max} milliseconds.");
    Ok(ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_ref_is_the_number_a_snapshot_printed() {
        for written in [json!(7), json!("7"), json!(" 7 "), json!("#7"), json!("[7]"), json!({"ref": 7})] {
            assert_eq!(Target::parse(&written).unwrap().number(), 7, "{written}");
        }
        assert_eq!(Target::parse(&json!({"kind": "ref", "ref": "12"})).unwrap().number(), 12);
        for refused in [json!(0), json!(-1), json!("seven"), json!(""), json!(100_001), json!({}), json!(null)] {
            assert!(Target::parse(&refused).is_err(), "{refused} is not a ref");
        }
        let css = Target::parse(&json!({"kind": "css", "selector": "#go"})).unwrap_err().to_string();
        assert!(css.contains("not supported"), "a target kind other than a ref says so: {css}");
    }

    #[test]
    fn a_key_carries_its_modifiers_and_code() {
        let enter = Key::parse(&json!("Enter")).unwrap();
        assert_eq!((enter.key.as_str(), enter.vk, enter.insertable()), ("Enter", 13, Some("\r")));
        let shortcut = Key::parse(&json!("Control+A")).unwrap();
        assert_eq!(shortcut.modifiers, CONTROL);
        assert_eq!((shortcut.key.as_str(), shortcut.code.as_deref(), shortcut.vk), ("A", Some("KeyA"), 65));
        assert!(shortcut.insertable().is_none(), "a shortcut types nothing");
        let command = Key::parse(&json!("Meta+Shift+P")).unwrap();
        assert_eq!(command.modifiers, META | SHIFT);
        assert_eq!((command.key.as_str(), command.text.as_deref()), ("P", Some("P")));
        let letter = Key::parse(&json!("a")).unwrap();
        assert_eq!((letter.key.as_str(), letter.insertable()), ("a", Some("a")));
        let digit = Key::parse(&json!("7")).unwrap();
        assert_eq!((digit.code.as_deref(), digit.vk), (Some("Digit7"), 55));
        let escape = Key::parse(&json!("Escape")).unwrap();
        assert_eq!((escape.vk, escape.insertable()), (27, None));
        for refused in [json!("Control+"), json!("Hyper+A"), json!("Control+Control+A"), json!("Nope"), json!("")] {
            assert!(Key::parse(&refused).is_err(), "{refused} is not a key");
        }
    }

    #[test]
    fn the_actions_read_as_the_tools_write_them() {
        let click = Action::parse("click", &json!({"ref": "3", "button": "right", "clickCount": 2})).unwrap();
        assert_eq!(
            click.kind,
            Kind::Click { target: Target::parse(&json!(3)).unwrap(), button: Button::Right, count: 2, modifiers: 0 }
        );
        assert_eq!(click.timeout, DEFAULT_TIMEOUT);
        assert!(!click.snapshot);
        let typed =
            Action::parse("type", &json!({"ref": 2, "text": "hello", "mode": "append", "submit": true})).unwrap();
        assert_eq!(
            typed.kind,
            Kind::Type {
                target: Some(Target::parse(&json!(2)).unwrap()),
                text: "hello".into(),
                replace: false,
                submit: true
            }
        );
        let scrolled = Action::parse("scroll", &json!({"deltaY": 600, "snapshot": true})).unwrap();
        assert_eq!(scrolled.kind, Kind::Scroll { target: None, x: 0.0, y: 600.0 });
        assert!(scrolled.snapshot);
        let waited = Action::parse("wait_for", &json!({"text": "Results"})).unwrap();
        assert_eq!(waited.kind, Kind::Wait { text: Some("Results".into()), url: None, state: None });
    }

    #[test]
    fn a_call_keeps_its_limits() {
        assert!(Action::parse("wait_for", &json!({"text": "x", "timeoutMs": 20_001})).is_err());
        assert!(Action::parse("evaluate", &json!({"expression": "1", "timeoutMs": 1_200_000})).is_err());
        assert!(Action::parse("type", &json!({"text": "x".repeat(20_001)})).is_err());
        assert!(Action::parse("evaluate", &json!({"expression": " ".repeat(64_001)})).is_err());
        assert!(Action::parse("run", &json!({"steps": []})).is_err());
        assert!(Action::parse("wait_for", &json!({})).is_err(), "a wait names what it waits for");
        assert!(Action::parse("navigate", &json!({})).is_err(), "navigate needs a url or a direction");
        assert!(Action::parse("navigate", &json!({"url": "https://a.com", "direction": "back"})).is_err(), "not both");
        assert!(Action::parse("click", &json!({})).is_err(), "click needs a ref");
    }

    #[test]
    fn a_run_holds_up_to_two_hundred_steps_and_no_run() {
        let steps = |count: usize| json!({"steps": vec![json!({"action": "snapshot"}); count]});
        assert_eq!(
            Action::parse("run", &steps(200)).unwrap().kind,
            Kind::Run { steps: vec![Action::parse("snapshot", &json!({})).unwrap(); 200] }
        );
        let too_many = Action::parse("run", &steps(201)).unwrap_err().to_string();
        assert!(too_many.contains("at most 200"), "the step cap says its number: {too_many}");
        let nested = Action::parse("run", &json!({"steps": [{"action": "run", "steps": []}]})).unwrap_err().to_string();
        assert!(nested.contains("cannot hold another run"), "{nested}");
        let unknown = Action::parse("run", &json!({"steps": [{"action": "teleport"}]})).unwrap_err().to_string();
        assert!(unknown.contains("Step 1"), "the failing step is named: {unknown}");
        let slept = Action::parse("run", &json!({"steps": [{"action": "sleep", "ms": 500}]})).unwrap();
        assert_eq!(
            slept.kind,
            Kind::Run {
                steps: vec![Action { timeout: DEFAULT_TIMEOUT, snapshot: false, kind: Kind::Sleep { ms: 500 } }]
            }
        );
        assert!(Action::parse("run", &json!({"steps": [{"action": "sleep", "ms": 25_001}]})).is_err());
    }

    #[test]
    fn a_run_has_its_own_time_limit() {
        let run = Action::parse("run", &json!({"steps": [{"action": "snapshot"}]})).unwrap();
        assert_eq!(run.timeout, Duration::from_millis(25_000));
        assert!(Action::parse("run", &json!({"steps": [{"action": "snapshot"}], "timeoutMs": 25_001})).is_err());
    }

    #[test]
    fn only_web_addresses_are_opened() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com");
        assert_eq!(normalize_url(" https://example.com/a?b=c ").unwrap(), "https://example.com/a?b=c");
        assert_eq!(normalize_url("about:blank").unwrap(), "about:blank");
        assert!(normalize_url("").is_err());
        assert!(normalize_url("javascript:alert(1)").is_err());
        assert!(normalize_url("file:///c:/secret").is_err());
        assert!(normalize_url("https://a b.com").is_err());
        assert!(normalize_url("https://").is_err());
        assert!(normalize_url(&format!("https://x.com/{}", "a".repeat(2_100))).is_err());
    }
}
