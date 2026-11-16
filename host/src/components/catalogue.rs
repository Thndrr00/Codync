//! Codync's own component gallery, version 1: the cards a bot or the playground may show, what each
//! one accepts, and the data functions each may have the host resolve for it.
//!
//! This is the whole catalogue. A card is a name plus props, never markup: the host checks the props
//! here and every client draws the name with its own controls, so nothing from the host (or a model)
//! ever runs as HTML, JavaScript or a React component. That is the difference from OpenBot's gallery,
//! which renders a component's own React from its plugin folder.

use serde_json::{Value, json};

/// The catalogue version this host accepts and serves. It moves when a component's props change
/// shape, never for a new card.
pub const VERSION: u64 = 1;

/// What one prop may hold. A deliberately small subset of JSON Schema: text, numbers, true/false, a
/// choice, lists, table rows, and the three shapes a card's content takes (timeline points, checklist
/// rows, form fields). Nothing here can name a URL, an expression or a script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prop {
    /// One line of plain text.
    Text {
        max: usize,
    },
    /// Plain text that may hold newlines.
    Multiline {
        max: usize,
    },
    Number,
    Bool,
    /// One of a fixed set of words.
    Choice(&'static [&'static str]),
    /// Text entries, each bounded.
    TextList {
        max: usize,
        chars: usize,
    },
    /// Numbers, for a sparkline.
    NumberList {
        max: usize,
    },
    /// Table rows: each a list of cells (text, number, true/false or empty).
    Rows {
        max: usize,
        chars: usize,
    },
    /// Timeline points `{at, title, detail?, tone?}`.
    Events {
        max: usize,
    },
    /// Checklist rows `{label, done, note?}`.
    Items {
        max: usize,
    },
    /// Form fields `{id, label, type, required?, options?}`.
    Fields {
        max: usize,
    },
}

/// One prop of a component: its key, how the playground labels it, and what it holds.
#[derive(Clone, Copy, Debug)]
pub struct PropSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub prop: Prop,
    pub required: bool,
}

/// One catalogue component.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub name: &'static str,
    pub version: u64,
    pub summary: &'static str,
    pub props: &'static [PropSpec],
    /// The data functions this component may have the host resolve, as `function`.
    pub data: &'static [&'static str],
    /// Props that pass the schema, for the gallery's live preview and the playground's starting point.
    pub sample: &'static str,
}

const fn prop(key: &'static str, label: &'static str, help: &'static str, prop: Prop, required: bool) -> PropSpec {
    PropSpec { key, label, help, prop, required }
}

const METRIC: [PropSpec; 6] = [
    prop("label", "Label", "What the number counts.", Prop::Text { max: 80 }, true),
    prop(
        "value",
        "Value",
        "The number itself, already formatted: 12.4k, $1,204, 3m 20s.",
        Prop::Text { max: 32 },
        true,
    ),
    prop("change", "Change", "How it moved, as a short range: +12%, -3.", Prop::Text { max: 16 }, false),
    prop("trend", "Trend", "Which way the change points.", Prop::Choice(&["up", "down", "flat"]), false),
    prop("caption", "Caption", "One line under the number.", Prop::Text { max: 120 }, false),
    prop(
        "series",
        "Series",
        "Up to 24 numbers, drawn as a sparkline under the value.",
        Prop::NumberList { max: 24 },
        false,
    ),
];

const COMPARISON: [PropSpec; 3] = [
    prop(
        "columns",
        "Columns",
        "Two to four sides to compare, including their names: Plan A, Plan B.",
        Prop::TextList { max: 4, chars: 40 },
        true,
    ),
    prop(
        "rows",
        "Rows",
        "One row per criterion: the criterion, then one cell per column.",
        Prop::Rows { max: 20, chars: 200 },
        true,
    ),
    prop("caption", "Caption", "One line under the comparison.", Prop::Text { max: 120 }, false),
];

const TIMELINE: [PropSpec; 2] = [
    prop(
        "events",
        "Events",
        "Points in order: when, what happened, and an optional detail.",
        Prop::Events { max: 20 },
        true,
    ),
    prop("caption", "Caption", "One line under the timeline.", Prop::Text { max: 120 }, false),
];

const TABLE: [PropSpec; 3] = [
    prop("columns", "Columns", "One to six column headings.", Prop::TextList { max: 6, chars: 40 }, true),
    prop("rows", "Rows", "One row per line, with one cell per column.", Prop::Rows { max: 50, chars: 200 }, true),
    prop("caption", "Caption", "One line under the table.", Prop::Text { max: 120 }, false),
];

const CHECKLIST: [PropSpec; 2] = [
    prop(
        "items",
        "Items",
        "What to do, and whether it is done; an optional note under each.",
        Prop::Items { max: 20 },
        true,
    ),
    prop("caption", "Caption", "One line under the checklist.", Prop::Text { max: 120 }, false),
];

const FORM: [PropSpec; 3] = [
    prop(
        "fields",
        "Fields",
        "What to ask: an id, a label, and a type (text, multiline, number, toggle or select).",
        Prop::Fields { max: 12 },
        true,
    ),
    prop("submit", "Submit label", "The button's words; Send by default.", Prop::Text { max: 40 }, false),
    prop("caption", "Caption", "One line above the fields.", Prop::Text { max: 120 }, false),
];

/// Every component the host serves, in the order the gallery lists them.
pub const ALL: [Spec; 6] = [
    Spec {
        name: "metric",
        version: 1,
        summary: "One number, its change and a sparkline.",
        props: &METRIC,
        data: &[],
        sample: r#"{"label":"Builds today","value":"42","change":"+12%","trend":"up",
            "caption":"Across all three projects","series":[12,18,15,22,31,28,42]}"#,
    },
    Spec {
        name: "comparison",
        version: 1,
        summary: "Two to four sides side by side, one row per criterion.",
        props: &COMPARISON,
        data: &[],
        sample: r#"{"columns":["Plan A","Plan B"],
            "rows":[["Monthly cost","$20","$45"],["Seats","3","10"],["Support","Email","24/7"]],
            "caption":"Priced for this team"}"#,
    },
    Spec {
        name: "timeline",
        version: 1,
        summary: "What happened, in order, with optional detail.",
        props: &TIMELINE,
        data: &[],
        sample: r#"{"events":[
            {"at":"09:12","title":"Build started","tone":"info"},
            {"at":"09:24","title":"Tests failed","detail":"Two assertions in checkout.test.ts","tone":"warning"},
            {"at":"09:31","title":"Fixed and merged","tone":"done"}]}"#,
    },
    Spec {
        name: "table",
        version: 1,
        summary: "A grid of plain cells, up to six columns.",
        props: &TABLE,
        data: &["routines.list", "threads.recent", "bots.roster"],
        sample: r#"{"columns":["Routine","When","On"],
            "rows":[["Morning digest","Weekdays at 08:00","On"],["Weekly review","Fridays at 16:00","Off"]]}"#,
    },
    Spec {
        name: "checklist",
        version: 1,
        summary: "Steps with a done mark and an optional note.",
        props: &CHECKLIST,
        data: &["routines.status"],
        sample: r#"{"items":[
            {"label":"Rotate the deploy key","done":true},
            {"label":"Update the staging config","done":false,"note":"Waits on the new host"}]}"#,
    },
    Spec {
        name: "form",
        version: 1,
        summary: "A short form; what the person fills in comes back to the bot.",
        props: &FORM,
        data: &[],
        sample: r#"{"fields":[
            {"id":"title","label":"Title","type":"text","required":true},
            {"id":"when","label":"When","type":"date","required":true},
            {"id":"note","label":"Notes","type":"multiline"}],"submit":"Create the note"}"#,
    },
];

/// The component with this name, if the catalogue has it.
pub fn find(name: &str) -> Option<&'static Spec> {
    ALL.iter().find(|spec| spec.name == name)
}

/// Every component name, for refusals and the gallery.
pub fn names() -> Vec<&'static str> {
    ALL.iter().map(|spec| spec.name).collect()
}

/// The data functions a component may name.
pub fn grants(name: &str) -> &'static [&'static str] {
    find(name).map_or(&[], |spec| spec.data)
}

/// The card's sample props, checked by the catalogue's own test.
pub fn sample(spec: &Spec) -> Value {
    serde_json::from_str(spec.sample).unwrap_or(Value::Null)
}

/// What a refusal adds after its reason: every component and the props each accepts, so the agent
/// fixes a card in one step. `*` marks a required prop.
pub fn guide() -> String {
    let mut lines = vec![
        "Allowed shape: {\"name\": \"…\", \"props\": {…}, \"title\": \"…\"}. Plain text only: no HTML, Markdown or scripts."
            .to_owned(),
    ];
    for spec in &ALL {
        let props: Vec<String> =
            spec.props.iter().map(|p| format!("{}{}", p.key, if p.required { "*" } else { "" })).collect();
        lines.push(format!("- {} {{{}}}", spec.name, props.join(", ")));
    }
    lines.join("\n")
}

/// The catalogue as the gallery and the playground read it: name, version, summary, the props' JSON
/// schema, the data functions, and a sample instance to preview.
pub fn doc() -> Value {
    let components: Vec<Value> = ALL
        .iter()
        .map(|spec| {
            json!({
                "name": spec.name,
                "version": spec.version,
                "summary": spec.summary,
                "schema": schema(spec),
                "data": spec.data,
                "sample": sample(spec),
            })
        })
        .collect();
    json!({"version": VERSION, "components": components})
}

/// A component's props as JSON Schema, for the playground's form and the gallery's documentation.
fn schema(spec: &Spec) -> Value {
    let properties: serde_json::Map<String, Value> =
        spec.props.iter().map(|spec| (spec.key.to_owned(), prop_schema(spec))).collect();
    let required: Vec<&str> = spec.props.iter().filter(|spec| spec.required).map(|spec| spec.key).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": required,
    })
}

fn prop_schema(spec: &PropSpec) -> Value {
    let mut schema = match spec.prop {
        Prop::Text { max } | Prop::Multiline { max } => json!({"type": "string", "maxLength": max}),
        Prop::Number => json!({"type": "number"}),
        Prop::Bool => json!({"type": "boolean"}),
        Prop::Choice(choices) => json!({"type": "string", "enum": choices}),
        Prop::TextList { max, chars } => {
            json!({"type": "array", "minItems": 1, "maxItems": max, "items": {"type": "string", "maxLength": chars}})
        }
        Prop::NumberList { max } => {
            json!({"type": "array", "minItems": 1, "maxItems": max, "items": {"type": "number"}})
        }
        Prop::Rows { max, chars } => json!({
            "type": "array",
            "minItems": 1,
            "maxItems": max,
            "items": {"type": "array", "items": {"type": ["string", "number", "boolean", "null"], "maxLength": chars}},
            "description": "One cell per column.",
        }),
        Prop::Events { max } => json!({
            "type": "array",
            "minItems": 1,
            "maxItems": max,
            "items": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "at": {"type": "string"},
                    "title": {"type": "string"},
                    "detail": {"type": "string"},
                    "tone": {"type": "string", "enum": ["info", "warning", "done"]},
                },
                "required": ["at", "title"],
            },
        }),
        Prop::Items { max } => json!({
            "type": "array",
            "minItems": 1,
            "maxItems": max,
            "items": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "label": {"type": "string"},
                    "done": {"type": "boolean"},
                    "note": {"type": "string"},
                },
                "required": ["label"],
            },
        }),
        Prop::Fields { max } => json!({
            "type": "array",
            "minItems": 1,
            "maxItems": max,
            "items": {"type": "object", "additionalProperties": false, "properties": {
                "id": {"type": "string"},
                "label": {"type": "string"},
                "type": {"type": "string", "enum": ["text", "multiline", "number", "date", "time", "select", "toggle"]},
                "required": {"type": "boolean"},
                "placeholder": {"type": "string"},
                "options": {"type": "array", "items": {"type": "object"}},
            }},
        }),
    };
    schema["title"] = spec.label.into();
    schema["description"] = spec.help.into();
    schema
}
