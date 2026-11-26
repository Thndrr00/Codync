//! A client-reported call receipt; it is never proof that audio was spoken or heard.
use crate::{
    hub::Hub,
    store::{EntryKind, Lane},
};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// Pure: a reply dictated into one chat that arrives in a *separate* window is read aloud in the
/// window it arrived in. A reply still in its own window, a notice, an empty or over-long reply and
/// a dictation that never crossed windows are left alone. Only the text to speak is returned.
pub(crate) fn read_aloud_target(reply: &Value, window: &str) -> Option<String> {
    if window.trim().is_empty() || reply["dictated"] != true {
        return None;
    }
    let arrived = reply["window"].as_str()?;
    let started = reply["dictatedWindow"].as_str()?;
    if arrived != window || started == window {
        return None;
    }
    if matches!(reply["data"]["source"].as_str(), Some("voice_receipt" | "notice" | "voice_decisions")) {
        return None;
    }
    let text = reply["data"]["text"].as_str()?.trim();
    if text.is_empty() || text.chars().count() > 4000 {
        return None;
    }
    Some(text.to_owned())
}

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
    if let (Some(window), Some(reply)) = (body["window"].as_str(), body.get("reply"))
        && let Some(text) = read_aloud_target(reply, window)
    {
        data["voiceReadAloud"] = json!({"window": window, "text": text});
    }
    // New clients end the host-owned call first. Enrich its existing row so its conversation,
    // generated title and the owner's decision confirmation share one receipt.
    let call_id = call.to_string();
    if let Some(entry) = hub.store.call_notice(bot, &call_id)? {
        let entry = hub.edit_entry(&entry.id, |entry| {
            if !entry.data["voiceReceipt"].is_null() {
                if entry.data["voiceReceipt"] != data["voiceReceipt"] {
                    bail!("The call receipt already exists with different content.");
                }
                return Ok(false);
            }
            entry.data["source"] = data["source"].clone();
            entry.data["voiceReceipt"] = data["voiceReceipt"].clone();
            if let Some(text) = data.get("voiceTranscriptText") {
                entry.data["voiceTranscriptText"] = text.clone();
            }
            if let Some(aloud) = data.get("voiceReadAloud") {
                entry.data["voiceReadAloud"] = aloud.clone();
            }
            Ok(true)
        })?;
        return Ok(json!({"entry":entry}));
    }
    let (entry, added) =
        hub.add_entry_once(&id, &Lane::main(bot), EntryKind::Notice, hub.store.max_turn(bot), &data)?;
    if !added && entry.data["voiceReceipt"] != data["voiceReceipt"] {
        bail!("The call receipt already exists with different content.");
    }
    Ok(json!({"entry":entry}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{BotConfig, Store};

    #[test]
    fn a_host_call_gets_one_idempotent_receipt_without_losing_its_title() {
        let dir = std::env::temp_dir().join(format!("codync-receipt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("test.db")).unwrap();
        let cfg: BotConfig = serde_json::from_value(json!({"id":"bot","name":"Bot","backend":"claude"})).unwrap();
        store.save_bot(&cfg).unwrap();
        let hub = Hub::new(
            store,
            "test".into(),
            crate::remote::identity::Identity::load_or_create(&dir).unwrap(),
            "token".into(),
            19223,
        );
        let call = uuid::Uuid::new_v4().to_string();
        let original = hub
            .add_entry(
                &Lane::main("bot"),
                EntryKind::Notice,
                0,
                &json!({"call":call,"callSeconds":10,"text":"A useful title","transcript":[]}),
            )
            .unwrap();
        let body = json!({"botId":"bot","callId":call,"seconds":10});
        let first = log(&hub, &body).unwrap();
        let repeated = log(&hub, &body).unwrap();
        assert_eq!(first["entry"]["id"], original.id);
        assert_eq!(repeated["entry"]["id"], original.id);
        assert_eq!(first["entry"]["data"]["text"], "A useful title");
        assert_eq!(first["entry"]["data"]["source"], "voice_receipt");
        assert_eq!(hub.store.history("bot", i64::MAX, 100).unwrap().len(), 1);
        assert!(log(&hub, &json!({"botId":"bot","callId":call,"seconds":11})).is_err());
        drop(hub);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_dictated_reply_is_read_aloud_only_in_the_window_it_arrived_in() {
        let reply = json!({
            "dictated": true,
            "dictatedWindow": "main",
            "window": "thread-7",
            "data": {"source": "assistant", "text": "  Here is the answer.  "}
        });
        assert_eq!(read_aloud_target(&reply, "thread-7").as_deref(), Some("Here is the answer."));
        assert_eq!(read_aloud_target(&reply, "main"), None, "it never crossed to another window");
        let same = json!({"dictated":true,"dictatedWindow":"main","window":"main","data":{"text":"hi"}});
        assert_eq!(read_aloud_target(&same, "main"), None);
        let plain = json!({"window":"thread-7","dictatedWindow":"main","data":{"text":"hi"}});
        assert_eq!(read_aloud_target(&plain, "thread-7"), None, "a reply that wasn't dictated stays quiet");
        let notice =
            json!({"dictated":true,"dictatedWindow":"main","window":"thread-7","data":{"source":"notice","text":"hi"}});
        assert_eq!(read_aloud_target(&notice, "thread-7"), None);
        let long = json!({"dictated":true,"dictatedWindow":"main","window":"thread-7",
            "data":{"text":"x".repeat(4001)}});
        assert_eq!(read_aloud_target(&long, "thread-7"), None);
    }

    #[test]
    fn a_receipt_carries_the_read_aloud_target_to_the_client() {
        let dir = std::env::temp_dir().join(format!("codync-receipt-read-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("test.db")).unwrap();
        let cfg: BotConfig = serde_json::from_value(json!({"id":"bot","name":"Bot","backend":"claude"})).unwrap();
        store.save_bot(&cfg).unwrap();
        let hub = Hub::new(
            store,
            "test".into(),
            crate::remote::identity::Identity::load_or_create(&dir).unwrap(),
            "token".into(),
            19224,
        );
        let body = json!({
            "botId":"bot", "seconds":3,
            "window":"thread-7",
            "reply":{"dictated":true,"dictatedWindow":"main","window":"thread-7",
                "data":{"source":"assistant","text":"Read this in the other window"}}
        });
        let result = log(&hub, &body).unwrap();
        assert_eq!(result["entry"]["data"]["voiceReadAloud"]["window"], "thread-7");
        assert_eq!(result["entry"]["data"]["voiceReadAloud"]["text"], "Read this in the other window");
        drop(hub);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
