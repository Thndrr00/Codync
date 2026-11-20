//! Sunra media steps, ported from OpenBot nightly `src/backend/workflows/sunra-media.ts` and
//! `sunra-catalog.ts`. Sunra is a remote MCP server (`https://mcp.sunra.ai`) the person signs in to
//! in Extensions; the file is paid with that sign-in's credits and kept as a shared file of the bot,
//! like the files made by a bot's own agent.
//!
//! The host owns all of it: it reads the live model catalog and the credits through the installed
//! connector, uploads reference pictures, calls `create_image`/`create_video`, polls `get_job`,
//! downloads the result only from Sunra's CDN hosts through the egress allowlist, and snapshots it
//! with the ordinary file path. No token, address or description is written to logs or errors, and
//! every call has a timeout and a size cap. When the connector is missing or signed out, `available`
//! is false and a flow without Sunra behaves exactly as before.

use super::{Kind, MAX_PROMPT_BYTES};
use crate::egress::{self, Policy};
use crate::hub::Hub;
use crate::market::Connector;
use crate::store::Store;
use anyhow::{Context, Result, anyhow, bail, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// The provider name clients use for a Sunra model.
pub const PROVIDER: &str = "sunra";
/// The catalog id of the app that installs the connector; it is also the usual connector id.
pub const CONNECTOR_CATALOG: &str = "sunra";
/// The only host allowed to receive Sunra's sign-in and tool calls.
pub const MCP_HOST: &str = "mcp.sunra.ai";
/// The hosts Sunra serves results from. Every download hop must be one of these, over https.
pub const MEDIA_HOSTS: [&str; 3] = ["cdn.sunra.ai", "cdn.ponpon.ai", "assets.sunra.ai"];

const MEDIA_POLICY: Policy = Policy::Only(&MEDIA_HOSTS);
const MCP_POLICY: Policy = Policy::Only(&[MCP_HOST]);
const SIGN_IN: &str = "Sign in to Sunra in Extensions first.";
const PROJECT_NAME: &str = "OpenBot";
const UNSET_REFERENCES: usize = 4;
const MAX_REFERENCES: usize = 16;
const MAX_DURATION_CHOICES: usize = 5;
const REASON_LIMIT: usize = 300;
const MAX_REPLY: usize = 1_000_000;
const CALL_TIMEOUT: Duration = Duration::from_secs(120);
const CATALOG_TIMEOUT: Duration = Duration::from_secs(30);
const CREDITS_TIMEOUT: Duration = Duration::from_secs(20);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(120);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const IMAGE_DEADLINE: Duration = Duration::from_secs(300);
const VIDEO_DEADLINE: Duration = Duration::from_secs(1200);
const IMAGE_POLL: Duration = Duration::from_secs(3);
const VIDEO_POLL: Duration = Duration::from_secs(6);
const MAX_POLL_FAILURES: usize = 3;

/// The makers of the model families Sunra resells, from the start of a model id.
const MAKERS: [(&str, &str); 18] = [
    ("nano-banana", "Google"),
    ("veo-", "Google"),
    ("gemini-", "Google"),
    ("gpt-image", "OpenAI"),
    ("seedream", "ByteDance"),
    ("seedance", "ByteDance"),
    ("flux-", "Black Forest Labs"),
    ("kling-", "Kuaishou"),
    ("wan-", "Alibaba"),
    ("happy-horse", "Alibaba"),
    ("minimax-", "MiniMax"),
    ("grok-", "xAI"),
    ("midjourney", "Midjourney"),
    ("ideogram", "Ideogram"),
    ("krea", "Krea"),
    ("heygen", "HeyGen"),
    ("pika", "Pika"),
    ("veed", "VEED"),
];

// MARK: pure parsing

/// A model of Sunra's live list, with the settings the host can send it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    /// The id clients name: `sunra-<model id>`.
    pub id: String,
    pub provider: &'static str,
    /// The id Sunra names in its own tools.
    pub provider_model: String,
    pub name: String,
    pub maker: String,
    pub kinds: Vec<Kind>,
    pub aspects: Vec<String>,
    pub durations: Vec<u32>,
    pub resolutions: Vec<String>,
    pub transparent: bool,
    /// Which pictures and which background the model takes.
    pub plan: Plan,
}

/// How the host sends the pictures and the background of a step to one Sunra model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The most reference pictures: `image_urls` of an image, `reference_images` of a video.
    pub references: usize,
    /// Whether a video takes `start_image` and `end_image`.
    pub start: bool,
    pub end: bool,
    /// Whether an image takes a transparent `background`.
    pub transparent: bool,
}

/// The credits a sign-in can spend, from `get_balance`. `quota` is null for a personal account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credits {
    pub available: u64,
    pub quota: Option<u64>,
}

/// The decision of one `get_job` answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Failed(String),
    Done(String),
}

fn text(value: &Value) -> Option<&str> {
    value.as_str().filter(|value| !value.is_empty())
}

fn strings(value: &Value) -> Vec<&str> {
    value.as_array().into_iter().flatten().filter_map(Value::as_str).collect()
}

fn numbers(value: &Value) -> Vec<u32> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .filter(|number| *number > 0 && *number <= u64::from(u32::MAX))
        .map(|number| number as u32)
        .collect()
}

fn count(value: &Value) -> Option<usize> {
    value.as_u64().and_then(|number| usize::try_from(number).ok()).filter(|number| *number > 0)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-'))
}

fn maker(model: &str) -> &'static str {
    MAKERS.iter().find(|(prefix, _)| model.starts_with(prefix)).map_or("Sunra", |(_, maker)| *maker)
}

/// The aspect ratios a model makes, in the order the canvas shows them.
fn aspects(value: &Value) -> Vec<String> {
    let offered: BTreeSet<&str> = strings(value).into_iter().collect();
    super::ASPECTS.iter().filter(|aspect| offered.contains(**aspect)).map(|aspect| (*aspect).to_owned()).collect()
}

/// The lengths of a video to show: all of them when few, else the ends and every fifth second.
fn durations(value: &Value) -> Vec<u32> {
    let sorted: BTreeSet<u32> =
        numbers(value).into_iter().filter(|seconds| crate::media::catalog::valid_duration(*seconds)).collect();
    if sorted.len() <= MAX_DURATION_CHOICES {
        return sorted.into_iter().collect();
    }
    let first = sorted.iter().next().copied().unwrap_or_default();
    let last = sorted.iter().next_back().copied().unwrap_or_default();
    sorted.into_iter().filter(|seconds| *seconds == first || *seconds == last || *seconds % 5 == 0).collect()
}

fn resolutions(item: &Value) -> Vec<String> {
    let mut listed: Vec<String> = strings(&item["supported_resolutions"])
        .into_iter()
        .filter(|value| value.len() <= 16)
        .map(str::to_owned)
        .collect();
    let mut seen = BTreeSet::new();
    listed.retain(|value| seen.insert(value.clone()));
    if let Some(fallback) = text(&item["default_resolution"]).filter(|value| value.len() <= 16)
        && let Some(position) = listed.iter().position(|value| value == fallback)
    {
        let first = listed.remove(position);
        listed.insert(0, first);
    }
    listed.truncate(16);
    listed
}

/// The catalog entry of one `list_models` item, or none when the canvas cannot run it.
fn entry(item: &Value) -> Option<Model> {
    let provider_model = text(&item["model_id"])?;
    if !valid_id(provider_model) {
        return None;
    }
    let kind = match text(&item["type"])? {
        "image" => Kind::Image,
        "video" => Kind::Video,
        _ => return None,
    };
    let aspects = aspects(&item["supported_aspect_ratios"]);
    if aspects.is_empty() {
        return None;
    }
    let capabilities: BTreeSet<&str> = strings(&item["capabilities"]).into_iter().collect();
    let edits_video = capabilities.iter().any(|value| value.starts_with("video-to-video"));
    if kind == Kind::Video && edits_video && !capabilities.contains("image-to-video") {
        return None;
    }
    let durations = if kind == Kind::Video { durations(&item["supported_durations"]) } else { Vec::new() };
    if kind == Kind::Video && durations.is_empty() {
        return None;
    }
    let takes_references = match kind {
        Kind::Image => capabilities.contains("image-editing"),
        Kind::Video => capabilities.contains("reference-images"),
    };
    let references = if !takes_references {
        0
    } else {
        count(&item["max_ref_images"]).unwrap_or(UNSET_REFERENCES).min(MAX_REFERENCES)
    };
    let transparent = kind == Kind::Image && capabilities.contains("transparent-background");
    let name = text(&item["name"]).unwrap_or(provider_model).trim().chars().take(160).collect::<String>();
    Some(Model {
        id: format!("{PROVIDER}-{provider_model}"),
        provider: PROVIDER,
        provider_model: provider_model.to_owned(),
        name: if name.is_empty() { provider_model.to_owned() } else { name },
        maker: maker(provider_model).to_owned(),
        kinds: vec![kind],
        aspects,
        durations,
        resolutions: resolutions(item),
        transparent,
        plan: Plan {
            references,
            start: kind == Kind::Video && capabilities.contains("image-to-video"),
            end: kind == Kind::Video && capabilities.contains("end-frame"),
            transparent,
        },
    })
}

/// The runnable models of a `list_models` answer, in the order Sunra sent them, without duplicates.
/// A malformed item is skipped, never fatal to the other models.
pub fn parse_models(value: &Value) -> Vec<Model> {
    let list = value.as_array().or_else(|| value.get("models").and_then(Value::as_array));
    let Some(list) = list else { return Vec::new() };
    let mut seen = BTreeSet::new();
    list.iter().filter_map(entry).filter(|model| seen.insert(model.id.clone())).collect()
}

/// The plan used when Sunra's live list is not available: the pictures OpenBot sent before it.
pub fn fixed_plan(kind: Kind) -> Plan {
    Plan { references: MAX_REFERENCES, start: kind == Kind::Video, end: kind == Kind::Video, transparent: false }
}

/// A model as a client reads it: the same shape as the fixed media catalog, plus its id at Sunra.
pub fn model_json(model: &Model) -> Value {
    let aspects: Vec<&str> = model.aspects.iter().map(String::as_str).collect();
    let resolutions: Vec<&str> = model.resolutions.iter().map(String::as_str).collect();
    let mut value = json!({
        "id": model.id,
        "provider": model.provider,
        "backend": model.provider,
        "providerModel": model.provider_model,
        "name": model.name,
        "maker": model.maker,
        "subscription": "Sunra",
        "kinds": model.kinds.iter().map(Kind::as_str).collect::<Vec<_>>(),
        "aspects": aspects,
        "durations": model.durations,
        "resolutions": resolutions,
        "transparent": model.transparent,
        "voices": Value::Null,
        "thinkingLevels": [],
    });
    value["capabilities"] =
        super::catalog::capabilities_view(&aspects, &model.durations, &resolutions, model.transparent, false);
    value
}

/// The credits of one `get_balance` answer. A team member's quota is the member's own; the team
/// balance is not, and a team answer without the member's quota has no number to show.
pub fn parse_credits(value: &Value) -> Option<Credits> {
    let member = &value["memberQuota"]["effectiveQuota"];
    if member.is_object() {
        let available = credit_count(&member["remaining"])?;
        let quota = credit_count(&member["limit"]).filter(|quota| *quota > 0);
        return Some(Credits { available, quota });
    }
    if value.get("memberQuota").is_some() {
        return None;
    }
    let available =
        credit_count(&value["availableBalance"]).or_else(|| credit_count(&value["balance"]["availableBalance"]))?;
    Some(Credits { available, quota: None })
}

fn credit_count(value: &Value) -> Option<u64> {
    value.as_f64().filter(|number| number.is_finite() && *number >= 0.0).map(|number| number.floor() as u64)
}

/// Whether a result address is on Sunra's file hosts, over https, without a user name or password.
pub fn is_media_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else { return false };
    https_url(value)
        && url.host_str().is_some_and(|host| MEDIA_HOSTS.iter().any(|allowed| host.eq_ignore_ascii_case(allowed)))
}

fn https_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else { return false };
    url.scheme() == "https" && url.host_str().is_some() && url.username().is_empty() && url.password().is_none()
}

fn at_host(value: &str, host: &str) -> bool {
    https_url(value)
        && reqwest::Url::parse(value)
            .is_ok_and(|url| url.host_str().is_some_and(|name| name.eq_ignore_ascii_case(host)))
}

/// The result address of a finished job, if it is on Sunra's file hosts.
pub fn result_url(job: &Value, kind: Kind) -> Option<String> {
    let field = if kind == Kind::Video { "out_video" } else { "out_image" };
    std::iter::once(text(&job[field]))
        .chain(job["urls"].as_array().into_iter().flatten().map(Value::as_str))
        .flatten()
        .find(|url| is_media_url(url))
        .map(str::to_owned)
}

/// What one `get_job` answer says: the file, a failure, or keep polling.
pub fn job_status(job: &Value, kind: Kind) -> JobStatus {
    let status = text(&job["task_status"]).or_else(|| text(&job["status"])).unwrap_or_default().to_ascii_uppercase();
    if ["COMPLETED", "SUCCEEDED", "SUCCESS"].contains(&status.as_str()) {
        return result_url(job, kind)
            .map_or_else(|| JobStatus::Failed("Sunra made no file".to_owned()), JobStatus::Done);
    }
    if ["FAILED", "ERROR", "CANCELLED", "CANCELED"].contains(&status.as_str()) {
        let reason =
            text(&job["error_hint"]).or_else(|| text(&job["error_reason"])).unwrap_or("Sunra could not make the file");
        return JobStatus::Failed(reason.chars().take(REASON_LIMIT).collect());
    }
    JobStatus::Pending
}

// MARK: sign-in and MCP tools

/// The one connector installed for Sunra's MCP server, signed in and ready to call.
fn signed_in(store: &Store) -> Option<Connector> {
    crate::market::connectors(store).ok()?.into_iter().find(|connector| {
        connector.url.as_deref().is_some_and(|url| at_host(url, MCP_HOST))
            && connector.oauth.as_ref().is_some_and(|oauth| oauth.signed_in())
    })
}

/// Whether this computer can run Sunra right now. A flow without it keeps its old behaviour.
pub fn available(store: &Store) -> bool {
    signed_in(store).is_some()
}

#[derive(Debug)]
struct Unauthorized;

impl std::fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unauthorized")
    }
}

impl std::error::Error for Unauthorized {}

/// One streamable-HTTP MCP session on the installed connector. Its headers hold the sign-in token:
/// they are sent only to `mcp.sunra.ai`, and never logged.
struct Mcp<'a> {
    store: &'a Store,
    client: reqwest::Client,
    url: String,
    headers: reqwest::header::HeaderMap,
    session: Option<String>,
    next: u64,
    stale: bool,
}

impl<'a> Mcp<'a> {
    async fn connect(store: &'a Store) -> Result<Self> {
        Self::connect_with(store, false).await
    }

    async fn connect_with(store: &'a Store, stale: bool) -> Result<Self> {
        let id = signed_in(store).map(|connector| connector.id).ok_or_else(|| anyhow!(SIGN_IN))?;
        let target = crate::market::oauth::target(store, &id, stale).await?;
        let url = text(&target["url"]).ok_or_else(|| anyhow!("Sunra has no address"))?.to_owned();
        if !at_host(&url, MCP_HOST) {
            bail!("Sunra's address is not its MCP server");
        }
        let mut headers = reqwest::header::HeaderMap::new();
        for (name, value) in target["headers"].as_object().into_iter().flatten() {
            if let Some(value) = value.as_str() {
                let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| anyhow!("Sunra's sign-in headers are invalid"))?;
                let value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|_| anyhow!("Sunra's sign-in headers are invalid"))?;
                headers.insert(name, value);
            }
        }
        ensure!(headers.contains_key(reqwest::header::AUTHORIZATION), "{SIGN_IN}");
        let client = egress::client(MCP_POLICY, &url).await?;
        let mut mcp = Self { store, client, url, headers, session: None, next: 1, stale };
        mcp.rpc(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "codync", "version": env!("CARGO_PKG_VERSION")}}),
            Some(0),
            CALL_TIMEOUT,
        )
        .await?;
        mcp.rpc("notifications/initialized", json!({}), None, CALL_TIMEOUT).await?;
        Ok(mcp)
    }

    /// One JSON-RPC message. The answer may be JSON or an SSE stream; only the reply to this id is read.
    async fn rpc(&mut self, method: &str, params: Value, id: Option<u64>, timeout: Duration) -> Result<Value> {
        let mut body = json!({"jsonrpc": "2.0", "method": method, "params": params});
        if let Some(id) = id {
            body["id"] = json!(id);
        }
        let mut request = self
            .client
            .post(&self.url)
            .headers(self.headers.clone())
            .header("accept", "application/json, text/event-stream")
            .json(&body)
            .timeout(timeout);
        if let Some(session) = self.session.as_deref() {
            request = request.header("mcp-session-id", session);
        }
        let response = match tokio::time::timeout(timeout, request.send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => bail!("can't reach Sunra"),
            Err(_) => bail!("Sunra did not answer in time"),
        };
        if let Some(session) = response.headers().get("mcp-session-id").and_then(|value| value.to_str().ok())
            && !session.is_empty()
            && session.len() <= 512
        {
            self.session = Some(session.to_owned());
        }
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(Unauthorized.into());
        }
        if !response.status().is_success() {
            bail!("Sunra's server answered {}", response.status().as_u16());
        }
        if id.is_none() {
            return Ok(Value::Null);
        }
        let bytes = match tokio::time::timeout(timeout, egress::body(response, MAX_REPLY)).await {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(error)) => bail!("Sunra's answer could not be read: {error}"),
            Err(_) => bail!("Sunra did not answer in time"),
        };
        let answer = replies(&bytes)?
            .into_iter()
            .find(|reply| reply.get("id").and_then(Value::as_u64) == id && reply.get("method").is_none())
            .ok_or_else(|| anyhow!("Sunra's answer had no result"))?;
        if let Some(error) = answer.get("error") {
            if error.get("code").and_then(Value::as_i64) == Some(401) {
                return Err(Unauthorized.into());
            }
            bail!("Sunra refused the call");
        }
        Ok(answer.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Calls one tool and answers its structured value. A 401 refreshes the sign-in once.
    async fn call(&mut self, name: &str, args: Value, timeout: Duration) -> Result<Value> {
        self.next = self.next.wrapping_add(1);
        let id = self.next;
        let result = self
            .rpc("tools/call", json!({"name": name, "arguments": args.clone()}), Some(id), timeout)
            .await
            .and_then(|value| tool_value(&value));
        if result.as_ref().is_err_and(|error| error.downcast_ref::<Unauthorized>().is_some()) && !self.stale {
            let store = self.store;
            *self = Self::connect_with(store, true).await?;
            self.next = self.next.wrapping_add(1);
            return self
                .rpc("tools/call", json!({"name": name, "arguments": args}), Some(self.next), timeout)
                .await
                .and_then(|value| tool_value(&value));
        }
        result
    }
}

/// The replies in a JSON, batched JSON, or SSE body.
fn replies(bytes: &[u8]) -> Result<Vec<Value>> {
    let body = String::from_utf8_lossy(bytes);
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let value: Value = serde_json::from_str(trimmed)?;
        return Ok(match value {
            Value::Array(all) => all,
            one => vec![one],
        });
    }
    Ok(trimmed
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
        .collect())
}

/// The value of a tool answer: its structured content, or its first text block as JSON.
fn tool_value(result: &Value) -> Result<Value> {
    if result.get("isError") == Some(&Value::Bool(true)) {
        let reason = tool_reason(result).unwrap_or_else(|| "Sunra refused the call".to_owned());
        bail!("{reason}");
    }
    if let Some(structured) = result.get("structuredContent").and_then(Value::as_object) {
        return Ok(Value::Object(structured.clone()));
    }
    for block in result.get("content").and_then(Value::as_array).into_iter().flatten() {
        if block.get("type").and_then(Value::as_str) == Some("text")
            && let Some(text) = block.get("text").and_then(Value::as_str)
            && let Ok(value) = serde_json::from_str::<Value>(text)
            && value.is_object()
        {
            return Ok(value);
        }
    }
    bail!("Sunra sent no usable answer")
}

fn tool_reason(result: &Value) -> Option<String> {
    let structured = result.get("structuredContent");
    let parsed = structured
        .and_then(|value| value.get("content"))
        .and_then(Value::as_array)
        .and_then(|blocks| blocks.first())
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    let record = structured.and_then(|value| if value.is_object() { Some(value) } else { None }).or(parsed.as_ref());
    let reason = record
        .and_then(|value| {
            text(&value["error_hint"]).or_else(|| text(&value["message"])).or_else(|| text(&value["error"]))
        })
        .or_else(|| {
            result
                .get("content")
                .and_then(Value::as_array)
                .and_then(|blocks| blocks.first())
                .and_then(|block| block.get("text"))
                .and_then(Value::as_str)
        })?;
    Some(reason.chars().take(REASON_LIMIT).collect())
}

// MARK: reads

/// The image and video models Sunra offers right now, or an empty list when it does not answer.
pub async fn list_models(store: &Store) -> Result<Vec<Model>> {
    let mut mcp = Mcp::connect(store).await?;
    let answer = mcp.call("list_models", json!({}), CATALOG_TIMEOUT).await?;
    Ok(parse_models(&answer))
}

/// The credits the sign-in can spend now, or none when Sunra does not tell them.
pub async fn credits(store: &Store) -> Result<Option<Credits>> {
    let mut mcp = Mcp::connect(store).await?;
    let answer = mcp.call("get_balance", json!({}), CREDITS_TIMEOUT).await?;
    Ok(parse_credits(&answer))
}

// MARK: a step

/// One input picture of a step, as a local path and its mime type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub path: String,
    pub mime_type: String,
}

/// The pictures handed to a step: a first and last frame, and reference pictures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frames {
    pub first: Option<Image>,
    pub last: Option<Image>,
    pub references: Vec<Image>,
}

/// The request of a Sunra flow step. The model id is the Sunra id with or without the `sunra-`
/// prefix; the other settings are the ones saved on the media node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub model: String,
    pub kind: Kind,
    pub prompt: String,
    pub aspect: String,
    pub duration: u32,
    pub resolution: Option<String>,
    pub transparent: bool,
    pub frames: Frames,
}

/// Whether a media node names a Sunra model.
pub fn is_model(id: &str) -> bool {
    id.strip_prefix("sunra-").is_some_and(valid_id)
}

fn normalize_model(id: &str) -> Option<&str> {
    if valid_id(id) { Some(id) } else { id.strip_prefix("sunra-").filter(|model| valid_id(model)) }
}

impl Request {
    /// Builds a request from a stored media node, before any network call.
    pub fn from_node(node: &crate::store::FlowMedia) -> Result<Self> {
        let kind = match node.kind.as_str() {
            "image" => Kind::Image,
            "video" => Kind::Video,
            other => bail!("unknown media kind `{other}`"),
        };
        ensure!(is_model(&node.model), "unknown media model `{}`", node.model);
        ensure!(!node.prompt.trim().is_empty(), "the step has no description yet");
        ensure!(node.prompt.len() <= MAX_PROMPT_BYTES, "the description is too long");
        ensure!(
            super::ASPECTS.contains(&node.aspect.as_str()),
            "{} does not make the frame {}",
            node.model,
            node.aspect
        );
        if kind == Kind::Video {
            ensure!(
                crate::media::catalog::valid_duration(node.duration),
                "{} does not make {} seconds",
                node.model,
                node.duration
            );
        }
        Ok(Self {
            model: node.model.clone(),
            kind,
            prompt: node.prompt.trim().to_owned(),
            aspect: node.aspect.clone(),
            duration: node.duration,
            resolution: (kind == Kind::Video).then(|| node.resolution.clone()).flatten(),
            transparent: node.transparent && kind == Kind::Image,
            frames: Frames::default(),
        })
    }
}

/// Makes the file of one Sunra step and keeps it as a shared file of the owner bot.
/// `timeout` is the routine step's own limit; the Sunra deadline is applied as well.
pub async fn generate(
    hub: &Arc<Hub>,
    owner: &crate::store::BotConfig,
    request: &Request,
    timeout: Duration,
) -> Result<Vec<SharedFile>> {
    if !available(&hub.store) {
        bail!("{SIGN_IN}");
    }
    let model = normalize_model(&request.model).ok_or_else(|| anyhow!("unknown media model `{}`", request.model))?;
    ensure!(!request.prompt.trim().is_empty(), "the step has no description yet");
    ensure!(request.prompt.len() <= MAX_PROMPT_BYTES, "the description is too long");
    let url = make(&hub.store, request, model, timeout.min(deadline(request.kind))).await?;
    keep(owner, request.kind, &url).await.map(|file| vec![file])
}

fn deadline(kind: Kind) -> Duration {
    if kind == Kind::Video { VIDEO_DEADLINE } else { IMAGE_DEADLINE }
}

async fn make(store: &Store, request: &Request, model: &str, timeout: Duration) -> Result<String> {
    let work = async {
        let mut mcp = Mcp::connect(store).await?;
        let project = project(&mut mcp).await?;
        // The live list says what this model takes; the fixed plan is the fallback when Sunra
        // can't answer. A failed read must not lose a job the person asked to run.
        let plan = match mcp.call("list_models", json!({}), CATALOG_TIMEOUT).await {
            Ok(answer) => parse_models(&answer)
                .into_iter()
                .find(|listed| listed.provider_model == model)
                .map_or_else(|| fixed_plan(request.kind), |listed| listed.plan),
            Err(_) => fixed_plan(request.kind),
        };
        let images = inputs(request, &plan);
        let mut uploaded = BTreeMap::new();
        for (index, image) in images.iter().enumerate() {
            if uploaded.contains_key(&image.path) {
                continue;
            }
            let url = upload(&mut mcp, image, index).await?;
            uploaded.insert(image.path.clone(), url);
        }
        let at = |image: Option<&Image>| image.and_then(|image| uploaded.get(&image.path)).cloned();
        let references: Vec<String> = request
            .frames
            .references
            .iter()
            .filter_map(|image| uploaded.get(&image.path).cloned())
            .take(plan.references)
            .collect();
        let mut args = json!({
            "project_id": project,
            "prompt": request.prompt,
            "model": model,
            "aspect_ratio": request.aspect,
            "auto_confirm": true,
            "request_id": format!("codync-{}", uuid::Uuid::new_v4()),
        });
        if let Some(resolution) = text_as_value(&request.resolution) {
            args["resolution"] = json!(resolution);
        }
        if request.kind == Kind::Video {
            args["duration"] = json!(request.duration);
        } else {
            args["count"] = json!(1);
            if plan.transparent && request.transparent {
                args["background"] = json!("transparent");
            }
        }
        match request.kind {
            Kind::Image => {
                if !references.is_empty() {
                    args["image_urls"] = json!(references);
                }
            }
            Kind::Video => {
                if let Some(start) = at(request.frames.first.as_ref()) {
                    args["start_image"] = json!(start);
                }
                if let Some(end) = at(request.frames.last.as_ref()).filter(|_| request.frames.first.is_some()) {
                    args["end_image"] = json!(end);
                }
                if !references.is_empty() {
                    args["reference_images"] = json!(references);
                }
            }
        }
        let created = mcp
            .call(if request.kind == Kind::Video { "create_video" } else { "create_image" }, args, CALL_TIMEOUT)
            .await?;
        let task = text(&created["task_id"])
            .or_else(|| created["task_ids"].as_array().and_then(|list| list.first()).and_then(Value::as_str))
            .ok_or_else(|| anyhow!("Sunra started no job"))?
            .to_owned();
        wait(&mut mcp, &task, request.kind).await
    };
    match tokio::time::timeout(timeout, work).await {
        Ok(result) => result,
        Err(_) => bail!("making the Sunra {} took too long", request.kind.as_str()),
    }
}

fn text_as_value(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

/// Only the pictures the plan takes: a last frame alone is refused by most models.
fn inputs(request: &Request, plan: &Plan) -> Vec<&Image> {
    let first = request.frames.first.as_ref().filter(|_| plan.start);
    let last = request.frames.last.as_ref().filter(|_| plan.end && first.is_some());
    first.into_iter().chain(last).chain(request.frames.references.iter().take(plan.references)).collect()
}

async fn project(mcp: &mut Mcp<'_>) -> Result<String> {
    let workspaces = mcp.call("list_workspaces", json!({}), CALL_TIMEOUT).await?;
    let team = text(&workspaces["selectedWorkspaceId"]).map(str::to_owned);
    let listed = mcp.call("list_projects", json!({"limit": 100}), CALL_TIMEOUT).await?;
    let found = listed["projects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item.is_object())
        .find(|item| text(&item["name"]) == Some(PROJECT_NAME))
        .and_then(|item| text(&item["project_id"]).or_else(|| text(&item["project"]["project_id"])));
    if let Some(id) = found {
        return Ok(id.to_owned());
    }
    let mut args = json!({"name": PROJECT_NAME});
    if let Some(team) = team {
        let safe: String =
            team.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).take(120).collect();
        args["workspace_id"] = json!(team);
        args["request_id"] = json!(format!("codync-project-{safe}"));
    }
    let made = mcp.call("create_project", args, CALL_TIMEOUT).await?;
    text(&made["project_id"])
        .or_else(|| text(&made["project"]["project_id"]))
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Sunra made no project"))
}

async fn upload(mcp: &mut Mcp<'_>, image: &Image, index: usize) -> Result<String> {
    ensure!(image.mime_type.starts_with("image/"), "a reference is not a picture");
    let answer = mcp
        .call(
            "get_upload_url",
            json!({"content_type": image.mime_type.as_str(), "filename": format!("codync-input-{}.{}", index + 1, picture_extension(&image.mime_type))}),
            CALL_TIMEOUT,
        )
        .await?;
    let upload_url = text(&answer["upload_url"]).ok_or_else(|| anyhow!("Sunra gave no upload address"))?;
    let asset_url = text(&answer["asset_url"]).ok_or_else(|| anyhow!("Sunra gave no asset address"))?;
    if !https_url(upload_url) || !https_url(asset_url) {
        bail!("Sunra gave an unsafe upload address");
    }
    let method = match text(&answer["method"]).unwrap_or("PUT") {
        "PUT" => reqwest::Method::PUT,
        "POST" => reqwest::Method::POST,
        _ => bail!("Sunra asked for an unsafe upload method"),
    };
    let size = tokio::fs::metadata(&image.path).await?.len();
    ensure!(size > 0 && size <= crate::chat::uploads::MAX_FILE, "the reference picture is too large");
    let bytes = tokio::fs::read(&image.path).await?;
    let client = egress::client(Policy::Public, upload_url).await?;
    let mut request = client
        .request(method, upload_url)
        .header("content-type", image.mime_type.as_str())
        .body(bytes)
        .timeout(UPLOAD_TIMEOUT);
    if let Some(headers) = answer["headers"].as_object() {
        for (name, value) in headers {
            if let Some(value) = value.as_str() {
                let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| anyhow!("Sunra's upload headers are invalid"))?;
                let value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|_| anyhow!("Sunra's upload headers are invalid"))?;
                request = request.header(name, value);
            }
        }
    }
    let response = tokio::time::timeout(UPLOAD_TIMEOUT, request.send())
        .await
        .map_err(|_| anyhow!("the upload took too long"))?
        .map_err(|_| anyhow!("Sunra did not accept the reference picture"))?;
    ensure!(response.status().is_success(), "Sunra did not accept the reference picture");
    Ok(asset_url.to_owned())
}

fn picture_extension(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    }
}

async fn wait(mcp: &mut Mcp<'_>, task: &str, kind: Kind) -> Result<String> {
    let mut failures = 0;
    loop {
        let answer = mcp.call("get_job", json!({"task_id": task}), CALL_TIMEOUT).await;
        match answer {
            Ok(job) => {
                failures = 0;
                match job_status(&job, kind) {
                    JobStatus::Done(url) => return Ok(url),
                    JobStatus::Failed(reason) => bail!("{reason}"),
                    JobStatus::Pending => {}
                }
            }
            Err(error) => {
                failures += 1;
                if failures >= MAX_POLL_FAILURES {
                    return Err(error);
                }
            }
        }
        tokio::time::sleep(if kind == Kind::Video { VIDEO_POLL } else { IMAGE_POLL }).await;
    }
}

async fn keep(owner: &crate::store::BotConfig, kind: Kind, url: &str) -> Result<SharedFile> {
    let response = egress::get_following(MEDIA_POLICY, url, DOWNLOAD_TIMEOUT).await?;
    let header = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let bytes = egress::body(response, crate::chat::uploads::MAX_FILE as usize).await?;
    ensure!(!bytes.is_empty(), "Sunra sent an empty file");
    let ext = match header.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        "video/mp4" => "mp4",
        _ => {
            if kind == Kind::Video {
                "mp4"
            } else {
                "png"
            }
        }
    };
    let folder = crate::service::data_dir().join("media/sunra");
    tokio::fs::create_dir_all(&folder).await?;
    let temp = folder.join(format!("{}-{kind}.{ext}", uuid::Uuid::new_v4()));
    tokio::fs::write(&temp, &bytes).await?;
    let root = crate::chat::files::root(&owner.id);
    let cwd = PathBuf::from(&owner.cwd);
    let source = temp.clone();
    let display = format!("sunra-{}.{ext}", kind.as_str());
    let copied = tokio::task::spawn_blocking(move || {
        crate::chat::files::snapshot(
            &root,
            &cwd,
            source.to_string_lossy().as_ref(),
            Some(&display),
            &AtomicBool::new(false),
        )
    })
    .await;
    let _ = tokio::fs::remove_file(&temp).await;
    let mut snapshot = copied.context("the file copy ended early")??;
    snapshot.commit();
    Ok(snapshot.meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> Value {
        json!({
            "model_id": "nano-banana-pro",
            "name": " Nano Banana Pro ",
            "type": "image",
            "supported_aspect_ratios": ["9:16", "1:1", "7:9"],
            "capabilities": ["image-editing", "transparent-background"],
            "max_ref_images": 6,
            "supported_resolutions": ["2K", "1K", "A-VERY-LONG-RESOLUTION-NAME-THAT-IS-NOT-SHOWN"]
        })
    }

    #[test]
    fn catalog_parser_keeps_runnable_models_and_skips_malformed_ones() {
        let video = json!({
            "model_id": "veo-3", "type": "video", "supported_aspect_ratios": ["16:9"],
            "capabilities": ["image-to-video", "end-frame", "reference-images"],
            "supported_durations": [4, 5, 6, 8, 10, 15, 20, 25, 30], "default_resolution": "720p",
            "supported_resolutions": ["480p", "720p"], "max_ref_images": 2
        });
        let answer = json!({"models": [
            item(),
            video,
            json!({"model_id": "BAD ID", "type": "image", "supported_aspect_ratios": ["1:1"]}),
            json!({"model_id": "no-aspect", "type": "image", "capabilities": []}),
            json!({"model_id": "edit-only", "type": "video", "supported_aspect_ratios": ["1:1"],
                   "capabilities": ["video-to-video"], "supported_durations": [6]}),
            json!({"model_id": "no-length", "type": "video", "supported_aspect_ratios": ["1:1"],
                   "capabilities": ["image-to-video"], "supported_durations": []}),
            item()
        ]});
        let models = parse_models(&answer);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "sunra-nano-banana-pro");
        assert_eq!(models[0].provider, "sunra");
        assert_eq!(models[0].provider_model, "nano-banana-pro");
        assert_eq!(models[0].maker, "Google");
        assert_eq!(models[0].name, "Nano Banana Pro");
        assert_eq!(models[0].kinds, [Kind::Image]);
        assert_eq!(models[0].aspects, ["1:1", "9:16"]);
        assert!(models[0].transparent);
        assert_eq!(models[0].plan, Plan { references: 6, start: false, end: false, transparent: true });
        assert_eq!(models[1].id, "sunra-veo-3");
        assert_eq!(models[1].kinds, [Kind::Video]);
        assert_eq!(models[1].durations, [4, 5, 10, 15, 20, 25, 30]);
        assert_eq!(models[1].resolutions[0], "720p");
        assert!(models[1].resolutions.iter().all(|value| value.len() <= 16));
        assert_eq!(models[1].plan, Plan { references: 2, start: true, end: true, transparent: false });
        let listed = model_json(&models[0]);
        assert_eq!(listed["capabilities"]["aspects"], json!(["1:1", "9:16"]));
        assert_eq!(listed["thinkingLevels"], json!([]));
        assert!(parse_models(&json!({"models": 3})).is_empty());
    }

    #[test]
    fn downloads_accept_only_sunra_cdn_over_https() {
        for url in ["https://cdn.sunra.ai/a.png", "https://cdn.ponpon.ai/a.mp4", "https://assets.sunra.ai/a.png"] {
            assert!(is_media_url(url), "{url}");
        }
        for url in [
            "http://cdn.sunra.ai/a.png",
            "https://user:pass@cdn.sunra.ai/a.png",
            "https://cdn.sunra.ai.evil.test/a.png",
            "https://evil.test/a.png",
            "not a url",
        ] {
            assert!(!is_media_url(url), "{url}");
        }
        assert_eq!(
            result_url(&json!({"urls": ["https://evil.test/a.png", "https://cdn.sunra.ai/a.png"]}), Kind::Image),
            Some("https://cdn.sunra.ai/a.png".to_owned())
        );
    }

    #[test]
    fn credits_come_from_the_member_quota_or_a_personal_balance() {
        let team = json!({"memberQuota": {"effectiveQuota": {"remaining": 120.9, "limit": 500}}});
        assert_eq!(parse_credits(&team), Some(Credits { available: 120, quota: Some(500) }));
        let personal = json!({"availableBalance": 2140});
        assert_eq!(parse_credits(&personal), Some(Credits { available: 2140, quota: None }));
        let nested = json!({"balance": {"availableBalance": 12}});
        assert_eq!(parse_credits(&nested), Some(Credits { available: 12, quota: None }));
        assert_eq!(parse_credits(&json!({"memberQuota": {"teamBalance": 900}})), None);
        assert_eq!(parse_credits(&json!({"availableBalance": -1})), None);
    }

    #[test]
    fn job_status_chooses_the_result_or_the_failure() {
        let running = json!({"task_status": "IN_QUEUE"});
        assert_eq!(job_status(&running, Kind::Image), JobStatus::Pending);
        let done = json!({"status": "SUCCEEDED", "out_image": "https://cdn.sunra.ai/a.png"});
        assert_eq!(job_status(&done, Kind::Image), JobStatus::Done("https://cdn.sunra.ai/a.png".to_owned()));
        let wrong = json!({"status": "COMPLETED", "out_image": "https://evil.test/a.png"});
        assert_eq!(job_status(&wrong, Kind::Image), JobStatus::Failed("Sunra made no file".to_owned()));
        let failed = json!({"task_status": "FAILED", "error_hint": "Insufficient balance"});
        assert_eq!(job_status(&failed, Kind::Image), JobStatus::Failed("Insufficient balance".to_owned()));
    }

    #[test]
    fn sunra_model_ids_are_recognised_with_and_without_the_prefix() {
        assert!(is_model("sunra-nano-banana-pro"));
        assert!(normalize_model("nano-banana-pro") == Some("nano-banana-pro"));
        assert!(normalize_model("sunra-nano-banana-pro") == Some("nano-banana-pro"));
        assert!(!is_model("gpt-image"));
        assert!(normalize_model("sunra-BAD ID").is_none());
    }
}
