//! The bridge from the browser tools to this bot's own browser: one Chrome or Edge process started
//! over the inherited CDP pipe of `protected_browser::pipe`, in a browser context Codync created,
//! with one tab. Every call attaches to that tab, acts, and detaches. Nothing here depends on CDP
//! events: the pipe answers requests and drops events, so the page's load and quiet states are polled
//! through `Runtime.evaluate` instead.
//!
//! Which CDP methods each action uses:
//! - `Target.getTargets`, `Target.attachToTarget`, `Target.detachFromTarget`, `Target.createTarget`,
//!   `Target.createBrowserContext`, `Target.closeTarget`: the bot's tab, every call.
//! - `Page.navigate`, `Page.reload`, `Page.getNavigationHistory`,
//!   `Page.navigateToHistoryEntry`: `navigate`.
//! - `Runtime.evaluate`: every page-side op (the snapshot tree, an element's box and focus, the
//!   select and checkbox changes, and the load probe) and `evaluate` itself.
//! - `Input.dispatchMouseEvent`: `click` (moved, pressed, released), `hover` (moved) and `scroll`
//!   (mouse wheel).
//! - `Input.insertText`: `type`. `Input.dispatchKeyEvent`: `press` and the Enter of a submitting
//!   `type`.

use super::action::{Action, Button, Direction, Key, Kind, Load, Target};
use super::snapshot::{self, format as format_snapshot};
use super::{cut_at, folder_slug};
use crate::LockExt as _;
use crate::agent::process::Process;
use crate::screen::protected_browser::{
    browser_executable,
    pipe::{self, Pipe, rpc},
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{Instant, sleep, timeout};

/// The value `evaluate` may return, in characters.
const MAX_EVALUATE_RESULT: usize = 64_000;
/// How long the page must change nothing before `dom-quiet` counts as met.
const QUIET_MS: u64 = 500;
/// How often a poll asks the page whether its condition holds.
const POLL: Duration = Duration::from_millis(150);
/// How long the whole action may take beyond its own deadline, before the browser counts as hung.
const HUNG: Duration = Duration::from_secs(5);

/// The page-side helpers, evaluated with each call: an expression that builds the page's API.
const PAGE: &str = include_str!("page.js");

/// One bot's browser, or `None` while it has none.
type Slot = Arc<AsyncMutex<Option<Session>>>;
static SESSIONS: LazyLock<Mutex<HashMap<String, Slot>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct Session {
    process: Process,
    folder: PathBuf,
    pipe: Pipe,
    /// The browser context Codync created for this bot; no page outside it is touched.
    context: String,
    /// The tab every call acts on.
    target: String,
}

/// Runs one action against `bot`'s browser, opening the browser for a `navigate` when it has none.
pub(super) async fn perform(bot: &str, action: &Action) -> Result<Value> {
    let slot = match &action.kind {
        Kind::Navigate { .. } => open(bot).await?,
        _ => existing(bot)
            .await
            .context("This bot has no browser open yet. Open a page with the navigate tool first.")?,
    };
    let mut guard = slot.lock().await;
    let answered = {
        let Some(session) = guard.as_mut() else {
            bail!("This bot has no browser open yet. Open a page with the navigate tool first.");
        };
        ensure!(!session.process.exited()?, "The browser closed. Open the page again with navigate.");
        timeout(action.timeout.saturating_add(HUNG), run(session, action)).await
    };
    match answered {
        Ok(answer) => answer,
        Err(_) => {
            // A cancelled read leaves the pipe unsure of where the next reply starts, so the browser
            // is stopped instead of reused.
            stop(guard.take()).await;
            bail!(
                "The browser did not answer within {} seconds, so it was closed. Open the page again with navigate.",
                action.timeout.as_secs()
            )
        }
    }
}

/// The slot of `bot`, with its browser started when it has none.
async fn open(bot: &str) -> Result<Slot> {
    let slot = slot(bot);
    let mut guard = slot.lock().await;
    if guard.as_ref().is_some_and(|session| session.process.exited().unwrap_or(true)) {
        stop(guard.take()).await;
    }
    if guard.is_none() {
        *guard = Some(spawn(bot).await?);
    }
    drop(guard);
    Ok(slot)
}

/// The slot of `bot` when its browser is still running.
async fn existing(bot: &str) -> Option<Slot> {
    let slot = {
        let registry = SESSIONS.locked();
        registry.get(bot).cloned()
    }?;
    let alive = {
        let guard = slot.lock().await;
        guard.as_ref().is_some_and(|session| !session.process.exited().unwrap_or(true))
    };
    if alive {
        Some(slot)
    } else {
        close(bot).await;
        None
    }
}

fn slot(bot: &str) -> Slot {
    let mut registry = SESSIONS.locked();
    registry.entry(bot.to_owned()).or_default().clone()
}

/// Stops a bot's browser and forgets it.
async fn close(bot: &str) {
    let removed = SESSIONS.locked().remove(bot);
    let Some(slot) = removed else { return };
    let ended = {
        let mut guard = slot.lock().await;
        guard.take()
    };
    stop(ended).await;
}

/// Starts a browser in the bot's own profile, in a context of its own, with a blank tab.
async fn spawn(bot: &str) -> Result<Session> {
    let executable = browser_executable().context("Install Chrome or Edge to use the browser tools.")?;
    let parent = crate::service::data_dir().join("protected-browsers").join("bots");
    std::fs::create_dir_all(&parent)?;
    let folder = parent.join(folder_slug(bot));
    // A profile left behind by a browser that is no longer running is cleared, so a stale lock does
    // not keep the next browser from starting.
    if folder.exists()
        && let Err(error) = std::fs::remove_dir_all(&folder)
    {
        tracing::warn!(%error, "could not clear a bot's old browser profile");
    }
    std::fs::create_dir_all(&folder)?;
    let (process, mut socket) =
        pipe::spawn(&executable, &folder, None).context("The browser tools could not start a browser")?;
    let targets = rpc(&mut socket, "Target.getTargets", json!({}), None).await?;
    let context = rpc(&mut socket, "Target.createBrowserContext", json!({"disposeOnDetach": false}), None).await?;
    let context = context["browserContextId"]
        .as_str()
        .filter(|context| !context.is_empty())
        .context("The browser did not create an owned context")?
        .to_owned();
    for page in targets["targetInfos"].as_array().into_iter().flatten().filter(|target| target["type"] == "page") {
        rpc(&mut socket, "Target.closeTarget", json!({"targetId": page["targetId"]}), None).await.ok();
    }
    let target = rpc(
        &mut socket,
        "Target.createTarget",
        json!({"url": "about:blank", "browserContextId": context.as_str()}),
        None,
    )
    .await?;
    let target = target["targetId"].as_str().context("The browser did not open a tab")?.to_owned();
    Ok(Session { process, folder, pipe: socket, context, target })
}

/// Stops a browser and removes its profile, retrying a profile Chrome still holds.
async fn stop(session: Option<Session>) {
    let Some(mut session) = session else { return };
    if let Err(error) = session.process.terminate_owned().await {
        tracing::warn!(%error, "could not stop a bot's browser");
        return;
    }
    let folder = session.folder.clone();
    if let Err(error) = remove_folder(&folder) {
        tracing::warn!(%error, "could not remove a bot's browser profile yet");
        tokio::spawn(async move {
            for _ in 0..6 {
                sleep(Duration::from_millis(500)).await;
                if remove_folder(&folder).is_ok() {
                    break;
                }
            }
        });
    }
}

/// Removes a closed browser's profile, never anything outside Codync's own folder.
fn remove_folder(folder: &Path) -> Result<()> {
    if !folder.try_exists()? {
        return Ok(());
    }
    let parent = crate::service::data_dir().join("protected-browsers").canonicalize()?;
    let folder = folder.canonicalize()?;
    ensure!(folder.starts_with(&parent), "A bot's browser profile is outside Codync's own folder");
    std::fs::remove_dir_all(&folder)?;
    Ok(())
}

/// One action on the bot's tab, with a session attached and detached around it.
async fn run(session: &mut Session, action: &Action) -> Result<Value> {
    ensure_target(session).await?;
    let tab = attach(session).await?;
    let answer = act(session, &tab, action).await;
    rpc(&mut session.pipe, "Target.detachFromTarget", json!({"sessionId": tab.as_str()}), None).await.ok();
    answer
}

/// Makes `session.target` the bot's open tab, adopting or opening one when the page closed it.
async fn ensure_target(session: &mut Session) -> Result<()> {
    let targets = rpc(&mut session.pipe, "Target.getTargets", json!({}), None).await?;
    let pages: Vec<&Value> = targets["targetInfos"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|target| target["type"] == "page" && target["browserContextId"] == session.context.as_str())
        .collect();
    if pages.iter().any(|page| page["targetId"] == session.target.as_str()) {
        return Ok(());
    }
    if let Some(page) = pages.first() {
        session.target = page["targetId"].as_str().context("Could not find the bot's tab")?.to_owned();
        return Ok(());
    }
    let created = rpc(
        &mut session.pipe,
        "Target.createTarget",
        json!({"url": "about:blank", "browserContextId": session.context.as_str()}),
        None,
    )
    .await?;
    session.target = created["targetId"].as_str().context("The browser did not open a tab")?.to_owned();
    Ok(())
}

async fn attach(session: &mut Session) -> Result<String> {
    let attached = rpc(
        &mut session.pipe,
        "Target.attachToTarget",
        json!({"targetId": session.target.as_str(), "flatten": true}),
        None,
    )
    .await?;
    attached["sessionId"].as_str().map(str::to_owned).context("Could not reach the bot's tab")
}

async fn act(session: &mut Session, tab: &str, action: &Action) -> Result<Value> {
    match &action.kind {
        Kind::Navigate { url, direction } => navigate(session, tab, url.as_deref(), *direction, action.timeout).await,
        Kind::Snapshot => snapshot(session, tab).await,
        Kind::Click { target, button, count, modifiers } => {
            click(session, tab, *target, *button, *count, *modifiers).await
        }
        Kind::Type { target, text, replace, submit } => typed(session, tab, *target, text, *replace, *submit).await,
        Kind::Press { key, target } => press(session, tab, key, *target).await,
        Kind::Hover { target } => hover(session, tab, *target).await,
        Kind::Scroll { target, x, y } => scroll(session, tab, *target, *x, *y).await,
        Kind::Select { target, values } => select(session, tab, *target, values).await,
        Kind::Check { target, checked } => check(session, tab, *target, *checked).await,
        Kind::Wait { text, url, state } => {
            wait(session, tab, text.as_deref(), url.as_deref(), *state, action.timeout).await
        }
        Kind::Evaluate { expression, await_promise } => evaluate(session, tab, expression, *await_promise).await,
        Kind::Run { .. } | Kind::Sleep { .. } => bail!("`{}` runs in the host, not in the page.", action.kind.name()),
    }
}

async fn navigate(
    session: &mut Session,
    tab: &str,
    url: Option<&str>,
    direction: Option<Direction>,
    limit: Duration,
) -> Result<Value> {
    let before = page(session, tab, "probe", &json!({})).await?["url"].as_str().unwrap_or_default().to_owned();
    match (url, direction) {
        (Some(url), None) => {
            let result = rpc(&mut session.pipe, "Page.navigate", json!({"url": url}), Some(tab)).await?;
            if let Some(error) = result["errorText"].as_str() {
                bail!("The page did not load: {error}");
            }
        }
        (None, Some(Direction::Reload)) => {
            rpc(&mut session.pipe, "Page.reload", json!({"ignoreCache": false}), Some(tab)).await?;
        }
        (None, Some(direction)) => {
            let history = rpc(&mut session.pipe, "Page.getNavigationHistory", json!({}), Some(tab)).await?;
            let current = history["currentIndex"].as_i64().context("The tab keeps no history to move in")?;
            let next = match direction {
                Direction::Back => current.checked_sub(1).context("The tab has no earlier page.")?,
                _ => current.checked_add(1).context("The tab has no later page.")?,
            };
            let entries = history["entries"].as_array().context("The tab keeps no history to move in")?;
            let entry = usize::try_from(next).ok().and_then(|next| entries.get(next)).with_context(|| {
                if direction == Direction::Back {
                    "The tab has no earlier page.".to_owned()
                } else {
                    "The tab has no later page.".to_owned()
                }
            })?;
            rpc(&mut session.pipe, "Page.navigateToHistoryEntry", json!({"entryId": entry["id"]}), Some(tab)).await?;
        }
        (Some(_), Some(_)) => bail!("Name a url or a direction, not both."),
        (None, None) => bail!("Name a url, or a direction: back, forward or reload."),
    }
    let settled = settle(session, tab, &before, limit).await?;
    let mut answer = snapshot(session, tab).await?;
    answer["settled"] = json!(settled);
    if !settled {
        answer["note"] = json!("The page was still loading when the wait ended; take a snapshot to see where it is.");
    }
    Ok(answer)
}

/// Waits until the page reports itself loaded twice in a row, after it moved or had time to.
async fn settle(session: &mut Session, tab: &str, before: &str, limit: Duration) -> Result<bool> {
    let started = Instant::now();
    let mut complete = 0_u8;
    loop {
        let probe = page(session, tab, "probe", &json!({})).await?;
        let ready = probe["readyState"].as_str() == Some("complete");
        let moved = probe["url"].as_str().is_some_and(|url| url != before);
        if ready && (moved || started.elapsed() >= Duration::from_millis(600)) {
            complete += 1;
            if complete >= 2 {
                return Ok(true);
            }
        } else {
            complete = 0;
        }
        if started.elapsed() >= limit {
            return Ok(false);
        }
        sleep(POLL).await;
    }
}

async fn snapshot(session: &mut Session, tab: &str) -> Result<Value> {
    let read = call_page(session, tab, "snapshot", &json!({})).await?;
    let text = format_snapshot(&read, snapshot::MAX_CHARS);
    Ok(json!({"url": read["url"], "title": read["title"], "snapshot": text}))
}

async fn click(
    session: &mut Session,
    tab: &str,
    target: Target,
    button: Button,
    count: u8,
    modifiers: u8,
) -> Result<Value> {
    let (x, y, label) = point(session, tab, target).await?;
    let mut moved = json!({"type": "mouseMoved", "x": x, "y": y, "button": "none", "buttons": 0});
    if modifiers != 0 {
        moved["modifiers"] = json!(modifiers);
    }
    mouse(session, tab, moved).await?;
    mouse(
        session,
        tab,
        json!({"type":"mousePressed","x":x,"y":y,"button":button.name(),"buttons":button.buttons(),"clickCount":count,"modifiers":modifiers}),
    )
    .await?;
    mouse(
        session,
        tab,
        json!({"type":"mouseReleased","x":x,"y":y,"button":button.name(),"buttons":0,"clickCount":count,"modifiers":modifiers}),
    )
    .await?;
    Ok(json!({"clicked": label, "x": x, "y": y}))
}

async fn hover(session: &mut Session, tab: &str, target: Target) -> Result<Value> {
    let (x, y, label) = point(session, tab, target).await?;
    mouse(session, tab, json!({"type": "mouseMoved", "x": x, "y": y, "button": "none", "buttons": 0})).await?;
    Ok(json!({"hovered": label, "x": x, "y": y}))
}

async fn scroll(session: &mut Session, tab: &str, target: Option<Target>, x: f64, y: f64) -> Result<Value> {
    let (at_x, at_y) = match target {
        Some(target) => {
            let (x, y, _) = point(session, tab, target).await?;
            (x, y)
        }
        None => {
            let center = call_page(session, tab, "center", &json!({})).await?;
            (center["x"].as_i64().unwrap_or(0), center["y"].as_i64().unwrap_or(0))
        }
    };
    mouse(session, tab, json!({"type":"mouseWheel","x":at_x,"y":at_y,"deltaX":x,"deltaY":y})).await?;
    Ok(json!({"scrolled": {"x": x, "y": y}}))
}

async fn typed(
    session: &mut Session,
    tab: &str,
    target: Option<Target>,
    text: &str,
    replace: bool,
    submit: bool,
) -> Result<Value> {
    let into = match target {
        Some(target) => {
            let focused =
                call_page(session, tab, "focus", &json!({"ref": target.number(), "typing": true, "replace": replace}))
                    .await?;
            focused["label"].as_str().unwrap_or("the field").to_owned()
        }
        None => "the focused control".to_owned(),
    };
    rpc(&mut session.pipe, "Input.insertText", json!({"text": text}), Some(tab)).await?;
    if submit {
        press_key(session, tab, &Key::enter()).await?;
    }
    Ok(json!({"typed": text.chars().count(), "into": into}))
}

async fn press(session: &mut Session, tab: &str, key: &Key, target: Option<Target>) -> Result<Value> {
    if let Some(target) = target {
        call_page(session, tab, "focus", &json!({"ref": target.number(), "typing": false, "replace": false})).await?;
    }
    press_key(session, tab, key).await?;
    Ok(json!({"pressed": key.label}))
}

async fn press_key(session: &mut Session, tab: &str, key: &Key) -> Result<()> {
    let mut down = json!({
        "type": "keyDown",
        "modifiers": key.modifiers,
        "key": key.key.as_str(),
        "windowsVirtualKeyCode": key.vk,
        "nativeVirtualKeyCode": key.vk,
    });
    if let Some(code) = &key.code {
        down["code"] = json!(code);
    }
    if let Some(text) = key.insertable() {
        down["text"] = json!(text);
        down["unmodifiedText"] = json!(text);
    }
    rpc(&mut session.pipe, "Input.dispatchKeyEvent", down, Some(tab)).await?;
    let mut up = json!({
        "type": "keyUp",
        "modifiers": key.modifiers,
        "key": key.key.as_str(),
        "windowsVirtualKeyCode": key.vk,
        "nativeVirtualKeyCode": key.vk,
    });
    if let Some(code) = &key.code {
        up["code"] = json!(code);
    }
    rpc(&mut session.pipe, "Input.dispatchKeyEvent", up, Some(tab)).await?;
    Ok(())
}

async fn select(session: &mut Session, tab: &str, target: Target, values: &[String]) -> Result<Value> {
    let answer = call_page(session, tab, "select", &json!({"ref": target.number(), "values": values})).await?;
    Ok(json!({"selected": answer["selected"], "in": answer["label"]}))
}

async fn check(session: &mut Session, tab: &str, target: Target, checked: bool) -> Result<Value> {
    let answer = call_page(session, tab, "check", &json!({"ref": target.number(), "checked": checked})).await?;
    Ok(json!({"checked": answer["checked"], "element": answer["label"]}))
}

async fn wait(
    session: &mut Session,
    tab: &str,
    text: Option<&str>,
    url: Option<&str>,
    state: Option<Load>,
    limit: Duration,
) -> Result<Value> {
    let started = Instant::now();
    loop {
        let probe = call_page(session, tab, "probe", &json!({"text": text})).await?;
        let ready = match state {
            None => true,
            Some(Load::Load) => probe["readyState"].as_str() == Some("complete"),
            Some(Load::DomContentLoaded) => matches!(probe["readyState"].as_str(), Some("interactive" | "complete")),
            Some(Load::Quiet) => probe["quietMs"].as_u64().is_some_and(|quiet| quiet >= QUIET_MS),
        };
        let has_text = text.is_none() || probe["hasText"] == true;
        let at_url = url.is_none()
            || probe["url"].as_str().is_some_and(|current| url.is_some_and(|wanted| current.contains(wanted)));
        if ready && has_text && at_url {
            return snapshot(session, tab).await;
        }
        if started.elapsed() >= limit {
            bail!("{}", missing_wait(text, url, state, limit));
        }
        sleep(POLL).await;
    }
}

fn missing_wait(text: Option<&str>, url: Option<&str>, state: Option<Load>, limit: Duration) -> String {
    let mut wanted = Vec::new();
    if let Some(text) = text {
        wanted.push(format!("show \"{text}\""));
    }
    if let Some(url) = url {
        wanted.push(format!("reach an address with \"{url}\""));
    }
    if let Some(state) = state {
        wanted.push(state.waiting().to_owned());
    }
    format!(
        "The page did not {} within {} seconds; take a snapshot to see where it is.",
        wanted.join(" or "),
        limit.as_secs()
    )
}

async fn evaluate(session: &mut Session, tab: &str, source: &str, await_promise: bool) -> Result<Value> {
    let answer = page(session, tab, "evaluate", &json!({"source": source, "await": await_promise})).await?;
    if answer["ok"] == true {
        let encoded = answer["json"].as_str().unwrap_or("null");
        let (encoded, truncated) = cut_at(encoded, MAX_EVALUATE_RESULT);
        let value = if truncated {
            Value::String(encoded)
        } else {
            serde_json::from_str(&encoded).unwrap_or(Value::String(encoded))
        };
        return Ok(json!({"value": value, "truncated": truncated}));
    }
    if answer["fallback"] != true {
        bail!("{}", answer["error"].as_str().unwrap_or("The page refused the expression."));
    }
    // A page whose Content Security Policy forbids eval still takes Runtime.evaluate itself, whose
    // thrown text the pipe replaces with one line.
    let result = rpc(
        &mut session.pipe,
        "Runtime.evaluate",
        json!({"expression": source, "returnByValue": true, "awaitPromise": await_promise, "userGesture": true}),
        Some(tab),
    )
    .await
    .context("The page did not evaluate the expression: it may have thrown, or the page refused it")?;
    Ok(json!({"value": result["result"]["value"], "truncated": false}))
}

/// An element's point on the page, with the label the snapshot gave it.
async fn point(session: &mut Session, tab: &str, target: Target) -> Result<(i64, i64, String)> {
    let box_ = call_page(session, tab, "box", &json!({"ref": target.number()})).await?;
    Ok((
        box_["x"].as_i64().context("The element has no point on the page")?,
        box_["y"].as_i64().context("The element has no point on the page")?,
        box_["label"].as_str().unwrap_or("the element").to_owned(),
    ))
}

async fn mouse(session: &mut Session, tab: &str, params: Value) -> Result<()> {
    rpc(&mut session.pipe, "Input.dispatchMouseEvent", params, Some(tab)).await?;
    Ok(())
}

/// One page-side op, answered as the page's own object.
async fn page(session: &mut Session, tab: &str, op: &str, args: &Value) -> Result<Value> {
    let expression = expression(op, args);
    let answer = rpc(
        &mut session.pipe,
        "Runtime.evaluate",
        json!({"expression": expression, "returnByValue": true, "awaitPromise": true, "userGesture": true}),
        Some(tab),
    )
    .await
    .with_context(|| format!("The page did not answer `{op}`"))?;
    let value = answer["result"]["value"].clone();
    ensure!(value.is_object(), "The page answered nothing to `{op}`; take a new snapshot.");
    Ok(value)
}

/// One page-side op that has to succeed, with its error as the tool's error.
async fn call_page(session: &mut Session, tab: &str, op: &str, args: &Value) -> Result<Value> {
    let answer = page(session, tab, op, args).await?;
    if answer["ok"] != true {
        bail!("{}", answer["error"].as_str().unwrap_or("The page refused that action; take a new snapshot."));
    }
    Ok(answer)
}

/// The expression that binds the page's helpers to this document's refs and runs one op.
fn expression(op: &str, args: &Value) -> String {
    format!(
        "((refs) => {{ const api = ({PAGE})(refs); window.__codync = api; return api.{op}({args}); }})(window.__codyncRefs || (window.__codyncRefs = new Map()))"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expression_runs_the_named_op_with_the_given_arguments() {
        let text = expression("box", &json!({"ref": 7}));
        assert!(text.contains("api.box({\"ref\":7})"), "{text}");
        assert!(text.contains("window.__codyncRefs || (window.__codyncRefs = new Map())"));
        assert!(text.contains("((refs) =>"), "the helpers are bound to the document's refs: {text}");
    }

    #[test]
    fn what_a_wait_missed_is_named() {
        let text = missing_wait(Some("Inbox"), Some("/mail"), Some(Load::Load), Duration::from_secs(15));
        assert!(text.contains("show \"Inbox\""), "{text}");
        assert!(text.contains("reach an address with \"/mail\""), "{text}");
        assert!(text.contains("finish loading"), "{text}");
        assert!(text.contains("within 15 seconds"), "{text}");
    }
}
