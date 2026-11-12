//! Read-only document and channel receipt metadata for the desktop overlay.

use crate::hub::Hub;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn read(hub: &Arc<Hub>, bot: &str) -> Result<Value> {
    hub.store.bot(bot)?.filter(|row| !row.deleted).context("Unknown bot")?;
    let mut errors = Vec::new();
    let documents = match hub.store.recent_documents(bot, 10) {
        Ok(documents) => documents.into_iter().map(|document| json!({
            "id": document.id, "title": document.title.chars().take(180).collect::<String>(), "updatedAt": document.updated_at,
        })).collect::<Vec<_>>(),
        Err(error) => { errors.push(format!("Documents: {error}")); Vec::new() }
    };
    // Existing receipts are computer-wide and bounded by their store readers. No per-bot scope is claimed.
    let mut channel_failures = Vec::new();
    match hub.store.slack_deliveries() {
        Ok(deliveries) => {
            channel_failures.extend(failure_rows("slack", deliveries.into_iter().map(|item| (item.id, item.state))));
        }
        Err(error) => errors.push(format!("Slack delivery history: {error}")),
    }
    match hub.store.discord_deliveries() {
        Ok(deliveries) => {
            channel_failures.extend(failure_rows("discord", deliveries.into_iter().map(|item| (item.id, item.state))));
        }
        Err(error) => errors.push(format!("Discord delivery history: {error}")),
    }
    let discord_state = match hub.store.kv_read("discord.state") {
        Ok(state) => classify_state(state.as_deref()),
        Err(error) => {
            errors.push(format!("Discord connection state: {error}"));
            "unavailable"
        }
    };
    Ok(json!({
        "documents": documents, "channelFailures": channel_failures,
        "channelState": [{"source":"slack","state":"notReported"},{"source":"discord","state":discord_state}],
        "scope":"computer", "error": (!errors.is_empty()).then(|| errors.join(" · ")),
    }))
}

fn failure_rows(source: &str, deliveries: impl Iterator<Item = (String, String)>) -> Vec<Value> {
    deliveries
        .filter(|(_, state)| matches!(state.as_str(), "failed" | "unknown"))
        .take(10)
        .map(|(id, state)| json!({"id":id,"source":source,"state":state}))
        .collect()
}

fn classify_state(state: Option<&str>) -> &'static str {
    match state {
        Some("ok") => "ok",
        Some("starting") => "starting",
        Some(value) if value.starts_with("error:") => "error",
        _ => "notReported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_rows_are_bounded_failure_metadata_without_outbound_text_or_external_ids() {
        let rows = failure_rows(
            "slack",
            (0..30).map(|id| (id.to_string(), if id % 2 == 0 { "failed" } else { "unknown" }.into())),
        );
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0], json!({"id":"0","source":"slack","state":"failed"}));
        assert!(rows.iter().all(|row| row.as_object().unwrap().len() == 3));
        assert_eq!(failure_rows("discord", [("ok".into(), "sent".into())].into_iter()).len(), 0);
    }

    #[test]
    fn channel_state_never_copies_raw_errors_or_claims_an_unknown_connection_is_healthy() {
        assert_eq!(classify_state(Some("error: PRIVATE TOKEN")), "error");
        assert_eq!(classify_state(Some("ok")), "ok");
        assert_eq!(classify_state(None), "notReported");
        assert_eq!(classify_state(Some("future-state")), "notReported");
    }
}
