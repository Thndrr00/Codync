//! `POST /hooks/teams/{id}`: one Bot Framework activity from Teams. The registration points here;
//! the Bot Framework JWT is checked before anything is read, then the message is queued for the
//! bot bound to the app. A card submit is answered through the host's `respondPermission`.

use super::{Config, Hub, activity, client, load};
use crate::api::devices::Caller;
use crate::store::teams::TeamsIncoming;
use anyhow::{Context, Result};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) fn stable_id(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("teams-{}", crate::chat::files::digest_hex(&digest.finalize()))
}

pub(super) fn root_id(generation: &str, conversation: &str) -> String {
    stable_id(&[generation, conversation, "root"])
}

/// The route the Azure registration calls. No bearer token: the Bot Framework JWT authenticates
/// the call, and a connection the host doesn't know is a 404.
pub(crate) async fn receive(
    State(hub): State<Arc<Hub>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(activity): Json<Value>,
) -> Response {
    match accept(hub, &id, &headers, &activity).await {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            let status = if message.contains("unknown Teams connection") {
                StatusCode::NOT_FOUND
            } else if message.contains("token") || message.contains("JWT") || message.contains("Bearer") {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, Json(json!({"ok": false, "error": message}))).into_response()
        }
    }
}

async fn accept(hub: Arc<Hub>, id: &str, headers: &HeaderMap, activity: &Value) -> Result<()> {
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(c) = load(&hub.store)?.filter(|c| c.id == id) else { anyhow::bail!("unknown Teams connection") };
    // Conversation updates do arrive; there is nothing to answer, and Teams expects 200.
    if activity["type"].as_str() != Some("message") {
        return Ok(());
    }
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .context("The Teams activity has no Bearer token")?;
    let service_url = activity::service_url(activity).to_owned();
    client::verify(token, &c.app_id, &service_url).await?;
    anyhow::ensure!(activity::allowed(&c, activity), "This person or tenant isn't allowed to write to the bot");
    let conversation = activity::conversation(activity).to_owned();
    let who = activity::sender(activity);
    hub.store.save_teams_ref(&c.generation, &conversation, &service_url, &who.tenant, &who.id, &who.name)?;
    if let Some(action) = activity::action(activity) {
        answer(hub, &c, activity, &conversation, action).await?;
        return Ok(());
    }
    accept_message(&hub, &c, activity, &conversation, &who).await
}

async fn accept_message(
    hub: &Arc<Hub>,
    c: &Config,
    activity: &Value,
    conversation: &str,
    who: &activity::Sender,
) -> Result<()> {
    let text = activity::text(activity);
    let names = activity::attachments(activity);
    if text.is_empty() && names.is_empty() {
        return Ok(());
    }
    if !activity::mentions_bot(activity) {
        return Ok(());
    }
    let prompt =
        if names.is_empty() { text } else { format!("{text}\n\n[Attached, not downloaded: {}]", names.join(", ")) };
    let activity_id = activity["id"].as_str().filter(|id| !id.is_empty()).context("The Teams activity has no id")?;
    let identity = activity::identity(c, activity).unwrap_or_else(|| who.id.clone());
    let incoming = TeamsIncoming {
        activity: activity_id.to_owned(),
        generation: c.generation.clone(),
        bot: c.bot_id.clone(),
        conversation: conversation.to_owned(),
        entry: stable_id(&[&c.generation, conversation, activity_id]),
        text: prompt,
        user: identity,
        service_url: activity::service_url(activity).to_owned(),
    };
    let mentioned = activity::mentions_bot(activity);
    hub.routines.receive_event(
        &hub.store,
        &json!({
            "source": "teams", "event": if mentioned {"message.mention"} else {"message.created"},
            "conversation": conversation, "user": who.id, "userName": who.name,
            "text": activity::text(activity), "activityId": activity_id,
        }),
        Some(activity_id.to_owned()),
    );
    hub.store.enqueue_teams(&incoming)?;
    hub.store.map_teams_thread(&incoming, &root_id(&c.generation, conversation))?;
    hub.store.record_channel_activity(&crate::store::ChannelActivity {
        id: format!("teams-{}", incoming.entry),
        source: "teams".into(),
        bot_id: incoming.bot.clone(),
        thread_id: Some(root_id(&c.generation, conversation)),
        channel: conversation.to_owned(),
        mentioned,
        at: crate::store::now_ms(),
    })?;
    Ok(())
}

/// Answers the card an `Action.Submit` carries. The card's own print and entry are checked by the
/// same `respondPermission` path every channel uses.
async fn answer(hub: Arc<Hub>, c: &Config, activity: &Value, conversation: &str, action: &str) -> Result<()> {
    let Some((entry_id, option, print)) = crate::channels::approval::parse_action(action) else {
        anyhow::bail!("The Teams card action is not an approval");
    };
    let Some(card) = hub.store.entry(entry_id).filter(|entry| entry.kind == "permission") else {
        return Ok(());
    };
    anyhow::ensure!(card.bot_id == c.bot_id, "That approval belongs to another bot");
    let expected = crate::channels::approval::print(&lens(&card.data)).context("The approval card has no print")?;
    anyhow::ensure!(expected == print, "That approval card changed; open the newest card");
    if let Some(card_activity) = activity["replyToId"].as_str().filter(|id| !id.is_empty()) {
        let shown = hub.store.teams_card_entry(&c.generation, conversation, card_activity)?;
        anyhow::ensure!(shown.as_deref() == Some(entry_id), "That card was not shown in this conversation");
    }
    if card.data["status"] != "pending" {
        return Ok(());
    }
    let option = option.to_owned();
    let entry_id = entry_id.to_owned();
    tokio::spawn(async move {
        let _ = crate::api::dispatch(
            &hub,
            &Caller::Local,
            "respondPermission",
            json!({"entryId": entry_id, "optionId": option}),
        )
        .await;
    });
    Ok(())
}

/// `approval::print` takes the entry-shaped card; `data` alone is not one.
fn lens(data: &Value) -> Value {
    json!({"data": data})
}
