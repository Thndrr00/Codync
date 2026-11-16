//! The policy API: the list and its presets, saving (compiled and dry run first), enabling, removing,
//! the saved versions, and the what-if that never acts.
//!
//! Nothing here decides anything: a save is checked with the same [`engine::validate`] a request's rule
//! goes through, and the what-if calls the same [`engine::evaluate`].

use super::engine;
use super::{Request, presets, variables};
use crate::api::devices::Caller;
use crate::hub::Hub;
use crate::store::Policy;
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub const METHODS: &[&str] =
    &["policies", "savePolicy", "removePolicy", "setPolicyEnabled", "policyVersions", "previewPolicy"];

pub fn call(hub: &Hub, caller: &Caller, method: &str, body: &Value) -> Result<Value> {
    match method {
        "policies" => list(hub),
        "savePolicy" => save(hub, body),
        "removePolicy" => Ok(json!({"removed": hub.store.remove_policy(str_arg(body, "id")?)?})),
        "setPolicyEnabled" => {
            let enabled = body["enabled"].as_bool().ok_or_else(|| anyhow::anyhow!("`enabled` is required"))?;
            Ok(json!({"policy": hub.store.set_policy_enabled(str_arg(body, "id")?, enabled)?}))
        }
        "policyVersions" => Ok(json!({"versions": hub.store.policy_versions(str_arg(body, "id")?)?})),
        "previewPolicy" => preview(hub, caller, body),
        _ => bail!("unknown method"),
    }
}

fn str_arg<'a>(body: &'a Value, key: &str) -> Result<&'a str> {
    body[key].as_str().ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

/// The saved rules, the presets to start from, the variables a rule may use and the limits.
fn list(hub: &Hub) -> Result<Value> {
    let policies = hub.store.policies()?;
    let enforcing = policies.iter().any(|policy| policy.enabled);
    Ok(json!({
        "policies": policies,
        "presets": presets::list(),
        "variables": variables::VARIABLES
            .iter()
            .map(|(name, about)| json!({"name": name, "about": about}))
            .collect::<Vec<Value>>(),
        "limits": engine::limits(),
        // Whether any rule is switched on: the approval rules decide until one is.
        "enforcing": enforcing,
        // An allow rule makes the list an allowlist, so nothing outside it goes through.
        "allowlist": policies.iter().any(|policy| policy.enabled && policy.effect == crate::store::PolicyEffect::Allow),
    }))
}

/// Adds or updates a rule: its shape, its compile and a dry run on a sample of every initiator first.
fn save(hub: &Hub, body: &Value) -> Result<Value> {
    let given = body.get("policy").cloned().unwrap_or_else(|| body.clone());
    let policy: Policy = serde_json::from_value(given)?;
    engine::validate(&policy)?;
    Ok(json!({"policy": hub.store.save_policy(policy)?}))
}

/// What the policy list says about a proposed call, its variables and a sentence, without acting.
fn preview(hub: &Hub, caller: &Caller, body: &Value) -> Result<Value> {
    let policies: Vec<Policy> = match body.get("policies") {
        Some(list) if list.is_array() => serde_json::from_value(list.clone())?,
        _ => hub.store.policies()?,
    };
    let request = request_of(hub, caller, body)?;
    let decision = engine::evaluate(&policies, &request);
    let allowlist = match &decision {
        engine::Decision::Rule { allowlist, .. } | engine::Decision::NoMatch { allowlist } => *allowlist,
        _ => policies.iter().any(|policy| policy.enabled && policy.effect == crate::store::PolicyEffect::Allow),
    };
    let answer = match &decision {
        engine::Decision::Rule { rule, .. } if rule.effect == crate::store::PolicyEffect::Allow => "allow",
        decision if decision.denied() => "deny",
        _ => "unset",
    };
    Ok(json!({"preview": {
        "decision": answer,
        "source": decision.source(),
        "rule": decision.rule(),
        "allowlist": allowlist,
        "refusal": decision.refusal(),
        // Every enabled rule's own answer, in the order they are tried: the candidates, with outcomes.
        "rules": engine::trace(&policies, &request),
        // What the rules read, with credentials masked: for the person reading the answer.
        "variables": request.snapshot(),
        "explanation": engine::explain(&decision, &request),
    }}))
}

/// The request a what-if describes. `tool` is required; everything else is optional.
fn request_of(hub: &Hub, caller: &Caller, body: &Value) -> Result<Request> {
    let tool = str_arg(body, "tool")?.trim();
    if tool.is_empty() {
        bail!("`tool` is required");
    }
    let mut request = Request::tool(tool);
    if let Some(kind) = body["kind"].as_str().filter(|kind| !kind.trim().is_empty()) {
        request.kind = kind.trim().to_owned();
    }
    request.intent = body["intent"].as_str().unwrap_or(tool).to_owned();
    request.command = body["command"].as_str().unwrap_or_default().to_owned();
    request.file_path = body["filePath"].as_str().unwrap_or_default().to_owned();
    request.file_op = body["fileOp"].as_str().unwrap_or_default().to_owned();
    request.mcp_server = body["mcpServer"].as_str().unwrap_or_default().to_owned();
    request.mcp_tool = body["mcpTool"].as_str().unwrap_or_default().to_owned();
    request.page_url = body["pageUrl"].as_str().unwrap_or_default().to_owned();
    request.key = body["key"].as_str().unwrap_or_default().to_owned();
    request.initiator =
        body["initiator"].as_str().filter(|kind| !kind.trim().is_empty()).unwrap_or("person").trim().to_owned();
    // A bot names itself and the person whose it is; without one the caller is who asks.
    match body["botId"].as_str().and_then(|id| hub.store.bot(id).ok().flatten()).filter(|row| !row.deleted) {
        Some(row) => {
            request.bot = row.config.id.clone();
            request.bot_name = row.config.name.clone();
            match &row.config.person {
                Some(person) => {
                    request.actor = person.clone();
                    request.actor_role =
                        hub.store.person(person).map_or_else(|| "member".into(), |found| found.role.as_str().into());
                }
                None => request.actor_role = "owner".into(),
            }
        }
        None => match crate::people::caller_person(hub, caller) {
            Some(person) => {
                request.actor_role =
                    hub.store.person(&person).map_or_else(|| "member".into(), |found| found.role.as_str().into());
                request.actor = person;
            }
            None => request.actor_role = "owner".into(),
        },
    }
    Ok(request)
}
