//! The strict JSON reader the block catalogue validates with: every prop must be asked for, and a
//! value outside what the catalogue allows is refused with the path that broke.

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

/// Longest component, field, action or type id.
pub(super) const MAX_ID: usize = 32;

/// A strict reader for one object: every prop must be asked for, and an unknown one is refused.
pub(super) struct Props<'a> {
    object: &'a Map<String, Value>,
    path: String,
    used: HashSet<String>,
}

impl<'a> Props<'a> {
    pub(super) fn new(value: &'a Value, path: &str) -> Result<Self> {
        let Some(object) = value.as_object() else { bail!("`{path}` must be an object.") };
        Ok(Self { object, path: path.to_owned(), used: HashSet::new() })
    }

    pub(super) fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        self.used.insert(key.to_owned());
        self.object.get(key)
    }

    pub(super) fn text(&mut self, key: &'static str, required: bool, max: usize) -> Result<Option<String>> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else {
            if required {
                bail!("`{}.{key}` is required.", self.path);
            }
            return Ok(None);
        };
        Ok(Some(text(value, &format!("{}.{key}", self.path), max)?))
    }

    pub(super) fn number(&mut self, key: &'static str) -> Result<Option<f64>> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else { return Ok(None) };
        let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
            bail!("`{}.{key}` must be a number.", self.path);
        };
        Ok(Some(number))
    }

    pub(super) fn bool(&mut self, key: &'static str) -> Result<bool> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else { return Ok(false) };
        let Some(on) = value.as_bool() else { bail!("`{}.{key}` must be true or false.", self.path) };
        Ok(on)
    }

    pub(super) fn array(&mut self, key: &'static str, required: bool) -> Result<Option<&'a Vec<Value>>> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else {
            if required {
                bail!("`{}.{key}` is required.", self.path);
            }
            return Ok(None);
        };
        let Some(list) = value.as_array() else { bail!("`{}.{key}` must be a list.", self.path) };
        Ok(Some(list))
    }

    pub(super) fn string_list(
        &mut self,
        key: &'static str,
        required: bool,
        max_items: usize,
        max_chars: usize,
    ) -> Result<Option<Vec<String>>> {
        let Some(list) = self.array(key, required)? else { return Ok(None) };
        if list.len() > max_items {
            bail!("`{}.{key}` may hold at most {max_items} entries.", self.path);
        }
        Ok(Some(
            list.iter().map(|value| text(value, &format!("{}.{key}", self.path), max_chars)).collect::<Result<_>>()?,
        ))
    }

    /// A component or field id: lowercase letters, digits, `_` and `-`, 1 to [`MAX_ID`] characters.
    pub(super) fn id(&mut self) -> Result<String> {
        let Some(id) = self.text("id", true, MAX_ID)? else { bail!("`{}.id` is required.", self.path) };
        if !id.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-') {
            bail!("`{}.id` must use lowercase letters, digits, _ and - only.", self.path);
        }
        Ok(id)
    }

    pub(super) fn finish(self) -> Result<()> {
        if let Some(key) = self.object.keys().find(|key| !self.used.contains(*key)) {
            bail!("`{}` has an unknown prop \"{key}\".", self.path);
        }
        Ok(())
    }
}

/// A table cell: plain text, a finite number, true/false, or empty.
pub(super) fn cell_value(cell: &Value, path: &str, max_chars: usize) -> Result<Value> {
    Ok(match cell {
        Value::Null => Value::Null,
        Value::Bool(on) => json!(on),
        Value::Number(number) if number.as_f64().is_some_and(|number| number.is_finite()) => {
            Value::Number(number.clone())
        }
        Value::String(cell) => json!(text(cell, path, max_chars)?),
        _ => bail!("`{path}` cells must be text, numbers, true/false or empty."),
    })
}

/// Plain text, trimmed and bounded: no markup and no control characters (except a newline or tab).
pub(super) fn text(value: &Value, path: &str, max: usize) -> Result<String> {
    let Some(text) = value.as_str() else { bail!("`{path}` must be text.") };
    let text = text.trim();
    if text.is_empty() {
        bail!("`{path}` can't be empty.");
    }
    if text.chars().count() > max {
        bail!("`{path}` must be at most {max} characters.");
    }
    if text.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        bail!("`{path}` can't hold control characters.");
    }
    if holds_markup(text) {
        bail!("`{path}` must be plain text: no HTML, Markdown or scripts.");
    }
    Ok(text.to_owned())
}

/// Whether a string looks like markup: a tag, a comment, or a script URL.
fn holds_markup(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes
        .windows(2)
        .any(|pair| pair[0] == b'<' && (pair[1].is_ascii_alphabetic() || pair[1] == b'/' || pair[1] == b'!'))
        || text.to_ascii_lowercase().contains("javascript:")
}

pub(super) fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes.iter().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                return true;
            }
            byte.is_ascii_digit()
        })
}

pub(super) fn is_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return false;
    }
    if !bytes.iter().enumerate().all(|(index, byte)| index == 2 || byte.is_ascii_digit()) {
        return false;
    }
    let Ok(hour) = text[0..2].parse::<u8>() else { return false };
    let Ok(minute) = text[3..5].parse::<u8>() else { return false };
    hour < 24 && minute < 60
}
