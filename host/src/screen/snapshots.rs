//! Metadata-only freshness checks after a user takeover or permission revocation.

use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};

const MAX_IDS: usize = 64;
const STALE: &str = "The computer snapshot is stale or unknown. Capture a fresh snapshot before acting.";

#[derive(Default)]
struct BotSnapshots {
    fresh: bool,
    ids: VecDeque<String>,
}

/// Stores only driver-issued IDs, never screenshots, element data or tool arguments.
#[derive(Default)]
pub(super) struct SnapshotState {
    epoch: u64,
    bots: HashMap<String, BotSnapshots>,
}

impl SnapshotState {
    pub(super) fn invalidate(&mut self) {
        self.epoch = self.epoch.wrapping_add(1).max(1);
        self.bots.clear();
    }

    pub(super) fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Called only for successful capture tools, with their epoch from before the request.
    pub(super) fn record(&mut self, bot: &str, request_epoch: u64, result: &Value) {
        if request_epoch != self.epoch || result["isError"].as_bool() == Some(true) {
            return;
        }
        let metadata = &result["structuredContent"];
        if let Some(invalidated) = metadata["invalidated_snapshot_ids"].as_array() {
            let invalidated: Vec<String> = invalidated.iter().filter_map(identifier).collect();
            for state in self.bots.values_mut() {
                let previous = state.ids.len();
                state.ids.retain(|id| !invalidated.contains(id));
                if previous != state.ids.len() && state.ids.is_empty() {
                    state.fresh = false;
                }
            }
        }
        let state = self.bots.entry(bot.to_owned()).or_default();
        // Linux's plain screenshot has no driver-issued ID but still refreshes pixel actions.
        state.fresh = true;
        for field in ["snapshot_id", "capture_id", "screenshot_capture_id"] {
            if let Some(id) = identifier(&metadata[field]) {
                state.ids.retain(|existing| existing != &id);
                state.ids.push_back(id);
                while state.ids.len() > MAX_IDS {
                    state.ids.pop_front();
                }
            }
        }
    }

    pub(super) fn validate(&self, bot: &str, args: &Value) -> Result<()> {
        // Avoid imposing a new token format on drivers until the first invalidation.
        if self.epoch == 0 {
            return Ok(());
        }
        let Some(state) = self.bots.get(bot).filter(|state| state.fresh) else {
            bail!(STALE);
        };
        if let Some(token) = args.get("element_token") {
            let Some((snapshot, index)) = token.as_str().and_then(|token| token.rsplit_once(':')) else {
                bail!(STALE);
            };
            if snapshot.is_empty()
                || index.is_empty()
                || !index.bytes().all(|byte| byte.is_ascii_digit())
                || !state.ids.iter().any(|id| id == snapshot)
            {
                bail!(STALE);
            }
        }
        if let Some(capture) = args.get("capture_id") {
            let Some(capture) = identifier(capture) else {
                bail!(STALE);
            };
            if !state.ids.contains(&capture) {
                bail!(STALE);
            }
        }
        Ok(())
    }

    pub(super) fn forget(&mut self, bot: &str) {
        self.bots.remove(bot);
    }
}

fn identifier(value: &Value) -> Option<String> {
    match value {
        Value::String(id) if !id.is_empty() => Some(id.clone()),
        Value::Number(id) => Some(id.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
