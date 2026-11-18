//! The browser tools a bot drives: navigate, snapshot, click, type, press, hover, scroll,
//! select_option, set_checked, wait_for, evaluate, and run. Ported from `OpenBot` nightly's
//! `src/backend/browser-tools.ts` and `browser-host.ts` (the tool names, descriptions and what each
//! action does), bridged onto Codync's own CDP pipe (`protected_browser::pipe`) instead of a second
//! browser stack: a bot gets one browser of its own, in a context Codync created, with one tab that
//! every call acts on. The page-side half is `page.js`; the bridge is `cdp.rs`, which needs Windows
//! like the rest of Codync's CDP browser.
//!
//! Limits are explicit: 20 s for one action, 25 s and 200 steps for a run, 20000 characters typed,
//! 64000 characters evaluated, 400 elements and 40000 characters of snapshot. A failure names what
//! was missing, as `The page has no element 7; take a new snapshot.`

// The bridge that reads these arguments and formats a snapshot is `cdp.rs`, which needs Windows like
// the rest of Codync's CDP browser. Everywhere else the parser is built and tested, and unused.
#![cfg_attr(not(windows), allow(dead_code, reason = "only the Windows CDP bridge reads these values"))]

use crate::hub::Hub;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

mod action;
#[cfg(windows)]
mod cdp;
mod snapshot;
mod tools;

use action::{Action, DEFAULT_TIMEOUT, Kind};

/// The tools this module answers, in the order the chat server lists them. Every name is also a
/// `run` step, except that a run adds `sleep`.
pub const TOOLS: [&str; 12] = [
    "navigate",
    "snapshot",
    "click",
    "type",
    "press",
    "hover",
    "scroll",
    "select_option",
    "set_checked",
    "wait_for",
    "evaluate",
    "run",
];

/// The tool definitions for the chat MCP server. Off Windows no CDP browser runs here, so nothing is
/// listed rather than a set of tools that always refuse.
pub fn tools() -> Vec<Value> {
    if cfg!(windows) { tools::definitions() } else { Vec::new() }
}

/// Runs one browser tool for `bot`.
pub async fn call(hub: &Hub, bot: &str, name: &str, args: &Value) -> Result<Value> {
    let action = Action::parse(name, args)?;
    // The same gate as the computer tools: computer access for this bot, with its browser permission.
    let lease = hub.computer.begin(hub, bot, crate::computer::Capability::Browser)?;
    execute(hub, bot, &lease, &action).await
}

/// Runs one parsed action: a run under its budget, or one step of it.
async fn execute(hub: &Hub, bot: &str, lease: &crate::computer::Lease, action: &Action) -> Result<Value> {
    lease.check()?;
    match &action.kind {
        Kind::Run { steps } => run(hub, bot, lease, action.timeout, steps).await,
        _ => step(hub, bot, lease, action).await,
    }
}

/// One action that is not a run, with the snapshot a call asks for after it.
async fn step(hub: &Hub, bot: &str, lease: &crate::computer::Lease, action: &Action) -> Result<Value> {
    lease.check()?;
    match &action.kind {
        Kind::Sleep { ms } => {
            tokio::time::sleep(Duration::from_millis(*ms)).await;
            Ok(json!({"slept": ms}))
        }
        // A run never holds another run, and a step's own `snapshot` is not repeated: the run answers
        // with one snapshot at its end.
        Kind::Run { .. } => bail!("A run cannot hold another run."),
        Kind::Snapshot => perform(hub, bot, action).await,
        _ => {
            let mut answer = perform(hub, bot, action).await?;
            if action.snapshot {
                answer["snapshot"] = json!(snapshot(hub, bot).await?);
            }
            Ok(answer)
        }
    }
}

async fn run(
    hub: &Hub,
    bot: &str,
    lease: &crate::computer::Lease,
    budget: Duration,
    steps: &[Action],
) -> Result<Value> {
    let started = Instant::now();
    for (index, step) in steps.iter().enumerate() {
        let left = budget.saturating_sub(started.elapsed());
        if left.is_zero() {
            bail!(
                "The run reached its {} second limit before step {} of {}; the steps before it stay done, and a snapshot shows where it is.",
                budget.as_secs(),
                index + 1,
                steps.len()
            );
        }
        lease.check()?;
        let mut step = step.clone();
        step.snapshot = false;
        step.timeout = step.timeout.min(left);
        if let Err(error) = step(hub, bot, lease, &step).await {
            bail!(
                "Step {} of {} ({}) failed and the run stopped: {error:#}. The steps before it stay done; take a snapshot to see the page.",
                index + 1,
                steps.len(),
                step.kind.name()
            );
        }
    }
    Ok(json!({"steps": steps.len(), "snapshot": snapshot(hub, bot).await?}))
}

/// The page's snapshot text, for a call that asks for one after its action.
async fn snapshot(hub: &Hub, bot: &str) -> Result<String> {
    let action = Action { timeout: DEFAULT_TIMEOUT, snapshot: false, kind: Kind::Snapshot };
    let answer = perform(hub, bot, &action).await?;
    Ok(answer["snapshot"].as_str().unwrap_or_default().to_owned())
}

async fn perform(_hub: &Hub, bot: &str, action: &Action) -> Result<Value> {
    #[cfg(windows)]
    {
        cdp::perform(bot, action).await
    }
    #[cfg(not(windows))]
    {
        let _ = (bot, action);
        bail!(
            "The browser tools drive Chrome or Edge on Windows; this computer's host runs on {}.",
            std::env::consts::OS
        )
    }
}

/// `text` cut to `max` characters, with a marker when it was longer.
pub(super) fn cut(text: &str, max: usize) -> String {
    let (text, truncated) = cut_at(text, max);
    if truncated { format!("{text}\n… (cut at {max} characters)") } else { text }
}

/// `text` cut to `max` characters, and whether it was longer.
pub(super) fn cut_at(text: &str, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text.to_owned(), false);
    }
    (text.chars().take(max).collect(), true)
}

/// A safe folder name for a bot's browser profile: its id, shortened and stripped of anything a path
/// could read as more than a name.
pub(super) fn folder_slug(bot: &str) -> String {
    let slug: String = bot
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '-' })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() { "bot".to_owned() } else { slug.chars().take(40).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cut_says_where_it_stopped_and_a_short_text_does_not() {
        assert_eq!(cut("short", 10), "short");
        assert_eq!(cut_at("short", 5), ("short".to_owned(), false));
        let (head, truncated) = cut_at("abcdef", 3);
        assert_eq!((head.as_str(), truncated), ("abc", true));
        assert_eq!(cut("abcdef", 3), "abc\n… (cut at 3 characters)");
        assert_eq!(cut("ééé", 2), "éé\n… (cut at 2 characters)", "characters, not bytes");
    }

    #[test]
    fn a_bot_id_makes_a_safe_short_folder() {
        assert_eq!(folder_slug(" 4f2a9b6c-1d3e "), "4f2a9b6c-1d3e");
        assert_eq!(folder_slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(folder_slug(""), "bot");
        assert_eq!(folder_slug(&"x".repeat(80)).len(), 40);
    }

    #[test]
    fn the_names_the_host_routes_are_the_names_the_server_lists() {
        if !cfg!(windows) {
            assert!(tools().is_empty(), "off Windows nothing is offered");
            return;
        }
        let listed: Vec<String> = tools().iter().filter_map(|tool| tool["name"].as_str().map(str::to_owned)).collect();
        assert_eq!(listed.len(), TOOLS.len());
        for name in TOOLS {
            assert!(listed.iter().any(|listed| listed == name), "{name} is listed");
        }
    }
}
