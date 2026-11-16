//! The gallery and the playground, as the host serves them: the catalogue with each component's
//! schema and a sample, the cards the playground saved, and per bot what is withheld.
//!
//! The playground writes only what this catalogue allows: a saved card is a component name plus props
//! the same validator the tool runs has already accepted, so there is no path from the playground to
//! HTML, a script or a URL. A draft stays out of the gallery until it is published.

use super::catalogue::{self, VERSION};
use super::{card, data, schema};
use crate::hub::Hub;
use crate::store::components::ComponentInstance;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// The host API methods of the gallery. `listComponents` and `previewComponent` read; the rest change
/// this computer's gallery and are owner-only.
pub const METHODS: &[&str] = &[
    "listComponents",
    "previewComponent",
    "saveComponent",
    "publishComponent",
    "deleteComponent",
    "setComponentAccess",
];

pub fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    match method {
        "listComponents" => list(hub, b),
        "previewComponent" => preview(hub, b),
        "saveComponent" => save(hub, b),
        "publishComponent" => publish(hub, b),
        "deleteComponent" => delete(hub, b),
        "setComponentAccess" => access(hub, b),
        other => bail!("unknown component method: {other}"),
    }
}

/// The whole gallery: the catalogue, the saved cards, and each bot's withhold list. With `bot`, each
/// component says whether that bot may use it.
fn list(hub: &Hub, b: &Value) -> Result<Value> {
    let bot = requested_bot(b);
    let mut doc = catalogue::doc();
    if let Some(components) = doc["components"].as_array_mut() {
        for entry in components {
            let name = entry["name"].as_str().unwrap_or_default().to_owned();
            let available = match bot {
                Some(bot) => hub.store.component_available(bot, &name)?,
                None => true,
            };
            entry["available"] = available.into();
        }
    }
    doc["instances"] = Value::Array(
        hub.store
            .component_instances()?
            .into_iter()
            .map(|instance| {
                let mut value = json!(instance);
                value["valid"] = still_valid(&instance).into();
                value
            })
            .collect(),
    );
    doc["access"] = json!(hub.store.component_access()?);
    doc["bots"] = Value::Array(bots(hub)?);
    Ok(doc)
}

/// One card as a bot would see it, with its data functions resolved: the gallery's live preview.
fn preview(hub: &Hub, b: &Value) -> Result<Value> {
    let name = b["name"].as_str().context("`name` is required")?;
    let component = card::document(hub, requested_bot(b), name, b["title"].as_str(), &b["props"], b.get("data"))
        .map_err(|error| anyhow::anyhow!("{error:#}\n\n{}", catalogue::guide()))?;
    Ok(json!({"component": component.doc}))
}

/// Saves a playground draft: the props are checked by the same validator the tool uses, and the data
/// functions are only checked (not resolved), since a draft belongs to nobody's chat yet.
fn save(hub: &Hub, b: &Value) -> Result<Value> {
    let name = b["name"].as_str().map(str::trim).filter(|name| !name.is_empty()).context("`name` is required")?;
    let Some(spec) = catalogue::find(name) else {
        bail!("`name` is \"{name}\"; the components are: {}.", catalogue::names().join(", "));
    };
    let id = b["id"]
        .as_str()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let existing = hub.store.component_instance(&id)?;
    let title = b["title"].as_str().unwrap_or_default().trim().to_owned();
    let props = match &b["props"] {
        Value::Object(props) => Value::Object(props.clone()),
        _ => bail!("`props` must be an object."),
    };
    let requests = b.get("data").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| json!([]));
    data::check(spec, Some(&requests))?;
    let component = schema::build(spec, (!title.is_empty()).then(|| title.clone()), None, &props)
        .map_err(|error| anyhow::anyhow!("{error:#}\n\n{}", catalogue::guide()))?;
    let mut instance = ComponentInstance {
        id,
        name: spec.name.to_owned(),
        version: i64::try_from(VERSION).unwrap_or(1),
        title,
        props: component.doc["props"].clone(),
        data: requests,
        author: b["bot"].as_str().map(str::to_owned),
        published: b["publish"].as_bool().unwrap_or_else(|| existing.as_ref().is_some_and(|saved| saved.published)),
        created_at: existing.as_ref().map_or(0, |saved| saved.created_at),
        updated_at: 0,
    };
    hub.store.save_component_instance(&mut instance)?;
    Ok(json!({"instance": instance}))
}

/// Shows a saved card in the gallery, or takes it out again.
fn publish(hub: &Hub, b: &Value) -> Result<Value> {
    let id = b["id"].as_str().context("`id` is required")?;
    let published = b["published"].as_bool().context("`published` is required")?;
    hub.store.set_component_published(id, published)?;
    let instance = hub.store.component_instance(id)?.context("unknown component instance")?;
    Ok(json!({"instance": instance}))
}

fn delete(hub: &Hub, b: &Value) -> Result<Value> {
    hub.store.delete_component_instance(b["id"].as_str().context("`id` is required")?)?;
    Ok(json!({}))
}

/// Lets one bot use a component, or withholds it from that bot alone.
fn access(hub: &Hub, b: &Value) -> Result<Value> {
    let bot = b["bot"].as_str().context("`bot` is required")?;
    let component = b["component"].as_str().context("`component` is required")?;
    let Some(spec) = catalogue::find(component) else {
        bail!("`component` is \"{component}\"; the components are: {}.", catalogue::names().join(", "));
    };
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("unknown bot")?;
    let published = b["published"].as_bool().context("`published` is required")?;
    hub.store.set_component_access(bot, component, published)?;
    Ok(json!({"bot": bot, "component": spec.name, "published": published}))
}

/// Every bot on this computer with the components it may not use.
fn bots(hub: &Hub) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for row in hub.store.bots()?.into_iter().filter(|row| !row.deleted) {
        let mut withheld = Vec::new();
        for name in catalogue::names() {
            if !hub.store.component_available(&row.config.id, name)? {
                withheld.push(name);
            }
        }
        out.push(json!({
            "id": row.config.id,
            "name": row.config.name,
            "kind": if row.config.is_group() { "group" } else { "agent" },
            "withheld": withheld,
        }));
    }
    Ok(out)
}

fn requested_bot(b: &Value) -> Option<&str> {
    b["bot"].as_str().map(str::trim).filter(|bot| !bot.is_empty())
}

/// Whether a saved card still passes the catalogue it names: a host update can change a component's
/// props, and the gallery says so instead of drawing a card it can't check.
fn still_valid(instance: &ComponentInstance) -> bool {
    catalogue::find(&instance.name).is_some_and(|spec| {
        let title = (!instance.title.is_empty()).then(|| instance.title.clone());
        schema::build(spec, title, None, &instance.props).is_ok()
    })
}
