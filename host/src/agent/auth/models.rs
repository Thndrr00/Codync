//! The models an agent offers, read from a session it opens for that alone.

use super::{Started, probe_dir, start_with_env, text};
use crate::LockExt;
use crate::agent::acp::Incoming;
use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

/// How often an agent whose catalog can grow while it runs is read again (OpenBot nightly `triage-12308`).
const RELIST_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// A short first read for an account added after the first; a timeout retries with the full budget
/// (OpenBot nightly `triage-12627`).
const SHORT_LIST_TIMEOUT: Duration = Duration::from_secs(5);
/// The full model-list read budget.
const LIST_TIMEOUT: Duration = Duration::from_mins(1);
/// Cursor sometimes answers its own model method with an empty list on the first ask (OpenBot
/// triage-16062). One quick retry, then a plain hint rather than an empty picker.
const CURSOR_EMPTY_HINT: &str = "Cursor returned no models. Check it in Settings, Providers.";
/// The group hidden and superseded ChatGPT models are collected under.
const OLDER_MODELS: &str = "Older models";

/// Model lists being fetched, by agent, and the last list that came back. A settings screen asks for one
/// from several places at once (the model picker, Approvals), and each would otherwise start the agent. An
/// agent whose list can change while it stays running keeps its last list and is read again every
/// [`RELIST_INTERVAL`]; a quiet re-read that fails or comes back empty keeps the older list.
#[derive(Default)]
pub struct InFlight {
    flights: Mutex<HashMap<String, Slot>>,
    cached: Mutex<HashMap<String, (Value, Instant)>>,
}

/// One fetch's answer, for every ask that came while it ran (a failure as its text).
type Slot = Arc<OnceCell<Result<Value, String>>>;

impl InFlight {
    async fn get(&self, agent: &str, fetch: impl Future<Output = Result<Value>>) -> Result<Value> {
        let slot = self.flights.locked().entry(agent.to_owned()).or_default().clone();
        let answer = slot
            .get_or_init(|| async {
                let answer = fetch.await.map_err(|e| format!("{e:#}"));
                self.flights.locked().remove(agent);
                answer
            })
            .await;
        answer.clone().map_err(anyhow::Error::msg)
    }

    /// A list still fresh enough to use without waking the agent again.
    pub(crate) fn fresh(&self, key: &str, now: Instant) -> Option<Value> {
        let cached = self.cached.lock().ok()?;
        let (value, at) = cached.get(key)?;
        (!should_relist(Some(*at), now)).then(|| value.clone())
    }

    /// The last list for `key`, however old: a failed re-read keeps it.
    pub(crate) fn stale(&self, key: &str) -> Option<Value> {
        self.cached.lock().ok()?.get(key).map(|(value, _)| value.clone())
    }

    pub(crate) fn remember(&self, key: &str, value: Value, now: Instant) {
        if let Ok(mut cached) = self.cached.lock() {
            cached.insert(key.to_owned(), (value, now));
        }
    }

    /// The catalogs of one backend's isolated accounts, for account-only backup choices.
    fn account_catalogs(&self, backend: &str) -> Vec<(String, Value)> {
        let Ok(cached) = self.cached.lock() else { return Vec::new() };
        let prefix = format!("{backend}:");
        cached
            .iter()
            .filter_map(|(key, (value, _))| key.strip_prefix(&prefix).map(|id| (id.to_owned(), value.clone())))
            .collect()
    }
}

/// Agents whose list Google (or the provider) can extend while the process keeps running.
pub(crate) fn relists_while_running(backend: &str) -> bool {
    matches!(backend, "antigravity" | "gemini")
}

/// Whether a list last read at `last` is due for another read (OpenBot nightly `triage-12308`).
pub(crate) fn should_relist(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|at| now.saturating_duration_since(at) >= RELIST_INTERVAL)
}

/// A timed-out first read of a secondary account gets one full-length retry (OpenBot nightly `triage-12627`).
pub(crate) fn retry_slow_account(is_secondary: bool, timed_out: bool) -> bool {
    is_secondary && timed_out
}

/// Grok saves every `session/new` under its own home folder and has no `session/delete`
/// (OpenBot nightly `triage-15353`). A model check's probe therefore has to remove the
/// session it just created; a bot's own chat session is started elsewhere and is untouched.
pub(crate) fn model_check_session(backend: &str) -> bool {
    backend == "grok"
}

/// `encodeURIComponent`'s unreserved set, which is how Grok encodes a session's cwd.
const GROK_PATH_ENCODE: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// The one session file a Grok probe left, when `session_id` is one Grok uses. Nothing is
/// removed for another id or an empty cwd: a wrong path must never be deleted.
pub(crate) fn grok_probe_session_file(grok_home: &Path, cwd: &str, session_id: &str) -> Option<PathBuf> {
    if cwd.is_empty()
        || session_id.is_empty()
        || !session_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let encoded = percent_encoding::utf8_percent_encode(cwd, GROK_PATH_ENCODE).to_string();
    Some(grok_home.join("sessions").join(encoded).join(session_id))
}

fn timed_out(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.is::<tokio::time::error::Elapsed>())
}

/// A re-read with no models is quiet: keep the older list (OpenBot nightly `triage-12308`).
pub(crate) fn has_models(catalog: &Value) -> bool {
    catalog["models"].as_array().is_some_and(|models| !models.is_empty())
}

/// Cursor's own method can answer with an empty list once. Only that case gets one retry; a
/// failed request (`Null`) and every other agent keep their current behavior (OpenBot 16062).
pub(crate) fn cursor_retry_empty(backend: &str, options: &Value, attempt: usize) -> bool {
    backend == "cursor" && attempt == 0 && options["models"].as_array().is_some_and(|models| models.is_empty())
}

/// The plain catalog note when Cursor still has no models after the one retry. No error text
/// from the agent reaches the picker.
pub(crate) fn empty_catalog_hint(catalog: &Value) -> Option<&'static str> {
    (!has_models(catalog)).then_some(CURSOR_EMPTY_HINT)
}

/// A name the agent itself marks legacy, deprecated or superseded belongs under "Older models".
fn superseded_model(model: &Value) -> bool {
    let text =
        format!("{} {}", model["name"].as_str().unwrap_or_default(), model["description"].as_str().unwrap_or_default())
            .to_ascii_lowercase();
    text.contains("legacy") || text.contains("deprecated") || text.contains("superseded")
}

/// Hidden and superseded ChatGPT models stay in the picker, but at the end under one group
/// (OpenBot 15268). Pure: the store is read by the caller, so both sides are testable.
pub(crate) fn group_older_models(mut catalog: Value, hidden: &[String]) -> Value {
    if let Some(models) = catalog["models"].as_array_mut() {
        for model in models.iter_mut() {
            let id = model["id"].as_str().unwrap_or_default();
            if hidden.iter().any(|turned_off| turned_off == id) || superseded_model(model) {
                model["group"] = json!(OLDER_MODELS);
            }
        }
        models.sort_by_key(|model| model["group"] == OLDER_MODELS);
    }
    catalog
}

/// The model to show while a provider's list loads slowly (OpenBot triage-16212). A list that
/// names a current model wins; one that came back empty or before the app applied the choice keeps
/// the model the person picked, so a restart does not silently move the bot to the provider default.
pub(crate) fn stable_model<'a>(cached: Option<&'a str>, fresh: Option<&'a str>) -> Option<&'a str> {
    fresh.or(cached)
}

/// [`models`], shared with the asks for the same agent (and account) meanwhile.
pub async fn shared_models(hub: &Hub, backend: &str, account: Option<&str>) -> Result<Value> {
    let key = account.map_or_else(|| backend.to_owned(), |id| format!("{backend}:{id}"));
    let relists = relists_while_running(backend);
    if relists && let Some(fresh) = hub.model_lists.fresh(&key, Instant::now()) {
        return Ok(fresh);
    }
    let result = hub.model_lists.get(&key, models(&hub.store, backend, account)).await;
    let mut value = match result {
        Ok(value) => value,
        Err(error) if relists => {
            tracing::warn!(backend, error = format!("{error:#}"), "model list re-read failed; keeping the old list");
            return hub.model_lists.stale(&key).ok_or(error);
        }
        Err(error) => return Err(error),
    };
    // A slow list must not put a restarted bot back on the provider's default (16212).
    if let Some(cached) = hub.model_lists.stale(&key)
        && let Some(chosen) = stable_model(cached["currentModelId"].as_str(), value["currentModelId"].as_str())
    {
        value["currentModelId"] = json!(chosen);
    }
    if backend == "codex" {
        let turned_off: Vec<String> = hidden_map(&hub.store).remove(backend).unwrap_or_default();
        value = group_older_models(value, &turned_off);
    }
    if account.is_none() {
        let others = hub.model_lists.account_catalogs(backend);
        value["backupModelChoices"] = json!(backup_model_choices(&value, &others));
    }
    if relists && has_models(&value) {
        hub.model_lists.remember(&key, value.clone(), Instant::now());
    }
    Ok(value)
}

/// Discover the agent's advertised models in an isolated, prompt-free session.
/// A secondary account gets a short first read and one full-length retry when that times out.
pub async fn models(store: &Store, backend: &str, account: Option<&str>) -> Result<Value> {
    let first =
        discover(store, backend, account, if account.is_some() { SHORT_LIST_TIMEOUT } else { LIST_TIMEOUT }).await;
    match first {
        Err(error) if retry_slow_account(account.is_some(), timed_out(&error)) => {
            discover(store, backend, account, LIST_TIMEOUT).await
        }
        other => other,
    }
}

async fn discover(store: &Store, backend: &str, account: Option<&str>, budget: Duration) -> Result<Value> {
    let extra_env = account.map(|id| crate::agent::account::env(id, backend)).transpose()?.unwrap_or_default();
    let Started { acp, mut inbox, .. } = start_with_env(store, backend, &extra_env).await?;
    // Keep the exact folder the probe was started in and the home its environment names;
    // cleanup then touches no file system path we did not just use (OpenBot nightly
    // `triage-15353`, second pass).
    let cleanup = if model_check_session(backend) {
        extra_env
            .iter()
            .find(|(name, _)| name == "GROK_HOME")
            .map(|(_, value)| PathBuf::from(value))
            .or_else(|| dirs::home_dir().map(|home| home.join(".grok")))
            .map(|home| (home, probe_dir().to_string_lossy().into_owned()))
    } else {
        None
    };
    let result = tokio::time::timeout(budget, async {
        let request = acp.request("session/new", json!({"cwd": probe_dir(), "mcpServers": []}));
        tokio::pin!(request);
        let session = loop {
            tokio::select! {
                result = &mut request => break result?,
                incoming = inbox.recv() => match incoming {
                    Some(Incoming::Request { id, .. }) => {
                        acp.respond_error(id, -32601, "not supported during model discovery").await?;
                    }
                    Some(Incoming::Notification { .. }) => {}
                    Some(Incoming::Closed { .. }) | None => bail!("agent closed during model discovery"),
                }
            }
        };
        let session_id = session["sessionId"].as_str().map(str::to_owned);
        // Cursor lists each model's options (effort, fast…) only through its own method. An empty
        // first answer gets one retry; a second empty answer becomes a plain Providers hint.
        let mut options = Value::Null;
        if backend == "cursor" {
            for attempt in 0..=1 {
                options = acp.request("cursor/list_available_models", json!({})).await.unwrap_or_default();
                if !cursor_retry_empty(backend, &options, attempt) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
        let mut catalog = model_catalog(&session, &options);
        if backend == "cursor"
            && let Some(hint) = empty_catalog_hint(&catalog)
        {
            catalog["modelsError"] = json!(hint);
        }
        catalog["modes"] =
            crate::agent::modes::Modes::from_session(&session, backend).map_or(json!([]), |m| m.catalog());
        if backend != "cursor" {
            discover_options(&acp, &session, &mut catalog).await?;
        }
        Ok((catalog, session_id))
    })
    .await;
    acp.kill().await;
    let (catalog, session_id) = result.context("the agent's model list timed out")?;
    if let (Some((home, cwd)), Some(session_id)) = (cleanup, session_id)
        && let Some(file) = grok_probe_session_file(&home, &cwd, &session_id)
        && let Err(error) = tokio::fs::remove_file(&file).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %file.display(), %error, "couldn't remove the Grok probe session");
    }
    Ok(catalog)
}

/// Probe model-dependent settings in the disposable discovery session, never in a bot's session.
async fn discover_options(acp: &crate::agent::acp::Acp, session: &Value, catalog: &mut Value) -> Result<()> {
    let Some(sid) = session["sessionId"].as_str() else { return Ok(()) };
    let Some(config) = model_config(session).and_then(|c| c["id"].as_str()) else { return Ok(()) };
    let current = catalog["currentModelId"].clone();
    let Some(models) = catalog["models"].as_array_mut() else { return Ok(()) };
    for model in models {
        let response = if model["id"] == current {
            session.clone()
        } else {
            match acp
                .request(
                    "session/set_config_option",
                    json!({"sessionId": sid,
                "configId": config, "value": model["id"]}),
                )
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    model["options"] = json!([]);
                    model["optionsError"] = format!("Couldn't load model settings: {error}").into();
                    continue;
                }
            }
        };
        model["options"] = json!(reasoning_options(&response));
    }
    Ok(())
}

/// Only reasoning belongs here: permission/mode and Fast mode have their own controls.
fn reasoning_options(session: &Value) -> Vec<Value> {
    session["configOptions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|o| {
            o["category"] == "thought_level"
                || (o["category"] != "mode"
                    && matches!(
                        o["id"].as_str(),
                        Some("reasoning_effort" | "reasoning-effort" | "thought_level" | "effort")
                    ))
        })
        .filter_map(parse_select_option)
        .map(|mut option| {
            option["kind"] = "reasoning".into();
            // Inherit/reset has one dedicated control in every client, not a second effort level.
            if let Some(values) = option["values"].as_array_mut() {
                values.retain(|v| v["id"] != "default");
            }
            option
        })
        .collect()
}

/// IDs are agent-defined: use the category, with legacy conventional IDs as a fallback.
pub(crate) fn model_config(session: &Value) -> Option<&Value> {
    let options = session["configOptions"].as_array()?;
    let is_select = |v: &&Value| v["type"] == "select" || v.get("type").is_none();
    options
        .iter()
        .filter(is_select)
        .find(|v| v["category"] == "model")
        .or_else(|| options.iter().filter(is_select).find(|v| v["id"] == "model" || v["id"] == "models"))
}

/// The model ids the agent lists, in its order.
pub(crate) fn model_ids(session: &Value) -> Vec<String> {
    let catalog = model_catalog(session, &Value::Null);
    catalog["models"].as_array().into_iter().flatten().filter_map(|m| text(&m["id"])).collect()
}

/// `{models: [{id, name, description, options}], currentModelId, fastMode}`; `options` is
/// Cursor's `cursor/list_available_models` answer (`Null` for other agents).
fn model_catalog(session: &Value, options: &Value) -> Value {
    fn collect(options: &Value, out: &mut Vec<Value>) {
        for option in options.as_array().into_iter().flatten() {
            if option["options"].is_array() {
                collect(&option["options"], out);
            } else if let Some(id) = text(&option["value"])
                && !out.iter().any(|v| v["id"] == id)
            {
                out.push(json!({"id": id, "name": text(&option["name"]).unwrap_or(id),
                    "description": text(&option["description"])}));
            }
        }
    }
    let mut models = Vec::new();
    let current = if let Some(config) = model_config(session) {
        collect(&config["options"], &mut models);
        text(&config["currentValue"])
    } else {
        for model in session["models"]["availableModels"].as_array().into_iter().flatten() {
            if let Some(id) = text(&model["modelId"])
                && !models.iter().any(|v| v["id"] == id)
            {
                models.push(json!({"id": id, "name": text(&model["name"]).unwrap_or(id),
                    "description": text(&model["description"])}));
            }
        }
        text(&session["models"]["currentModelId"])
    };
    let initial_options = reasoning_options(session);
    for model in &mut models {
        let own = model_options(options, model["id"].as_str().unwrap_or_default());
        model["options"] = json!(if options.is_null() { initial_options.clone() } else { own });
    }
    json!({"models": models, "currentModelId": current,
        "defaultModelOptions": true,
        "fastMode": crate::agent::config::fast_mode_config(session).is_some()})
}

/// One model's options as `[{id, name, values: [{id, name}], default}]`, from Cursor's
/// `{models: [{value, configOptions}]}` (ACP select config options).
fn model_options(options: &Value, model: &str) -> Vec<Value> {
    let entry = options["models"].as_array().into_iter().flatten().find(|m| m["value"] == model);
    entry.and_then(|m| m["configOptions"].as_array()).into_iter().flatten().filter_map(parse_select_option).collect()
}

fn parse_select_option(option: &Value) -> Option<Value> {
    if option["type"] != "select" {
        return None;
    }
    let id = text(&option["id"])?;
    let mut values = Vec::new();
    collect_values(&option["options"], &mut values);
    if values.is_empty() {
        return None;
    }
    Some(json!({"id": id, "name": text(&option["name"]).unwrap_or(id), "values": values,
        "default": text(&option["currentValue"])}))
}

fn collect_values(options: &Value, out: &mut Vec<Value>) {
    for value in options.as_array().into_iter().flatten() {
        if value["options"].is_array() {
            collect_values(&value["options"], out);
        } else if let Some(id) = text(&value["value"])
            && !out.iter().any(|v| v["id"] == id)
        {
            out.push(json!({"id": id, "name": text(&value["name"]).unwrap_or(id)}));
        }
    }
}

/// Models offered as backups: the shared ones first, then each account's own, tagged with that account. A
/// model every account serves is listed once, on the shared side (OpenBot nightly `triage-12627`).
pub(crate) fn backup_model_choices(shared: &Value, per_account: &[(String, Value)]) -> Vec<Value> {
    let shared_ids: HashSet<&str> =
        shared["models"].as_array().into_iter().flatten().filter_map(|model| model["id"].as_str()).collect();
    let mut out: Vec<Value> = shared["models"].as_array().into_iter().flatten().cloned().collect();
    for (account, catalog) in per_account {
        for model in catalog["models"].as_array().into_iter().flatten() {
            let Some(id) = model["id"].as_str() else { continue };
            if shared_ids.contains(id)
                || out.iter().any(|m| m["id"].as_str() == Some(id) && m["account"].as_str() == Some(account.as_str()))
            {
                continue;
            }
            let mut choice = model.clone();
            choice["account"] = json!(account);
            out.push(choice);
        }
    }
    out
}

/// Custom-provider punctuation an agent's listed-id charset refuses. OpenBot nightly `triage-7961`.
const ESCAPED_MODEL_CHARS: [char; 19] =
    ['!', '#', '$', '%', '&', '(', ')', '*', '+', ';', '<', '>', '?', '@', '^', '{', '|', '}', '~'];

/// A custom model id with punctuation gets a listed id in the charset agents accept: each such
/// character becomes `=` and its hex code (`@cf/openai/gpt-oss-120b` -> `=40cf/openai/gpt-oss-120b`).
/// An id the charset already accepts keeps its key, so saved agents do not change.
pub(crate) fn escaped_custom_model_id(full: &str) -> Option<String> {
    let (provider, model) = full.split_once('/')?;
    if provider.is_empty() || model.is_empty() {
        return None;
    }
    let escaped: String = model
        .chars()
        .map(|c| if ESCAPED_MODEL_CHARS.contains(&c) { format!("={:02X}", c as u32) } else { c.to_string() })
        .collect();
    Some(format!("{provider}/{escaped}"))
}

/// A listed id and the raw vendor id of one custom model name the same model.
pub(crate) fn model_ids_match(listed: &str, chosen: &str) -> bool {
    listed == chosen
        || escaped_custom_model_id(listed).zip(escaped_custom_model_id(chosen)).is_some_and(|(a, b)| a == b)
}

const KV_HIDDEN: &str = "models.hidden";

fn hidden_map(store: &Store) -> HashMap<String, Vec<String>> {
    store.kv_read(KV_HIDDEN).ok().flatten().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

/// Hidden models that no agent on this computer still uses. A turned-off model
/// stays in the picker and the Backup models list for the agent that uses it as
/// its own model or a fallback; agents that never chose it (a new agent among
/// them) do not start on it (OpenBot nightly triage-12582). Pure so both sides
/// of the decision can be tested without a store.
pub(crate) fn hidden_except_used(hidden: &[String], used: &[String]) -> Vec<String> {
    hidden.iter().filter(|id| !used.contains(id)).cloned().collect()
}

/// Models `backend`'s agents still use, as the primary model or a fallback.
fn used_models(store: &Store, backend: &str) -> Vec<String> {
    store
        .bots()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| !row.deleted && row.config.backend == backend)
        .flat_map(|row| {
            std::iter::once(row.config.model.clone())
                .chain(row.config.fallbacks.into_iter().filter(|f| f.backend == backend).map(|f| f.model))
                .flatten()
        })
        .collect()
}

/// The models of `backend` turned off in the picker that no agent still uses.
pub fn hidden(store: &Store, backend: &str) -> Vec<String> {
    let stored = hidden_map(store).remove(backend).unwrap_or_default();
    hidden_except_used(&stored, &used_models(store, backend))
}

/// Turns one model off or back on in the picker.
pub fn set_hidden(store: &Store, backend: &str, model: &str, hidden: bool) -> Result<()> {
    // A custom provider's raw vendor id has a different listed form; keep hidden models in that form.
    let listed = if backend == "custom" {
        escaped_custom_model_id(model).unwrap_or_else(|| model.to_owned())
    } else {
        model.to_owned()
    };
    let model = listed.as_str();
    if backend.is_empty() || backend.len() > 80 || model.is_empty() || model.len() > 200 {
        bail!("invalid model or provider");
    }
    let mut map = hidden_map(store);
    let list = map.entry(backend.to_owned()).or_default();
    list.retain(|id| id != model);
    if hidden {
        if list.len() >= 5_000 {
            bail!("too many hidden models");
        }
        list.push(model.to_owned());
    }
    if list.is_empty() {
        map.remove(backend);
    }
    store.kv_set(KV_HIDDEN, &serde_json::to_string(&map)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slow_model_list_keeps_the_chosen_model() {
        assert_eq!(stable_model(Some("chosen"), None), Some("chosen"), "no fresh list yet");
        assert_eq!(stable_model(Some("old"), Some("new")), Some("new"), "a fresh answer wins");
        assert_eq!(stable_model(None, None), None);
    }

    #[tokio::test]
    async fn an_unavailable_model_keeps_other_models_and_their_options() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/model_options_agent.py");
        let command = crate::shell::python(&fixture);
        let (acp, _inbox) = crate::agent::acp::Acp::spawn(&command, env!("CARGO_MANIFEST_DIR"), &[]).unwrap();
        let session = json!({"sessionId": "s1", "configOptions": [{"id": "model", "category": "model",
            "type": "select", "currentValue": "first", "options": [
                {"value": "first"}, {"value": "unavailable"}, {"value": "second"}]}]});
        let mut catalog = model_catalog(&session, &Value::Null);
        discover_options(&acp, &session, &mut catalog).await.unwrap();
        acp.kill().await;
        assert_eq!(catalog["models"].as_array().unwrap().len(), 3);
        assert!(catalog["models"][1]["optionsError"].as_str().unwrap().contains("model not available"));
        assert_eq!(catalog["models"][1]["options"], json!([]));
        assert_eq!(catalog["models"][2]["options"][0]["default"], "high");
        assert_eq!(catalog["defaultModelOptions"], true);
    }

    #[test]
    fn reasoning_catalog_preserves_defaults_and_grouped_levels_without_permission_options() {
        let session = json!({"models": {"currentModelId": "opus", "availableModels": [{"modelId": "opus"}]},
        "configOptions": [
            {"id": "effort", "category": "thought_level", "type": "select", "currentValue": "high",
             "options": [{"group": "levels", "options": [{"value": "low", "name": "Low"}, {"value": "high", "name": "High"}]}]},
            {"id": "mode", "category": "mode", "type": "select", "options": [{"value": "bypassPermissions"}]},
            {"id": "fast", "type": "boolean"}
        ]});
        let catalog = model_catalog(&session, &Value::Null);
        let options = catalog["models"][0]["options"].as_array().unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0]["id"], "effort");
        assert_eq!(options[0]["default"], "high");
        assert_eq!(options[0]["values"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn model_catalog_reads_grouped_config_and_preserves_agent_ids() {
        let session = json!({"configOptions": [{
            "id": "provider-model", "category": "model", "type": "select", "currentValue": "m2",
            "options": [{"group": "a", "options": [
                {"value": "m1", "name": "First"}, {"value": "m2", "name": "Second"},
                {"value": "m1", "name": "Duplicate"}
            ]}]
        }], "models": {"availableModels": [{"modelId": "legacy", "name": "Legacy"}]}});
        assert_eq!(model_config(&session).unwrap()["id"], "provider-model");
        let result = model_catalog(&session, &Value::Null);
        assert_eq!(result["currentModelId"], "m2");
        assert_eq!(result["models"].as_array().unwrap().len(), 2);
        assert_eq!(result["models"][0]["id"], "m1");
        assert_eq!(result["models"][1]["name"], "Second");
        assert_eq!(result["models"][1]["options"], json!([]));
        assert_eq!(model_ids(&session), ["m1", "m2"]);
    }

    #[test]
    fn model_catalog_reads_legacy_and_missing_lists() {
        let result = model_catalog(
            &json!({"models": {
                "currentModelId": "a", "availableModels": [
                    {"modelId": "a", "name": "Agent model", "description": "Available here"},
                    {"name": "No ID"}
                ]
            }}),
            &Value::Null,
        );
        assert_eq!(result["models"].as_array().unwrap().len(), 1);
        assert_eq!(result["models"][0]["description"], "Available here");
        assert_eq!(
            model_catalog(&json!({}), &Value::Null),
            json!({"models": [], "currentModelId": null, "fastMode": false, "defaultModelOptions": true})
        );
    }

    #[test]
    fn model_catalog_adds_cursor_model_options() {
        let session = json!({"configOptions": [{"id": "model", "category": "model", "type": "select",
            "currentValue": "grok-4.7", "options": [{"value": "grok-4.7", "name": "Grok 4.7"}, {"value": "auto"}]}]});
        let listed = json!({"models": [{"value": "grok-4.7", "name": "Grok 4.7", "configOptions": [
            {"id": "reasoning_effort", "name": "Reasoning", "category": "thought_level", "type": "select",
             "currentValue": "high", "options": [{"value": "medium", "name": "Medium"}, {"value": "high", "name": "High"}]},
            {"id": "fast", "type": "select", "currentValue": "true", "options": [{"value": "false", "name": "Off"}, {"value": "true"}]},
            {"id": "verbose", "type": "boolean"}
        ]}]});
        let result = model_catalog(&session, &listed);
        assert_eq!(
            result["models"][0]["options"],
            json!([
                {"id": "reasoning_effort", "name": "Reasoning", "default": "high",
                 "values": [{"id": "medium", "name": "Medium"}, {"id": "high", "name": "High"}]},
                {"id": "fast", "name": "fast", "default": "true",
                 "values": [{"id": "false", "name": "Off"}, {"id": "true", "name": "true"}]}
            ])
        );
        assert_eq!(result["models"][1]["options"], json!([]));
    }

    #[test]
    fn model_config_prefers_category_and_ignores_unknown_types() {
        let session = json!({"configOptions": [
            {"id": "model", "type": "boolean"},
            {"id": "models", "type": "select"},
            {"id": "engine", "category": "model", "type": "select"}
        ]});
        assert_eq!(model_config(&session).unwrap()["id"], "engine");
        assert!(model_config(&json!({"configOptions": [{"id": "model", "type": "boolean"}]})).is_none());
    }

    #[test]
    fn a_list_is_read_again_about_every_ten_minutes_only_where_it_can_grow() {
        assert!(relists_while_running("antigravity"));
        assert!(relists_while_running("gemini"));
        assert!(!relists_while_running("claude"));
        let now = Instant::now();
        assert!(should_relist(None, now));
        assert!(!should_relist(Some(now - RELIST_INTERVAL + Duration::from_secs(1)), now));
        assert!(should_relist(Some(now - RELIST_INTERVAL), now));
    }

    #[test]
    fn a_failed_or_empty_relist_keeps_the_last_good_list() {
        let now = Instant::now();
        let lists = InFlight::default();
        let old = json!({"models": [{"id": "neon"}]});
        lists.remember("antigravity", old.clone(), now - RELIST_INTERVAL);
        assert!(should_relist(Some(now - RELIST_INTERVAL), now));
        assert!(lists.fresh("antigravity", now).is_none(), "the old list is past its interval");
        assert_eq!(lists.stale("antigravity"), Some(old));
        assert!(!has_models(&json!({"models": []})), "an empty re-read is quiet");
        assert!(has_models(&json!({"models": [{"id": "neon"}]})));
    }

    #[test]
    fn a_grok_model_check_removes_only_its_own_probe_session() {
        assert!(model_check_session("grok"));
        assert!(!model_check_session("claude"), "a bot's chat session is not a model-check probe");
        assert!(!model_check_session("gemini"));
        let home = Path::new("/home/me/.grok");
        assert_eq!(
            grok_probe_session_file(home, "/Users/me/project", "abc_12-3").as_deref(),
            Some(Path::new("/home/me/.grok/sessions/%2FUsers%2Fme%2Fproject/abc_12-3"))
        );
        assert_eq!(
            grok_probe_session_file(home, "/a b!x", "s").as_deref(),
            Some(Path::new("/home/me/.grok/sessions/%2Fa%20b!x/s")),
            "encodeURIComponent keeps !"
        );
        for (cwd, id) in [("/p", ""), ("/p", "../bad"), ("/p", "a/b"), ("", "ok")] {
            assert_eq!(grok_probe_session_file(home, cwd, id), None, "{cwd} / {id}");
        }
    }

    #[test]
    fn an_empty_cursor_list_is_retried_once_then_hinted() {
        let empty = json!({"models": []});
        assert!(cursor_retry_empty("cursor", &empty, 0));
        assert!(!cursor_retry_empty("cursor", &empty, 1), "one retry only");
        assert!(!cursor_retry_empty("cursor", &Value::Null, 0), "a failed request is not an empty answer");
        assert!(!cursor_retry_empty("codex", &empty, 0));
        let hint = empty_catalog_hint(&json!({"models": []})).expect("a plain hint");
        assert!(hint.contains("Settings, Providers"));
        assert_eq!(empty_catalog_hint(&json!({"models": [{"id": "m"}]})), None);
    }

    #[test]
    fn hidden_and_legacy_models_move_to_older_models() {
        let catalog = json!({"models": [
            {"id": "current", "name": "Current"},
            {"id": "gpt-4", "name": "GPT-4", "description": "Legacy model"},
            {"id": "turned-off", "name": "Turned off"},
            {"id": "new", "name": "New"}
        ]});
        let grouped = group_older_models(catalog, &["turned-off".to_owned()]);
        let models = grouped["models"].as_array().unwrap();
        assert_eq!(
            models.iter().map(|m| m["id"].as_str().unwrap()).collect::<Vec<_>>(),
            ["current", "new", "gpt-4", "turned-off"]
        );
        assert_eq!(models[0].get("group"), None);
        assert_eq!(models[1].get("group"), None);
        assert_eq!(models[2]["group"], OLDER_MODELS);
        assert_eq!(models[3]["group"], OLDER_MODELS);
    }

    #[test]
    fn only_a_secondary_account_retries_its_timed_out_list() {
        assert!(retry_slow_account(true, true));
        assert!(!retry_slow_account(true, false), "a failure that isn't a timeout is not retried");
        assert!(!retry_slow_account(false, true), "the first account already reads with the full budget");
    }

    #[test]
    fn backup_choices_include_an_accounts_own_models() {
        let shared = json!({"models": [{"id": "opus", "name": "Opus"}]});
        let others = vec![
            ("acc_a".to_owned(), json!({"models": [{"id": "opus"}, {"id": "paid-a", "name": "Paid A"}]})),
            ("acc_b".to_owned(), json!({"models": [{"id": "paid-b", "name": "Paid B"}]})),
        ];
        let choices = backup_model_choices(&shared, &others);
        let ids: Vec<&str> = choices.iter().filter_map(|m| m["id"].as_str()).collect();
        assert_eq!(ids, ["opus", "paid-a", "paid-b"]);
        assert!(choices[0].get("account").is_none(), "the shared model is not copied per account");
        assert_eq!(choices[1]["account"], "acc_a");
        assert_eq!(choices[2]["account"], "acc_b");
        assert!(backup_model_choices(&json!({"models": []}), &[]).is_empty());
    }

    #[test]
    fn a_custom_model_id_with_punctuation_has_a_listed_form() {
        assert_eq!(
            escaped_custom_model_id("cloudflare/@cf/openai/gpt-oss-120b").as_deref(),
            Some("cloudflare/=40cf/openai/gpt-oss-120b")
        );
        assert_eq!(escaped_custom_model_id("spark/gpt-6-astral").as_deref(), Some("spark/gpt-6-astral"));
        assert!(escaped_custom_model_id("no-slash").is_none());
        assert!(model_ids_match("cloudflare/=40cf/openai/gpt-oss-120b", "cloudflare/@cf/openai/gpt-oss-120b"));
        assert!(!model_ids_match("spark/a", "spark/b"));
    }
}

#[test]
fn hidden_models_are_saved_and_forgotten() {
    let store = Store::open(std::path::Path::new(":memory:")).expect("store");
    set_hidden(&store, "claude", "m1", true).expect("hide");
    assert_eq!(hidden(&store, "claude"), ["m1"]);
    assert_eq!(hidden(&store, "codex"), Vec::<String>::new());
    set_hidden(&store, "claude", "m1", false).expect("show");
    assert_eq!(hidden(&store, "claude"), Vec::<String>::new());
}

#[test]
fn a_turned_off_model_is_kept_only_for_an_agent_that_uses_it() {
    let hidden = vec!["m1".to_owned(), "m2".to_owned()];
    // One agent still answers on m1; m2 has no user, and a new agent has no model yet.
    let used = vec!["m1".to_owned()];
    assert_eq!(hidden_except_used(&hidden, &used), ["m2"]);
    assert_eq!(hidden_except_used(&hidden, &[]), ["m1", "m2"]);
    assert_eq!(hidden_except_used(&hidden, &["m1".to_owned(), "m2".to_owned()]), Vec::<String>::new());
}
