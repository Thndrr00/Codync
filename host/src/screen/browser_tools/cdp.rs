//! The bridge from the browser tools to this bot's own browser: one Chrome or Edge process started
//! over the inherited CDP pipe of `protected_browser::pipe`, in a browser context Codync created,
//! with one tab. Every call attaches to that tab, acts, and detaches. The pipe answers requests and
//! drops events, so load and quiet states are polled through `Runtime.evaluate` instead. A dead
//! renderer is the one event that must reach the agent (OpenBot 0.78.0): `Inspector.targetCrashed`
//! is detected with a bounded Inspector liveness check when a renderer-side call fails.
//!
//! Which CDP methods each action uses:
//! - `Target.getTargets`, `Target.attachToTarget`, `Target.detachFromTarget`, `Target.createTarget`,
//!   `Target.createBrowserContext`, `Target.closeTarget`: the bot's tab, every call.
//! - `Page.navigate`, `Page.reload`, `Page.getNavigationHistory`,
//!   `Page.navigateToHistoryEntry`: `navigate`. `Page.captureScreenshot`: `screenshot` and the
//!   picture attached to an empty snapshot.
//! - `Runtime.evaluate`: every page-side op (the accessible and drawn snapshot trees, an element's
//!   box and focus, the select and checkbox changes, and the load probe) and `evaluate` itself.
//! - `Input.dispatchMouseEvent`: `click` (moved, pressed, released), `drag` (pressed, held move,
//!   released), `hover` (moved) and `scroll` (mouse wheel).
//! - `Input.insertText`: `type`. `Input.dispatchKeyEvent`: `press` and the Enter of a submitting
//!   `type`.

use super::action::{Action, Button, Direction, Key, Kind, Load, Target};
use super::decisions::{Point, SnapshotPass, TARGET_CRASHED, crash_notice, drag_points, needs_visual, snapshot_pass};
use super::idle;
use super::snapshot::{self, format as format_snapshot};
use super::{cut_at, folder_slug};
use crate::LockExt as _;
use crate::agent::process::Process;
use crate::hub::{BotStatus, Hub};
use crate::screen::browser_cards::SignInProgress;
use crate::screen::protected_browser::{
    browser_executable,
    pipe::{self, Pipe, rpc},
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
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
/// OpenBot 0.78.0: bound the crash check so a renderer that stopped answering cannot hold the tool
/// call after its own failure.
const CRASH_PROBE: Duration = Duration::from_millis(600);
/// OpenBot 0.74.0: a close cannot wait for an earlier action that a page which stopped answering
/// holds. Wait this long for the tab to free itself, then close the browser under the blocked
/// action so the queued calls end and the agent's turn does not stay on "working".
const CLOSE_QUEUE_WAIT: Duration = Duration::from_secs(10);
/// Grace period after the browser is closed for its queued calls to end on the closed page.
const CLOSED_PAGE_DRAIN: Duration = Duration::from_secs(2);
/// After a page script, a saved login or card fill can still be blocked by that script living on in
/// another tab of the site, including a popup the agent never scripted. A secret is only ever sent
/// to the page by a secure fill, never returned to the agent; Codync's browser tools have no such
/// fill today, so this reason goes back with `evaluate` as guidance.
const AGENT_SCRIPTED_TABS_NOTE: &str = "A script you ran with evaluate can still run in this page or another open page of this site. First close or navigate each other tab of this site, including a popup or opener tab where you did not run evaluate. A tab of another chat or agent can also block a secure fill, and it is not in this bot's tab list: when you see no other tab of this site, ask the user to close the other tabs of this site. Then reload this page with navigate direction=reload, not with evaluate or by opening its address again, take a fresh snapshot, and retry.";

/// The page-side helpers, evaluated with each call: an expression that builds the page's API.
const PAGE: &str = include_str!("page.js");

/// One bot's browser, or `None` while it has none. The live tab shares this slot with the tools:
/// there is still one browser per bot, and each CDP call briefly borrows the pipe.
pub(super) type Slot = Arc<AsyncMutex<Option<Session>>>;
static SESSIONS: LazyLock<Mutex<HashMap<String, Slot>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct Session {
    process: Process,
    folder: PathBuf,
    pipe: Pipe,
    /// The browser context Codync created for this bot; no page outside it is touched.
    context: String,
    /// The tab every call acts on.
    target: String,
    /// Last time this tab saw a CDP call (a tool action, a live frame or an input event).
    last_activity: Instant,
    /// Last time a CDP call actually answered. A page with a script that never ends keeps taking
    /// calls but returns nothing, so only this clock can tell the agent the page is stalled.
    last_response: Instant,
    /// OpenBot 0.78.0: the fixed crash notice from a failed call, kept until `perform` reports it
    /// instead of replacing it with a generic timeout message.
    crash: Option<&'static str>,
    /// Whether `Page.setWebLifecycleState('frozen')` is in effect and has not been undone.
    frozen: bool,
}

impl Session {
    fn touch(&mut self) {
        self.last_activity = Instant::now();
    }
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
        // Any tool use is activity: it pushes the freeze deadline out and wakes a frozen tab.
        session.touch();
        timeout(action.timeout.saturating_add(HUNG), run(session, action)).await
    };
    match answered {
        Ok(answer) => {
            if let Some(session) = guard.as_mut().and_then(|held| held.as_mut()) {
                session.last_response = Instant::now();
            }
            answer
        }
        Err(_) => {
            // 0.77.0: a page whose script never ends stops answering. If the last CDP answer was
            // longer ago than the stall window, say so plainly and name the fix instead of only
            // reporting a timeout. 0.78.0: a crash notice takes priority over both.
            let (crashed, stalled) = {
                let session = guard.as_mut().and_then(|held| held.as_mut());
                (
                    session.as_mut().and_then(|session| session.crash.take()),
                    session.is_some_and(|session| {
                        crate::screen::browser_cards::browser_page_stalled(
                            session.last_response,
                            Instant::now(),
                            crate::screen::browser_cards::STALL_AFTER,
                        )
                    }),
                )
            };
            // A cancelled read leaves the pipe unsure of where the next reply starts, so the browser
            // is stopped instead of reused.
            stop(guard.take()).await;
            if let Some(notice) = crashed {
                bail!("{notice}")
            }
            if stalled {
                bail!(
                    "{} The page was silent for over {} seconds, so the browser was closed. Open it again with navigate direction=reload, then take a new snapshot.",
                    crate::screen::browser_cards::PAGE_NOT_ANSWERING,
                    crate::screen::browser_cards::STALL_AFTER.as_secs()
                )
            }
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

/// Stops a bot's browser and forgets it. OpenBot 0.74.0: an earlier action can hold the tab (a
/// snapshot on a page that stopped answering), so the close waits only briefly for it instead of
/// keeping the agent's turn on "working" until the user presses Stop.
async fn close(bot: &str) {
    let removed = SESSIONS.locked().remove(bot);
    let Some(slot) = removed else { return };
    let ended = match timeout(CLOSE_QUEUE_WAIT, slot.lock()).await {
        Ok(mut guard) => guard.take(),
        Err(_) => {
            tracing::warn!(bot = %bot, "A bot browser tab closed while an earlier action still held it");
            None
        }
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
    Ok(Session {
        process,
        folder,
        pipe: socket,
        context,
        target,
        last_activity: Instant::now(),
        last_response: Instant::now(),
        crash: None,
        frozen: false,
    })
}

/// Stops a browser and removes its profile, retrying a profile Chrome still holds. The terminate
/// itself is bounded: a page that stopped answering can keep a command pending for ever, and the
/// close must finish without waiting for it (OpenBot 0.74.0).
async fn stop(session: Option<Session>) {
    let Some(mut session) = session else { return };
    match timeout(CLOSED_PAGE_DRAIN, session.process.terminate_owned()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::warn!(%error, "could not stop a bot's browser"),
        Err(_) => tracing::warn!("A bot browser did not stop in time; finishing the close anyway"),
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

// --- Idle tab watcher -----------------------------------------------------------------------------
//
// OpenBot 0.75: "Unused browser tabs freeze to reduce resource use." Every session records its last
// CDP activity; one bounded task scans them and freezes the tabs that have been quiet for five
// minutes. A tab watched live, held by a tool call or owned by a bot on a turn is left alone. The page
// is woken on the next frame, input event or tool call, so frames and input keep working.

/// Starts the single watcher task on the first browser call. It reads only these sessions and the
/// hub's bot status; it stops with the host.
pub(super) fn start_watcher(hub: &Arc<Hub>) {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    let hub = Arc::clone(hub);
    tokio::spawn(async move {
        loop {
            sleep(idle::SCAN_EVERY).await;
            watch(&hub).await;
        }
    });
}

/// Freezes or wakes this tab through a short-lived CDP session on the bot's own target.
async fn set_lifecycle(session: &mut Session, state: &'static str) -> Result<()> {
    let tab = attach(session).await?;
    let answer = rpc(&mut session.pipe, "Page.setWebLifecycleState", json!({"state": state}), Some(tab.as_str())).await;
    rpc(&mut session.pipe, "Target.detachFromTarget", json!({"sessionId": tab.as_str()}), None).await.ok();
    answer.map(|_| ())
}

/// Wakes a frozen tab before the next CDP action. A failed wake reaches the caller, whose `perform`
/// closes a browser whose pipe a cancelled read poisoned.
async fn resume(session: &mut Session) -> Result<()> {
    if session.frozen {
        set_lifecycle(session, "active").await?;
        session.frozen = false;
    }
    Ok(())
}

/// One bounded pass over all sessions. A slot held by a call or a live frame is never frozen.
async fn watch(hub: &Hub) {
    let sessions: Vec<(String, Slot)> = {
        let registry = SESSIONS.locked();
        registry.iter().map(|(bot, slot)| (bot.clone(), slot.clone())).collect()
    };
    for (bot, slot) in sessions {
        // A held slot means a tool call or live frame owns the tab: the pure rule keeps it awake.
        let (mut guard, pending) = match slot.try_lock() {
            Ok(guard) => (Some(guard), false),
            Err(_) => (None, true),
        };
        let live = super::live::is_watched(&bot).await;
        let turn_running = matches!(hub.runtime(&bot).status, BotStatus::Working | BotStatus::NeedsInput);
        let known = guard.as_ref().and_then(|held| held.as_ref());
        let state = idle::Idle {
            idle_for: known.map(|session| session.last_activity.elapsed()).unwrap_or_default(),
            live,
            pending,
            turn_running,
            frozen: known.is_some_and(|session| session.frozen),
        };
        if !idle::should_freeze(state, idle::IDLE_AFTER) {
            continue;
        }
        // The borrow ends with the await, so the arms below can take the whole session on a timeout.
        let freeze = {
            let Some(session) = guard.as_mut().and_then(|held| held.as_mut()) else { continue };
            if session.process.exited().unwrap_or(true) {
                continue;
            }
            timeout(idle::FREEZE_TIMEOUT, set_lifecycle(session, "frozen")).await
        };
        match freeze {
            Ok(Ok(())) => {
                if let Some(session) = guard.as_mut().and_then(|held| held.as_mut()) {
                    session.frozen = true;
                }
            }
            Ok(Err(error)) => tracing::warn!(bot = %bot, %error, "could not freeze an idle bot tab"),
            Err(_) => {
                // A cancelled CDP read poisons the pipe; stop this browser instead of reusing it.
                tracing::warn!(bot = %bot, "an idle bot tab did not freeze in time; closing its browser");
                let dead = guard.as_mut().and_then(|held| held.take());
                stop(dead).await;
            }
        }
    }
}

/// One action on the bot's tab, with a session attached and detached around it.
async fn run(session: &mut Session, action: &Action) -> Result<Value> {
    // A frozen page is woken inside the action's own deadline, so a stuck resume is bounded like any
    // other CDP call here. Live views wake it on their frame and input paths (`live_frame`, `live_input`).
    resume(session).await?;
    ensure_target(session).await?;
    let tab = attach(session).await?;
    let answer = act(session, &tab, action).await;
    // OpenBot 0.78.0: a crashed page is reported as such, with the reload the agent can do, instead
    // of a generic refusal or a snapshot from before the crash. Check while the session is still
    // attached; a cancelled check poisons the pipe, which `perform` then closes rather than reuses.
    let crash = match &answer {
        Ok(_) => None,
        Err(_) => timeout(CRASH_PROBE, crash_reason(session, &tab)).await.ok().flatten(),
    };
    rpc(&mut session.pipe, "Target.detachFromTarget", json!({"sessionId": tab.as_str()}), None).await.ok();
    match answer {
        Ok(value) => Ok(value),
        Err(error) => match crash {
            Some(reason) => {
                let notice = crash_notice(reason);
                session.crash = Some(notice);
                bail!("{notice}")
            }
            None => Err(error),
        },
    }
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

/// OpenBot 0.78.0: `Inspector.targetCrashed` is the CDP event a dead renderer raises. The inherited
/// pipe forwards replies, not events, so the same Inspector agent is asked directly: `Inspector.enable`
/// cannot succeed on a crashed page, while the browser's own target list still names the tab. Only
/// the fixed event label comes back; neither the address nor the page text is read or logged.
async fn crash_reason(session: &mut Session, tab: &str) -> Option<&'static str> {
    if rpc(&mut session.pipe, "Inspector.enable", json!({}), Some(tab)).await.is_ok() {
        return None;
    }
    let targets = rpc(&mut session.pipe, "Target.getTargets", json!({}), None).await.ok()?;
    let listed = targets["targetInfos"]
        .as_array()
        .is_some_and(|pages| pages.iter().any(|page| page["targetId"] == session.target.as_str()));
    listed.then_some(TARGET_CRASHED)
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
        Kind::Drag { from, to } => drag(session, tab, *from, *to).await,
        Kind::Screenshot => capture(session, tab).await,
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
    // 0.77.0: "Reload, Back, Forward and a new address also failed" on a page whose script never
    // ends. Stop that script first, so every navigation control can load the new page.
    stop_loading(session, tab).await;
    let before = page(session, tab, "probe", &json!({})).await?["url"].as_str().unwrap_or_default().to_owned();
    match (url, direction) {
        (Some(url), None) => {
            let result = rpc(&mut session.pipe, "Page.navigate", json!({"url": url}), Some(tab)).await?;
            if let Some(error) = result["errorText"].as_str() {
                bail!("The page did not load: {error}");
            }
        }
        (None, Some(Direction::Reload)) => {
            // 0.70: a reload gets fresh bytes, not the cache (shared rule in browser_reload).
            let params =
                crate::screen::browser_reload::reload_strategy(crate::screen::browser_reload::ReloadKind::Agent);
            rpc(&mut session.pipe, "Page.reload", params, Some(tab)).await?;
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

/// Stops a page that stopped answering before a navigation. A refused stop is not fatal: the
/// navigation still gets its own bounded deadline (0.77.0).
async fn stop_loading(session: &mut Session, tab: &str) {
    let _ = rpc(&mut session.pipe, "Page.stopLoading", json!({}), Some(tab)).await;
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
    let read = call_page(session, tab, "snapshot", &snapshot_pass(SnapshotPass::Accessible)).await?;
    let mut text = format_snapshot(&read, snapshot::MAX_CHARS);
    let controls = read["elements"].as_array().map_or(0, Vec::len);
    let body = read["text"].as_str().unwrap_or_default();
    let mut needs_image = false;
    if controls == 0 {
        // 0.70: a page can paint controls it hides from screen readers or makes transparent. `innerText`
        // still sees their words, so an empty control tree — not just an empty page — tries the drawn
        // pass. A page with nothing in either pass becomes a picture (see `needs_visual`).
        let drawn = call_page(session, tab, "snapshot", &snapshot_pass(SnapshotPass::Drawn)).await?;
        let drawn_controls = drawn["elements"].as_array().map_or(0, Vec::len);
        let drawn_body = drawn["text"].as_str().unwrap_or_default();
        if drawn_controls > 0 || (body.trim().is_empty() && !drawn_body.trim().is_empty()) {
            text = format_snapshot(&drawn, snapshot::MAX_CHARS);
        } else if needs_visual(drawn_body, drawn_controls) {
            needs_image = true;
        }
    }
    Ok(json!({
        "url": read["url"],
        "title": read["title"],
        "snapshot": text,
        "needsImage": needs_image,
    }))
}

/// A PNG of the bot's tab as a data URL, with its address for the saved file's name (OpenBot 0.68).
/// Uses the same `Page.captureScreenshot` primitive as the panel's shot, on the tools' own tab.
async fn capture(session: &mut Session, tab: &str) -> Result<Value> {
    let image = rpc(
        &mut session.pipe,
        "Page.captureScreenshot",
        json!({"format": "png", "captureBeyondViewport": false}),
        Some(tab),
    )
    .await?;
    let data = image["data"].as_str().context("The tab gave no picture")?;
    let probe = page(session, tab, "probe", &json!({})).await.ok();
    let url = probe.as_ref().and_then(|answer| answer["url"].as_str()).unwrap_or_default();
    Ok(json!({"image": format!("data:image/png;base64,{data}"), "url": url}))
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

/// A real press–move–release drag between two refs (OpenBot 0.70), down the same mouse path `click`
/// uses. The pure rule checks both points before any input is sent, so a twitch is not sent as a drag.
async fn drag(session: &mut Session, tab: &str, from: Target, to: Target) -> Result<Value> {
    let (from_x, from_y, from_label) = point(session, tab, from).await?;
    let (to_x, to_y, to_label) = point(session, tab, to).await?;
    let (start, end) = drag_points(Point { x: from_x, y: from_y }, Point { x: to_x, y: to_y }).context(
        "The two elements are too close together or outside the page; move them apart and take a new snapshot.",
    )?;
    mouse(session, tab, json!({"type": "mouseMoved", "x": start.x, "y": start.y, "button": "none", "buttons": 0}))
        .await?;
    mouse(
        session,
        tab,
        json!({"type":"mousePressed","x":start.x,"y":start.y,"button":"left","buttons":1,"clickCount":1}),
    )
    .await?;
    // The button stays down across the move: that is what a page listens for as a drag.
    mouse(session, tab, json!({"type":"mouseMoved","x":end.x,"y":end.y,"button":"left","buttons":1})).await?;
    mouse(session, tab, json!({"type":"mouseReleased","x":end.x,"y":end.y,"button":"left","buttons":0,"clickCount":1}))
        .await?;
    Ok(json!({
        "dragged": from_label,
        "onto": to_label,
        "from": {"x": start.x, "y": start.y},
        "to": {"x": end.x, "y": end.y},
    }))
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
    let mut answer = json!({"typed": text.chars().count(), "into": into});
    if submit {
        let before = page(session, tab, "probe", &json!({}))
            .await
            .ok()
            .and_then(|probe| probe["url"].as_str().map(str::to_owned))
            .unwrap_or_default();
        press_key(session, tab, &Key::enter()).await?;
        // OpenBot nightly: a saved password's Enter can land on a slow page. Give it a bounded grace to
        // move so the agent reads the signed-in page, instead of handing the browser to the user.
        answer["signIn"] = json!(await_sign_in(session, tab, &before).await);
    }
    Ok(answer)
}

/// After a submitting `type`, polls until the site moves off the login form or the grace ends. The
/// answer carries only a fixed word; no address or page text is logged. This runs after the Enter
/// press only, so a password or payment approval card is never skipped.
async fn await_sign_in(session: &mut Session, tab: &str, url_before: &str) -> &'static str {
    let started = Instant::now();
    loop {
        let probe = match page(session, tab, "probe", &json!({})).await {
            Ok(probe) => probe,
            Err(error) => {
                // A page being replaced can refuse an evaluate. That is not a sign-in failure; the
                // existing handoff path stays exactly as it was.
                tracing::debug!(%error, "the sign-in page did not answer a probe");
                return "handoff";
            }
        };
        let url_after = probe["url"].as_str().unwrap_or_default();
        let still_on_form = probe["hasPassword"] == true || crate::screen::browser_cards::is_sign_in_page(url_after);
        match crate::screen::browser_cards::sign_in_complete(url_before, url_after, still_on_form, started.elapsed()) {
            SignInProgress::Continue => return "complete",
            SignInProgress::Handoff => return "handoff",
            SignInProgress::Waiting => sleep(POLL).await,
        }
    }
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
        return Ok(json!({"value": value, "truncated": truncated, "note": AGENT_SCRIPTED_TABS_NOTE}));
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
    Ok(json!({"value": result["result"]["value"], "truncated": false, "note": AGENT_SCRIPTED_TABS_NOTE}))
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

// --- The live tab: watching and taking over one of the bot's open tabs -------------------------
//
// These reuse the same CDP pipe, the same per-bot browser slot and the same input path (`mouse` and
// `press_key`) as the tools. A live view attaches once to the tab the tools use, and every frame or
// input event briefly borrows the pipe, so the bot's own tools and a person's take-over cannot send
// CDP commands at the same time. No frame, title or address is logged here.

/// The furthest a pointer or wheel value may be from the page's origin, in CSS pixels.
const MAX_COORD: f64 = 100_000.0;

/// One input event a person sends to a taken-over tab. Parsed and bounded here, before it reaches
/// CDP: a pointer action or one key, never page text or a free-form payload.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Input {
    Move { x: f64, y: f64, buttons: u8 },
    Down { x: f64, y: f64, button: Button, buttons: u8, count: u8 },
    Up { x: f64, y: f64, button: Button, count: u8 },
    Wheel { x: f64, y: f64, dx: f64, dy: f64 },
    Key(Key),
}

impl Input {
    /// Parses `{kind, x, y, …}` (a pointer event) or `{kind: "key", key}` into a bounded event.
    pub(super) fn parse(value: &Value) -> Result<Self> {
        let coord = |key: &str| -> Result<f64> {
            let number = value.get(key).and_then(Value::as_f64).context(format!("`{key}` is a number"))?;
            ensure!(number.is_finite() && number.abs() <= MAX_COORD, "`{key}` is out of range");
            Ok(number)
        };
        let delta = |key: &str| -> Result<f64> {
            let Some(value) = value.get(key).filter(|value| !value.is_null()) else { return Ok(0.0) };
            let number = value.as_f64().context(format!("`{key}` is a number"))?;
            ensure!(number.is_finite() && number.abs() <= MAX_COORD, "`{key}` is out of range");
            Ok(number)
        };
        let button = || -> Result<Button> {
            match value.get("button").filter(|value| !value.is_null()) {
                None => Ok(Button::Left),
                Some(name) => Button::parse(name),
            }
        };
        let buttons = |default: u8| -> Result<u8> {
            let Some(value) = value.get("buttons").filter(|value| !value.is_null()) else { return Ok(default) };
            let mask = value.as_u64().context("`buttons` is a mouse button mask")?;
            u8::try_from(mask).ok().filter(|mask| *mask <= 7).context("`buttons` is out of range")
        };
        let count = || -> Result<u8> {
            let Some(value) = value.get("clickCount").filter(|value| !value.is_null()) else { return Ok(1) };
            let count = value.as_u64().context("`clickCount` is 1 or 2.")?;
            u8::try_from(count).ok().filter(|count| (1..=2).contains(count)).context("`clickCount` is 1 or 2.")
        };
        Ok(match value["kind"].as_str() {
            Some("mouseMove") => Self::Move { x: coord("x")?, y: coord("y")?, buttons: buttons(0)? },
            Some("mouseDown") => {
                let button = button()?;
                Self::Down {
                    x: coord("x")?,
                    y: coord("y")?,
                    button,
                    buttons: buttons(button.buttons())?,
                    count: count()?,
                }
            }
            Some("mouseUp") => Self::Up { x: coord("x")?, y: coord("y")?, button: button()?, count: count()? },
            Some("wheel") => {
                let (dx, dy) = (delta("deltaX")?, delta("deltaY")?);
                ensure!(dx.abs() > f64::EPSILON || dy.abs() > f64::EPSILON, "A wheel needs deltaX or deltaY.");
                Self::Wheel { x: coord("x")?, y: coord("y")?, dx, dy }
            }
            Some("key") => {
                let key = value.get("key").context("`key` names the key to press, such as Enter.")?;
                Self::Key(Key::parse(key)?)
            }
            _ => bail!("An input event is a mouseMove, mouseDown, mouseUp, wheel or key."),
        })
    }
}

/// Attaches a live view to the tab the bot's tools act on, returning the shared slot and the CDP
/// session id. The session stays attached; later calls reuse it instead of re-attaching each frame.
pub(super) async fn live_attach(bot: &str) -> Result<(Slot, String)> {
    let slot =
        existing(bot).await.context("This bot has no browser open yet. Open a page with the navigate tool first.")?;
    let mut guard = slot.lock().await;
    let session =
        guard.as_mut().context("This bot has no browser open yet. Open a page with the navigate tool first.")?;
    ensure!(!session.process.exited()?, "The browser closed. Open the page again with navigate.");
    // Watching is activity: it wakes a frozen tab and keeps it from freezing while frames flow.
    session.touch();
    resume(session).await?;
    ensure_target(session).await?;
    let tab = attach(session).await?;
    Ok((slot.clone(), tab))
}

/// One bounded JPEG frame and the tab's title and address, from the live session's tab.
pub(super) async fn live_frame(slot: &Slot, session: &str) -> Result<Value> {
    let mut guard = slot.lock().await;
    let state = guard.as_mut().context("The browser closed. Reopen the live tab.")?;
    ensure!(!state.process.exited()?, "The browser closed. Reopen the live tab.");
    state.touch();
    resume(state).await?;
    let image = rpc(
        &mut state.pipe,
        "Page.captureScreenshot",
        json!({"format": "jpeg", "quality": 45, "captureBeyondViewport": false}),
        Some(session),
    )
    .await?;
    let data = image["data"].as_str().context("The tab gave no image")?;
    // The frame is captured from the same tab the tools drive; only its browser-level target list
    // gives the title and address, so no page content is read just to label a frame.
    let targets = rpc(&mut state.pipe, "Target.getTargets", json!({}), None).await?;
    let info = targets["targetInfos"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|target| target["targetId"] == state.target.as_str());
    Ok(json!({
        "image": format!("data:image/jpeg;base64,{data}"),
        "title": info.and_then(|target| target["title"].as_str()).unwrap_or_default(),
        "url": info.and_then(|target| target["url"].as_str()).unwrap_or_default(),
    }))
}

/// Sends a validated pointer or key event down the same CDP input path the tools use.
pub(super) async fn live_input(slot: &Slot, session: &str, event: Input) -> Result<()> {
    let mut guard = slot.lock().await;
    let state = guard.as_mut().context("The browser closed. Reopen the live tab.")?;
    ensure!(!state.process.exited()?, "The browser closed. Reopen the live tab.");
    state.touch();
    resume(state).await?;
    match event {
        Input::Move { x, y, buttons } => {
            mouse(state, session, json!({"type": "mouseMoved", "x": x, "y": y, "button": "none", "buttons": buttons})).await
        }
        Input::Down { x, y, button, buttons, count } => {
            mouse(
                state,
                session,
                json!({"type": "mousePressed", "x": x, "y": y, "button": button.name(), "buttons": buttons, "clickCount": count}),
            )
            .await
        }
        Input::Up { x, y, button, count } => {
            mouse(
                state,
                session,
                json!({"type": "mouseReleased", "x": x, "y": y, "button": button.name(), "buttons": 0, "clickCount": count}),
            )
            .await
        }
        Input::Wheel { x, y, dx, dy } => {
            mouse(state, session, json!({"type": "mouseWheel", "x": x, "y": y, "deltaX": dx, "deltaY": dy})).await
        }
        Input::Key(key) => press_key(state, session, &key).await,
    }
}

/// Detaches the live session. A closed browser or a stale session is not an error here.
pub(super) async fn live_detach(slot: &Slot, session: String) {
    let mut guard = slot.lock().await;
    if let Some(state) = guard.as_mut() {
        rpc(&mut state.pipe, "Target.detachFromTarget", json!({"sessionId": session}), None).await.ok();
    }
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
    fn a_crashed_tab_is_reported_with_the_reload_it_offers() {
        assert_eq!(TARGET_CRASHED, "Inspector.targetCrashed");
        let notice = crash_notice(TARGET_CRASHED);
        assert!(notice.contains("page crashed"), "{notice}");
        assert!(notice.contains("navigate direction=reload"), "{notice}");
        assert!(CRASH_PROBE < Duration::from_secs(5), "a dead page must not hold the tool call");
    }

    #[test]
    fn a_page_script_gets_the_clear_other_tabs_reason() {
        let note = AGENT_SCRIPTED_TABS_NOTE;
        assert!(note.contains("each other tab of this site"), "{note}");
        assert!(note.contains("popup or opener tab"), "{note}");
        assert!(note.contains("ask the user to close"), "{note}");
        assert!(note.contains("navigate direction=reload"), "{note}");
        assert!(CLOSE_QUEUE_WAIT < Duration::from_secs(60), "a blocked close must finish");
    }

    #[test]
    fn what_a_wait_missed_is_named() {
        let text = missing_wait(Some("Inbox"), Some("/mail"), Some(Load::Load), Duration::from_secs(15));
        assert!(text.contains("show \"Inbox\""), "{text}");
        assert!(text.contains("reach an address with \"/mail\""), "{text}");
        assert!(text.contains("finish loading"), "{text}");
        assert!(text.contains("within 15 seconds"), "{text}");
    }

    #[test]
    fn a_taken_over_pointer_or_key_is_bounded_before_it_reaches_cdp() {
        assert_eq!(
            Input::parse(&json!({"kind": "mouseMove", "x": 12, "y": 30, "buttons": 1})).unwrap(),
            Input::Move { x: 12.0, y: 30.0, buttons: 1 }
        );
        assert_eq!(
            Input::parse(&json!({"kind": "mouseDown", "x": 1, "y": 2, "button": "right", "clickCount": 2})).unwrap(),
            Input::Down { x: 1.0, y: 2.0, button: Button::Right, buttons: 2, count: 2 }
        );
        assert_eq!(
            Input::parse(&json!({"kind": "mouseUp", "x": 1, "y": 2, "button": "middle"})).unwrap(),
            Input::Up { x: 1.0, y: 2.0, button: Button::Middle, count: 1 }
        );
        assert_eq!(
            Input::parse(&json!({"kind": "wheel", "x": 5, "y": 6, "deltaY": -120})).unwrap(),
            Input::Wheel { x: 5.0, y: 6.0, dx: 0.0, dy: -120.0 }
        );
        assert_eq!(
            Input::parse(&json!({"kind": "key", "key": "Control+A"})).unwrap(),
            Input::Key(Key::parse(&json!("Control+A")).unwrap())
        );
        for refused in [
            json!({"kind": "mouseMove", "x": 1}),
            json!({"kind": "mouseMove", "x": -100_001, "y": 0}),
            json!({"kind": "mouseMove", "x": 1, "y": 200_000}),
            json!({"kind": "mouseDown", "x": 1, "y": 2, "button": "side"}),
            json!({"kind": "mouseDown", "x": 1, "y": 2, "buttons": 9}),
            json!({"kind": "mouseDown", "x": 1, "y": 2, "clickCount": 0}),
            json!({"kind": "mouseUp", "x": 1, "y": 2, "clickCount": 3}),
            json!({"kind": "wheel", "x": 1, "y": 2}),
            json!({"kind": "wheel", "x": 1, "y": 2, "deltaX": 100_001}),
            json!({"kind": "key", "key": "Control+"}),
            json!({"kind": "key"}),
            json!({"kind": "teleport", "x": 1, "y": 2}),
            json!({}),
        ] {
            assert!(Input::parse(&refused).is_err(), "{refused} is not a live input event");
        }
    }
}
