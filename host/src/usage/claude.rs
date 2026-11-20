//! Claude plan windows: the ACP adapter's own `/usage` report for the signed-in
//! account, the CLI's `/usage` report, the status-line hook, and the ACP
//! rate-limit event.

use super::Window;
use crate::agent::acp::{Acp, Incoming};
use crate::agent::auth;
use crate::hub::Hub;
use crate::store::Store;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_TEXT: usize = 64 * 1024;

/// `claude -p /usage`: Claude Code's own local usage report, no model call. The
/// ACP adapter answers the same command for the session's signed-in account; it
/// is tried first because the CLI can be on another account.
pub(super) async fn collect(store: &Store) -> Option<Vec<Window>> {
    collect_with(store, &[]).await
}

/// The same for an isolated account, launched with its profile environment. OpenBot 0.70.0: the
/// CLI fallback obeys "Use my Claude Code setup" (see [`collect`]), so a slow user setup can't time
/// out the check; the ACP probe already runs in the host's probe folder, away from the user's projects.
pub(super) async fn collect_with(store: &Store, env: &[(String, String)]) -> Option<Vec<Window>> {
    if let Some(windows) = acp_usage(store, env).await {
        return Some(windows);
    }
    let use_setup = crate::agent::own_memory::claude_checks_use_setup(&store.bots().unwrap_or_default());
    cli_usage(env, use_setup).await
}

async fn acp_usage(store: &Store, env: &[(String, String)]) -> Option<Vec<Window>> {
    let Ok(auth::Started { acp, mut inbox, .. }) = auth::start_registry(store, "claude", env).await else {
        return None;
    };
    let result = async {
        let session =
            request(&acp, &mut inbox, "session/new", json!({"cwd": auth::probe_dir(), "mcpServers": []}), None)
                .await
                .ok()?;
        let session_id = session["sessionId"].as_str()?.to_owned();
        let mut text = String::new();
        request(
            &acp,
            &mut inbox,
            "session/prompt",
            json!({"sessionId": session_id, "prompt": [{"type": "text", "text": "/usage"}]}),
            Some((&session_id, &mut text)),
        )
        .await
        .ok()?;
        let windows = parse_acp_usage(&text);
        (!windows.is_empty()).then_some(windows)
    }
    .await;
    acp.kill().await;
    result
}

async fn request(
    acp: &Arc<Acp>,
    inbox: &mut mpsc::UnboundedReceiver<Incoming>,
    method: &str,
    params: Value,
    mut capture: Option<(&str, &mut String)>,
) -> Result<Value> {
    let pending = acp.request(method, params);
    tokio::pin!(pending);
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            tokio::select! {
                result = &mut pending => break result,
                incoming = inbox.recv() => consume(acp, incoming, &mut capture).await?,
            }
        }
    })
    .await??;
    while let Ok(incoming) = inbox.try_recv() {
        consume(acp, Some(incoming), &mut capture).await?;
    }
    Ok(result)
}

async fn consume(acp: &Acp, incoming: Option<Incoming>, capture: &mut Option<(&str, &mut String)>) -> Result<()> {
    match incoming {
        Some(Incoming::Request { id, .. }) => {
            acp.respond_error(id, -32601, "tools are unavailable during usage inspection").await?;
        }
        Some(Incoming::Notification { method, params }) => {
            if let Some((session, text)) = capture.as_mut()
                && method == "session/update"
                && params["sessionId"].as_str() == Some(*session)
                && params["update"]["sessionUpdate"] == "agent_message_chunk"
                && let Some(chunk) = params["update"]["content"]["text"].as_str()
            {
                if text.len().saturating_add(chunk.len()) > MAX_TEXT {
                    bail!("usage response too large")
                }
                text.push_str(chunk);
            }
        }
        Some(Incoming::Closed { .. }) | None => bail!("usage inspection ended"),
    }
    Ok(())
}

/// The length of a Claude plan window. The adapter, the CLI and the status line name it only by kind
/// or label (`five_hour`, `5-hour`, `Weekly · Opus`), so the length is derived from that kind rather
/// than read from the provider.
fn derived_seconds(kind: &str) -> Option<i64> {
    if kind == "five_hour" {
        return Some(5 * 60 * 60);
    }
    kind.starts_with("seven_day").then_some(7 * 24 * 60 * 60)
}

/// A Claude window carrying the length its kind implies.
fn plan_window(id: &str, label: String, percent: f64, resets_at: Option<i64>) -> Window {
    Window::new(id, label, percent, resets_at).lasting(derived_seconds(id))
}

/// The ACP adapter's Markdown report: one `**label** - **N%** - Resets ...`
/// line per window. The separators are read as Markdown markers, not as
/// particular punctuation, so the report parses whatever dash and dot it uses.
pub fn parse_acp_usage(text: &str) -> Vec<Window> {
    let mut windows = Vec::new();
    for line in text.lines().map(str::trim) {
        let Some(rest) = line.strip_prefix("**") else { continue };
        let Some((label, rest)) = rest.split_once("**") else { continue };
        let Some((_, rest)) = rest.split_once("**") else { continue };
        let Some((percent, rest)) = rest.split_once("**") else { continue };
        let Ok(percent) = percent.trim().trim_end_matches('%').parse::<f64>() else { continue };
        if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
            continue;
        }
        let Some((id, name)) = acp_window(label) else { continue };
        let resets = rest.split_once("Resets ").map(|(_, r)| r.trim()).filter(|r| !r.is_empty());
        windows.push(Window { resets_text: resets.map(str::to_owned), ..plan_window(&id, name, percent, None) });
    }
    windows
}

fn acp_window(label: &str) -> Option<(String, String)> {
    if label == "5-hour limit" {
        return Some(("five_hour".to_owned(), "5-hour".to_owned()));
    }
    let rest = label.strip_prefix("Weekly")?;
    let model = rest.trim_start_matches(|c: char| !c.is_alphanumeric()).trim();
    if model.is_empty() || model.eq_ignore_ascii_case("all models") {
        return Some(("seven_day".to_owned(), "Weekly".to_owned()));
    }
    Some((format!("seven_day_{}", label_key(model)), format!("Weekly · {model}")))
}

fn label_key(label: &str) -> String {
    label.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect()
}

async fn cli_usage(env: &[(String, String)], use_setup: bool) -> Option<Vec<Window>> {
    let bin = super::command::program("claude")?;
    let home = dirs::home_dir()?;
    let probe = crate::service::data_dir().join("probe");
    let out = tokio::time::timeout(
        Duration::from_mins(1),
        tokio::process::Command::new(bin)
            .args(["-p", "/usage", "--output-format", "json", "--no-session-persistence"])
            .current_dir(crate::agent::config::claude_check_dir(&home, &probe, use_setup))
            .envs(env.iter().map(|(key, value)| (key, value)))
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    let windows = parse_claude_usage(v["result"].as_str()?);
    (!windows.is_empty()).then_some(windows)
}

/// Lines like `Current week (all models): 53% used · resets Sep 26 at 12pm (Asia/Taipei)`.
pub fn parse_claude_usage(text: &str) -> Vec<Window> {
    text.lines()
        .filter_map(|line| {
            let (name, rest) = line.trim().split_once(": ")?;
            let percent: f64 = rest.split_once("% used")?.0.trim().parse().ok()?;
            let resets = rest.split_once("resets ").map(|(_, r)| r.trim().to_owned());
            let (id, label) = match name {
                "Current session" => ("five_hour".to_owned(), "5-hour".to_owned()),
                "Current week (all models)" => ("seven_day".to_owned(), "Weekly".to_owned()),
                n => {
                    let model = n.strip_prefix("Current week (")?.strip_suffix(')')?;
                    (format!("seven_day_{}", model.to_lowercase()), format!("Weekly · {model}"))
                }
            };
            Some(Window { resets_text: resets, ..plan_window(&id, label, percent, None) })
        })
        .collect()
}

/// `POST /ingest/statusline` body: Claude Code's statusline JSON.
pub fn ingest_statusline(hub: &Hub, body: &Value) {
    let rl = &body["rate_limits"];
    let windows: Vec<Window> = [("five_hour", "5-hour"), ("seven_day", "Weekly")]
        .into_iter()
        .filter_map(|(key, label)| {
            let percent = rl[key]["used_percentage"].as_f64()?;
            let resets_at = rl[key]["resets_at"].as_i64().map(|s| s.saturating_mul(1000));
            Some(plan_window(key, label.to_owned(), percent, resets_at))
        })
        .collect();
    if !windows.is_empty() {
        super::merge(hub, "claude", "Claude", "statusline", windows);
    }
}

/// Claude ACP adapter: `usage_update._meta["_claude/rateLimit"]` (the SDK's `SDKRateLimitInfo`).
pub fn ingest_claude_rate_limit(hub: &Hub, info: &Value) {
    if let Some(w) = claude_window(info) {
        super::merge(hub, "claude", "Claude", "agent", vec![w]);
    }
}

fn claude_window(info: &Value) -> Option<Window> {
    let kind = info["rateLimitType"].as_str()?;
    let label = match kind {
        "five_hour" => "5-hour",
        "seven_day" => "Weekly",
        "seven_day_opus" => "Weekly · Opus",
        "seven_day_sonnet" => "Weekly · Sonnet",
        _ => return None,
    };
    let u = info["utilization"].as_f64()?;
    // The SDK reports a 0-1 fraction (from the unified rate-limit headers).
    let percent = if u <= 1.0 { u * 100.0 } else { u };
    // Seconds, but accept milliseconds too.
    let resets = info["resetsAt"].as_i64().map(|t| if t > 100_000_000_000 { t } else { t.saturating_mul(1000) });
    Some(plan_window(kind, label.to_owned(), percent, resets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_usage_report() {
        let w = parse_claude_usage(
            "You are currently using your subscription\n\nCurrent session: 11% used · resets Sep 25 at 5am (Asia/Taipei)\nCurrent week (all models): 53% used · resets Sep 26 at 12pm (Asia/Taipei)\nCurrent week (Fable): 83% used · resets Sep 26 at 12pm (Asia/Taipei)\n\nLast 24h · 794 requests",
        );
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].id, "five_hour");
        assert!((w[0].percent - 11.0).abs() < f64::EPSILON);
        assert_eq!(w[0].resets_text.as_deref(), Some("Sep 25 at 5am (Asia/Taipei)"));
        assert_eq!(w[0].window_seconds, Some(18_000), "the 5-hour label gives the length");
        assert_eq!(w[1].window_seconds, Some(604_800), "a weekly limit is a week long");
        assert_eq!(w[2].label, "Weekly · Fable");
    }

    #[test]
    fn parses_the_acp_markdown_report() {
        let w = parse_acp_usage(
            "## Usage\n\n### Limits\n\n**5-hour limit** — **5%** · Resets Nov 5, 2:00 AM GMT-6\n\n**Weekly · all models** — **6%** · Resets Nov 10, 2:00 PM GMT-6\n\n**Weekly · Opus** — **0%** · Resets Nov 10, 2:00 PM GMT-6",
        );
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].id, "five_hour");
        assert!((w[0].percent - 5.0).abs() < f64::EPSILON);
        assert_eq!(w[0].resets_text.as_deref(), Some("Nov 5, 2:00 AM GMT-6"));
        assert_eq!(w[0].window_seconds, Some(18_000));
        assert_eq!(w[1].label, "Weekly");
        assert_eq!(w[2].label, "Weekly · Opus");
        assert_eq!(w[2].window_seconds, Some(604_800), "a per-model weekly limit is a week long");
    }

    #[test]
    fn acp_report_ignores_other_bold_lines() {
        let w = parse_acp_usage(
            "**Last 24h** — 23 requests · 14 sessions\n\n**92%** See what fills your context\n\n**5-hour limit** — **7%** · Resets Nov 5, 5:09 PM CST",
        );
        assert_eq!(w.len(), 1);
        assert!((w[0].percent - 7.0).abs() < f64::EPSILON);
    }

    #[test]
    fn claude_rate_limit_fraction_becomes_percent() {
        let w = claude_window(&json!({"rateLimitType": "five_hour", "utilization": 0.42, "resetsAt": 1_790_000_000}))
            .unwrap();
        assert!((w.percent - 42.0).abs() < 1e-9);
        assert_eq!(w.resets_at, Some(1_790_000_000_000));
        assert_eq!(w.window_seconds, Some(18_000));
        assert!(claude_window(&json!({"rateLimitType": "overage", "utilization": 0.1})).is_none());
    }
}
