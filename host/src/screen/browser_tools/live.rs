//! The live tab: let a person watch one of a bot's open browser tabs and, while they hold control,
//! drive it with real pointer and key input. This is the take-over half of OpenBot's live browser
//! view, built on the same CDP bridge as the twelve tools (`cdp.rs`), not a second browser: a live
//! view attaches to the tab the tools drive, and every frame or input event briefly borrows the
//! pipe, so a person and the bot can never send CDP commands at the same time.
//!
//! Frames are bounded in both directions: the host captures at most two JPEGs a second and drops a
//! frame larger than `MAX_FRAME_CHARS`; the desk polls for the latest one. The title and address
//! come from the browser's own target list. Nothing here logs a frame, a title, an address or an
//! input value, and nothing is sent to analytics: the API methods carry no page content.

use super::cdp::{self, Slot};
use crate::computer::{Capability, Lease};
use crate::hub::Hub;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Two frames a second: a bare `Page.captureScreenshot` on every poll would flood the pipe.
const FRAME_INTERVAL: Duration = Duration::from_millis(500);
/// The most base64 JPEG characters held for a frame. Larger captures are dropped, keeping the last
/// small frame, so a busy page cannot fill memory or an API answer.
const MAX_FRAME_CHARS: usize = 400_000;
/// Stop capturing when nobody has polled for this long (a sheet closed without calling `stop`).
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);

/// One desk's live view of a bot's tab. The frame is behind its own lock so polls do not wait on
/// the registry; the task stops itself on revocation, idle polls or `stop`.
struct Live {
    slot: Slot,
    session: String,
    frame: Arc<AsyncMutex<Option<Value>>>,
    last_poll: Arc<StdMutex<Instant>>,
    /// Checked on every read and input so turning access off pauses this view too.
    lease: Lease,
    controlling: bool,
    done: Arc<AtomicBool>,
    task: JoinHandle<()>,
}

static LIVE: LazyLock<AsyncMutex<HashMap<String, Live>>> = LazyLock::new(|| AsyncMutex::new(HashMap::new()));

/// Whether a person has a live view of this bot's tab. The idle watcher never freezes one of these.
pub(super) async fn is_watched(bot: &str) -> bool {
    LIVE.lock().await.contains_key(bot)
}

/// Whether another frame is due after `elapsed` since the last one. Kept pure for the rate test.
fn frame_due(elapsed: Duration, interval: Duration) -> bool {
    elapsed >= interval
}

/// Whether a captured frame is small enough to hand to the desk. Kept pure for the size test.
fn frame_fits(frame: &Value) -> bool {
    frame["image"].as_str().is_some_and(|image| image.len() <= MAX_FRAME_CHARS)
}

/// A poll's answer: the latest frame and who is controlling, with no page text beyond the tab's own
/// title and address that a person is watching anyway.
fn answer(frame: Option<&Value>, controlling: bool) -> Value {
    match frame {
        Some(frame) => json!({
            "image": frame["image"], "title": frame["title"], "url": frame["url"], "controlling": controlling,
        }),
        None => json!({"image": null, "title": "", "url": "", "controlling": controlling}),
    }
}

fn touch(last_poll: &StdMutex<Instant>) {
    // A poisoned lock still has a usable value; keep polling rather than fail the view.
    let mut last = last_poll.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *last = Instant::now();
}

fn idle(last_poll: &StdMutex<Instant>) -> bool {
    let last = last_poll.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    last.elapsed() >= IDLE_TIMEOUT
}

/// Captures bounded frames until the capability is revoked, nobody polls, or the view is stopped.
async fn pump(
    slot: Slot,
    session: String,
    frame: Arc<AsyncMutex<Option<Value>>>,
    last_poll: Arc<StdMutex<Instant>>,
    lease: Lease,
    done: Arc<AtomicBool>,
) {
    let mut last_capture = Instant::now();
    loop {
        if lease.check().is_err() || idle(&last_poll) {
            break;
        }
        sleep(FRAME_INTERVAL).await;
        if lease.check().is_err() || idle(&last_poll) {
            break;
        }
        if frame_due(last_capture.elapsed(), FRAME_INTERVAL) {
            last_capture = Instant::now();
            match cdp::live_frame(&slot, &session).await {
                Ok(captured) if frame_fits(&captured) => *frame.lock().await = Some(captured),
                // An oversized or failed capture keeps the previous frame; no page text is logged.
                _ => {}
            }
        }
    }
    done.store(true, Ordering::Release);
    cdp::live_detach(&slot, session).await;
}

/// Aborts a view's capture task and detaches its CDP session, without waiting for a queued capture
/// on a stalled page.
async fn retire(live: Live) {
    live.done.store(true, Ordering::Release);
    live.task.abort();
    cdp::live_detach(&live.slot, live.session).await;
}

/// Attaches to a bot's open tab and starts its bounded frame stream.
async fn start(hub: &Hub, bot: &str) -> Result<Value> {
    let lease = hub.computer.begin(hub, bot, Capability::Browser)?;
    let mut registry = LIVE.lock().await;
    if let Some(previous) = registry.remove(bot) {
        retire(previous).await;
    }
    let (slot, session) = cdp::live_attach(bot).await?;
    // One frame up front, so a sheet opening does not show an empty picture for half a second.
    let first = cdp::live_frame(&slot, &session).await.ok().filter(|frame| frame_fits(frame));
    let frame = Arc::new(AsyncMutex::new(first.clone()));
    let last_poll = Arc::new(StdMutex::new(Instant::now()));
    let done = Arc::new(AtomicBool::new(false));
    let task = tokio::spawn(pump(
        slot.clone(),
        session.clone(),
        frame.clone(),
        last_poll.clone(),
        lease.clone(),
        done.clone(),
    ));
    registry.insert(bot.to_owned(), Live { slot, session, frame, last_poll, lease, controlling: false, done, task });
    Ok(answer(first.as_ref(), false))
}

/// The newest frame. Each read re-checks access, so a protected entry or a turned-off permission
/// stops the view instead of quietly streaming a page the person may no longer watch.
async fn frame(hub: &Hub, bot: &str) -> Result<Value> {
    hub.computer.begin(hub, bot, Capability::Browser)?;
    let registry = LIVE.lock().await;
    let live = registry.get(bot).context("Open the live tab first")?;
    live.lease.check()?;
    ensure!(!live.done.load(Ordering::Acquire), "The live tab ended. Open it again.");
    touch(&live.last_poll);
    let latest = live.frame.lock().await.clone();
    Ok(answer(latest.as_ref(), live.controlling))
}

/// Takes control: from here the desk's pointer and key events reach the tab. The fresh capability
/// gate means a protected entry in progress cannot be taken over.
async fn takeover(hub: &Hub, bot: &str) -> Result<Value> {
    hub.computer.begin(hub, bot, Capability::Browser)?;
    let mut registry = LIVE.lock().await;
    let live = registry.get_mut(bot).context("Open the live tab first")?;
    live.lease.check()?;
    ensure!(!live.done.load(Ordering::Acquire), "The live tab ended. Open it again.");
    live.controlling = true;
    Ok(answer(None::<&Value>, true))
}

/// Forwards one validated input event while control is held.
async fn input(hub: &Hub, bot: &str, body: &Value) -> Result<Value> {
    hub.computer.begin(hub, bot, Capability::Browser)?;
    let event = cdp::Input::parse(&body["event"])?;
    let registry = LIVE.lock().await;
    let live = registry.get(bot).context("Open the live tab first")?;
    live.lease.check()?;
    ensure!(!live.done.load(Ordering::Acquire), "The live tab ended. Open it again.");
    ensure!(live.controlling, "Take control of the tab first.");
    cdp::live_input(&live.slot, &live.session, event).await?;
    Ok(json!({"sent": true}))
}

/// Gives control back. Watching continues until the sheet closes.
async fn release(bot: &str) -> Result<Value> {
    let mut registry = LIVE.lock().await;
    if let Some(live) = registry.get_mut(bot) {
        live.controlling = false;
    }
    Ok(json!({"controlling": false}))
}

/// Stops the stream and detaches, called when the sheet closes.
async fn stop(bot: &str) -> Result<Value> {
    let mut registry = LIVE.lock().await;
    if let Some(live) = registry.remove(bot) {
        retire(live).await;
    }
    Ok(json!({"stopped": true}))
}

pub(super) async fn call(hub: &Hub, method: &str, body: &Value) -> Result<Value> {
    let bot = body["botId"].as_str().context("`botId` is required")?;
    match method {
        "browserLiveStart" => start(hub, bot).await,
        "browserLiveFrame" => frame(hub, bot).await,
        "browserLiveTakeover" => takeover(hub, bot).await,
        "browserLiveInput" => input(hub, bot, body).await,
        "browserLiveRelease" => release(bot).await,
        "browserLiveStop" => stop(bot).await,
        other => bail!("unknown browser live method `{other}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_due_only_after_the_interval() {
        assert_eq!(FRAME_INTERVAL, Duration::from_millis(500), "two frames a second");
        assert!(!frame_due(Duration::from_millis(499), FRAME_INTERVAL));
        assert!(frame_due(Duration::from_millis(500), FRAME_INTERVAL));
        assert!(frame_due(Duration::from_secs(2), FRAME_INTERVAL));
        assert!(IDLE_TIMEOUT > FRAME_INTERVAL, "an idle sheet stops the stream");
    }

    #[test]
    fn an_oversized_frame_is_dropped() {
        let small = json!({"image": "x".repeat(MAX_FRAME_CHARS), "title": "t", "url": "u"});
        let large = json!({"image": "x".repeat(MAX_FRAME_CHARS + 1), "title": "t", "url": "u"});
        assert!(frame_fits(&small));
        assert!(!frame_fits(&large));
        assert!(!frame_fits(&json!({"title": "t", "url": "u"})), "a frame without an image is not sent");
    }

    #[test]
    fn an_answer_carries_only_the_picture_the_label_and_control_state() {
        let frame = json!({"image": "data:image/jpeg;base64,AA", "title": "Inbox", "url": "https://example.com"});
        let answer = answer(Some(&frame), true);
        assert_eq!(answer["image"], frame["image"]);
        assert_eq!(answer["title"], "Inbox");
        assert_eq!(answer["url"], "https://example.com");
        assert_eq!(answer["controlling"], true);
        assert_eq!(answer.as_object().map_or(0, |fields| fields.len()), 4, "no extra fields");
        let empty = answer(None, false);
        assert_eq!(empty["image"], Value::Null);
        assert_eq!(empty["controlling"], false);
    }
}
