//! Read plan quotas only from adapters that handle `/usage` without a model turn.

use super::{Provider, Window};
use crate::LockExt;
use crate::agent::acp::{Acp, Incoming};
use crate::agent::{account, auth, backends};
use crate::hub::Hub;
use anyhow::{Result, bail};
use chrono::NaiveDateTime;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const MAX_TEXT: usize = 64 * 1024;

pub(super) async fn refresh(hub: &Hub) {
    if backends::which("agy").is_none() {
        return;
    }
    let profiles = match account::list(Some("antigravity")) {
        Ok(profiles) => profiles,
        Err(error) => {
            tracing::warn!(%error, "could not list Antigravity quota profiles");
            return;
        }
    };
    let mut targets = vec![(None, "Antigravity".to_owned())];
    targets.extend(profiles.into_iter().map(|p| (Some(p.id), format!("Antigravity · {}", p.name))));
    let ids: Vec<_> = targets.iter().map(|(account, _)| report_id(account.as_deref())).collect();
    let mut usage = hub.usage.locked().clone();
    usage.providers.retain(|p| !p.id.starts_with("antigravity:") || ids.contains(&p.id));
    hub.set_usage(usage);
    for (account, name) in targets {
        let (windows, status) = match probe(&hub.store, account.as_deref()).await {
            Ok(Some(windows)) if !windows.is_empty() => (windows, None),
            Ok(None) => (vec![], Some("unsupported")),
            Ok(Some(_)) => (vec![], Some("unavailable")),
            // No provider output or credentials enter logs. Missing quota is not zero.
            Err(_) => {
                tracing::debug!("Antigravity quota is unavailable");
                (vec![], Some("unavailable"))
            }
        };
        let id = report_id(account.as_deref());
        let mut usage = hub.usage.locked().clone();
        usage.providers.retain(|p| p.id != id);
        usage.providers.push(Provider {
            id,
            name,
            windows,
            source: "antigravity",
            updated_at: crate::store::now_ms(),
            status,
        });
        usage.providers.sort_by(|a, b| a.id.cmp(&b.id));
        hub.set_usage(usage);
    }
}

fn report_id(account: Option<&str>) -> String {
    account.map_or_else(|| "antigravity".into(), |id| format!("antigravity:{id}"))
}

async fn probe(store: &crate::store::Store, account: Option<&str>) -> Result<Option<Vec<Window>>> {
    let env = account.map(|id| account::env(id, "antigravity")).transpose()?.unwrap_or_default();
    let (acp, mut inbox, init, _) = auth::start_with_env(store, "antigravity", &env).await?;
    let result = async {
        if !supports_local_usage(&init) {
            return Ok(None);
        }
        let session =
            request(&acp, &mut inbox, "session/new", json!({"cwd": auth::probe_dir(), "mcpServers": []}), None).await?;
        let session_id = session["sessionId"].as_str().ok_or_else(|| anyhow::anyhow!("missing session ID"))?;
        let mut text = String::new();
        request(
            &acp,
            &mut inbox,
            "session/prompt",
            json!({"sessionId": session_id, "prompt": [{"type": "text", "text": "/usage"}]}),
            Some((session_id, &mut text)),
        )
        .await?;
        Ok(Some(parse(&text)))
    }
    .await;
    acp.kill().await;
    result
}

fn supports_local_usage(init: &Value) -> bool {
    let Some(version) = init["agentInfo"]["version"].as_str() else { return false };
    let Ok(parts) = version.trim_start_matches('v').split('.').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>()
    else {
        return false;
    };
    parts.len() == 3 && parts.as_slice() >= [1, 4, 1].as_slice()
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
    let result = tokio::time::timeout(Duration::from_secs(20), async {
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
            acp.respond_error(id, -32601, "tools are unavailable during quota inspection").await?;
        }
        Some(Incoming::Notification { method, params }) => {
            if let Some((session, text)) = capture.as_mut()
                && method == "session/update"
                && params["sessionId"].as_str() == Some(*session)
                && params["update"]["sessionUpdate"] == "agent_message_chunk"
                && let Some(chunk) = params["update"]["content"]["text"].as_str()
            {
                if text.len().saturating_add(chunk.len()) > MAX_TEXT {
                    bail!("quota response too large")
                }
                text.push_str(chunk);
            }
        }
        Some(Incoming::Closed { .. }) | None => bail!("quota inspection ended"),
    }
    Ok(())
}

fn parse(text: &str) -> Vec<Window> {
    let mut in_limits = false;
    let mut group = String::new();
    let mut windows = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            in_limits = line.trim_start_matches('#').trim() == "Quota Limits";
            continue;
        }
        if !in_limits {
            continue;
        }
        let Some(item) = line.strip_prefix("- **") else { continue };
        if let Some(label) = item.strip_suffix("**") {
            label.clone_into(&mut group);
            continue;
        }
        let Some((label, value)) = item.split_once(":** ") else { continue };
        let Some((remaining, reset)) = value.split_once("% remaining") else { continue };
        let Ok(remaining) = remaining.parse::<f64>() else { continue };
        if !remaining.is_finite() || !(0.0..=100.0).contains(&remaining) || group.is_empty() {
            continue;
        }
        let bucket = label.trim_end_matches(" Remaining");
        let resets = reset.trim().strip_prefix("(resets ").and_then(|s| s.strip_suffix(')'));
        let at = resets
            .and_then(|s| NaiveDateTime::parse_from_str(s, "%b %d, %Y, %H:%M UTC").ok())
            .map(|t| t.and_utc().timestamp_millis());
        windows.push(Window {
            id: format!("{group}:{bucket}"),
            label: format!("{group} · {bucket}"),
            percent: 100.0 - remaining,
            resets_at: at,
            resets_text: resets.filter(|_| at.is_none()).map(str::to_owned),
        });
    }
    windows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_sends_usage_to_old_or_unknown_adapters() {
        for version in ["1.3.0", "1.4.0", "1.4.1-beta", "unknown", "1.4"] {
            assert!(!supports_local_usage(&json!({"agentInfo": {"version": version}})));
        }
        assert!(!supports_local_usage(&json!({})));
        assert!(supports_local_usage(&json!({"agentInfo": {"version": "1.4.1"}})));
        assert!(supports_local_usage(&json!({"agentInfo": {"version": "1.5.0"}})));
    }

    #[test]
    fn keeps_model_groups_separate_and_converts_remaining_to_used() {
        let windows = parse(
            "### Quota Limits\n- **Gemini Models**\n  - **Weekly Limit Remaining:** 96% remaining (resets Oct 29, 2026, 10:12 UTC)\n- **Claude and GPT models**\n  - **Weekly Limit Remaining:** 100% remaining\n### Other\n- **Weekly Limit Remaining:** 50% remaining",
        );
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].percent, 4.0);
        assert_eq!(windows[1].percent, 0.0);
        assert_ne!(windows[0].id, windows[1].id);
        assert!(windows[0].resets_at.is_some());
        assert!(windows[1].resets_at.is_none());
    }

    #[test]
    fn unavailable_or_invalid_numbers_are_not_zero_usage() {
        assert_eq!(parse("Not applicable for current authentication method").len(), 0);
        assert_eq!(parse("### Quota Limits\n- **Gemini Models**\n- **Weekly Limit Remaining:** NaN% remaining\n- **Weekly Limit Remaining:** 101% remaining").len(), 0);
    }

    #[test]
    fn separate_accounts_have_separate_report_identities() {
        assert_ne!(report_id(None), report_id(Some("acc_12345678")));
        assert_ne!(report_id(Some("acc_12345678")), report_id(Some("acc_87654321")));
    }
}
