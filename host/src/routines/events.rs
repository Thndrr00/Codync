//! The event triggers an editor may offer, and the filters each source takes.
//!
//! The host keeps the catalog so desktop, Android and the bot describe the same events. Stored
//! triggers stay valid when a source later loses an event: [`validate`] is used by the editor's
//! save, not by [`super::Trigger::validate`], which must keep older triggers savable.

use anyhow::{Result, bail};
use base64::Engine as _;
use hmac::{KeyInit as _, Mac as _};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Filter {
    pub key: &'static str,
    /// The field's label, translated by the clients.
    pub label: &'static str,
    /// What to type, shown under the field.
    pub hint: &'static str,
}

pub struct Event {
    /// The `event` a payload carries: `<name>.<action>`, or `*` for every one.
    pub id: &'static str,
    pub label: &'static str,
}

pub struct Source {
    pub id: &'static str,
    pub label: &'static str,
    pub events: &'static [Event],
    pub filters: &'static [Filter],
}

const ANY: Event = Event { id: "*", label: "Any event" };
const MESSAGE_CONTAINS: Filter =
    Filter { key: "messageContains", label: "Text contains", hint: "Only when the text has this" };

pub const SOURCES: &[Source] = &[
    Source {
        id: "github",
        label: "GitHub",
        events: &[
            ANY,
            // OpenBot's 12 workflow events (ipc-workflow-github.ts) are the names a trigger saves.
            // The raw GitHub names below stay for routines saved by older Codync clients; `aliases`
            // maps both spellings onto one delivery so either trigger fires.
            Event { id: "issue.opened", label: "Issue opened" },
            Event { id: "issue.closed", label: "Issue closed" },
            Event { id: "issue.labeled", label: "Issue labeled" },
            Event { id: "pr.opened", label: "Pull request opened" },
            Event { id: "pr.updated", label: "Pull request updated" },
            Event { id: "pr.reviewRequested", label: "Review requested" },
            Event { id: "pr.reviewSubmitted", label: "Review submitted" },
            Event { id: "pr.checksFailed", label: "Checks failed" },
            Event { id: "pr.merged", label: "Pull request merged" },
            Event { id: "comment", label: "Comment" },
            Event { id: "mention", label: "Mention" },
            Event { id: "assigned", label: "Assigned" },
            Event { id: "issues.opened", label: "Issue opened (raw)" },
            Event { id: "issues.closed", label: "Issue closed (raw)" },
            Event { id: "issue_comment.created", label: "Comment on an issue" },
            Event { id: "pull_request.opened", label: "Pull request opened (raw)" },
            Event { id: "pull_request.closed", label: "Pull request closed (raw)" },
            Event { id: "pull_request_review.submitted", label: "Pull request review" },
            Event { id: "pull_request_review_comment.created", label: "Comment on a review" },
            Event { id: "check_run.completed", label: "Checks finished" },
            Event { id: "push", label: "Push" },
            Event { id: "release.published", label: "Release published" },
            Event { id: "discussion.created", label: "Discussion opened" },
        ],
        filters: &[
            Filter { key: "repo", label: "Repository", hint: "owner/name" },
            Filter { key: "sender", label: "Sender", hint: "A GitHub login" },
            Filter { key: "conclusion", label: "Conclusion", hint: "failure for failed checks" },
            Filter { key: "notBot", label: "Skip bots", hint: "true to ignore bot actors" },
            Filter { key: "firstTime", label: "First time", hint: "true for a first-time contributor" },
            Filter { key: "title", label: "Title contains", hint: "Part of the issue or pull request title" },
            Filter { key: "branch", label: "Branch", hint: "The pull request's head branch" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "slack",
        label: "Slack",
        events: &[ANY, Event { id: "app_mention", label: "Mention" }],
        filters: &[
            Filter { key: "channel", label: "Channel", hint: "A channel id, such as C0123ABCD" },
            Filter { key: "user", label: "Sender", hint: "A Slack user id" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "telegram",
        label: "Telegram",
        events: &[ANY, Event { id: "mention", label: "Mention" }],
        filters: &[
            Filter { key: "chat", label: "Chat", hint: "A chat id" },
            Filter { key: "user", label: "Sender", hint: "A Telegram user id" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "discord",
        label: "Discord",
        events: &[
            ANY,
            Event { id: "message.created", label: "Message posted" },
            Event { id: "message.mention", label: "Mention" },
        ],
        filters: &[
            Filter { key: "channel", label: "Channel", hint: "A channel id" },
            Filter { key: "user", label: "Sender", hint: "A Discord user id" },
            MESSAGE_CONTAINS,
        ],
    },
    Source {
        id: "whatsapp",
        label: "WhatsApp",
        events: &[
            ANY,
            Event { id: "message", label: "Message" },
            Event { id: "groupMention", label: "Mention in a group" },
        ],
        filters: &[
            Filter { key: "chat", label: "Chat", hint: "A chat id, such as 5215555555555@s.whatsapp.net" },
            Filter { key: "from", label: "Sender", hint: "A WhatsApp id" },
            MESSAGE_CONTAINS,
        ],
    },
];

pub fn source(id: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|source| source.id == id)
}

/// Why `source`, `event` and `filters` can't be saved, or `None` when they can.
pub fn problem(source_id: &str, event: &str, filters: &std::collections::BTreeMap<String, String>) -> Option<String> {
    let Some(source) = source(source_id) else {
        // A source the catalog doesn't know (an older or bot-set trigger) still saves: its event and
        // filters can't be checked here, so they pass through.
        if source_id.is_empty() || event.is_empty() {
            return Some("Choose a service for this trigger.".into());
        }
        return filters
            .iter()
            .find(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
            .map(|(key, _)| format!("The {key} filter is empty."));
    };
    if !source.events.iter().any(|known| known.id == event) {
        return Some(format!("Choose an event {} can send.", source.label));
    }
    if filters.len() > 20 {
        return Some("Use at most 20 filters.".into());
    }
    for (key, value) in filters {
        if !source.filters.iter().any(|filter| filter.key == key) {
            return Some(format!("{} has no filter named {key}.", source.label));
        }
        if value.trim().is_empty() || value.len() > 4000 {
            return Some(format!("The {key} filter is empty or too long."));
        }
    }
    None
}

/// Checks [`problem`] for a save or a preview.
pub fn validate(source_id: &str, event: &str, filters: &std::collections::BTreeMap<String, String>) -> Result<()> {
    match problem(source_id, event, filters) {
        Some(why) => bail!("{why}"),
        None => Ok(()),
    }
}

/// Whether a filter matches `payload`; the special keys read where the services put them. The
/// GitHub-only filters share [`github_filter`] with [`github_trigger`], so a saved trigger and a
/// delivery use one rule.
pub fn matches(payload: &Value, key: &str, value: &str) -> bool {
    match key {
        "messageContains" | "notBot" | "firstTime" | "branch" | "title" => github_filter(payload, key, value),
        "conclusion" => {
            ["check_run", "check_suite"].iter().any(|kind| payload[*kind]["conclusion"].as_str() == Some(value))
        }
        _ => payload[key].as_str() == Some(value),
    }
}

/// How far from the host's clock a Standard Webhooks delivery may be; older ones are replayed ones.
const WEBHOOK_SKEW_MS: i64 = 5 * 60 * 1000;
const MAX_WEBHOOK_ID: usize = 200;
const MAX_WEBHOOK_SIGNATURE: usize = 2048;

/// The bytes a Standard Webhooks sender signs: `<webhook-id>.<webhook-timestamp>.<body>`.
fn signed_content(id: &str, timestamp: &str, body: &[u8]) -> Vec<u8> {
    let mut content = Vec::with_capacity(id.len() + timestamp.len() + body.len() + 2);
    content.extend_from_slice(id.as_bytes());
    content.push(b'.');
    content.extend_from_slice(timestamp.as_bytes());
    content.push(b'.');
    content.extend_from_slice(body);
    content
}

/// A `whsec_<base64>` secret is the form Standard Webhooks libraries hand out; a plain string is
/// accepted too, so a sender that names its own secret can paste it as-is.
fn secret_bytes(secret: &str) -> Vec<u8> {
    secret
        .strip_prefix("whsec_")
        .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok())
        .unwrap_or_else(|| secret.as_bytes().to_vec())
}

/// The signature a test delivery carries, in the same form the sender's library produces.
pub fn sign_sender(secret: &str, id: &str, timestamp: &str, body: &[u8]) -> Option<String> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret_bytes(secret)).ok()?;
    mac.update(&signed_content(id, timestamp, body));
    Some(format!("v1,{}", base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())))
}

/// Whether a delivery was signed with the sender's own Standard Webhooks secret and is recent.
/// The body is part of the signed content, so the decision needs it; no error names the secret.
/// Pure, so the host's verification path and its tests use the same rule.
pub fn sender_secret_ok(headers: &super::hooks::Headers, secret: &str, body: &[u8], now_ms: i64) -> bool {
    let Some(id) = headers.get("webhook-id") else { return false };
    let Some(timestamp) = headers.get("webhook-timestamp") else { return false };
    let Some(signatures) = headers.get("webhook-signature") else { return false };
    if secret.trim().is_empty()
        || id.is_empty()
        || id.len() > MAX_WEBHOOK_ID
        || timestamp.len() > 20
        || signatures.len() > MAX_WEBHOOK_SIGNATURE
    {
        return false;
    }
    let Ok(at) = timestamp.parse::<i64>() else { return false };
    let Some(at_ms) = at.checked_mul(1000) else { return false };
    // Reject replays and clocks far ahead, without overflowing at i64's ends.
    if !now_ms.checked_sub(at_ms).is_some_and(|skew| (-WEBHOOK_SKEW_MS..=WEBHOOK_SKEW_MS).contains(&skew)) {
        return false;
    }
    let Ok(mut mac) = hmac::Hmac::<sha2::Sha256>::new_from_slice(&secret_bytes(secret)) else { return false };
    mac.update(&signed_content(id, timestamp, body));
    let expected = mac.finalize().into_bytes();
    signatures.split_ascii_whitespace().any(|entry| {
        entry
            .strip_prefix("v1,")
            .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok())
            .is_some_and(|given| crate::remote::crypto::ct_eq(&given, &expected))
    })
}

/// A sample delivery for the routine's event triggers, so the editor's test uses the same filters a
/// real sender has to match. A webhook-only routine gets the plain `{source:"test"}` event.
pub fn test_event(routine: &super::Routine, now_ms: i64) -> Value {
    let mut event = json!({"source": "test", "event": "test", "text": "Routine webhook test", "at": now_ms});
    for trigger in &routine.triggers {
        if let super::Trigger::Event { source, event: name, filters } = trigger {
            event["source"] = json!(source);
            event["event"] = json!(if name == "*" { "test" } else { name });
            for (key, value) in filters {
                // `messageContains` needs text that contains the wanted substring; an exact filter
                // is satisfied by its own value (the source puts ids and names in these fields).
                if key == "firstTime" && flag(value) {
                    event["firstTime"] = json!(true);
                } else {
                    event[key.as_str()] =
                        json!(if key == "messageContains" || !value.is_empty() { value } else { "test" });
                }
            }
            break;
        }
    }
    event
}

/// The catalog the editors offer.
pub fn catalog() -> Value {
    json!({
        "sources": SOURCES.iter().map(|source| json!({
            "id": source.id,
            "label": source.label,
            "events": source.events.iter().map(|event| json!({"id": event.id, "label": event.label})).collect::<Vec<_>>(),
            "filters": source.filters.iter().map(|filter| json!({"key": filter.key, "label": filter.label, "hint": filter.hint})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// A readable trigger line for `source` and `event`, when the catalog knows them.
pub fn describe(source_id: &str, event: &str) -> Option<String> {
    let source = source(source_id)?;
    let event = source.events.iter().find(|known| known.id == event)?;
    Some(format!("When {}: {}", source.label, event.label.to_lowercase()))
}

// ---------------------------------------------------------------------------
// GitHub workflow triggers (OpenBot 0.70.0, `ipc-workflow-github.ts`)
// ---------------------------------------------------------------------------

/// OpenBot's 12 workflow events, in the contract's order. A trigger saves one of these (or `*`).
pub const GITHUB_EVENTS: [&str; 12] = [
    "issue.opened",
    "issue.closed",
    "issue.labeled",
    "pr.opened",
    "pr.updated",
    "pr.reviewRequested",
    "pr.reviewSubmitted",
    "pr.checksFailed",
    "pr.merged",
    "comment",
    "mention",
    "assigned",
];
/// The four filters OpenBot's GitHub trigger offers, beyond Codync's older repo/sender filters.
pub const GITHUB_FILTERS: [&str; 4] = ["notBot", "firstTime", "title", "branch"];
/// At most this many deliveries are kept in a trigger's history, as in OpenBot.
pub const GITHUB_DELIVERY_LIMIT: usize = 50;
/// Three runs on one item in ten minutes are a chain (`workflow_github_deliveries`).
pub const GITHUB_CHAIN_RUNS: usize = 3;
pub const GITHUB_CHAIN_WINDOW_MS: i64 = 10 * 60 * 1000;

/// Why a signed GitHub delivery did not start a run. The names match OpenBot's skip reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitHubSkip {
    Filter,
    /// The event echoes a change a run of this workflow made.
    #[serde(rename = "self")]
    SelfEcho,
    /// The bot already ran three times on the item in this window.
    Chain,
    /// The host had no GitHub route while the event was sent.
    Offline,
    /// The run could not start (paused, busy or refused).
    Refused,
}

impl GitHubSkip {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filter => "filter",
            Self::SelfEcho => "self",
            Self::Chain => "chain",
            Self::Offline => "offline",
            Self::Refused => "refused",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitHubDecision {
    Run,
    Skip(GitHubSkip),
}

impl GitHubDecision {
    pub fn is_run(self) -> bool {
        matches!(self, Self::Run)
    }
}

/// The item a delivery is about, kept in the history without the event body (OpenBot's
/// `WorkflowGitHubItem`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubItem {
    pub repository: String,
    pub number: i64,
    pub pull_request: bool,
    pub title: String,
}

fn flag(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

/// A GitHub actor is a bot when GitHub says so or when the event names a bot sender.
fn is_bot(event: &Value) -> bool {
    event["payload"]["sender"]["type"].as_str() == Some("Bot")
        || event["payload"]["sender"]["bot"] == true
        || event["author"]["bot"] == true
        || event["sender"]["bot"] == true
        || event["payload"]["author"]["bot"] == true
}

/// Reads the first non-empty string at one of the dotted paths of `value`.
fn first_str(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| {
        let mut at = value;
        for key in *path {
            at = &at[*key];
        }
        at.as_str().map(str::trim).filter(|text| !text.is_empty()).map(str::to_owned)
    })
}

fn first_time(event: &Value) -> bool {
    event["firstTime"] == true
        || event["payload"]["firstTime"] == true
        || matches!(
            event["payload"]["sender"]["author_association"].as_str(),
            Some("FIRST_TIMER" | "FIRST_TIME_CONTRIBUTOR")
        )
        || matches!(event["payload"]["author_association"].as_str(), Some("FIRST_TIMER" | "FIRST_TIME_CONTRIBUTOR"))
}

fn github_text(event: &Value, key: &str) -> Option<String> {
    if key == "title" {
        return first_str(event, &[&["title"], &["payload", "issue", "title"], &["payload", "pull_request", "title"]]);
    }
    if key == "branch" {
        return first_str(
            event,
            &[
                &["branch"],
                &["payload", "pull_request", "head", "ref"],
                &["payload", "check_run", "head_branch"],
                &["payload", "check_suite", "head_branch"],
            ],
        );
    }
    None
}

/// One GitHub filter. `github_trigger` and [`matches`] both use it, so the editor's preview and a
/// real delivery cannot disagree.
fn github_filter(event: &Value, key: &str, value: &str) -> bool {
    match key {
        "notBot" => flag(value) == !is_bot(event),
        "firstTime" => flag(value) == first_time(event),
        "title" | "branch" => github_text(event, key)
            .is_some_and(|text| text.to_ascii_lowercase().contains(&value.trim().to_ascii_lowercase())),
        "messageContains" => event["text"]
            .as_str()
            .is_some_and(|text| text.to_ascii_lowercase().contains(&value.trim().to_ascii_lowercase())),
        _ => event[key].as_str() == Some(value),
    }
}

/// The canonical names a raw GitHub delivery may satisfy. `hooks::event` keeps the GitHub spelling
/// (`issues.opened`) in `event`; a workflow saved with OpenBot's `issue.opened` must fire on the
/// same delivery, so both spellings are offered and the older raw trigger keeps matching too.
pub fn github_aliases(event: &Value) -> Vec<String> {
    let raw = event["event"].as_str().unwrap_or_default();
    let payload = &event["payload"];
    let action = payload["action"].as_str().unwrap_or_default();
    let mut names = vec![raw.to_owned()];
    let canonical = match (raw, action) {
        ("issues", "opened") => Some("issue.opened"),
        ("issues", "closed") => Some("issue.closed"),
        ("issues", "labeled") => Some("issue.labeled"),
        ("issues", "assigned") => Some("assigned"),
        ("pull_request", "opened") => Some("pr.opened"),
        ("pull_request", "closed") if payload["pull_request"]["merged"] == true => Some("pr.merged"),
        ("pull_request", "review_requested") | ("pull_request_review_requested", _) => Some("pr.reviewRequested"),
        ("pull_request", _) => Some("pr.updated"),
        ("pull_request_review", "submitted") => Some("pr.reviewSubmitted"),
        ("issue_comment", "created") | ("pull_request_review_comment", "created") => Some("comment"),
        ("check_run", _) if payload["check_run"]["conclusion"].as_str() == Some("failure") => Some("pr.checksFailed"),
        ("check_suite", _) if payload["check_suite"]["conclusion"].as_str() == Some("failure") => {
            Some("pr.checksFailed")
        }
        ("mention", _) => Some("mention"),
        _ => None,
    };
    if let Some(canonical) = canonical {
        names.push(canonical.to_owned());
    }
    names
}

/// The pure decision behind a GitHub trigger: is the event one of ours and do all filters match?
/// The event id is checked by [`github_aliases`] at the call site so old and new spellings both work.
pub fn github_trigger(event: &Value, filters: &BTreeMap<String, String>) -> GitHubDecision {
    if event["source"].as_str() != Some("github")
        || !filters.iter().all(|(key, value)| github_filter(event, key, value))
    {
        GitHubDecision::Skip(GitHubSkip::Filter)
    } else {
        GitHubDecision::Run
    }
}

/// The context a delivery's skip reason needs. Every field is a fact the host already has; no
/// network call or clock read happens inside [`delivery_reason`].
#[derive(Clone, Copy)]
pub struct DeliveryContext<'a> {
    /// The GitHub object id a run of this workflow just changed, when known.
    pub changed_object: Option<&'a str>,
    /// Runs on the item in the last ten minutes, including the bot's own runs.
    pub recent_runs: usize,
    /// Whether the host still has a GitHub route.
    pub online: bool,
    /// Whether a run may start now (routine enabled and not already running).
    pub can_run: bool,
}

/// The skip reason for one delivery, in OpenBot's order: filters first, then the echoes and limits
/// that only apply to a matching event. `None` means the delivery starts a run. Pure and tested.
pub fn delivery_reason(
    event: &Value,
    filters: &BTreeMap<String, String>,
    context: &DeliveryContext<'_>,
) -> Option<GitHubSkip> {
    if !github_trigger(event, filters).is_run() {
        return Some(GitHubSkip::Filter);
    }
    if !context.online {
        return Some(GitHubSkip::Offline);
    }
    if let (Some(changed), Some(object)) = (context.changed_object, github_object_id(event))
        && changed == object
    {
        return Some(GitHubSkip::SelfEcho);
    }
    if context.recent_runs >= GITHUB_CHAIN_RUNS {
        return Some(GitHubSkip::Chain);
    }
    if !context.can_run {
        return Some(GitHubSkip::Refused);
    }
    None
}

/// The id of the GitHub object a delivery carries, used to recognise our own echo.
pub fn github_object_id(event: &Value) -> Option<String> {
    let payload = &event["payload"];
    for path in [
        &["comment", "id"] as &[&str],
        &["review", "id"],
        &["check_run", "id"],
        &["issue", "id"],
        &["pull_request", "id"],
    ] {
        if let Some(id) = payload[path[0]][path[1]].as_i64() {
            return Some(id.to_string());
        }
    }
    event["messageId"].as_str().map(str::to_owned)
}

/// The issue or pull request a delivery is about, for the history and "run with last event".
pub fn github_item(event: &Value) -> Option<GitHubItem> {
    let payload = &event["payload"];
    let repository = first_str(event, &[&["repo"], &["repository"], &["payload", "repository", "full_name"]])?;
    let number = payload["issue"]["number"]
        .as_i64()
        .or_else(|| payload["pull_request"]["number"].as_i64())
        .or_else(|| event["number"].as_i64())?;
    let pull_request = !payload["pull_request"].is_null() || {
        let name = event["event"].as_str().unwrap_or_default();
        name.starts_with("pull_request") || name.starts_with("pr.")
    };
    let title = github_text(event, "title").or_else(|| first_str(event, &[&["text"]])).unwrap_or_default();
    Some(GitHubItem { repository, number, pull_request, title })
}

// ---------------------------------------------------------------------------
// Chat workflow triggers (OpenBot 0.70.0, `ipc-workflow-discord/telegram/slack.ts`)
// ---------------------------------------------------------------------------

/// A chat trigger node: a connection, an optional channel, author roles and a text filter. Empty
/// connection, channel, roles and text all mean "any", matching OpenBot's matching table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatNode {
    pub source: String,
    pub connection: String,
    pub channel: String,
    pub roles: Vec<String>,
    pub text: String,
}

fn list_value(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == ',' || c == '\n' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Reads a chat node out of a `Trigger::Event`'s filters. `chat`, `user` and `messageContains` are
/// Codync's older names and keep working beside OpenBot's `channel`, `roles` and `text`.
pub fn chat_node(source: &str, filters: &BTreeMap<String, String>) -> ChatNode {
    let get = |keys: &[&str]| -> String {
        keys.iter().find_map(|key| filters.get(*key).map(|v| v.trim().to_owned())).unwrap_or_default()
    };
    ChatNode {
        source: source.to_owned(),
        connection: get(&["connection"]),
        channel: get(&["channel", "channelId", "chat"]),
        roles: list_value(&get(&["roles"])),
        text: get(&["text", "messageContains"]),
    }
}

fn has_role(event: &Value, wanted: &[String]) -> bool {
    let roles = event["roles"].as_array().map(|list| {
        list.iter()
            .filter_map(|role| role.as_str().or_else(|| role["id"].as_str()))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    roles.is_some_and(|roles| wanted.iter().any(|role| roles.contains(role)))
}

/// The pure match behind Discord, Telegram and Slack workflow triggers. A mention in a thread of a
/// named channel does not match that channel's trigger: the event names the thread, not the parent
/// (OpenBot's `discord-matching.ts`). Slack has no author-role filter because the host has no Slack
/// people catalog.
pub fn chat_trigger_match(event: &Value, node: &ChatNode) -> bool {
    if event["source"].as_str() != Some(node.source.as_str()) {
        return false;
    }
    if !matches!(event["event"].as_str(), Some("mention" | "app_mention" | "message.mention")) {
        return false;
    }
    if !node.connection.is_empty() && event["connection"].as_str() != Some(node.connection.as_str()) {
        return false;
    }
    if !node.channel.is_empty()
        && event["channel"].as_str() != Some(node.channel.as_str())
        && event["channelId"].as_str() != Some(node.channel.as_str())
    {
        return false;
    }
    if node.source == "discord" && !node.roles.is_empty() && !has_role(event, &node.roles) {
        return false;
    }
    if !node.text.is_empty()
        && !event["text"]
            .as_str()
            .is_some_and(|text| text.to_ascii_lowercase().contains(&node.text.to_ascii_lowercase()))
    {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Workflow webhook triggers (OpenBot 0.70.0, docs/architecture/events.md)
// ---------------------------------------------------------------------------

/// The raw request a workflow webhook trigger verifies: exact signed bytes and the headers that
/// carry the proof. The secret never travels with it.
pub struct WebhookBody {
    pub headers: super::hooks::Headers,
    pub raw: Vec<u8>,
}

/// Why a webhook request does not start a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebhookSkip {
    Ping,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebhookDecision {
    Unauthorized,
    TooLarge,
    Ignored(WebhookSkip),
    Run(Value),
}

/// The pure decision a workflow webhook request goes through. Verification reuses the host's own
/// webhook proof ([`super::hooks::authorized`] and [`sender_secret_ok`]) rather than a second HMAC
/// implementation; parsing reuses [`super::hooks::event`]. Busy, duplicate and receipt decisions
/// stay in `routines::mod`, which owns the state.
pub fn workflow_webhook(body: &WebhookBody, secret: &str, now_ms: i64) -> WebhookDecision {
    if body.raw.len() > super::hooks::MAX_BODY {
        return WebhookDecision::TooLarge;
    }
    if !super::hooks::authorized(secret, &body.headers, &body.raw)
        && !sender_secret_ok(&body.headers, secret, &body.raw, now_ms)
    {
        return WebhookDecision::Unauthorized;
    }
    match super::hooks::event(&body.headers, &body.raw) {
        Ok(None) => WebhookDecision::Ignored(WebhookSkip::Ping),
        Ok(Some(event)) => WebhookDecision::Run(event),
        Err(_) => WebhookDecision::Ignored(WebhookSkip::Invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use std::collections::BTreeMap;

    #[test]
    fn every_source_offers_any_event_and_known_filters() {
        for source in SOURCES {
            assert!(source.events.iter().any(|event| event.id == "*"), "{}", source.id);
            assert!(!source.filters.is_empty(), "{}", source.id);
        }
        assert!(source("github").is_some());
        assert!(source("nope").is_none());
    }

    #[test]
    fn validate_names_the_broken_field() {
        let empty = BTreeMap::new();
        assert!(validate("github", "issues.opened", &empty).is_ok());
        assert!(validate("github", "issues.assigned", &empty).is_err());
        assert!(validate("github", "issues.opened", &BTreeMap::from([("channel".into(), "C1".into())])).is_err());
        assert!(validate("github", "issues.opened", &BTreeMap::from([("repo".into(), " ".into())])).is_err());
        assert!(validate("slack", "*", &BTreeMap::from([("channel".into(), "C1".into())])).is_ok());
        assert!(
            validate("whatsapp", "message", &BTreeMap::from([("chat".into(), "5215555555555@s.whatsapp.net".into())]))
                .is_ok()
        );
        // A source outside the catalog keeps its event and filters as they are.
        assert!(validate("origin", "pull_request.opened", &BTreeMap::from([("repo".into(), "o/r".into())])).is_ok());
        assert!(validate("origin", "", &empty).is_err());
        assert!(validate("pagerduty", "incident.triggered", &empty).is_ok());
    }

    #[test]
    fn filters_read_the_payload_where_the_service_puts_them() {
        let failure = json!({"text": "Build failed", "repo": "o/r", "check_run": {"conclusion": "failure"}});
        assert!(matches(&failure, "repo", "o/r"));
        assert!(matches(&failure, "messageContains", "failed"));
        assert!(matches(&failure, "conclusion", "failure"));
        assert!(!matches(&failure, "conclusion", "success"));
        assert!(!matches(&failure, "repo", "o/other"));
    }

    #[test]
    fn description_uses_the_catalog() {
        assert_eq!(describe("github", "pull_request.opened").as_deref(), Some("When GitHub: pull request opened"));
        assert!(describe("github", "issues.assigned").is_none());
    }

    use super::super::hooks::Headers;

    fn signed_headers(secret: &str, id: &str, timestamp: &str, body: &[u8]) -> Headers {
        let mut headers = Headers::new();
        headers.insert("webhook-id".into(), id.into());
        headers.insert("webhook-timestamp".into(), timestamp.into());
        headers.insert("webhook-signature".into(), sign_sender(secret, id, timestamp, body).expect("a signature"));
        headers
    }

    #[test]
    fn a_sender_secret_accepts_only_a_recent_signature_of_the_exact_body() {
        const SECRET: &str = "senders-own-secret";
        let now = 1_700_000_000_000;
        let timestamp = "1700000000";
        let body = br#"{"action":"opened"}"#;
        let id = "msg-1";
        let headers = signed_headers(SECRET, id, timestamp, body);
        assert!(sender_secret_ok(&headers, SECRET, body, now));
        assert!(!sender_secret_ok(&headers, "another-secret", body, now));
        assert!(!sender_secret_ok(&headers, SECRET, br#"{"action":"closed"}"#, now), "the body is signed");
        assert!(!sender_secret_ok(&headers, "", body, now), "an empty secret proves nothing");

        let mut stale = signed_headers(SECRET, id, "1699999640", body); // six minutes ago
        assert!(!sender_secret_ok(&stale, SECRET, body, now));
        stale = signed_headers(SECRET, id, "1700000360", body); // six minutes ahead
        assert!(!sender_secret_ok(&stale, SECRET, body, now));
        let recent = signed_headers(SECRET, id, "1699999700", body); // five minutes ago is still in
        assert!(sender_secret_ok(&recent, SECRET, body, now));
    }

    #[test]
    fn a_sender_secret_needs_the_standard_headers_and_a_well_formed_signature() {
        let now = 1_700_000_000_000;
        let timestamp = "1700000000";
        let body = b"{}";
        let headers = signed_headers("secret", "msg-1", timestamp, body);
        assert!(sender_secret_ok(&headers, "secret", body, now));

        let mut missing = headers.clone();
        missing.remove("webhook-id");
        assert!(!sender_secret_ok(&missing, "secret", body, now));
        let mut malformed = headers.clone();
        malformed.insert("webhook-signature".into(), "v1,not-base64".into());
        assert!(!sender_secret_ok(&malformed, "secret", body, now));
        let mut unknown_version = headers.clone();
        unknown_version.insert("webhook-signature".into(), "v2,AAAA".into());
        assert!(!sender_secret_ok(&unknown_version, "secret", body, now));
        let mut no_time = headers.clone();
        no_time.insert("webhook-timestamp".into(), "yesterday".into());
        assert!(!sender_secret_ok(&no_time, "secret", body, now));
        // Rotating keys in flight: a sender may list several signatures, one of which is current.
        let mut rotated = headers.clone();
        let other = sign_sender("old-secret", "msg-1", timestamp, body).expect("a signature");
        rotated.insert("webhook-signature".into(), format!("{other} {}", headers["webhook-signature"]));
        assert!(sender_secret_ok(&rotated, "secret", body, now));

        // Libraries hand out `whsec_<base64>`; the same bytes must verify.
        let encoded = base64::engine::general_purpose::STANDARD.encode("secret");
        let whsec = format!("whsec_{encoded}");
        let with_prefix = signed_headers(&whsec, "msg-1", timestamp, body);
        assert!(sender_secret_ok(&with_prefix, &whsec, body, now));
    }

    #[test]
    fn a_test_event_matches_the_routines_event_filters() {
        let mut routine = super::super::tests::routine();
        routine.triggers = vec![super::super::Trigger::Event {
            source: "github".into(),
            event: "pull_request.opened".into(),
            filters: [("repo".into(), "o/r".into()), ("messageContains".into(), "fix".into())].into(),
        }];
        let event = test_event(&routine, 7_000);
        assert!(routine.triggers[0].matches(&event));
        assert_eq!(event["source"], "github");
        assert_eq!(event["event"], "pull_request.opened");
        assert_eq!(event["at"], 7_000);

        routine.triggers = vec![super::super::Trigger::Event {
            source: "slack".into(),
            event: "*".into(),
            filters: [("channel".into(), "C1".into())].into(),
        }];
        assert!(routine.triggers[0].matches(&test_event(&routine, 0)), "any event still matches a sample");

        routine.triggers = vec![super::super::Trigger::Webhook];
        assert_eq!(test_event(&routine, 0)["source"], "test", "a plain webhook gets a plain test event");
    }

    #[test]
    fn the_github_catalog_names_twelve_events_and_four_filters() {
        assert_eq!(GITHUB_EVENTS.len(), 12);
        assert_eq!(GITHUB_FILTERS.len(), 4);
        for event in GITHUB_EVENTS {
            assert!(validate("github", event, &BTreeMap::new()).is_ok(), "{event} is savable");
        }
        for key in GITHUB_FILTERS {
            assert!(validate("github", "issue.opened", &BTreeMap::from([(key.into(), "true".into())])).is_ok());
        }
    }

    #[test]
    fn a_raw_github_delivery_satisfies_both_event_spellings() {
        let event = json!({
            "source": "github", "event": "pull_request.opened",
            "payload": {"action": "opened", "pull_request": {"title": "Fix login", "head": {"ref": "topic"}},
                        "sender": {"login": "ana", "type": "User"}}
        });
        let aliases = github_aliases(&event);
        assert!(aliases.iter().any(|name| name == "pull_request.opened"), "legacy name kept");
        assert!(aliases.iter().any(|name| name == "pr.opened"), "OpenBot name added");

        let event = json!({"source":"github","event":"pull_request","payload":{
            "action":"closed","pull_request":{"merged":true,"title":"Fix","head":{"ref":"topic"}},
            "sender":{"login":"kai","type":"User"}}});
        assert!(github_aliases(&event).iter().any(|name| name == "pr.merged"));
        let event = json!({"source":"github","event":"check_run","payload":{
            "conclusion":"failure","check_run":{"conclusion":"failure","head_branch":"topic"}}});
        assert!(github_aliases(&event).iter().any(|name| name == "pr.checksFailed"));
    }

    #[test]
    fn github_filters_read_the_event_without_a_model_step() {
        let event = json!({
            "source": "github", "event": "issues.opened",
            "title": "Fix the login form", "branch": "topic", "text": "Fix the login form",
            "payload": {"sender": {"login": "ana", "type": "User"}, "issue": {"title": "Fix the login form"},
                        "author_association": "FIRST_TIMER"}
        });
        let filters = BTreeMap::from([
            ("notBot".to_owned(), "true".to_owned()),
            ("firstTime".to_owned(), "true".to_owned()),
            ("title".to_owned(), "login".to_owned()),
            ("branch".to_owned(), "topic".to_owned()),
        ]);
        assert_eq!(github_trigger(&event, &filters), GitHubDecision::Run);
        assert_eq!(
            github_trigger(
                &event,
                &BTreeMap::from([("notBot".into(), "true".into()), ("title".into(), "nope".into())])
            ),
            GitHubDecision::Skip(GitHubSkip::Filter)
        );
        let bot = json!({"source":"github","event":"comment","payload":{"sender":{"login":"bot","type":"Bot"}}});
        assert_eq!(
            github_trigger(&bot, &BTreeMap::from([("notBot".into(), "true".into())])),
            GitHubDecision::Skip(GitHubSkip::Filter)
        );
        assert_eq!(
            github_trigger(&json!({"source":"slack"}), &BTreeMap::new()),
            GitHubDecision::Skip(GitHubSkip::Filter)
        );
    }

    #[test]
    fn delivery_reason_orders_filters_echo_chain_offline_and_refusal() {
        let event = json!({"source":"github","event":"issue.opened","repo":"o/r","payload":{
            "issue":{"id":42,"number":7,"title":"Hi"},"sender":{"login":"ana","type":"User"}}});
        let none = BTreeMap::new();
        let ok = DeliveryContext { changed_object: None, recent_runs: 0, online: true, can_run: true };
        assert_eq!(delivery_reason(&event, &none, &ok), None);
        let filtered = BTreeMap::from([("title".into(), "absent".into())]);
        assert_eq!(delivery_reason(&event, &filtered, &ok), Some(GitHubSkip::Filter));
        let echoed = DeliveryContext { changed_object: Some("42"), ..ok };
        assert_eq!(delivery_reason(&event, &none, &echoed), Some(GitHubSkip::SelfEcho));
        let chained = DeliveryContext { recent_runs: GITHUB_CHAIN_RUNS, ..ok };
        assert_eq!(delivery_reason(&event, &none, &chained), Some(GitHubSkip::Chain));
        let offline = DeliveryContext { online: false, ..ok };
        assert_eq!(delivery_reason(&event, &none, &offline), Some(GitHubSkip::Offline));
        let refused = DeliveryContext { can_run: false, ..ok };
        assert_eq!(delivery_reason(&event, &none, &refused), Some(GitHubSkip::Refused));
        assert_eq!(
            github_item(&event),
            Some(GitHubItem { repository: "o/r".into(), number: 7, pull_request: false, title: "Hi".into() })
        );
    }

    #[test]
    fn chat_triggers_match_connection_channel_roles_and_text() {
        let event = json!({
            "source": "discord", "event": "message.mention", "connection": "guild-1",
            "channel": "C1", "roles": [{"id": "admin"}, "mod"], "text": "Deploy the fix please"
        });
        let node = chat_node(
            "discord",
            &BTreeMap::from([
                ("connection".into(), "guild-1".into()),
                ("channel".into(), "C1".into()),
                ("roles".into(), "admin, mod".into()),
                ("text".into(), "fix".into()),
            ]),
        );
        assert!(chat_trigger_match(&event, &node));
        let wrong_channel = ChatNode { channel: "C2".into(), ..node.clone() };
        assert!(!chat_trigger_match(&event, &wrong_channel));
        let wrong_role = ChatNode { roles: vec!["other".into()], ..node.clone() };
        assert!(!chat_trigger_match(&event, &wrong_role));
        let wrong_text = ChatNode { text: "deploy!".into(), ..node.clone() };
        assert!(!chat_trigger_match(&event, &wrong_text));
        let wrong_source = ChatNode { source: "slack".into(), ..node.clone() };
        assert!(!chat_trigger_match(&event, &wrong_source));
        // An empty connection, channel, roles and text accept every mention.
        assert!(chat_trigger_match(&event, &ChatNode { source: "discord".into(), ..ChatNode::default() }));
        let slack = json!({"source":"slack","event":"app_mention","channel":"C1","text":"hi"});
        assert!(chat_trigger_match(&slack, &chat_node("slack", &BTreeMap::new())));
        assert!(!chat_trigger_match(&slack, &chat_node("telegram", &BTreeMap::new())));
    }

    #[test]
    fn workflow_webhook_reuses_the_host_proof_and_bounds_the_body() {
        const SECRET: &str = "workflow-secret";
        let now = 1_700_000_000_000;
        let timestamp = "1700000000";
        let raw = br#"{"source":"webhook","text":"hello"}"#.to_vec();
        let signed = |secret: &str, bytes: &[u8]| WebhookBody {
            headers: signed_headers(secret, "delivery-1", timestamp, bytes),
            raw: bytes.to_vec(),
        };
        match workflow_webhook(&signed(SECRET, &raw), SECRET, now) {
            WebhookDecision::Run(event) => assert_eq!(event["text"], "hello"),
            other => panic!("a signed body runs: {other:?}"),
        }
        assert_eq!(workflow_webhook(&signed("other", &raw), SECRET, now), WebhookDecision::Unauthorized);
        assert_eq!(workflow_webhook(&signed(SECRET, &raw), SECRET, now + 400_000), WebhookDecision::Unauthorized);
        let ping = br#"{"type":"ping"}"#.to_vec();
        let mut headers = signed_headers(SECRET, "delivery-2", timestamp, &ping);
        headers.insert("x-github-event".into(), "ping".into());
        assert_eq!(
            workflow_webhook(&WebhookBody { headers, raw: ping }, SECRET, now),
            WebhookDecision::Ignored(WebhookSkip::Ping)
        );
        assert_eq!(
            workflow_webhook(
                &WebhookBody { headers: Headers::new(), raw: vec![b'x'; super::super::hooks::MAX_BODY + 1] },
                SECRET,
                now,
            ),
            WebhookDecision::TooLarge
        );
    }
}
