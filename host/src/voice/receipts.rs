//! A client-reported call receipt; it is never proof that audio was spoken or heard.
use crate::{
    hub::Hub,
    store::{EntryKind, Lane},
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

pub(crate) fn log(hub: &Hub, body: &Value) -> Result<Value> {
    let bot = body["botId"].as_str().context("botId is required")?;
    hub.store.bot(bot)?.filter(|b| !b.deleted && !b.config.is_group()).context("Unknown bot.")?;
    let seconds = body["seconds"].as_u64().filter(|s| *s <= 604_800).context("seconds must be between 0 and 604800")?;
    let call = match body.get("callId") {
        Some(value) => {
            uuid::Uuid::parse_str(value.as_str().context("callId must be a UUID")?).context("Invalid callId")?
        }
        None => uuid::Uuid::new_v4(),
    };
    let id = format!("voice-receipt-{bot}-{call}");
    let mut data = json!({"text":"Voice chat","callSeconds":seconds,"source":"voice_receipt",
        "voiceReceipt":{"version":1,"callId":call.to_string(),"kind":"client_reported_duration","seconds":seconds}});
    if let Some(value) = body.get("transcript") {
        if body.get("callId").is_none() {
            bail!("Transcript receipts require a stable callId.");
        }
        let transcript = super::transcript::Transcript::parse(value)?;
        data["voiceTranscriptText"] = json!(transcript.display());
        data["voiceReceipt"]["kind"] = json!("client_reported_transcript");
        data["voiceReceipt"]["transcript"] = serde_json::to_value(transcript)?;
    }
    let (entry, added) =
        hub.add_entry_once(&id, &Lane::main(bot), EntryKind::Notice, hub.store.max_turn(bot), &data)?;
    if !added && entry.data["voiceReceipt"] != data["voiceReceipt"] {
        bail!("The call receipt already exists with different content.");
    }
    Ok(json!({"entry":entry}))
}
