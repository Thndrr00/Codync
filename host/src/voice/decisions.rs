//! Only an owner's explicit confirmation creates a decision receipt; transcript text has no authority.
use crate::{
    api::devices::Caller,
    hub::Hub,
    store::{EntryKind, Lane},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    bot_id: String,
    receipt_id: String,
    #[serde(default)]
    summary: Option<String>,
}
pub(crate) fn call(hub: &Hub, caller: &Caller, method: &str, body: Value) -> Result<Value> {
    let request: Request = serde_json::from_value(body)?;
    hub.store.bot(&request.bot_id)?.filter(|row| !row.deleted && !row.config.is_group()).context("Unknown bot")?;
    let receipt = hub
        .store
        .entry(&request.receipt_id)
        .filter(|entry| entry.bot_id == request.bot_id && entry.data["source"] == "voice_receipt")
        .context("Unknown voice receipt")?;
    let id = format!("voice-decisions-{}", receipt.id);
    if method == "voiceDecisions" {
        return Ok(json!({"entry":hub.store.entry(&id)}));
    }
    let summary = request.summary.context("Write the decisions you personally confirm")?.trim().to_owned();
    ensure!(
        !summary.is_empty() && summary.chars().count() <= 4000 && !summary.contains('\0'),
        "Decision summary must contain 1–4000 characters"
    );
    let actor = match caller {
        Caller::Local => json!({"kind":"owner","id":"local"}),
        Caller::Device { key, .. } => json!({"kind":"device","id":key}),
        Caller::Pairing { .. } => anyhow::bail!("Owner control is required"),
    };
    let data = json!({"source":"voice_decisions","text":format!("Owner-confirmed call decisions\n\n{summary}\n\nThis receipt records confirmation only; it does not execute actions."),
        "voiceDecisions":{"version":1,"receiptId":receipt.id,"summary":summary,"confirmedBy":actor}});
    let (entry, added) = hub.add_entry_once(&id, &Lane::main(&request.bot_id), EntryKind::Notice, &data)?;
    ensure!(
        added || entry.data["voiceDecisions"]["summary"] == data["voiceDecisions"]["summary"],
        "These call decisions were already confirmed with different content"
    );
    Ok(json!({"entry":entry}))
}
