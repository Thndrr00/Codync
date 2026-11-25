//! Turns a merged flow document into the values the runnable flow tables take.
//!
//! A document is drawing state keyed by id; the store needs typed rows (links, canvas positions,
//! placed bots and media steps) before a canvas reload can show what was merged. This module owns
//! that conversion and its validation: every expected field must be there with the right name and
//! type, every cap is checked, and a bad entry is an error rather than one silently left out. It is
//! pure — no store, no clock — so it can be tested on its own and run before any write begins.

use super::MAX_INSTRUCTION_BYTES;
use super::format;
use super::merge::{FlowDocument, MAX_LINKS, MAX_NODES, MAX_SETTINGS, MAX_STROKES};
use super::nodes;
use crate::store::FlowMedia;
use crate::store::flow_media;
use crate::store::flows::{FlowLink, FlowNode, FlowPosition, check_coordinate, check_node_key, parse_port};
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::collections::BTreeMap;

/// How many model options a link may carry, and how long an id or a text field may be.
const MAX_MODEL_OPTIONS: usize = 10;
const MAX_TEXT_CHARS: usize = 200;
/// A clip's length in seconds; far beyond any media model's declared range.
const MAX_DURATION: u32 = 3600;

/// The runnable state read out of one merged document: links and media steps of the routines on the
/// canvas, and the nodes and placed bots of that canvas itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunnableFlow {
    pub links: Vec<FlowLink>,
    pub positions: Vec<FlowPosition>,
    pub placed: Vec<String>,
    pub media: Vec<FlowMedia>,
    /// The non-bot workflow nodes the document carried: routed steps and canvas-only nodes.
    pub nodes: Vec<FlowNode>,
}

/// A required string field of an entry.
fn text<'a>(entry: &'a serde_json::Map<String, Value>, name: &str, where_: &str) -> Result<&'a str> {
    entry.get(name).and_then(Value::as_str).with_context(|| format!("{where_} needs `{name}` as text"))
}

/// A required finite coordinate field of an entry.
fn coordinate(entry: &serde_json::Map<String, Value>, name: &str, where_: &str) -> Result<f64> {
    let value =
        entry.get(name).and_then(Value::as_f64).with_context(|| format!("{where_} needs `{name}` as a number"))?;
    check_coordinate(value).with_context(|| format!("{where_} has an out-of-range `{name}`"))?;
    Ok(value)
}

/// A small id or text value on the wire: present, short and free of control characters.
fn short(value: &str, where_: &str) -> Result<()> {
    if value.is_empty() || value.chars().count() > MAX_TEXT_CHARS || value.chars().any(char::is_control) {
        bail!("{where_} must be 1 to {MAX_TEXT_CHARS} characters");
    }
    Ok(())
}

/// A step's name on the wire: absent or empty on an older canvas reads as unnamed.
fn optional_name(entry: &serde_json::Map<String, Value>, where_: &str) -> Result<String> {
    let name = match entry.get("name") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(name)) => name.trim().to_owned(),
        _ => bail!("{where_}'s `name` must be text or null"),
    };
    crate::store::flows::check_name(&name).with_context(|| format!("{where_}'s name"))?;
    Ok(name)
}

fn node(position: &Value, key: &str, state: &mut RunnableFlow) -> Result<()> {
    check_node_key(key)?;
    let where_ = format!("node {key}");
    let entry = position.as_object().with_context(|| format!("{where_} must be an object"))?;
    let x = coordinate(entry, "x", &where_)?;
    let y = coordinate(entry, "y", &where_)?;
    if format::node_size(entry).is_none() {
        bail!("{where_} has an invalid `size`");
    }
    state.positions.push(FlowPosition { node_key: key.to_owned(), x, y });
    if format::node_locked(entry) {
        // A locked node is a newer kind's placeholder: it is drawn, never handed work.
        return Ok(());
    }
    if let Some(placed) = entry.get("placed") {
        let placed = placed.as_bool().with_context(|| format!("{where_}'s `placed` must be true or false"))?;
        if placed {
            let bot = key.strip_prefix("bot:").context("only a bot node can be placed")?;
            short(bot, &where_)?;
            state.placed.push(bot.to_owned());
        }
    }
    Ok(())
}

fn link(id: &str, value: &Value, bot: &str) -> Result<FlowLink> {
    let where_ = format!("link {id}");
    let entry = value.as_object().with_context(|| format!("{where_} must be an object"))?;
    let link_id = text(entry, "id", &where_)?;
    if link_id != id {
        bail!("{where_} does not match its id");
    }
    short(id, &where_)?;
    let routine_id = text(entry, "routineId", &where_)?;
    let from = text(entry, "fromBotId", &where_)?;
    let to = text(entry, "toBotId", &where_)?;
    short(routine_id, &format!("{where_} routineId"))?;
    short(from, &format!("{where_} fromBotId"))?;
    short(to, &format!("{where_} toBotId"))?;
    let instruction = text(entry, "instruction", &where_)?.to_owned();
    if instruction.len() > MAX_INSTRUCTION_BYTES {
        bail!("{where_}'s instruction must be at most {MAX_INSTRUCTION_BYTES} bytes");
    }
    let model = match entry.get("model") {
        None | Some(Value::Null) => None,
        Some(Value::String(model)) if !model.is_empty() => {
            short(model, &format!("{where_} model"))?;
            Some(model.clone())
        }
        _ => bail!("{where_}'s `model` must be text or null"),
    };
    let mut model_options = BTreeMap::new();
    if let Some(options) = entry.get("modelOptions").filter(|value| !value.is_null()) {
        let options = options.as_object().with_context(|| format!("{where_}'s `modelOptions` must be an object"))?;
        if options.len() > MAX_MODEL_OPTIONS {
            bail!("{where_} must have at most {MAX_MODEL_OPTIONS} model options");
        }
        for (name, value) in options {
            let value = value.as_str().with_context(|| format!("{where_}'s model option `{name}` must be text"))?;
            short(name, &format!("{where_} model option"))?;
            short(value, &format!("{where_} model option `{name}`"))?;
            model_options.insert(name.clone(), value.to_owned());
        }
    }
    // Condition edges carry a yes/no branch (OpenBot 0.70.0); a plain hand-off carries none.
    let branch = match entry.get("branch") {
        None | Some(Value::Null) => None,
        Some(Value::String(branch)) if branch == "yes" || branch == "no" => Some(branch.clone()),
        _ => bail!("{where_}'s `branch` must be yes, no or null"),
    };
    let port = parse_port(entry.get("port").and_then(Value::as_str))?;
    let created_at = entry
        .get("createdAt")
        .and_then(Value::as_i64)
        .with_context(|| format!("{where_} needs `createdAt` as a number"))?;
    // The wire form omits the owner (it is `#[serde(skip)]`), so a fresh link inherits this canvas.
    // The store keeps the owner of a link it already knows instead.
    let owner_bot_id = entry.get("ownerBotId").and_then(Value::as_str).unwrap_or(bot).to_owned();
    Ok(FlowLink {
        id: id.to_owned(),
        routine_id: routine_id.to_owned(),
        owner_bot_id,
        from_bot_id: from.to_owned(),
        to_bot_id: to.to_owned(),
        name: optional_name(entry, &where_)?,
        instruction,
        model,
        model_options,
        branch,
        port,
        created_at,
    })
}

fn media(id: &str, value: &Value, bot: &str) -> Result<FlowMedia> {
    let where_ = format!("step settings {id}");
    let entry = value.as_object().with_context(|| format!("{where_} must be an object"))?;
    let media_id = text(entry, "id", &where_)?;
    if media_id != id {
        bail!("{where_} do not match their id");
    }
    short(id, &where_)?;
    let routine_id = text(entry, "routineId", &where_)?;
    short(routine_id, &format!("{where_} routineId"))?;
    let kind = text(entry, "kind", &where_)?;
    if kind != "image" && kind != "video" {
        bail!("{where_} must be an image or a video");
    }
    let model = text(entry, "model", &where_)?;
    short(model, &format!("{where_} model"))?;
    let prompt = text(entry, "prompt", &where_)?.to_owned();
    if prompt.len() > flow_media::MAX_PROMPT_BYTES {
        bail!("{where_} must be at most {} bytes", flow_media::MAX_PROMPT_BYTES);
    }
    let aspect = text(entry, "aspect", &where_)?.to_owned();
    short(&aspect, &format!("{where_} aspect"))?;
    let resolution = text(entry, "resolution", &where_)?.to_owned();
    short(&resolution, &format!("{where_} resolution"))?;
    // A document saved before batches existed has no field, so it keeps making one result.
    let batch = entry
        .get("batch")
        .map(|value| {
            value
                .as_u64()
                .and_then(|number| u32::try_from(number).ok())
                .with_context(|| format!("{where_}'s `batch` must be 1 to 4"))
        })
        .transpose()?
        .map(crate::media::media_batch)
        .unwrap_or_else(crate::media::default_batch);
    let duration = entry
        .get("duration")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .context("`duration` must be seconds")?;
    if duration > MAX_DURATION {
        bail!("{where_} cannot be longer than {MAX_DURATION} seconds");
    }
    let transparent =
        entry.get("transparent").and_then(Value::as_bool).context("`transparent` must be true or false")?;
    let voice = match entry.get("voice") {
        None | Some(Value::Null) => None,
        Some(Value::String(voice)) if !voice.is_empty() => {
            short(voice, &format!("{where_} voice"))?;
            Some(voice.clone())
        }
        _ => bail!("{where_}'s `voice` must be text or null"),
    };
    let created_at = entry
        .get("createdAt")
        .and_then(Value::as_i64)
        .with_context(|| format!("{where_} needs `createdAt` as a number"))?;
    let owner_bot_id = entry.get("ownerBotId").and_then(Value::as_str).unwrap_or(bot).to_owned();
    Ok(FlowMedia {
        id: id.to_owned(),
        name: optional_name(entry, &where_)?,
        kind: kind.to_owned(),
        model: model.to_owned(),
        prompt,
        aspect,
        batch,
        duration,
        resolution,
        transparent,
        voice,
        created_at,
        routine_id: routine_id.to_owned(),
        owner_bot_id,
    })
}

/// A settings entry whose kind is not a media kind is one of the routed node kinds of the
/// OpenBot 0.70.0 registry. It is validated here and kept as a node; it has no row of its own until
/// an API creates it, so a document save can carry it without executing it.
fn workflow_node(id: &str, value: &Value, bot: &str) -> Result<FlowNode> {
    let where_ = format!("node settings {id}");
    let entry = value.as_object().with_context(|| format!("{where_} must be an object"))?;
    let node_id = text(entry, "id", &where_)?;
    if node_id != id {
        bail!("{where_} do not match their id");
    }
    short(id, &where_)?;
    let routine_id = text(entry, "routineId", &where_)?;
    short(routine_id, &format!("{where_} routineId"))?;
    let kind = text(entry, "kind", &where_)?;
    ensure!(
        matches!(kind, "llm" | "condition" | "approval" | "output" | "asset" | "trigger"),
        "{where_} has an unknown kind `{kind}`"
    );
    let fields: BTreeMap<String, Value> = match entry.get("fields") {
        None | Some(Value::Null) => BTreeMap::new(),
        Some(Value::Object(fields)) => fields.clone(),
        _ => bail!("{where_}'s `fields` must be an object"),
    };
    // Validate the fields the engine or the API will read, so a bad document never reaches them.
    match kind {
        "llm" => {
            nodes::llm_step_spec(value)?;
        }
        "output" => {
            nodes::output_action(value)?;
        }
        _ => {}
    }
    let created_at = entry
        .get("createdAt")
        .and_then(Value::as_i64)
        .with_context(|| format!("{where_} needs `createdAt` as a number"))?;
    let owner_bot_id = entry.get("ownerBotId").and_then(Value::as_str).unwrap_or(bot).to_owned();
    Ok(FlowNode {
        id: id.to_owned(),
        routine_id: routine_id.to_owned(),
        owner_bot_id,
        kind: kind.to_owned(),
        fields,
        created_at,
    })
}

/// Converts a merged document into the validated shapes the flow tables take. `bot` owns the canvas
/// and is the owner of last resort for a link or media entry the wire form does not name one for.
pub fn runnable(bot: &str, document: &FlowDocument) -> Result<RunnableFlow> {
    if document.nodes.len() > MAX_NODES {
        bail!("a document holds at most {MAX_NODES} nodes");
    }
    if document.links.len() > MAX_LINKS {
        bail!("a document holds at most {MAX_LINKS} links");
    }
    if document.settings.len() > MAX_SETTINGS {
        bail!("a document holds at most {MAX_SETTINGS} step settings");
    }
    if document.strokes.len() > MAX_STROKES {
        bail!("a document holds at most {MAX_STROKES} strokes");
    }
    // Strokes are persisted and merged but never executed: a bad one is still refused on save.
    format::check_strokes(&document.strokes)?;
    let mut state = RunnableFlow::default();
    for (key, value) in &document.nodes {
        node(value, key, &mut state)?;
    }
    for (id, value) in &document.links {
        state.links.push(link(id, value, bot)?);
    }
    for (id, value) in &document.settings {
        let kind = value.get("kind").and_then(Value::as_str).unwrap_or_default();
        if kind == "image" || kind == "video" {
            state.media.push(media(id, value, bot)?);
        } else {
            state.nodes.push(workflow_node(id, value, bot)?);
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routines::flow::merge;
    use serde_json::json;

    fn doc(value: Value) -> FlowDocument {
        serde_json::from_value(value).expect("a well-formed test document")
    }

    fn sample() -> FlowDocument {
        doc(json!({
            "rev": 4,
            "nodes": {"bot:ada": {"x": 1.0, "y": 2.0}, "media:pic": {"x": 10.0, "y": 20.0}},
            "links": {
                "l1": {"id": "l1", "routineId": "r1", "fromBotId": "owner", "toBotId": "ada",
                       "instruction": "Translate", "model": null, "modelOptions": {}, "createdAt": 7},
                "l2": {"id": "l2", "routineId": "r1", "fromBotId": "ada", "toBotId": "media:pic",
                       "instruction": "", "model": null, "modelOptions": {}, "createdAt": 8}
            },
            "settings": {
                "pic": {"id": "pic", "routineId": "r1", "kind": "image", "model": "gpt-image",
                        "prompt": "a picture", "aspect": "1:1", "batch": 2, "duration": 6, "resolution": "480p",
                        "transparent": false, "voice": null, "createdAt": 9}
            },
            "groups": {"g1": {"id": "g1", "name": "Pictures", "members": ["media:pic"]}}
        }))
    }

    #[test]
    fn a_valid_document_round_trips_into_runnable_shapes() {
        let document = sample();
        let state = runnable("owner", &document).expect("a valid document");
        assert_eq!(state.links.len(), 2);
        assert_eq!(state.media.len(), 1);
        assert_eq!(state.media[0].owner_bot_id, "owner");
        assert_eq!(state.positions.len(), 2);
        assert!(state.placed.is_empty());

        // The shapes serialize back to the same four maps (owner ids are `#[serde(skip)]`).
        let back = FlowDocument {
            rev: document.rev,
            nodes: state
                .positions
                .iter()
                .map(|position| (position.node_key.clone(), json!({"x": position.x, "y": position.y})))
                .collect(),
            links: state
                .links
                .iter()
                .map(|link| (link.id.clone(), serde_json::to_value(link).expect("a serializable link")))
                .collect(),
            settings: state
                .media
                .iter()
                .map(|media| (media.id.clone(), serde_json::to_value(media).expect("serializable media")))
                .collect(),
            groups: document.groups.clone(),
        };
        for (id, value) in &document.links {
            assert_eq!(back.links[id], *value, "link {id} survives the round trip");
        }
        for (id, value) in &document.settings {
            assert_eq!(back.settings[id], *value, "step settings {id} survive the round trip");
        }
        assert_eq!(back.nodes, document.nodes);
    }

    #[test]
    fn a_link_only_one_side_kept_is_applied() {
        let full = sample();
        let mut base_links = full.links.clone();
        base_links.remove("l2");
        let base = FlowDocument { links: base_links, ..full.clone() };
        let merged = merge::merge(Some(&base), &full, &base.clone());
        assert!(merged.conflicts.is_empty());

        let state = runnable("owner", &merged.document).expect("the merged document");
        assert!(state.links.iter().any(|link| link.id == "l2"), "the link only one editor had runs");
        assert_eq!(state.links.len(), 2);
    }

    #[test]
    fn a_removed_link_is_not_applied() {
        let base = sample();
        let mut local_links = base.links.clone();
        local_links.remove("l2");
        let local = FlowDocument { links: local_links, ..base.clone() };
        let merged = merge::merge(Some(&base), &local, &base.clone());
        assert!(merged.conflicts.is_empty());

        let state = runnable("owner", &merged.document).expect("the merged document");
        assert!(state.links.iter().all(|link| link.id != "l2"), "a removed link is gone from the canvas");
        assert_eq!(state.links.len(), 1);
    }

    #[test]
    fn a_malformed_entry_is_rejected() {
        let mut document = sample();
        document.nodes.insert("bot:bad".into(), json!({"x": "not a number", "y": 2.0}));
        assert!(runnable("owner", &document).is_err(), "a node without numeric coordinates is refused");

        let mut document = sample();
        document.links.insert("l3".into(), json!({"id": "another", "routineId": "r1"}));
        assert!(runnable("owner", &document).is_err(), "a link whose id does not match is refused");

        let mut document = sample();
        document.settings.insert("bad".into(), json!({"id": "bad", "kind": "sound"}));
        assert!(runnable("owner", &document).is_err(), "a media step of an unknown kind is refused");
    }

    #[test]
    fn a_step_name_survives_a_merged_round_trip() {
        let mut document = sample();
        document.links.insert(
            "l1".into(),
            json!({"id": "l1", "routineId": "r1", "fromBotId": "owner", "toBotId": "ada",
                   "instruction": "Translate", "model": null, "modelOptions": {}, "createdAt": 7,
                   "name": "Translation"}),
        );
        let media = document.settings.get_mut("pic").unwrap().as_object_mut().unwrap();
        media.insert("name".into(), json!("Hero shot"));
        let state = runnable("owner", &document).expect("a document with names");
        assert_eq!(state.links.iter().find(|link| link.id == "l1").unwrap().name, "Translation");
        assert_eq!(state.media[0].name, "Hero shot");
        assert_eq!(serde_json::to_value(&state.links[0]).unwrap()["name"], "Translation");
    }

    #[test]
    fn an_old_document_without_names_reads_as_unnamed() {
        let state = runnable("owner", &sample()).expect("an old document");
        assert!(state.links.iter().all(|link| link.name.is_empty()));
        assert!(state.media.iter().all(|media| media.name.is_empty()));
    }

    #[test]
    fn a_document_without_a_batch_makes_one_result_and_a_wide_one_is_bounded() {
        let mut document = sample();
        document.settings.get_mut("pic").unwrap().as_object_mut().unwrap().remove("batch");
        assert_eq!(runnable("owner", &document).unwrap().media[0].batch, 1);
        document.settings.get_mut("pic").unwrap().as_object_mut().unwrap().insert("batch".into(), json!(9));
        assert_eq!(runnable("owner", &document).unwrap().media[0].batch, 4);
        document.settings.get_mut("pic").unwrap().as_object_mut().unwrap().insert("batch".into(), json!("four"));
        assert!(runnable("owner", &document).is_err(), "a batch that is not a number is refused");
    }

    #[test]
    fn strokes_are_kept_out_of_the_runnable_tables_but_still_checked() {
        let mut document = sample();
        document.strokes.insert(
            "s1".into(),
            json!({"id": "s1", "brush": "pen", "color": "red", "width": 2.0, "points": [{"x": 1, "y": 2}]}),
        );
        let state = runnable("owner", &document).expect("a drawing on the canvas");
        assert_eq!(state.links.len(), 2, "a stroke never becomes a link");
        assert_eq!(state.positions.len(), 2);

        document.strokes.insert("bad".into(), json!({"id": "bad", "width": -1.0, "points": []}));
        assert!(runnable("owner", &document).is_err(), "a malformed stroke is refused");
    }

    #[test]
    fn a_sized_locked_node_is_drawn_but_never_placed() {
        let mut document = sample();
        document.nodes.insert(
            "bot:grace".into(),
            json!({"x": 3.0, "y": 4.0, "placed": true, "locked": true, "size": {"font": 12.0}}),
        );
        let state = runnable("owner", &document).expect("a locked node");
        assert_eq!(state.positions.len(), 3, "its place on the canvas survives");
        assert!(state.placed.is_empty(), "a locked node cannot be placed");

        document.nodes.insert("bot:bad".into(), json!({"x": 1.0, "y": 2.0, "size": {"width": 0.0}}));
        assert!(runnable("owner", &document).is_err(), "an invalid size is refused");
    }

    #[test]
    fn a_placed_bot_becomes_a_placed_row() {
        let mut document = sample();
        document.nodes.insert("bot:grace".into(), json!({"x": 3.0, "y": 4.0, "placed": true}));
        let state = runnable("owner", &document).expect("a placed bot with a position");
        assert_eq!(state.placed, vec!["grace".to_owned()]);
        assert_eq!(state.positions.len(), 3);
    }
}
