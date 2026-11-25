//! The pure decisions of the workflow node kinds that are not bots or media steps.
//!
//! Ported from OpenBot 0.70.0's `WORKFLOW_NODE_REGISTRY` (`ipc-workflow-nodes.ts`) and
//! `WorkflowEdge.branch` / `WorkflowTriggerState` (`ipc-workflows.ts`). A node is a small,
//! bounded map of fields on the canvas; the host keeps it beside the flow and the engine routes it
//! the same way it routes a bot. Nothing here reads a store or a clock, so every decision is tested
//! on its own before an API or the engine calls it.

use super::answers::{self, Answer};
use crate::chat::files::SharedFile;
use crate::routines::Trigger;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The longest text a node field may carry. One answer or prompt, well below the canvas limit.
pub const MAX_FIELD_CHARS: usize = 8_000;
/// The most fields, options or mode inputs one node may declare.
pub const MAX_FIELDS: usize = 20;
/// The longest id (provider, model, connection, file…) a node may name.
pub const MAX_ID_CHARS: usize = 200;

/// The fields object of a node, whether it arrived as `{kind, fields}` or as bare fields.
pub fn fields_of(node: &Value) -> Result<&BTreeMap<String, Value>> {
    let object = if let Some(fields) = node.get("fields") { fields.as_object() } else { node.as_object() }
        .ok_or_else(|| anyhow::anyhow!("a workflow node's `fields` must be an object"))?;
    ensure!(object.len() <= MAX_FIELDS, "a workflow node has at most {MAX_FIELDS} fields");
    Ok(object)
}

fn text_field(fields: &BTreeMap<String, Value>, key: &str) -> Result<String> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(text)) if text.chars().count() <= MAX_FIELD_CHARS => Ok(text.clone()),
        _ => bail!("`{key}` must be text of at most {MAX_FIELD_CHARS} characters"),
    }
}

fn id_field(fields: &BTreeMap<String, Value>, key: &str) -> Result<String> {
    let value = text_field(fields, key)?;
    ensure!(
        value.is_empty() || (value.chars().count() <= MAX_ID_CHARS && !value.chars().any(char::is_control)),
        "`{key}` must be an id of at most {MAX_ID_CHARS} characters"
    );
    Ok(value)
}

/// The yes/no way out of a condition. OpenBot 0.70.0 gives a condition two routed outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Branch {
    Yes,
    No,
}

impl Branch {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
        }
    }
}

/// OpenBot's `condition` node with no check is a pass-through: it takes the yes edge. A check that
/// does not match takes the no edge, so a downstream step can still run on the other branch.
pub fn branch_for(check: &str, answer: &Answer) -> Branch {
    if check.trim().is_empty() {
        return Branch::Yes;
    }
    match answers::parse_check(check.trim()) {
        Some(parsed) if answers::check_passes(&parsed, answer) => Branch::Yes,
        _ => Branch::No,
    }
}

/// Whether an edge leaves its source on the branch that was taken. A node with no branch (a bot,
/// a model or a media step) only continues along unbranched edges; a condition only along the one
/// edge that matches its answer. Ported from `WorkflowEdge.branch` (OpenBot 0.70.0).
pub fn follows_branch(edge: Option<&str>, taken: Option<Branch>) -> bool {
    match taken {
        Some(branch) => edge == Some(branch.as_str()),
        None => edge.is_none(),
    }
}

/// An `llm` node: OpenBot 0.70.0's standalone model step, with no bot to hand work to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStepSpec {
    pub provider: String,
    pub model: String,
    pub effort: String,
    pub prompt: String,
    pub system: String,
    pub answer: String,
}

/// Reads and bounds a standalone model step. The answer is a field spec like the one a bot step
/// declares, so the engine checks the model's reply the same way it checks a teammate's.
pub fn llm_step_spec(node: &Value) -> Result<LlmStepSpec> {
    let fields = fields_of(node)?;
    Ok(LlmStepSpec {
        provider: id_field(fields, "provider")?,
        model: id_field(fields, "model")?,
        effort: id_field(fields, "effort")?,
        prompt: text_field(fields, "prompt")?,
        system: text_field(fields, "system")?,
        answer: text_field(fields, "answer")?,
    })
}

/// How an approval step settles. OpenBot 0.70.0's `decideWorkflowStep` carries `approved: boolean`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalOutcome {
    Approved,
    Rejected,
}

impl ApprovalOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

/// The pure half of `decideWorkflowStep`: true approves, false rejects. The card and the step are
/// settled by the caller with the same answer.
pub fn approval_outcome(decision: bool) -> ApprovalOutcome {
    if decision { ApprovalOutcome::Approved } else { ApprovalOutcome::Rejected }
}

/// The id of the option a person picked on an approval card, using the card's `allow`/`deny`
/// vocabulary so an existing client can render the same card it already knows.
pub fn approval_option(outcome: ApprovalOutcome) -> &'static str {
    match outcome {
        ApprovalOutcome::Approved => "allow",
        ApprovalOutcome::Rejected => "deny",
    }
}

/// The card an approval step shows, in the shape of `external`'s permission cards: a pending
/// `Permission` entry with one allow and one deny option, answered by `decideWorkflowStep`.
pub fn approval_card(question: &str, step_id: &str, bot: &str) -> Value {
    json!({
        "title": if question.trim().is_empty() { "Approve this workflow step?" } else { question.trim() },
        "detail": "",
        "options": [
            {"optionId": "allow", "name": "Approve", "kind": "allow_once"},
            {"optionId": "deny", "name": "Reject", "kind": "reject_once"}
        ],
        "status": "pending",
        "selected": Value::Null,
        "workflowStepId": step_id,
        "author": bot,
    })
}

/// The targets and actions an `output` node accepts in OpenBot 0.70.0. An empty action means "send
/// the workflow result", which is what an output step without a chosen action already does here.
pub const OUTPUT_TARGETS: &[&str] =
    &["notify", "channel", "file", "github", "discord", "telegram", "slack", "chat-channel"];
pub const OUTPUT_ACTIONS: &[&str] = &["comment", "label", "assign", "review", "close", "react", "post", "reply"];
pub const OUTPUT_EMOJI: &[&str] = &["👍", "❤️", "😄", "🎉", "😮", "😢", "🙏", "👀", "🚀", "✅", "❌"];
pub const OUTPUT_CLOSE_REASONS: &[&str] = &["completed", "not_planned"];
pub const OUTPUT_ITEMS: &[&str] = &["event", "input"];
pub const OUTPUT_LABEL_SOURCES: &[&str] = &["agent", "list"];

/// The routed action of an output step, carried with its rendered message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputAction {
    pub target: String,
    pub action: String,
    pub emoji: String,
    pub reason: String,
    pub item: String,
    pub labels_from: String,
    pub labels: String,
    pub person: String,
    pub message: String,
    pub title: String,
    pub connection_id: String,
    pub channel_id: String,
    pub place_id: String,
    pub repository: String,
}

impl Default for OutputAction {
    fn default() -> Self {
        Self {
            target: "notify".into(),
            action: String::new(),
            emoji: String::new(),
            reason: String::new(),
            item: String::new(),
            labels_from: String::new(),
            labels: String::new(),
            person: String::new(),
            message: String::new(),
            title: String::new(),
            connection_id: String::new(),
            channel_id: String::new(),
            place_id: String::new(),
            repository: String::new(),
        }
    }
}

/// Reads an output node's target and action, refusing a value the contract does not know instead
/// of silently sending to the wrong place. Pure; the API and the engine both use this decision.
pub fn output_action(node: &Value) -> Result<OutputAction> {
    let fields = fields_of(node)?;
    let one = |key: &str, values: &[&str]| -> Result<String> {
        let value = text_field(fields, key)?;
        ensure!(value.is_empty() || values.contains(&value.as_str()), "`{key}` must be one of {}", values.join(", "));
        Ok(value)
    };
    let mut action = OutputAction {
        target: one("target", OUTPUT_TARGETS)?,
        action: one("action", OUTPUT_ACTIONS)?,
        emoji: one("emoji", OUTPUT_EMOJI)?,
        reason: one("reason", OUTPUT_CLOSE_REASONS)?,
        item: one("item", OUTPUT_ITEMS)?,
        labels_from: one("labelsFrom", OUTPUT_LABEL_SOURCES)?,
        ..OutputAction::default()
    };
    if action.target.is_empty() {
        action.target = "notify".into();
    }
    action.labels = text_field(fields, "labels")?;
    action.person = text_field(fields, "person")?;
    action.message = text_field(fields, "message")?;
    action.title = text_field(fields, "title")?;
    action.connection_id = id_field(fields, "connectionId")?;
    action.channel_id = id_field(fields, "channelId")?;
    action.place_id = id_field(fields, "placeId")?;
    action.repository = id_field(fields, "repository")?;
    ensure!(action.emoji.is_empty() || action.action == "react", "`emoji` only goes with the react action");
    ensure!(action.reason.is_empty() || action.action == "close", "`reason` only goes with the close action");
    Ok(action)
}

/// The four annotation node kinds of OpenBot 0.70.0. They are drawing only: the host keeps their
/// position, size and font but never executes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    Text,
    Prompt,
    Note,
    Shape,
}

impl AnnotationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Prompt => "prompt",
            Self::Note => "note",
            Self::Shape => "shape",
        }
    }

    /// The canvas node key prefix of this annotation.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Text => "text:",
            Self::Prompt => "prompt:",
            Self::Note => "note:",
            Self::Shape => "shape:",
        }
    }
}

/// Recognises an annotation node from a key (`note:<id>`) or from its `kind` field. Bounded so a
/// hostile key cannot name a node the flow would mistake for a step.
pub fn annotation_kind(node: &Value) -> Option<AnnotationKind> {
    let name = node.as_str().or_else(|| node.get("kind").and_then(Value::as_str))?;
    let bare = name.split_once(':').map_or(name, |(_, id)| id);
    if bare.is_empty() || bare.len() > MAX_ID_CHARS || bare.chars().any(char::is_control) {
        return None;
    }
    match name.split_once(':').map_or(name, |(kind, _)| kind) {
        "text" => Some(AnnotationKind::Text),
        "prompt" => Some(AnnotationKind::Prompt),
        "note" => Some(AnnotationKind::Note),
        "shape" => Some(AnnotationKind::Shape),
        _ => None,
    }
}

/// OpenBot 0.70.0 resizes annotation nodes and changes their font. Both are optional on the wire.
/// The pure decision checks their bounds; a document with one out of range is refused, not clamped.
pub fn annotation_size(node: &Value) -> Result<(Option<f64>, Option<f64>, Option<f64>)> {
    let size = node.get("size").filter(|value| !value.is_null()).unwrap_or(node);
    let object = size.as_object().ok_or_else(|| anyhow::anyhow!("a node's `size` must be an object"))?;
    let side = |key: &str| -> Result<Option<f64>> {
        match object.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(number)) => {
                let value = number.as_f64().unwrap_or_default();
                ensure!(value > 0.0 && value <= 100_000.0, "`{key}` must be a positive size at most 100000");
                Ok(Some(value))
            }
            _ => bail!("`{key}` must be a number"),
        }
    };
    Ok((side("width")?, side("height")?, side("font")?))
}

/// Where an `asset` node's file comes from and which port it feeds. OpenBot 0.70.0's asset step
/// names a shared file id; the port is the one of the edge that leaves it unless the file itself
/// makes the choice obvious (a picture to the image port, text to the text port).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetSource {
    pub entry_id: String,
    pub file: SharedFile,
    pub port: String,
}

/// Finds `fileId` among the shared files of one chat's final messages, and picks a port for it.
/// Pure: the caller supplies the files it can see, so no store is read here.
pub fn asset_source(node: &Value, files: &[(String, SharedFile)]) -> Option<AssetSource> {
    let fields = fields_of(node).ok()?;
    let file_id = fields.get("fileId")?.as_str()?;
    if file_id.is_empty() {
        return None;
    }
    let (entry_id, file) = files.iter().find(|(_, file)| file.id == file_id)?;
    let asked = fields.get("port").and_then(Value::as_str).unwrap_or_default();
    let port = if ["text", "image", "file"].contains(&asked) {
        asked.to_owned()
    } else {
        match file.name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" => "image".to_owned(),
            "txt" | "md" | "json" | "csv" => "text".to_owned(),
            _ => "file".to_owned(),
        }
    };
    Some(AssetSource { entry_id: entry_id.clone(), file: file.clone(), port })
}

/// The fields of a trigger node projected onto a canvas. OpenBot 0.70.0's trigger nodes carry the
/// event, its filters, the connection or place, the roles and the fixed answer, plus the state the
/// canvas shows: the next due time and the webhook url.
pub fn trigger_node_fields(trigger: &Trigger, now: i64, webhook_url: Option<&str>) -> Value {
    let (kind, event, filters, connection, place, roles, text, answer) = match trigger {
        Trigger::Interval { seconds } => (
            "interval",
            String::new(),
            BTreeMap::new(),
            String::new(),
            String::new(),
            String::new(),
            format!("every {seconds} seconds"),
            String::new(),
        ),
        Trigger::Once { at } => (
            "once",
            String::new(),
            BTreeMap::new(),
            String::new(),
            String::new(),
            String::new(),
            format!("at {at}"),
            String::new(),
        ),
        Trigger::Cron { expression, time_zone } => (
            "schedule",
            String::new(),
            BTreeMap::new(),
            String::new(),
            String::new(),
            String::new(),
            format!("{expression} in {time_zone}"),
            String::new(),
        ),
        Trigger::Webhook => (
            "webhook",
            String::new(),
            BTreeMap::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
        Trigger::Event { source, event, filters } => (
            "event",
            event.clone(),
            filters.clone(),
            source.clone(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ),
        Trigger::Watch { kind, target, every_minutes } => {
            let name = match kind {
                crate::routines::watch::WatchKind::Files => "files",
                crate::routines::watch::WatchKind::Url => "url",
                crate::routines::watch::WatchKind::Command => "command",
            };
            (
                "watch",
                name.to_owned(),
                BTreeMap::new(),
                String::new(),
                target.clone(),
                String::new(),
                format!("every {every_minutes} minutes"),
                String::new(),
            )
        }
    };
    let next_due_at = trigger
        .next(now)
        .ok()
        .flatten()
        .and_then(|at| chrono::DateTime::<chrono::Utc>::from_timestamp_millis(at).map(|at| at.to_rfc3339()));
    json!({
        "trigger": kind,
        "type": kind,
        "event": event,
        "filters": filters,
        "connectionId": connection,
        "placeId": place,
        "roles": roles,
        "text": text,
        "answer": answer,
        "state": {"nextDueAt": next_due_at, "webhookUrl": webhook_url},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_condition_routes_yes_only_when_its_check_matches() {
        let pass = Answer::from([("author".to_owned(), answers::AnswerValue::Text("ana".into()))]);
        let fail = Answer::new();
        assert_eq!(branch_for("author = ana", &pass), Branch::Yes);
        assert_eq!(branch_for("author = ana", &fail), Branch::No);
        assert_eq!(branch_for("", &fail), Branch::Yes, "an empty check passes through");
        assert_eq!(branch_for("not a check", &pass), Branch::No, "a broken check fails closed");
        assert_eq!(branch_for("author = ana", &pass).as_str(), "yes");

        let yes = Some("yes");
        let no = Some("no");
        assert!(follows_branch(yes, Some(Branch::Yes)));
        assert!(!follows_branch(no, Some(Branch::Yes)));
        assert!(follows_branch(no, Some(Branch::No)));
        assert!(follows_branch(None, None));
        assert!(!follows_branch(yes, None), "a condition's answer is not a plain hand-off");
    }

    #[test]
    fn a_standalone_model_step_reads_only_its_bounded_fields() {
        let node = json!({"kind": "llm", "fields": {
            "provider": "codex", "model": "gpt-5", "effort": "high", "prompt": "Summarize {{text}}",
            "system": "Be brief", "answer": "summary: text"
        }});
        assert_eq!(
            llm_step_spec(&node).unwrap(),
            LlmStepSpec {
                provider: "codex".into(),
                model: "gpt-5".into(),
                effort: "high".into(),
                prompt: "Summarize {{text}}".into(),
                system: "Be brief".into(),
                answer: "summary: text".into(),
            }
        );
        let mut bad = node.clone();
        bad["fields"]["prompt"] = json!("x".repeat(MAX_FIELD_CHARS + 1));
        assert!(llm_step_spec(&bad).is_err(), "an overlong prompt is refused");
        assert!(llm_step_spec(&json!({"fields": {"model": 7}})).is_err(), "a numeric model is refused");
    }

    #[test]
    fn an_approval_decision_settles_one_way_or_the_other() {
        assert_eq!(approval_outcome(true), ApprovalOutcome::Approved);
        assert_eq!(approval_outcome(false), ApprovalOutcome::Rejected);
        assert_eq!(approval_option(approval_outcome(true)), "allow");
        assert_eq!(approval_option(approval_outcome(false)), "deny");
        let card = approval_card("Ship it?", "s1", "a");
        assert_eq!(card["status"], json!("pending"));
        assert_eq!(card["workflowStepId"], json!("s1"));
        assert_eq!(card["options"][0]["optionId"], json!("allow"));
        assert_eq!(card["author"], json!("a"));
    }

    #[test]
    fn an_output_node_names_a_known_target_and_action() {
        let node = json!({"fields": {
            "target": "github", "action": "close", "reason": "completed", "item": "event",
            "labelsFrom": "list", "labels": "bug, urgent", "message": "Fixed {{title}}"
        }});
        let action = output_action(&node).unwrap();
        assert_eq!(action.target, "github");
        assert_eq!(action.action, "close");
        assert_eq!(action.reason, "completed");
        assert_eq!(action.message, "Fixed {{title}}");
        assert_eq!(output_action(&json!({"fields": {}})).unwrap().target, "notify");
        let mut bad = node.clone();
        bad["fields"]["action"] = json!("delete");
        assert!(output_action(&bad).is_err(), "an unknown action is refused");
        let mut bad = node.clone();
        bad["fields"]["emoji"] = json!("👍");
        assert!(output_action(&bad).is_err(), "an emoji is not a close action");
    }

    #[test]
    fn annotations_are_recognised_from_keys_and_kinds_and_bounded() {
        assert_eq!(annotation_kind(&json!("note:n1")), Some(AnnotationKind::Note));
        assert_eq!(annotation_kind(&json!({"kind": "shape"})), Some(AnnotationKind::Shape));
        assert_eq!(annotation_kind(&json!("prompt:p1")), Some(AnnotationKind::Prompt));
        assert_eq!(annotation_kind(&json!("text:t1")), Some(AnnotationKind::Text));
        assert_eq!(annotation_kind(&json!("bot:b1")), None);
        assert_eq!(annotation_kind(&json!("note:")), None);
        assert_eq!(AnnotationKind::Text.prefix(), "text:");
        let size = json!({"size": {"width": 320.0, "height": 180.0, "font": 14.0}});
        assert_eq!(annotation_size(&size).unwrap(), (Some(320.0), Some(180.0), Some(14.0)));
        assert_eq!(annotation_size(&json!({})).unwrap(), (None, None, None));
        assert!(annotation_size(&json!({"size": {"font": 0.0}})).is_err());
        assert!(annotation_size(&json!({"size": {"width": "wide"}})).is_err());
    }

    #[test]
    fn an_asset_finds_its_file_and_a_port() {
        let picture = SharedFile { id: "f1".into(), name: "chart.png".into(), size: 10, sha256: "abc".into() };
        let note = SharedFile { id: "f2".into(), name: "notes.md".into(), size: 2, sha256: "def".into() };
        let files = [("e1".to_owned(), picture.clone()), ("e2".to_owned(), note.clone())];
        let source = asset_source(&json!({"fields": {"fileId": "f1"}}), &files).unwrap();
        assert_eq!((source.entry_id.as_str(), source.port.as_str()), ("e1", "image"));
        assert_eq!(source.file, picture);
        assert_eq!(asset_source(&json!({"fields": {"fileId": "f2", "port": "file"}}), &files).unwrap().port, "file");
        assert_eq!(asset_source(&json!({"fields": {"fileId": "f2"}}), &files).unwrap().port, "text");
        assert!(asset_source(&json!({"fields": {"fileId": "missing"}}), &files).is_none());
        assert!(asset_source(&json!({"fields": {}}), &files).is_none());
    }

    #[test]
    fn a_trigger_node_carries_its_event_and_state() {
        let cron = Trigger::Cron { expression: "0 9 * * *".into(), time_zone: "UTC".into() };
        let node = trigger_node_fields(&cron, 1_767_323_045_000, Some("https://example.test/hook"));
        assert_eq!(node["trigger"], json!("schedule"));
        assert_eq!(node["state"]["webhookUrl"], json!("https://example.test/hook"));
        assert!(node["state"]["nextDueAt"].as_str().is_some());
        let event = Trigger::Event {
            source: "github".into(),
            event: "issues.opened".into(),
            filters: BTreeMap::from([("repo".into(), "codync".into())]),
        };
        let node = trigger_node_fields(&event, 0, None);
        assert_eq!(node["event"], json!("issues.opened"));
        assert_eq!(node["connectionId"], json!("github"));
        assert_eq!(node["filters"]["repo"], json!("codync"));
        assert_eq!(node["state"]["webhookUrl"], Value::Null);
        assert_eq!(trigger_node_fields(&Trigger::Webhook, 0, None)["trigger"], json!("webhook"));
    }
}
