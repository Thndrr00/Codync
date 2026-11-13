//! HTTP routes: bearer-token checks for this computer's apps, SSE streams, the channel
//! upgrade, OAuth callbacks, local routine webhooks and sandboxed bot mini-apps.

use super::devices::Caller;
use super::{dispatch, error_status, events_stream, term_events};
use crate::hub::Hub;
use crate::remote::crypto;
use crate::{market, usage};
use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

pub fn router(hub: Arc<Hub>) -> Router {
    let main = Router::new()
        .route("/health", get(health))
        .route("/hooks/routines/{id}", post(routine_hook))
        .route("/hooks/slack", post(crate::slack::ingress::receive))
        .route("/events", get(events))
        .route("/scim/v2/Users", get(scim_users).post(scim_create))
        .route("/scim/v2/Users/{id}", axum::routing::delete(scim_delete))
        .route("/api/{method}", post(command))
        .route("/agui/{bot}", post(agui))
        .route("/.well-known/agent.json", get(agent_card))
        .route("/a2a", post(a2a))
        .route("/share/{token}", get(shared_page))
        .route("/api/setIcon", post(icon_command).layer(axum::extract::DefaultBodyLimit::max(8 * 1024 * 1024)))
        .route("/term/{id}", get(term_stream))
        .route("/ingest/statusline", post(statusline))
        .route("/ingest/hook", post(hook))
        .route("/channel", get(channel))
        .route("/oauth/callback", get(oauth_callback))
        .layer(axum::middleware::from_fn(super::rebinding::guard))
        .layer(axum::middleware::from_fn_with_state(hub.port, super::origin::guard));
    Router::new()
        .merge(main)
        .merge(super::apps_runtime::router().layer(axum::middleware::from_fn(super::rebinding::guard)))
        .with_state(hub)
}

/// What another agent is told when it asks a Codync bot for work over A2A.
const A2A_SYSTEM: &str = "You are answering an agent that called this computer's Codync host over the A2A \
protocol. Reply directly in plain text, in the caller's language. You have no tools, no memory of this call and no \
access to the person's chat; if the request needs one of those, say what to do in the Codync chat instead.";

fn rpc_error(id: &Value, code: i64, message: &str) -> Json<Value> {
    Json(json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}))
}

/// `GET /.well-known/agent.json`: one A2A skill per agent bot on this computer.
async fn agent_card(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let mut skills = Vec::new();
    for row in hub.store.bots().unwrap_or_default().into_iter().filter(|row| !row.deleted && !row.config.is_group()) {
        let config = row.config;
        let name = config.name;
        let description = if config.description.trim().is_empty() {
            format!("{name} answers in its Codync chat.")
        } else {
            config.description
        };
        skills.push(json!({
            "id": config.id,
            "name": name,
            "description": description,
            "tags": ["codync", config.backend],
        }));
    }
    Ok(Json(json!({
        "protocolVersion": "0.3.0",
        "name": "Codync",
        "description": "Agents that run on this computer, in Codync.",
        "url": format!("http://{peer}/a2a"),
        "version": env!("CARGO_PKG_VERSION"),
        "capabilities": {"streaming": false, "pushNotifications": false, "stateTransitionHistory": false},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": skills,
    })))
}

/// `POST /a2a`: A2A JSON-RPC `message/send`. The caller names the bot in `params.metadata.botId`
/// (its A2A skill id); without one, the first agent bot answers. It runs in a throwaway session, so
/// nothing appears in the person's chats and nothing is remembered.
async fn a2a(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body["method"].as_str().unwrap_or_default();
    if method != "message/send" {
        return Ok(rpc_error(&id, -32601, &format!("unsupported method: {method}")));
    }
    let params = &body["params"];
    let text = params["message"]["parts"]
        .as_array()
        .map(|parts| parts.iter().filter_map(|part| part["text"].as_str()).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    let text = text.trim();
    if text.is_empty() {
        return Ok(rpc_error(&id, -32602, "message.parts needs a text part"));
    }
    let wanted = params["metadata"]["botId"]
        .as_str()
        .or_else(|| params["message"]["metadata"]["botId"].as_str())
        .map(str::to_owned);
    let row = match wanted {
        Some(bot) => hub.store.bot(&bot).ok().flatten().filter(|row| !row.deleted),
        None => {
            hub.store.bots().ok().and_then(|rows| rows.into_iter().find(|row| !row.deleted && !row.config.is_group()))
        }
    };
    let Some(row) = row else {
        return Ok(rpc_error(&id, -32602, "unknown bot; name one with params.metadata.botId"));
    };
    if row.config.is_group() {
        return Ok(rpc_error(&id, -32602, "a group has no single agent; ask one of its members"));
    }
    let task_id = format!("task-{}", uuid::Uuid::new_v4());
    let context_id =
        params["contextId"].as_str().filter(|c| !c.is_empty()).map(str::to_owned).unwrap_or_else(|| task_id.clone());
    let result = match crate::chat::oneshot::one_shot(&hub, &row.config, A2A_SYSTEM, text).await {
        Ok(answer) => json!({
            "kind": "task",
            "id": task_id.clone(),
            "contextId": context_id,
            "status": {"state": "completed", "timestamp": crate::store::now_ms()},
            "artifacts": [{
                "artifactId": format!("artifact-{task_id}"),
                "name": "reply",
                "parts": [{"kind": "text", "text": answer}],
            }],
        }),
        Err(error) => json!({
            "kind": "task",
            "id": task_id,
            "contextId": context_id,
            "status": {
                "state": "failed",
                "timestamp": crate::store::now_ms(),
                "message": {"role": "agent", "parts": [{"kind": "text", "text": format!("{error:#}")}]},
            },
            "artifacts": [],
        }),
    };
    Ok(Json(json!({"jsonrpc": "2.0", "id": id, "result": result})))
}

/// `GET /share/{token}`: the page a read-only link stands for. No login: the token is the secret.
async fn shared_page(State(hub): State<Arc<Hub>>, Path(token): Path<String>) -> Response {
    match crate::share::get(&hub, &token) {
        Ok(Some(link)) => {
            let mut response = axum::response::Html(link.html).into_response();
            if let Ok(value) = axum::http::HeaderValue::from_str("no-store") {
                response.headers_mut().insert(axum::http::header::CACHE_CONTROL, value);
            }
            response
        }
        Ok(None) => (StatusCode::NOT_FOUND, "This link is gone.").into_response(),
        Err(error) => {
            tracing::warn!(error = format!("{error:#}"), "couldn't read a shared page");
            (StatusCode::INTERNAL_SERVER_ERROR, "Couldn't open the link.").into_response()
        }
    }
}

async fn icon_command(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    dispatch(&hub, &Caller::Local, "setIcon", body).await.map(Json).map_err(|e| {
        let (status, message) = error_status(&e);
        ApiError(status, message)
    })
}

/// Loopback, also as an IPv4-mapped IPv6 address (`::ffff:127.0.0.1` under `--bind ::`).
fn is_loopback(ip: IpAddr) -> bool {
    ip.to_canonical().is_loopback()
}

/// The bearer token only works from this computer; phones use the E2E channel.
fn authorized(hub: &Hub, headers: &HeaderMap, peer: SocketAddr) -> bool {
    let given = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    is_loopback(peer.ip()) && crypto::ct_eq(given.as_bytes(), hub.token.as_bytes())
}

fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "invalid token".into())
}

/// `GET /scim/v2/Users`: the owner and this computer's paired devices, for a provisioning client with the SCIM
/// bearer token from Settings.
async fn scim_users(State(hub): State<Arc<Hub>>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let header = headers.get("authorization").and_then(|h| h.to_str().ok());
    if !crate::enterprise::scim_allowed(&hub.store, header) {
        return Err(unauthorized());
    }
    crate::enterprise::scim_users(&hub)
        .map(Json)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))
}

/// `POST /scim/v2/Users`: a provisioning client adds (or updates) a member by email.
async fn scim_create(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let header = headers.get("authorization").and_then(|h| h.to_str().ok());
    if !crate::enterprise::scim_allowed(&hub.store, header) {
        return Err(unauthorized());
    }
    let user_name = body["userName"].as_str().unwrap_or_default();
    let name = body["displayName"].as_str().unwrap_or(user_name);
    crate::enterprise::add_member(&hub.store, user_name, name)
        .map(|answer| (StatusCode::CREATED, Json(answer)))
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, format!("{error:#}")))
}

/// `DELETE /scim/v2/Users/{id}`: removes the member a provisioning client created.
async fn scim_delete(
    State(hub): State<Arc<Hub>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let header = headers.get("authorization").and_then(|h| h.to_str().ok());
    if !crate::enterprise::scim_allowed(&hub.store, header) {
        return Err(unauthorized());
    }
    match crate::enterprise::remove_member(&hub.store, &id) {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(ApiError(StatusCode::NOT_FOUND, "unknown member".into())),
        Err(error) => Err(ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{error:#}"))),
    }
}

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1}))).into_response()
    }
}

#[derive(Default, Deserialize)]
struct HealthQuery {
    /// Sent by the desktop app's own poll: it's open to carry out an update request.
    #[serde(default)]
    app: bool,
}

/// `GET /health`, open to the network. Where the binary lives (a path with the user's name) and whether
/// a turn is running are for this computer's updater and desktop app only.
async fn health(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(q): Query<HealthQuery>,
) -> Json<Value> {
    let mut health = json!({
        "ok": true,
        "hostId": hub.host_id,
        "computerId": hub.identity.computer_id(),
        "version": env!("CARGO_PKG_VERSION"),
        "environment": crate::environment::Environment::current().name(),
    });
    if is_loopback(peer.ip()) {
        if q.app {
            crate::update::note_app_poll();
        }
        let binary = crate::service::binary_identity();
        health["binaryPath"] = json!(binary.map(|(path, _)| path));
        health["binaryHash"] = json!(binary.map(|(_, hash)| hash));
        health["busy"] = hub.busy().into();
        health["updateRequested"] = crate::update::app_requested().into();
    }
    Json(health)
}

async fn statusline(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    if !authorized(&hub, &headers, peer) {
        return StatusCode::UNAUTHORIZED;
    }
    let Ok(body) = serde_json::from_slice::<Value>(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    usage::ingest_statusline(&hub, &body);
    StatusCode::NO_CONTENT
}

/// `POST /ingest/hook`: one event of an agent running in a terminal, from `codync-host hook`. A permission request
/// answers only once a person has decided (or the wait runs out), so the connection may stay open for minutes.
async fn hook(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let body = serde_json::from_slice::<Value>(&body)
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;
    Ok(Json(crate::external::ingest(&hub, &body).await))
}

async fn command(
    State(hub): State<Arc<Hub>>,
    Path(method): Path<String>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    // The token is checked before the body is even looked at.
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).map_err(|e| ApiError(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?
    };
    dispatch(&hub, &Caller::Local, &method, body).await.map(Json).map_err(|e| {
        let (status, message) = error_status(&e);
        ApiError(status, message)
    })
}

#[derive(Deserialize)]
struct OAuthCallback {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// `GET /oauth/callback`: where a connector's sign-in page returns when the browser runs on
/// this computer. No bearer token: the one-time `state` (and the host-held PKCE verifier) authenticate.
async fn oauth_callback(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(q): Query<OAuthCallback>,
) -> Response {
    if !is_loopback(peer.ip()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let error = q.error_description.or(q.error);
    let result =
        market::oauth::finish(&hub.store, q.state.as_deref().unwrap_or_default(), q.code.as_deref(), error.as_deref())
            .await;
    let (title, detail) = match &result {
        Ok(c) => (format!("{} is connected", c.name), "You can close this tab and go back to Codync.".to_owned()),
        Err(e) => ("Sign-in didn't finish".to_owned(), format!("{e:#}")),
    };
    let escape = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    axum::response::Html(format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>Codync</title>\
         <body style=\"font:16px system-ui;margin:20vh auto;max-width:28em;padding:0 16px;text-align:center\">\
         <h2>{}</h2><p>{}</p></body>",
        escape(&title),
        escape(&detail)
    ))
    .into_response()
}

#[derive(Deserialize)]
struct ChannelQuery {
    v: Option<u32>,
}

/// `GET /channel?v=1`: the direct E2E channel. No bearer token: the handshake authenticates.
async fn channel(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Query(q): Query<ChannelQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    if q.v != Some(1) {
        return (StatusCode::UPGRADE_REQUIRED, Json(json!({"error": {"code": "upgradeRequired"}}))).into_response();
    }
    ws.max_message_size(crate::remote::channel::MAX_WS_MESSAGE)
        .on_upgrade(move |socket| crate::remote::channel::serve_direct(hub, socket, peer.ip()))
}

async fn term_stream(
    State(hub): State<Arc<Hub>>,
    Path(id): Path<String>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let events =
        term_events(&hub, &id).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "that terminal is gone".into()))?;
    let events = events.map(|v| Ok(Event::default().data(v.to_string())));
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

/// `POST /agui/:bot`: a minimal AG-UI endpoint. It takes the last user message (or `message`),
/// starts a turn on the bot and streams the standard events over SSE once the final reply exists.
async fn agui(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(bot): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    use crate::store::Lane;
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let text = body["message"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            body["messages"]
                .as_array()
                .and_then(|list| list.iter().rev().find(|m| m["role"] == "user"))
                .and_then(|m| m["content"].as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "message is required".into()));
    }
    let run_id = format!("run-{}", uuid::Uuid::new_v4());
    let thread_id = body["threadId"].as_str().unwrap_or("main").to_owned();
    let sent = dispatch(
        &hub,
        &Caller::Local,
        "send",
        json!({"botId": bot, "text": text, "clientNonce": format!("agui-{}", uuid::Uuid::new_v4())}),
    )
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))?;
    let start = sent["entry"]["seq"].as_i64().unwrap_or(0);
    let lane = Lane::main(&bot);

    let opened = json!({"threadId": thread_id, "runId": run_id});
    let started = futures::stream::once(async move {
        Ok::<Event, Infallible>(Event::default().event("RUN_STARTED").data(opened.to_string()))
    });
    let finished = {
        let hub = hub.clone();
        let bot = bot.clone();
        let run_id = run_id.clone();
        let thread_id = thread_id.clone();
        futures::stream::once(async move {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
            loop {
                if tokio::time::Instant::now() >= deadline {
                    let error = json!({"message": "the bot did not finish in five minutes", "runId": run_id});
                    return vec![Ok(Event::default().event("RUN_ERROR").data(error.to_string()))];
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
                let entries = match hub.store.messages_after(&lane, start, 200) {
                    Ok(entries) => entries,
                    Err(error) => {
                        let data = json!({"message": format!("{error:#}"), "runId": run_id});
                        return vec![Ok(Event::default().event("RUN_ERROR").data(data.to_string()))];
                    }
                };
                let Some(entry) = entries
                    .iter()
                    .rev()
                    .find(|e| e.kind == "agent" && e.data["author"] == bot.as_str() && e.data["final"] == true)
                else {
                    continue;
                };
                let message_id = format!("msg-{}", entry.id);
                let text = entry.data["text"].as_str().unwrap_or_default().to_owned();
                let events: Vec<Result<Event, Infallible>> = vec![
                    Ok(Event::default()
                        .event("TEXT_MESSAGE_START")
                        .data(json!({"messageId": message_id, "role": "assistant"}).to_string())),
                    Ok(Event::default()
                        .event("TEXT_MESSAGE_CONTENT")
                        .data(json!({"messageId": message_id, "delta": text}).to_string())),
                    Ok(Event::default().event("TEXT_MESSAGE_END").data(json!({"messageId": message_id}).to_string())),
                    Ok(Event::default()
                        .event("RUN_FINISHED")
                        .data(json!({"threadId": thread_id, "runId": run_id}).to_string())),
                ];
                return events;
            }
        })
        .flat_map(|events| futures::stream::iter(events))
    };
    Ok(Sse::new(started.chain(finished)).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

#[derive(Deserialize)]
struct EventsQuery {
    since: Option<i64>,
    client: Option<String>,
}

async fn events(
    State(hub): State<Arc<Hub>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<EventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    if !authorized(&hub, &headers, peer) {
        return Err(unauthorized());
    }
    let events = events_stream(&hub, q.since.unwrap_or(0), q.client.as_deref(), &Caller::Local)
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")))?;
    let events = events.map(|v| Ok(Event::default().data(v.to_string())));
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

/// A scoped webhook credential authorizes only firing its own routine.
/// `POST /hooks/routines/:id`: a webhook delivery made on this computer (the public one
/// arrives through the relay, `remote::relay`). Same checks and answers either way.
async fn routine_hook(
    State(hub): State<Arc<Hub>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    use crate::routines::hooks::{MAX_BODY, Refused};
    if body.len() > MAX_BODY {
        return Err(ApiError(StatusCode::PAYLOAD_TOO_LARGE, "event too large".into()));
    }
    let headers =
        headers.iter().filter_map(|(k, v)| Some((k.as_str().to_owned(), v.to_str().ok()?.to_owned()))).collect();
    hub.routines.receive(&hub, &id, &headers, &body).map(Json).map_err(|e| {
        let status = match e {
            Refused::Unauthorized => StatusCode::UNAUTHORIZED,
            Refused::Paused | Refused::Busy => StatusCode::CONFLICT,
            Refused::Invalid(_) => StatusCode::BAD_REQUEST,
        };
        ApiError(status, e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hub on an in-memory store, with its identity in a temp folder the test removes.
    fn temp_hub() -> (Arc<Hub>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("codync-http-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let hub = Hub::new(
            crate::store::Store::open(std::path::Path::new(":memory:")).expect("store"),
            "test".into(),
            crate::remote::identity::Identity::load_or_create(&dir).expect("identity"),
            "test".into(),
            19222,
        );
        (hub, dir)
    }

    #[test]
    fn phone_streams_hold_pushes_until_the_last_one_closes() {
        let (hub, dir) = temp_hub();
        let desktop = events_stream(&hub, 0, Some("macos"), &Caller::Local).expect("desktop events");
        assert!(!hub.phone_connected());
        let android = events_stream(&hub, 0, Some("android"), &Caller::Local).expect("Android events");
        let ios = events_stream(&hub, 0, Some("ios"), &Caller::Local).expect("iOS events");
        assert!(hub.phone_connected());
        drop(android);
        assert!(hub.phone_connected(), "the iPhone remains connected");
        drop(ios);
        assert!(!hub.phone_connected(), "desktop streams do not hold phone alerts");
        drop(desktop);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn health_keeps_the_binary_and_busy_state_on_this_computer() {
        let (hub, dir) = temp_hub();
        let at = |addr: &str| ConnectInfo(addr.parse::<SocketAddr>().expect("address"));
        let lan = health(State(hub.clone()), at("192.168.1.20:50000"), Query(HealthQuery::default())).await.0;
        assert!(lan["computerId"].is_string());
        for key in ["binaryPath", "binaryHash", "busy", "updateRequested"] {
            assert!(lan.get(key).is_none(), "{key} reached the network");
        }
        let local = health(State(hub), at("[::ffff:127.0.0.1]:50000"), Query(HealthQuery::default())).await.0;
        assert_eq!(local["busy"], false);
        assert!(local["updateRequested"].is_null(), "nobody asked this computer to update");
        assert!(local.get("binaryPath").is_some());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_loopback_counts_as_this_computer() {
        let ip = |s: &str| s.parse::<IpAddr>().expect("test addresses are valid");
        assert!(is_loopback(ip("127.0.0.1")));
        assert!(is_loopback(ip("::1")));
        assert!(is_loopback(ip("::ffff:127.0.0.1")), "IPv4 loopback seen through `--bind ::`");
        assert!(!is_loopback(ip("192.168.1.20")));
        assert!(!is_loopback(ip("::ffff:192.168.1.20")));
        assert!(!is_loopback(ip("100.101.102.103")));
    }
}
