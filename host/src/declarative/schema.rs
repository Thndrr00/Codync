//! Codync's block catalogue, version 1: the only components, props and sizes a declarative block may
//! use. Validation is manual and strict on purpose: an unknown prop is refused (never silently
//! dropped, so a client never renders something the agent didn't mean), and every string is plain
//! text with no markup, so no block can carry HTML, a script or a URL scheme.

use super::props::{MAX_ID, Props, cell_value, is_date, is_time};
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

/// The schema version this host accepts and serves.
pub const VERSION: u64 = 1;
/// Components one block's body may hold.
pub const MAX_BODY: usize = 40;
/// Form fields one block may hold (each is also a body component).
pub const MAX_FIELDS: usize = 20;
/// Columns one table may hold.
pub const MAX_TABLE_COLUMNS: usize = 8;
/// Rows one table may hold.
pub const MAX_TABLE_ROWS: usize = 50;
/// Points one chart may hold.
pub const MAX_CHART_POINTS: usize = 12;
/// Buttons one block may offer.
pub const MAX_ACTIONS: usize = 4;
/// Choices one select field may offer.
const MAX_OPTIONS: usize = 20;
const MAX_TITLE: usize = 200;
const MAX_TEXT: usize = 4_000;
const MAX_LABEL: usize = 120;
const MAX_CELL: usize = 500;

/// The body components clients may render.
pub const COMPONENTS: [&str; 7] = ["heading", "text", "divider", "callout", "field", "table", "chart"];
/// The form field types clients may render.
pub const FIELD_TYPES: [&str; 7] = ["text", "multiline", "number", "date", "time", "select", "toggle"];
/// How an action draws: the card's main button, an ordinary one, or one that warns (Deny, Delete).
pub const ACTION_KINDS: [&str; 3] = ["primary", "secondary", "destructive"];

/// A checked block: the document clients receive, its fields, and the actions it offers.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// The document as stored on the card: exactly the shape clients render.
    pub doc: Value,
    /// Each form field, in body order.
    pub fields: Vec<Field>,
    /// Each action, in the order the block declared them (one `Submit` when a form declared none).
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub id: String,
    pub label: String,
    /// One of [`FIELD_TYPES`].
    pub kind: String,
    pub required: bool,
    /// A select field's choices, as (value, label).
    pub options: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub kind: String,
}

impl Block {
    /// The form's answers, checked field by field: only its own fields, each of the right type, every
    /// required one present. `Err` is what the user is told.
    pub fn answers(&self, input: &Value) -> Result<Value> {
        let Some(given) = input.as_object() else {
            bail!("The form's values must be an object of field ids.");
        };
        if let Some(unknown) = given.keys().find(|key| !self.fields.iter().any(|field| &field.id == *key)) {
            bail!("This form has no field \"{unknown}\".");
        }
        let mut answers = Map::new();
        for field in &self.fields {
            match given.get(&field.id).filter(|value| !value.is_null()) {
                Some(value) => {
                    answers.insert(field.id.clone(), field.answer(value)?);
                }
                None if field.required => bail!("\"{}\" is required.", field.label),
                None => {}
            }
        }
        Ok(Value::Object(answers))
    }
}

impl Field {
    fn answer(&self, value: &Value) -> Result<Value> {
        let label = &self.label;
        Ok(match self.kind.as_str() {
            "text" | "multiline" => json!(answer_text(value, label)?),
            "number" => json!(answer_number(value, label)?),
            "date" => {
                let text = answer_text(value, label)?;
                if !is_date(&text) {
                    bail!("\"{label}\" must be a date (YYYY-MM-DD).");
                }
                json!(text)
            }
            "time" => {
                let text = answer_text(value, label)?;
                if !is_time(&text) {
                    bail!("\"{label}\" must be a time (HH:MM).");
                }
                json!(text)
            }
            "select" => {
                let text = answer_text(value, label)?;
                if !self.options.iter().any(|(choice, _)| *choice == text) {
                    bail!("\"{label}\" must be one of: {}.", self.choices());
                }
                json!(text)
            }
            "toggle" => {
                let Some(on) = value.as_bool() else { bail!("\"{label}\" must be true or false.") };
                json!(on)
            }
            _ => bail!("\"{label}\" has an unknown type."),
        })
    }

    fn choices(&self) -> String {
        self.options.iter().map(|(_, label)| label.as_str()).collect::<Vec<_>>().join(", ")
    }
}

fn answer_text(value: &Value, label: &str) -> Result<String> {
    let Some(text) = value.as_str() else { bail!("\"{label}\" must be text.") };
    let text = text.trim();
    if text.chars().count() > MAX_TEXT {
        bail!("\"{label}\" must be at most {MAX_TEXT} characters.");
    }
    if text.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        bail!("\"{label}\" can't hold control characters.");
    }
    Ok(text.to_owned())
}

fn answer_number(value: &Value, label: &str) -> Result<f64> {
    let Some(number) = value.as_f64().filter(|n| n.is_finite()) else {
        bail!("\"{label}\" must be a number.");
    };
    Ok(number)
}

/// A block document, checked. `Err` names the first prop that broke the catalogue.
pub fn parse(value: &Value) -> Result<Block> {
    let mut block = Props::new(value, "block")?;
    let version = block.number("version")?.unwrap_or(f64::NAN);
    if version != VERSION as f64 {
        bail!("`block.version` must be {VERSION}.");
    }
    let title = block.text("title", true, MAX_TITLE)?.unwrap_or_default();
    let body = block.array("body", true)?.cloned().unwrap_or_default();
    if body.is_empty() || body.len() > MAX_BODY {
        bail!("`block.body` must have 1 to {MAX_BODY} components.");
    }
    if block.get("fields").is_some() || block.get("components").is_some() {
        bail!("Fields and components go inside `body`, not beside it.");
    }
    let mut fields = Vec::new();
    let mut components = Vec::with_capacity(body.len());
    for (index, node) in body.iter().enumerate() {
        components.push(component(node, &format!("block.body[{index}]"), &mut fields)?);
    }
    if fields.len() > MAX_FIELDS {
        bail!("A block may hold at most {MAX_FIELDS} form fields.");
    }
    let mut ids = HashSet::new();
    for field in &fields {
        if !ids.insert(field.id.as_str()) {
            bail!("Two fields share the id \"{}\".", field.id);
        }
    }
    let actions = actions(block.get("actions"), &fields)?;
    block.finish()?;
    let docs: Vec<Value> = actions.iter().map(Action::doc).collect();
    Ok(Block { doc: json!({"version": VERSION, "title": title, "body": components, "actions": docs}), fields, actions })
}

impl Action {
    fn doc(&self) -> Value {
        json!({"id": self.id, "label": self.label, "kind": self.kind})
    }
}

/// The buttons: the ones the block declared, or one `Submit` for a form that declared none.
fn actions(value: Option<&Value>, fields: &[Field]) -> Result<Vec<Action>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(if fields.is_empty() { Vec::new() } else { vec![Action::submit()] });
    };
    let Some(list) = value.as_array() else { bail!("`block.actions` must be a list.") };
    if list.len() > MAX_ACTIONS {
        bail!("A block may offer at most {MAX_ACTIONS} actions.");
    }
    let mut actions = Vec::with_capacity(list.len());
    for (index, node) in list.iter().enumerate() {
        let path = format!("block.actions[{index}]");
        let mut props = Props::new(node, &path)?;
        let id = props.id()?;
        if id == "__back" {
            bail!("`{path}.id` is reserved.");
        }
        if actions.iter().any(|action: &Action| action.id == id) {
            bail!("Two actions share the id \"{id}\".");
        }
        let label = props.text("label", true, MAX_LABEL)?.unwrap_or_default();
        let kind = props.text("kind", false, MAX_ID)?.unwrap_or_else(|| "primary".into());
        if !ACTION_KINDS.contains(&kind.as_str()) {
            bail!("`{path}.kind` must be one of: {}.", ACTION_KINDS.join(", "));
        }
        // `confirm` is accepted for compatibility with older drafts but changes nothing: the card
        // itself is the person's confirmation.
        let _ = props.bool("confirm")?;
        props.finish()?;
        actions.push(Action { id, label, kind });
    }
    if actions.is_empty() && !fields.is_empty() {
        bail!("A form needs at least one action to send its fields.");
    }
    Ok(actions)
}

impl Action {
    fn submit() -> Self {
        Self { id: "submit".into(), label: "Submit".into(), kind: "primary".into() }
    }
}

/// One body component, checked and normalized.
fn component(node: &Value, path: &str, fields: &mut Vec<Field>) -> Result<Value> {
    let mut props = Props::new(node, path)?;
    let kind = props.text("type", true, MAX_ID)?.unwrap_or_default();
    let rendered = match kind.as_str() {
        "heading" => {
            let text = props.text("text", true, MAX_LABEL)?.unwrap_or_default();
            json!({"type": "heading", "text": text})
        }
        "text" => {
            let text = props.text("text", true, MAX_TEXT)?.unwrap_or_default();
            json!({"type": "text", "text": text})
        }
        "divider" => json!({"type": "divider"}),
        "callout" => {
            let tone = props.text("tone", false, MAX_ID)?.unwrap_or_else(|| "info".into());
            if !["info", "warning"].contains(&tone.as_str()) {
                bail!("`{path}.tone` must be info or warning.");
            }
            let text = props.text("text", true, MAX_TEXT)?.unwrap_or_default();
            json!({"type": "callout", "tone": tone, "text": text})
        }
        "field" => {
            let Some(inner) = props.get("field") else { bail!("`{path}.field` is required.") };
            let (field, doc) = field(inner, &format!("{path}.field"))?;
            fields.push(field);
            json!({"type": "field", "field": doc})
        }
        "table" => table(&mut props, path)?,
        "chart" => chart(&mut props, path)?,
        other => bail!("`{path}.type` is \"{other}\"; the components are: {}.", COMPONENTS.join(", ")),
    };
    props.finish()?;
    Ok(rendered)
}

fn field(node: &Value, path: &str) -> Result<(Field, Value)> {
    let mut props = Props::new(node, path)?;
    let id = props.id()?;
    let label = props.text("label", true, MAX_LABEL)?.unwrap_or_default();
    let kind = props.text("type", true, MAX_ID)?.unwrap_or_default();
    if !FIELD_TYPES.contains(&kind.as_str()) {
        bail!("`{path}.type` is \"{kind}\"; the field types are: {}.", FIELD_TYPES.join(", "));
    }
    let required = props.bool("required")?;
    let placeholder = props.text("placeholder", false, MAX_LABEL)?;
    let mut options = Vec::new();
    if let Some(list) = props.array("options", false)? {
        if kind != "select" {
            bail!("`{path}.options` only belongs to a select field.");
        }
        if list.is_empty() || list.len() > MAX_OPTIONS {
            bail!("`{path}.options` must have 1 to {MAX_OPTIONS} choices.");
        }
        for (index, choice) in list.iter().enumerate() {
            let choice_path = format!("{path}.options[{index}]");
            let mut choice = Props::new(choice, &choice_path)?;
            let value = choice.text("value", true, MAX_LABEL)?.unwrap_or_default();
            let label = choice.text("label", true, MAX_LABEL)?.unwrap_or_default();
            choice.finish()?;
            if options.iter().any(|(known, _)| *known == value) {
                bail!("Two choices of \"{path}\" share the value \"{value}\".");
            }
            options.push((value, label));
        }
    } else if kind == "select" {
        bail!("`{path}.options` is required for a select field.");
    }
    let mut doc = json!({"id": id, "label": label, "type": kind, "required": required});
    let field = Field { id: id.clone(), label, kind, required, options };
    if let Some(placeholder) = placeholder {
        doc["placeholder"] = placeholder.into();
    }
    if !field.options.is_empty() {
        doc["options"] =
            Value::Array(field.options.iter().map(|(value, label)| json!({"value": value, "label": label})).collect());
    }
    if let Some(value) = props.get("value").filter(|value| !value.is_null()) {
        doc["value"] = field.answer(value).map_err(|error| anyhow::anyhow!("{path}.value: {error}"))?;
    }
    props.finish()?;
    Ok((field, doc))
}

fn table(props: &mut Props<'_>, path: &str) -> Result<Value> {
    let columns = props.string_list("columns", true, MAX_TABLE_COLUMNS, MAX_LABEL)?.unwrap_or_default();
    if columns.is_empty() {
        bail!("`{path}.columns` must have 1 to {MAX_TABLE_COLUMNS} columns.");
    }
    let rows = props.array("rows", true)?.cloned().unwrap_or_default();
    if rows.len() > MAX_TABLE_ROWS {
        bail!("`{path}.rows` may hold at most {MAX_TABLE_ROWS} rows.");
    }
    let mut checked = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let row_path = format!("{path}.rows[{index}]");
        let Some(cells) = row.as_array() else { bail!("`{row_path}` must be a list of cells.") };
        if cells.len() != columns.len() {
            bail!("`{row_path}` has {} cells; the table has {} columns.", cells.len(), columns.len());
        }
        checked
            .push(Value::Array(cells.iter().map(|cell| cell_value(cell, &row_path, MAX_CELL)).collect::<Result<_>>()?));
    }
    Ok(json!({"type": "table", "columns": columns, "rows": checked}))
}

fn chart(props: &mut Props<'_>, path: &str) -> Result<Value> {
    let kind = props.text("chartType", true, MAX_ID)?.unwrap_or_default();
    if !["bar", "line"].contains(&kind.as_str()) {
        bail!("`{path}.chartType` must be bar or line.");
    }
    let labels = props.string_list("labels", true, MAX_CHART_POINTS, MAX_LABEL)?.unwrap_or_default();
    let values = props.array("values", true)?.cloned().unwrap_or_default();
    if labels.is_empty() || labels.len() > MAX_CHART_POINTS {
        bail!("`{path}.labels` must have 1 to {MAX_CHART_POINTS} points.");
    }
    if values.len() != labels.len() {
        bail!("`{path}.values` needs one number for each of the {} labels.", labels.len());
    }
    let mut points = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let Some(point) = value.as_f64().filter(|n| n.is_finite()) else {
            bail!("`{path}.values[{index}]` must be a number.");
        };
        points.push(json!(point));
    }
    Ok(json!({"type": "chart", "chartType": kind, "labels": labels, "values": points}))
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
