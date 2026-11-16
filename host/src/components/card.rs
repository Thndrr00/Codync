//! `show_component`: a bot shows one of Codync's own cards in its chat, and the `form` card comes
//! back with what the person filled in.
//!
//! A card rides the approval-card path every client already routes (a `permission` entry with
//! `toolKind: "component"`), so it lands in the conversation with no new transport. The document it
//! carries is a catalogue name plus checked props: no markup and no script, so a client draws it with
//! its own controls.

use super::catalogue;
use super::data;
use super::schema::{self, Component};
use crate::agent::bot::Cmd;
use crate::hub::Hub;
use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;

/// The `chat` server's component tool.
pub const TOOLS: [&str; 1] = ["show_component"];

/// How long a `form` card waits for the person before the tool gives up; the card closes.
pub const ANSWER_TIMEOUT: Duration = Duration::from_mins(10);

/// The entry's title when the card has none of its own: the component's name, which every client
/// shows in the person's language.
pub const UNTITLED: &str = "Card";

pub fn tool() -> Value {
    json!({
        "name": "show_component",
        "description": "Show the person a Codync card in your chat: a metric, a comparison, a timeline, a table, a \
            checklist or a short form. The host checks the card against Codync's own catalogue before anything is \
            drawn, so a card with a prop the catalogue doesn't allow is refused with the exact field (fix it and call \
            again). Every client draws the name with its own controls: nothing here is HTML, Markdown or a script. \
            Cards: metric {label, value, change?, trend? up|down|flat, caption?, series?}; comparison {columns (2-4), \
            rows, caption?}; timeline {events: [{at, title, detail?, tone? info|warning|done}], caption?}; table \
            {columns (1-6), rows, caption?}; checklist {items: [{label, done, note?}], caption?}; form {fields: \
            [{id, label, type: text|multiline|number|date|time|select|toggle, required?, placeholder?, options?}], \
            submit?, caption?}. A form waits for the person and returns their values by field id. \
            Data functions fill props from this computer instead of you retyping them: a table may name \
            routines.list, threads.recent or bots.roster, and a checklist routines.status, as \
            data: [{function, limit?}]. A prop you pass wins over the one the host resolves. A title is optional; \
            without one the card shows its component's name.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "enum": catalogue::names()},
                "title": {"type": "string", "description": "The card's heading, in plain text."},
                "props": {"type": "object", "description": "The props the component accepts (see the tool description)."},
                "data": {
                    "type": "array",
                    "maxItems": data::MAX_REQUESTS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "function": {"type": "string"},
                            "limit": {"type": "integer", "minimum": 1}
                        },
                        "required": ["function"]
                    }
                },
                "bot": {"type": "string", "description": "Your own bot id, if you name one."}
            },
            "required": ["name", "props"]
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false}
    })
}

/// The `chat` server's component tool.
pub async fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    if !TOOLS.contains(&name) {
        bail!("unknown component tool: {name}");
    }
    let Some(component_name) = args["name"].as_str().map(str::trim).filter(|name| !name.is_empty()) else {
        bail!("`name` is required; the components are: {}.", catalogue::names().join(", "));
    };
    // A card belongs to the conversation that asked for it: the only bot it can be shown to is the
    // caller. Naming another one is refused, so a bot can't draw in someone else's chat.
    if let Some(target) = args["bot"].as_str().map(str::trim).filter(|target| !target.is_empty())
        && target != bot
    {
        bail!(
            "`bot` names whose chat the card is for, and a card belongs to the conversation that asked: use your own id \
             (or leave it out)."
        );
    }
    let title = args["title"].as_str();
    let component = document(hub, Some(bot), component_name, title, &args["props"], args.get("data"))
        .map_err(|error| anyhow!("{error:#}\n\n{}", catalogue::guide()))?;
    let interactive = component.asks();
    let (reply, answer) = oneshot::channel();
    let data = card(&component);
    hub.send_cmd(bot, Cmd::Block { data, reply: interactive.then_some(reply) })
        .map_err(|error| anyhow!("the card couldn't be shown: {error:#}"))?;
    if !interactive {
        return Ok(json!({"shown": true, "component": component_name}));
    }
    let answer = tokio::time::timeout(ANSWER_TIMEOUT, answer)
        .await
        .map_err(|_| {
            anyhow!("the person didn't answer this card within {} minutes; it closed", ANSWER_TIMEOUT.as_secs() / 60)
        })?
        .map_err(|_| anyhow!("the card closed before the person answered it (its turn ended or was stopped)"))?;
    Ok(match answer {
        Some(answer) => json!({"shown": true, "answered": true, "values": answer["values"]}),
        None => json!({"shown": true, "answered": false, "cancelled": true}),
    })
}

/// A card from the catalogue, checked and filled. `bot` scopes the data functions (and their
/// withhold flags); `None` is the playground's preview, which reads no bot's state.
pub fn document(
    hub: &Hub,
    bot: Option<&str>,
    name: &str,
    title: Option<&str>,
    props: &Value,
    data_requests: Option<&Value>,
) -> Result<Component> {
    let Some(spec) = catalogue::find(name) else {
        bail!("`name` is \"{name}\"; the components are: {}.", catalogue::names().join(", "));
    };
    if let Some(bot) = bot
        && !hub.store.component_available(bot, spec.name)?
    {
        bail!("The person withheld {} cards from this bot's gallery.", spec.name);
    }
    data::check(spec, data_requests)?;
    let mut given: Map<String, Value> = match props {
        Value::Object(props) => props.clone(),
        Value::Null => Map::new(),
        _ => bail!("`props` must be an object."),
    };
    let source = match bot {
        Some(bot) => data::fill(hub, bot, spec, data_requests, &mut given)?,
        None => None,
    };
    schema::build(spec, title.map(str::to_owned), source, &Value::Object(given))
}

/// The entry data every client routes to its component card: what older clients fall back to beside
/// the card (a plain title and, for a form, one button).
pub fn card(component: &Component) -> Value {
    let doc = &component.doc;
    let options: Vec<Value> = match &component.submit {
        Some(label) => vec![json!({"optionId": "submit", "name": label, "kind": "primary"})],
        None => Vec::new(),
    };
    let title = doc["title"].as_str().or_else(|| doc["name"].as_str()).unwrap_or(UNTITLED);
    json!({
        "title": title,
        "toolKind": "component",
        "command": Value::Null,
        "detail": "",
        "diffs": [],
        "options": options,
        "status": if component.asks() { "pending" } else { "shown" },
        "selected": Value::Null,
        "block": Value::Null,
        "component": doc,
        "values": Value::Null,
        "inputEntryIds": [],
    })
}

/// What a client sent back for a `form` card, checked against the card's own fields. A card that
/// asked nothing, or a `None`, passes through untouched.
pub fn answers(doc: &Value, values: Option<Value>) -> Result<Option<Value>> {
    Ok(schema::parse(doc)?.answers(values)?)
}

/// The component a saved instance describes, filled with a bot's state: the gallery's live preview.
pub fn preview(hub: &Hub, bot: Option<&str>, instance: &crate::store::components::ComponentInstance) -> Result<Value> {
    let title = (!instance.title.is_empty()).then_some(instance.title.as_str());
    let component = document(hub, bot, &instance.name, title, &instance.props, Some(&instance.data))?;
    Ok(json!({"component": component.doc, "schema": schema_of(&instance.name)}))
}

/// One component's props schema, as the playground renders its form.
pub fn schema_of(name: &str) -> Value {
    let doc = catalogue::doc();
    doc["components"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|entry| entry["name"] == name)
        .map(|entry| entry["schema"].clone())
        .unwrap_or(Value::Null)
}
