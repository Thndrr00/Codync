//! Host API methods for portable, paused and templated workflows.
//!
//! These methods sit beside the canvas API in `api.rs` but keep their own file: they build a
//! `openbot.workflow` file from the host's flow store, import one into a new paused routine,
//! duplicate a workflow as a paused copy, pause and resume one, and set or render an output step's
//! message and notification fields. All decisions they apply come from pure functions in `format`,
//! `portable`, `pause`, `trigger_fields` and `output_template`.

use super::{answers, api, format, groups, output_template, pause, portable, trigger_fields};
use crate::hub::Hub;
use crate::store::{FlowChange, FlowLink, Store, now_ms};
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const METHODS: &[&str] = &[
    "pauseRoutineWorkflow",
    "resumeRoutineWorkflow",
    "duplicateRoutineWorkflow",
    "exportRoutineWorkflow",
    "importRoutineWorkflow",
    "setRoutineFlowOutput",
    "workflowOutputNotification",
];

fn id(b: &Value, key: &str) -> Result<&str> {
    b[key].as_str().filter(|id| !id.is_empty()).with_context(|| format!("`{key}` is required"))
}

fn optional_text(b: &Value, key: &str) -> Result<Option<String>> {
    match b.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        _ => bail!("`{key}` must be text"),
    }
}

fn routine(hub: &Hub, bot: &str, routine_id: &str) -> Result<crate::routines::Routine> {
    let routine = api::routine(hub, routine_id)?;
    if routine.bot_id != bot {
        bail!("routine not found among this bot's routines");
    }
    Ok(routine)
}

/// Stamps a canvas revision so clients reload every card whose workflow changed. The owner's card
/// and every bot a link reaches are in the audience of one change event.
fn touch(hub: &Hub, bot: &str, routine_id: &str) -> Result<()> {
    hub.emit_flow_change(|| {
        let saved = groups::save(&hub.store, bot, &groups::load(&hub.store, bot))?;
        Ok(((), Some(FlowChange { rev: saved.rev, bots: affected_bots(&hub.store, routine_id, bot) })))
    })?;
    Ok(())
}

fn set_paused(hub: &Arc<Hub>, b: &Value, paused: bool) -> Result<Value> {
    let (bot, routine_id) = (id(b, "botId")?, id(b, "routineId")?);
    routine(hub, bot, routine_id)?;
    pause::set(&hub.store, routine_id, paused)?;
    if paused {
        // A workflow already running must stop when it is paused.
        hub.routines.flows.cancel_routine(routine_id);
        hub.routines.cancel_flow_deliveries(&hub.store, routine_id)?;
    }
    touch(hub, bot, routine_id)?;
    Ok(json!({"routineId": routine_id, "paused": paused}))
}

/// The settings of a file step that came from a link: its instruction and the step's answer/check
/// specs, with no ids, connections or accounts.
fn agent_settings(instruction: &str, spec: &answers::NodeSpec) -> BTreeMap<String, Value> {
    let mut settings = BTreeMap::from([("instruction".to_owned(), json!(instruction))]);
    if !spec.answer.is_empty() {
        settings.insert("answer".to_owned(), json!(spec.answer));
    }
    if !spec.check.is_empty() {
        settings.insert("check".to_owned(), json!(spec.check));
    }
    settings
}

/// Builds a portable workflow from the host's routine, links, media steps and specs.
fn export_data(hub: &Hub, r: &crate::routines::Routine) -> Result<portable::WorkflowData> {
    let mut steps = Vec::new();
    let mut agents = Vec::new();
    let mut channels = Vec::new();
    let zone = r
        .triggers
        .iter()
        .find_map(|trigger| match trigger {
            crate::routines::Trigger::Cron { time_zone, .. } => Some(time_zone.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let mut trigger_settings = BTreeMap::new();
    trigger_settings.insert("timeZone".to_owned(), json!(zone));
    steps.push(portable::WorkflowStep { name: r.name.clone(), kind: "trigger".into(), settings: trigger_settings });

    let specs = answers::load_specs(&hub.store, &r.bot_id);
    for link in hub.store.flow_links(&r.id)? {
        let spec = specs.get(&link.to_bot_id).cloned().unwrap_or_default();
        let name =
            if link.name.trim().is_empty() { hub.bot_name(&link.to_bot_id) } else { link.name.trim().to_owned() };
        let kind = if spec.check.trim().is_empty() { "agent" } else { "check" };
        steps.push(portable::WorkflowStep {
            name: name.clone(),
            kind: kind.to_owned(),
            settings: portable::portable_settings(&agent_settings(&link.instruction, &spec)),
        });
        if !agents.contains(&name) {
            agents.push(name);
        }
    }
    for node in hub.store.flow_media(&r.id)? {
        let saved: BTreeMap<String, Value> = serde_json::to_value(&node)
            .context("a media step cannot be serialized")?
            .as_object()
            .context("a media step is an object")?
            .clone()
            .into_iter()
            .collect();
        steps.push(portable::WorkflowStep {
            name: if node.name.trim().is_empty() { format!("{} step", node.kind) } else { node.name.trim().to_owned() },
            kind: "media".into(),
            settings: portable::portable_settings(&saved),
        });
    }
    for output in &r.outputs {
        let mut settings =
            BTreeMap::from([("kind".to_owned(), json!(output.kind)), ("target".to_owned(), json!(output.target))]);
        if let Some(value) = output.value.as_deref() {
            settings.insert("value".to_owned(), json!(value));
        }
        steps.push(portable::WorkflowStep {
            name: output.kind.clone(),
            kind: "output".into(),
            settings: portable::portable_settings(&settings),
        });
        if ["slack", "discord", "telegram", "whatsapp"].contains(&output.kind.as_str())
            && !channels.contains(&output.target)
        {
            channels.push(output.target.clone());
        }
    }
    let template = output_template::load(&hub.store, &r.id);
    if !template.is_empty() {
        let values = serde_json::to_value(&template).context("an output template cannot be serialized")?;
        if let Some(object) = values.as_object() {
            for (key, value) in object {
                if let Some(step) = steps.iter_mut().find(|step| step.kind == "output") {
                    step.settings.insert(key.clone(), value.clone());
                }
            }
        }
    }
    Ok(portable::WorkflowData {
        name: r.name.clone(),
        instruction: r.instruction.clone(),
        time_zone: zone,
        steps,
        agents,
        channels,
        notes: String::new(),
    })
}

/// The files a workflow's media steps made, read from the newest steps' results as `name (path)`
/// lines and carried in the file as base64. A file that no longer exists is not part of the export.
fn export_assets(hub: &Hub, r: &crate::routines::Routine) -> Result<BTreeMap<String, String>> {
    let mut assets = BTreeMap::new();
    for step in hub.store.flow_last_steps(&r.id)? {
        let Some(output) = step.output.as_deref() else { continue };
        for line in output.lines() {
            let Some(open) = line.rfind('(') else { continue };
            let Some(close) = line.rfind(')').filter(|close| *close > open) else { continue };
            let path = std::path::Path::new(&line[open + 1..close]);
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().map_or_else(|| "file".to_owned(), |name| name.to_string_lossy().into_owned());
            let bytes = std::fs::read(path).with_context(|| format!("could not read the asset {}", path.display()))?;
            assets.insert(name, BASE64.encode(bytes));
        }
    }
    Ok(assets)
}

fn export(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let (bot, routine_id) = (id(b, "botId")?, id(b, "routineId")?);
    let r = routine(hub, bot, routine_id)?;
    let workflow = export_data(hub, &r)?;
    let secrets = portable::secret_steps(&workflow);
    if !secrets.is_empty() {
        bail!("{}", portable::FileProblem::Secrets(secrets).message());
    }
    let file = portable::WorkflowFile {
        format_version: format::FORMAT_VERSION,
        workflow: workflow.clone(),
        assets: export_assets(hub, &r)?,
    };
    portable::validate(&file)?;
    let bytes = portable::encode(&file)?;
    Ok(json!({
        "fileName": portable::FILE_NAME,
        "formatVersion": file.format_version,
        "file": BASE64.encode(bytes),
        "workflow": workflow,
    }))
}

/// Rebuilds a media step from the file's settings. Connections and accounts are absent on purpose,
/// so the step is made with the model and description the file carried and no computer identity.
fn media_from_file(step: &portable::WorkflowStep, routine_id: &str, bot: &str) -> Result<crate::store::FlowMedia> {
    let mut node =
        crate::store::flow_media::new_media(uuid::Uuid::new_v4().to_string(), routine_id.to_owned(), bot.to_owned());
    node.name = step.name.clone();
    for (key, value) in &step.settings {
        match key.as_str() {
            "kind" => {
                if let Some(text) = value.as_str() {
                    node.kind = text.to_owned();
                }
            }
            "model" => {
                if let Some(text) = value.as_str() {
                    node.model = text.to_owned();
                }
            }
            "prompt" => {
                if let Some(text) = value.as_str() {
                    node.prompt = text.to_owned();
                }
            }
            "aspect" => {
                if let Some(text) = value.as_str() {
                    node.aspect = text.to_owned();
                }
            }
            "batch" => {
                if let Some(number) = value.as_u64().and_then(u32::try_from).ok() {
                    node.batch = crate::media::media_batch(number);
                }
            }
            "duration" => {
                if let Some(seconds) = value.as_u64().and_then(u32::try_from).ok() {
                    node.duration = seconds;
                }
            }
            "resolution" => {
                if let Some(text) = value.as_str() {
                    node.resolution = text.to_owned();
                }
            }
            "transparent" => {
                if let Some(flag) = value.as_bool() {
                    node.transparent = flag;
                }
            }
            "voice" => {
                if let Some(text) = value.as_str() {
                    node.voice = Some(text.to_owned());
                }
            }
            _ => {}
        }
    }
    crate::media::check_settings(&node)?;
    Ok(node)
}

/// Rebuilds the file's steps as links and media nodes of an imported routine. Agents and channels
/// were stored by name; `mapped` gives the id the person chose on this computer. The steps are
/// chained in file order starting at the routine's own bot, and the answer/check spec of each one
/// is saved with it through the same path a client uses when connecting bots.
fn import_steps(
    hub: &Hub,
    bot: &str,
    routine_id: &str,
    steps: &[portable::WorkflowStep],
    mapped: &BTreeMap<String, String>,
) -> Result<()> {
    let r = api::routine(hub, routine_id)?;
    for step in steps.iter().filter(|step| step.kind == "trigger") {
        let answer = step.settings.get("answer").and_then(Value::as_str).unwrap_or_default();
        let check = step.settings.get("check").and_then(Value::as_str).unwrap_or_default();
        answers::save_spec(&hub.store, bot, &format!("routine:{routine_id}"), Some(answer), Some(check))?;
    }
    let mut from = r.bot_id.clone();
    for step in steps.iter().filter(|step| matches!(step.kind.as_str(), "agent" | "check" | "media")) {
        let to = if step.kind == "media" {
            let node = media_from_file(step, routine_id, bot)?;
            let key = crate::store::flow_media::key(&node.id);
            hub.store.flow_add_media(&node)?;
            key
        } else {
            (*mapped.get(&step.name).with_context(|| format!("no agent or channel was chosen for {}", step.name))?)
                .clone()
        };
        let instruction = step.settings.get("instruction").and_then(Value::as_str).unwrap_or_default().to_owned();
        let answer = step.settings.get("answer").and_then(Value::as_str).unwrap_or_default();
        let check = step.settings.get("check").and_then(Value::as_str).unwrap_or_default();
        api::connect(
            hub,
            &r,
            &from,
            &to,
            instruction,
            Some(answer).filter(|text| !text.is_empty()),
            Some(check).filter(|text| !text.is_empty()),
        )?;
        from = to;
    }
    Ok(())
}

/// The bytes of an import request: either the JSON file itself or its base64 encoding.
fn file_bytes(raw: &str) -> Result<Vec<u8>> {
    if raw.trim_start().starts_with('{') {
        Ok(raw.as_bytes().to_vec())
    } else {
        BASE64.decode(raw.trim()).context("`file` must be an openbot.workflow file or its base64 text")
    }
}

fn import(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let bot = id(b, "botId")?;
    let raw = b["file"].as_str().context("`file` is required")?;
    let file = portable::decode(&file_bytes(raw)?).map_err(|problem| anyhow!(problem.message()))?;
    portable::validate(&file)?;
    let mut names = file.workflow.agents.clone();
    for name in &file.workflow.channels {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    let chosen: BTreeMap<String, String> = match b.get("names").filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone()).context("`names` must map names to ids")?,
        None => BTreeMap::new(),
    };
    let mapped = portable::resolve_names(&names, &chosen)
        .map_err(|missing| anyhow!("choose an agent or channel for: {}", missing.join(", ")))?;

    // Importing over a workflow on this computer keeps the settings the file left out, so a
    // transfer cannot undo a local choice (a model, a step's prompt, an answer spec).
    let replace = b["replaceRoutineId"].as_str().map(str::to_owned);
    let mut workflow = file.workflow.clone();
    if let Some(existing) = &replace {
        let old = routine(hub, bot, existing)?;
        let saved: BTreeMap<String, BTreeMap<String, Value>> =
            export_data(hub, &old)?.steps.into_iter().map(|step| (step.name, step.settings)).collect();
        portable::keep_saved_settings(&mut workflow, &saved);
    }

    let name = optional_text(b, "name")?.filter(|name| !name.is_empty()).unwrap_or_else(|| workflow.name.clone());
    let mut body = json!({"name": name, "instruction": workflow.instruction, "triggers": []});
    if let Some(existing) = &replace {
        body["id"] = json!(existing);
    }
    let saved = hub.routines.save(hub, bot, &body)?;
    let routine_id = saved["routine"]["id"].as_str().context("the imported routine has no id")?.to_owned();

    import_steps(hub, bot, &routine_id, &workflow.steps, &mapped)?;
    for step in &workflow.steps {
        let template = output_template::OutputTemplate {
            message: step.settings.get("message").and_then(Value::as_str).map(str::to_owned),
            title: step.settings.get("title").and_then(Value::as_str).map(str::to_owned),
            labels: step.settings.get("labels").and_then(Value::as_str).map(str::to_owned),
            person: step.settings.get("person").and_then(Value::as_str).map(str::to_owned),
        };
        if !template.is_empty() {
            output_template::save(&hub.store, &routine_id, &template)?;
        }
    }

    // An imported workflow starts paused, whatever the file said: its connections and accounts
    // still have to be made on this computer.
    pause::set(&hub.store, &routine_id, portable::import_starts_paused())?;
    format::save_version(&hub.store, bot, file.format_version)?;
    touch(hub, bot, &routine_id)?;
    Ok(json!({
        "routineId": routine_id,
        "paused": portable::import_starts_paused(),
        "names": mapped,
        "assets": file.assets.keys().collect::<Vec<_>>(),
    }))
}

fn duplicate(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let (bot, routine_id) = (id(b, "botId")?, id(b, "routineId")?);
    let r = routine(hub, bot, routine_id)?;
    let name = optional_text(b, "name")?.filter(|name| !name.is_empty()).unwrap_or_else(|| format!("{} copy", r.name));
    let body = json!({
        "name": name, "instruction": r.instruction, "triggers": r.triggers,
        "enabled": r.enabled, "timeoutSeconds": r.timeout_seconds, "outputs": r.outputs,
    });
    let saved = hub.routines.save(hub, bot, &body)?;
    let new_id = saved["routine"]["id"].as_str().context("the copy has no id")?.to_owned();
    for link in hub.store.flow_links(&r.id)? {
        let mut copy = link.clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.routine_id = new_id.clone();
        copy.owner_bot_id = bot.to_owned();
        copy.created_at = now_ms();
        hub.store.flow_add_link(&copy, |_| Ok(()))?;
    }
    for node in hub.store.flow_media(&r.id)? {
        let mut copy = node.clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.routine_id = new_id.clone();
        copy.owner_bot_id = bot.to_owned();
        hub.store.flow_add_media(&copy)?;
    }
    if let Some(spec) = answers::load_specs(&hub.store, bot).remove(&format!("routine:{routine_id}")) {
        answers::save_spec(&hub.store, bot, &format!("routine:{new_id}"), Some(&spec.answer), Some(&spec.check))?;
    }
    let template = output_template::load(&hub.store, routine_id);
    output_template::save(&hub.store, &new_id, &template)?;
    pause::set(&hub.store, &new_id, pause::duplicate_copy_paused())?;
    touch(hub, bot, &new_id)?;
    Ok(json!({"routineId": new_id, "paused": pause::duplicate_copy_paused()}))
}

fn set_output(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let (bot, routine_id) = (id(b, "botId")?, id(b, "routineId")?);
    routine(hub, bot, routine_id)?;
    let template = output_template::OutputTemplate {
        message: optional_text(b, "message")?,
        title: optional_text(b, "title")?,
        labels: optional_text(b, "labels")?,
        person: optional_text(b, "person")?,
    };
    output_template::check(&template)?;
    output_template::save(&hub.store, routine_id, &template)?;
    touch(hub, bot, routine_id)?;
    Ok(json!({"output": template}))
}

fn notification(hub: &Arc<Hub>, b: &Value) -> Result<Value> {
    let (bot, routine_id) = (id(b, "botId")?, id(b, "routineId")?);
    let r = routine(hub, bot, routine_id)?;
    let input = b["input"].as_str().unwrap_or_default();
    let mut values: BTreeMap<String, String> = match b.get("fields").filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone()).context("`fields` must map names to text")?,
        None => BTreeMap::new(),
    };
    let event = json!({"source": "test"});
    trigger_fields::merged_values(&mut values, &trigger_fields::fixed_answers(now_ms(), &r.triggers, &event));
    let note = output_template::notification(&output_template::load(&hub.store, routine_id), input, &values);
    Ok(json!(note))
}

/// Answers one of the portable-workflow methods, or hands the body back when it isn't one.
pub fn call(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Option<Value>> {
    if !METHODS.contains(&method) {
        return Ok(None);
    }
    Ok(Some(match method {
        "pauseRoutineWorkflow" => set_paused(hub, b, true)?,
        "resumeRoutineWorkflow" => set_paused(hub, b, false)?,
        "duplicateRoutineWorkflow" => duplicate(hub, b)?,
        "exportRoutineWorkflow" => export(hub, b)?,
        "importRoutineWorkflow" => import(hub, b)?,
        "setRoutineFlowOutput" => set_output(hub, b)?,
        "workflowOutputNotification" => notification(hub, b)?,
        _ => bail!("unknown workflow method {method}"),
    }))
}

/// The bots whose canvas must be reloaded after a workflow's data changes. Kept small and pure so
/// an import or a duplicate can name both the source and the new routine's audience.
pub fn affected_bots(store: &Store, routine_id: &str, owner: &str) -> Vec<String> {
    let mut bots = vec![owner.to_owned()];
    if let Ok(links) = store.flow_links(routine_id) {
        for link in links {
            if !bots.contains(&link.to_bot_id) {
                bots.push(link.to_bot_id);
            }
        }
    }
    bots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_import_names_every_bot_whose_canvas_changed() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let mut link = FlowLink {
            id: "l".into(),
            routine_id: "r".into(),
            owner_bot_id: "a".into(),
            from_bot_id: "a".into(),
            to_bot_id: "b".into(),
            name: String::new(),
            instruction: String::new(),
            model: None,
            model_options: BTreeMap::new(),
            created_at: 1,
        };
        store.flow_add_link(&link, |_| Ok(())).expect("a link");
        link.id = "l2".into();
        link.to_bot_id = "a".into();
        store.flow_add_link(&link, |_| Ok(())).expect("another link");
        assert_eq!(affected_bots(&store, "r", "a"), ["a", "b"]);
    }
}
