//! Approval cards in Slack: Block Kit with a button per option, answered from the card itself.
//!
//! A card is posted in the Slack thread the bot was asked in, and settles once anyone answers it (here or in
//! another Codync client). Buttons carry `p|<entry id>|<option>|<print>` in their `value`, the same code the
//! Discord and Telegram buttons carry, and every answer goes through the host's `respondPermission` method.

use super::socket::Endpoint;
use super::{Arc, Config, Hub, Result, SLOT, Value, json, load};
use crate::api::devices::Caller;
use crate::channels::{
    approval::{self, Approval},
    render,
};
use anyhow::Context as _;
use std::time::Duration;

/// Watches the host's events: each pending approval in a bound Slack thread becomes a card, and each answer
/// settles it.
pub(super) async fn run(hub: Arc<Hub>) {
    let mut events = hub.events.subscribe();
    loop {
        let event = match events.recv().await {
            Ok(event) => event,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        };
        if event["type"] != "entry" || event["entry"]["kind"] != "permission" {
            continue;
        }
        if let Err(error) = card(&hub, &event["entry"]).await {
            tracing::warn!(error = format!("{error:#}"), "Slack approval card was not shown");
        }
    }
}

async fn card(hub: &Arc<Hub>, entry: &Value) -> Result<()> {
    if hub.store.kv_read(SLOT)?.is_none() {
        return Ok(());
    }
    let Some(id) = entry["id"].as_str() else { return Ok(()) };
    let Some(thread) = entry["threadId"].as_str() else { return Ok(()) };
    let Some(root) = hub.store.entry(thread) else { return Ok(()) };
    let Some(channel) = root.data["slackChannel"].as_str() else { return Ok(()) };
    let Some(user) = root.data["slackUser"].as_str() else { return Ok(()) };
    let Some(bot) = entry["botId"].as_str() else { return Ok(()) };
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)? else { return Ok(()) };
    let Some(binding) = c.bindings.iter().find(|binding| binding.channel_id == channel && binding.bot_id == bot) else {
        return Ok(());
    };
    if !binding.allowed_users.iter().any(|allowed| allowed == user) {
        return Ok(());
    }
    if hub.store.bot(bot)?.is_none_or(|row| row.deleted || row.config.is_group()) {
        return Ok(());
    }
    let shown = entry["data"]["slackCard"].as_str().is_some();
    let settled = entry["data"]["slackSettled"] == true;
    match (entry["data"]["status"].as_str(), shown, settled) {
        (Some("pending"), false, _) => {
            let Some(model) = Approval::from_entry(entry) else { return Ok(()) };
            let blocks = model.blocks(|option| approval::action(entry, option));
            let body = render::slack(&blocks);
            let thread = root.data["slackThread"].as_str().unwrap_or_default();
            let ts = post(&c.bot_token, channel, thread, &body).await?;
            let mut data = entry["data"].clone();
            data["slackCard"] = ts.into();
            hub.set_entry(id, &data);
        }
        (Some(status), true, false) if status != "pending" => {
            let Some(ts) = entry["data"]["slackCard"].as_str() else { return Ok(()) };
            let selected = entry["data"]["selected"].as_str();
            let text = Approval::from_entry(entry)
                .map_or_else(|| format!("{status}\n(closed)"), |model| model.settled(selected, status));
            update(&c.bot_token, channel, ts, &text).await?;
            let mut data = entry["data"].clone();
            data["slackSettled"] = true.into();
            hub.set_entry(id, &data);
        }
        _ => {}
    }
    Ok(())
}

/// Posts a card in `channel` (inside `thread` when it has one) and returns its timestamp.
async fn post(token: &str, channel: &str, thread: &str, body: &Value) -> Result<String> {
    let ep = Endpoint::slack();
    let url = format!("{}/chat.postMessage", ep.api);
    let client = crate::egress::client(ep.policy, &url).await?;
    let mut request = json!({
        "channel": channel,
        "text": body["text"],
        "blocks": body["blocks"],
        "unfurl_links": false,
        "unfurl_media": false,
    });
    if !thread.is_empty() {
        request["thread_ts"] = thread.into();
    }
    let answer = call(&client, &url, token, &request).await?;
    answer["ts"].as_str().context("Slack sent no card timestamp").map(str::to_owned)
}

/// Replaces a settled card's text and removes its buttons.
async fn update(token: &str, channel: &str, ts: &str, text: &str) -> Result<()> {
    let ep = Endpoint::slack();
    let url = format!("{}/chat.update", ep.api);
    let client = crate::egress::client(ep.policy, &url).await?;
    call(&client, &url, token, &json!({"channel": channel, "ts": ts, "text": text, "blocks": []})).await?;
    Ok(())
}

async fn call(client: &reqwest::Client, url: &str, token: &str, body: &Value) -> Result<Value> {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("Slack card failed: {}", error.without_url()))?;
    let status = response.status();
    let answer = crate::egress::json(response, 64 * 1024).await.unwrap_or_default();
    anyhow::ensure!(
        status.is_success() && answer["ok"] == true,
        "Slack refused the card: {}",
        answer["error"].as_str().unwrap_or("unknown")
    );
    Ok(answer)
}

/// Answers an approval from a Slack button: allowed people only, and only for the card the button was made
/// for. The first answer wins, from any client.
pub(super) async fn answer(hub: &Arc<Hub>, c: &Config, channel: &str, user: &str, value: &str) -> Result<()> {
    let Some((entry_id, option, print)) = approval::parse_action(value) else {
        anyhow::bail!("That button is not for this.");
    };
    let Some(card) = hub.store.entry(entry_id).filter(|entry| entry.kind == "permission") else {
        anyhow::bail!("That approval is gone.");
    };
    let asking = c.bindings.iter().any(|binding| {
        binding.channel_id == channel
            && binding.bot_id == card.bot_id
            && binding.allowed_users.iter().any(|allowed| allowed == user)
    });
    anyhow::ensure!(asking, "You are not allowed to answer this.");
    let card = serde_json::to_value(&card)?;
    anyhow::ensure!(approval::print(&card).as_deref() == Some(print), "That approval changed.");
    let _ = crate::api::dispatch(
        hub,
        &Caller::Local,
        "respondPermission",
        json!({"entryId": entry_id, "optionId": option}),
    )
    .await;
    Ok(())
}
