//! The component catalogue's props, checked: the bounded subset of JSON Schema a card may declare,
//! the reader that validates an instance against it, and the document every client renders.
//!
//! Validation is manual and strict, like the declarative blocks it shares its text rules with: an
//! unknown prop is refused rather than dropped, every string is plain text, and a refusal names the
//! field (`props.rows[2]`), so the playground and the agent can both fix it in one step.

use super::catalogue::{self, Prop, PropSpec, Spec, VERSION};
use crate::declarative::props::{cell_value, text};
use crate::declarative::schema::{Field, check_answers, field as parse_field};
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

/// Longest card title a client shows.
pub const MAX_TITLE: usize = 200;
/// Longest label inside a card.
const MAX_LABEL: usize = 120;
/// Longest text inside a card.
const MAX_TEXT: usize = 2_000;
/// Longest id a component or form field may use.
pub const MAX_ID: usize = 32;

/// A checked card: the document clients receive, and the fields a `form` card asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    /// Exactly the shape clients render: `{version, name, title, props, source?}`.
    pub doc: Value,
    /// The fields of a `form` card, in order; empty for every other component.
    pub fields: Vec<Field>,
    /// A form's submit button, in the card's own words.
    pub submit: Option<String>,
}

impl Component {
    /// Whether the card waits for the person: only a form does.
    pub fn asks(&self) -> bool {
        self.submit.is_some()
    }

    /// What a client sent back for a form: only the card's own fields, each of the right type, every
    /// required one present. A card that asked nothing passes the values through untouched.
    pub fn answers(&self, values: Option<Value>) -> Result<Option<Value>> {
        match values {
            Some(values) if self.asks() => Ok(Some(check_answers(&self.fields, &values)?)),
            values => Ok(values),
        }
    }
}

/// A card document (`{version, name, title, props}`), checked. `Err` names the field that broke the
/// catalogue and what it allows.
pub fn parse(value: &Value) -> Result<Component> {
    let mut doc = Reader::new(value, "component")?;
    let version = doc.number("version")?.unwrap_or(f64::NAN);
    if version != VERSION as f64 {
        bail!("`component.version` must be {VERSION}.");
    }
    let name = doc.text("name", true, MAX_ID)?.unwrap_or_default();
    let Some(spec) = catalogue::find(&name) else {
        bail!("`component.name` is \"{name}\"; the components are: {}.", catalogue::names().join(", "));
    };
    let title = doc.text("title", false, MAX_TITLE)?;
    let Some(props) = doc.get("props") else { bail!("`component.props` is required.") };
    // The resolved data function that filled some of the props, kept so the gallery can say where a
    // card's content came from. Clients ignore it.
    let source = doc.text("source", false, MAX_ID)?;
    doc.finish()?;
    build(spec, title, source, props)
}

/// A card from a catalogue component, a title, and props the caller wrote. `Err` names the field.
pub fn build(spec: &Spec, title: Option<String>, source: Option<String>, props: &Value) -> Result<Component> {
    let title = title.map(|title| title.trim().to_owned()).filter(|title| !title.is_empty());
    let Some(given) = props.as_object() else { bail!("`props` must be an object.") };
    let mut reader = Reader::new(props, "props")?;
    let mut checked = Map::new();
    let mut fields = Vec::new();
    for PropSpec { key, prop, required, .. } in spec.props {
        let value = reader.take(key);
        match value.filter(|value| !value.is_null()) {
            Some(value) => {
                checked.insert((*key).to_owned(), check(prop, value, &format!("props.{key}"), &mut fields)?);
            }
            None if *required => bail!("`props.{key}` is required for a {} card.", spec.name),
            None => {}
        }
    }
    reader.finish()?;
    if let Some(caption) = title.as_deref() {
        text(&Value::String(caption.to_owned()), "title", MAX_TITLE)?;
    }
    cross_check(spec, &checked)?;
    let submit = if spec.name == "form" {
        Some(checked.get("submit").and_then(Value::as_str).unwrap_or("Send").to_owned())
    } else {
        None
    };
    let mut doc = json!({"version": VERSION, "name": spec.name, "props": checked});
    if let Some(title) = title.filter(|title| !title.is_empty()) {
        doc["title"] = title.into();
    }
    if let Some(source) = source {
        doc["source"] = source.into();
    }
    Ok(Component { doc, fields, submit })
}

/// A component's own rules across props: a table's rows carry one cell per column, and a comparison
/// needs two sides to compare.
fn cross_check(spec: &Spec, props: &Map<String, Value>) -> Result<()> {
    let columns = props.get("columns").and_then(Value::as_array);
    if spec.name == "comparison" && columns.is_some_and(|columns| columns.len() < 2) {
        bail!("`props.columns` must have 2 to 4 columns for a comparison.");
    }
    let (Some(columns), Some(rows)) = (columns, props.get("rows").and_then(Value::as_array)) else {
        return Ok(());
    };
    for (index, row) in rows.iter().enumerate() {
        let cells = row.as_array().map_or(0, Vec::len);
        if cells != columns.len() {
            bail!("`props.rows[{index}]` has {cells} cells; the card declares {} columns.", columns.len());
        }
    }
    Ok(())
}

/// One prop, checked against what the catalogue allows for it.
fn check(prop: &Prop, value: &Value, path: &str, fields: &mut Vec<Field>) -> Result<Value> {
    Ok(match *prop {
        Prop::Text { max } | Prop::Multiline { max } => json!(text(value, path, max)?),
        Prop::Number => {
            let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
                bail!("`{path}` must be a number.");
            };
            json!(number)
        }
        Prop::Bool => match value {
            Value::Bool(on) => json!(on),
            _ => bail!("`{path}` must be true or false."),
        },
        Prop::Choice(choices) => {
            let choice = text(value, path, MAX_ID)?;
            if !choices.contains(&choice.as_str()) {
                bail!("`{path}` must be one of: {}.", choices.join(", "));
            }
            json!(choice)
        }
        Prop::TextList { max, chars } => {
            let list = list(value, path, max)?;
            let items = list
                .iter()
                .enumerate()
                .map(|(index, item)| Ok(json!(text(item, &format!("{path}[{index}]"), chars)?)))
                .collect::<Result<Vec<_>>>()?;
            Value::Array(items)
        }
        Prop::NumberList { max } => {
            let list = list(value, path, max)?;
            let mut numbers = Vec::with_capacity(list.len());
            for (index, item) in list.iter().enumerate() {
                let Some(number) = item.as_f64().filter(|number| number.is_finite()) else {
                    bail!("`{path}[{index}]` must be a number.");
                };
                numbers.push(json!(number));
            }
            Value::Array(numbers)
        }
        Prop::Rows { max, chars } => {
            let list = maybe_list(value, path, max)?;
            let mut rows = Vec::with_capacity(list.len());
            for (index, row) in list.iter().enumerate() {
                let row_path = format!("{path}[{index}]");
                let Some(cells) = row.as_array() else { bail!("`{row_path}` must be a list of cells.") };
                rows.push(Value::Array(
                    cells.iter().map(|cell| cell_value(cell, &row_path, chars)).collect::<Result<_>>()?,
                ));
            }
            Value::Array(rows)
        }
        Prop::Events { max } => Value::Array(events(maybe_list(value, path, max)?, path)?),
        Prop::Items { max } => Value::Array(items(maybe_list(value, path, max)?, path)?),
        Prop::Fields { max } => {
            let list = list(value, path, max)?;
            let mut docs = Vec::with_capacity(list.len());
            for (index, node) in list.iter().enumerate() {
                let (field, doc) = parse_field(node, &format!("{path}[{index}]"))?;
                if fields.iter().any(|known| known.id == field.id) {
                    bail!("Two fields share the id \"{}\".", field.id);
                }
                fields.push(field);
                docs.push(doc);
            }
            Value::Array(docs)
        }
    })
}

/// A timeline's points: when, what, and an optional line about it.
fn events(list: &[Value], path: &str) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(list.len());
    for (index, node) in list.iter().enumerate() {
        let at = format!("{path}[{index}]");
        let mut reader = Reader::new(node, &at)?;
        let when = reader.text("at", true, MAX_LABEL)?.unwrap_or_default();
        let title = reader.text("title", true, MAX_LABEL)?.unwrap_or_default();
        let detail = reader.text("detail", false, MAX_TEXT)?;
        let tone = reader.text("tone", false, MAX_ID)?.unwrap_or_else(|| "info".into());
        if !["info", "warning", "done"].contains(&tone.as_str()) {
            bail!("`{at}.tone` must be info, warning or done.");
        }
        reader.finish()?;
        let mut event = json!({"at": when, "title": title, "tone": tone});
        if let Some(detail) = detail {
            event["detail"] = detail.into();
        }
        out.push(event);
    }
    Ok(out)
}

/// A checklist's rows: what to do, and whether it is done.
fn items(list: &[Value], path: &str) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(list.len());
    for (index, node) in list.iter().enumerate() {
        let at = format!("{path}[{index}]");
        let mut reader = Reader::new(node, &at)?;
        let label = reader.text("label", true, MAX_LABEL)?.unwrap_or_default();
        let done = reader.bool("done")?;
        let note = reader.text("note", false, MAX_TEXT)?;
        reader.finish()?;
        let mut item = json!({"label": label, "done": done});
        if let Some(note) = note {
            item["note"] = note.into();
        }
        out.push(item);
    }
    Ok(out)
}

/// A list prop, bounded: a card never carries more than its catalogue entry allows.
fn list<'a>(value: &'a Value, path: &str, max: usize) -> Result<&'a Vec<Value>> {
    let Some(list) = value.as_array() else { bail!("`{path}` must be a list.") };
    if list.is_empty() {
        bail!("`{path}` needs at least one entry.");
    }
    if list.len() > max {
        bail!("`{path}` may hold at most {max} entries.");
    }
    Ok(list)
}

/// A list a data function fills: the host may resolve nothing (a bot with no routines yet), and the
/// card then shows an empty table, timeline or checklist rather than being refused.
fn maybe_list<'a>(value: &'a Value, path: &str, max: usize) -> Result<&'a Vec<Value>> {
    let Some(list) = value.as_array() else { bail!("`{path}` must be a list.") };
    if list.len() > max {
        bail!("`{path}` may hold at most {max} entries.");
    }
    Ok(list)
}

/// The strict reader a card's document and props are checked with: every prop must be asked for, and
/// an unknown one is refused with the path that broke.
struct Reader<'a> {
    object: &'a Map<String, Value>,
    path: String,
    used: HashSet<String>,
}

impl<'a> Reader<'a> {
    fn new(value: &'a Value, path: &str) -> Result<Self> {
        let Some(object) = value.as_object() else { bail!("`{path}` must be an object.") };
        Ok(Self { object, path: path.to_owned(), used: HashSet::new() })
    }

    fn take(&mut self, key: &str) -> Option<&'a Value> {
        self.used.insert(key.to_owned());
        self.object.get(key)
    }

    fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        self.take(key)
    }

    fn text(&mut self, key: &'static str, required: bool, max: usize) -> Result<Option<String>> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else {
            if required {
                bail!("`{}.{key}` is required.", self.path);
            }
            return Ok(None);
        };
        Ok(Some(text(value, &format!("{}.{key}", self.path), max)?))
    }

    fn number(&mut self, key: &'static str) -> Result<Option<f64>> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else { return Ok(None) };
        let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
            bail!("`{}.{key}` must be a number.", self.path);
        };
        Ok(Some(number))
    }

    fn bool(&mut self, key: &'static str) -> Result<bool> {
        let Some(value) = self.get(key).filter(|value| !value.is_null()) else { return Ok(false) };
        let Some(on) = value.as_bool() else { bail!("`{}.{key}` must be true or false.", self.path) };
        Ok(on)
    }

    fn finish(self) -> Result<()> {
        if let Some(key) = self.object.keys().find(|key| !self.used.contains(*key)) {
            bail!("`{}` has an unknown prop \"{key}\".", self.path);
        }
        Ok(())
    }
}
