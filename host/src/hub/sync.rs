//! Bot snapshots shared by full, incremental and live synchronization.

use super::Hub;
use crate::LockExt;
use crate::store::BotRow;
use anyhow::Result;
use serde_json::{Value, json};

impl Hub {
    pub fn bot_json(&self, row: &BotRow) -> Value {
        let cfg = &row.config;
        let mut v = serde_json::to_value(cfg).expect("BotConfig is plain data and always serializes");
        v["managedWorkspace"] = crate::agent::workspace::is_managed(cfg).into();
        // Null where there is no switch to show.
        v["harnessMemory"] =
            (!cfg.is_group() && crate::agent::own_memory::switchable(cfg)).then_some(cfg.harness_memory).into();
        // Null where Codync doesn't summarize the chat. The sizes it may take: none until the agent tells its window.
        // A channel has no agent of its own: Compact context is hidden unless one of its members can do it.
        let compacts = if cfg.is_group() {
            crate::chat::group::channel_compact_available(&self.store, cfg)
        } else {
            crate::agent::bot::offers_threshold(&self.store, cfg)
        };
        v["compaction"] = if compacts { v["compaction"].take() } else { Value::Null };
        let window = compacts
            .then(|| {
                if cfg.is_group() {
                    cfg.members
                        .iter()
                        .filter_map(|id| self.store.bot(id).ok().flatten())
                        .filter(|row| crate::agent::bot::offers_threshold(&self.store, &row.config))
                        .find_map(|row| crate::agent::bot::context_window(&self.store, &row.config.id))
                } else {
                    crate::agent::bot::context_window(&self.store, &cfg.id)
                }
            })
            .flatten();
        v["compactionTokens"] = json!(window.and_then(codync_mem::compaction::sizes));
        if !cfg.is_group() {
            v["harnessMemoryShared"] = crate::agent::own_memory::shared(cfg).into();
            v["sharedMemory"] = crate::chat::memory::shared(cfg).into();
            v["memoryRev"] = self.store.memory_rev().into();
            v["devServers"] = self.dev_servers.wire(&cfg.id);
        }
        let rt = if cfg.is_group() {
            // A group has no agent: Fast mode belongs to its members.
            crate::hub::Runtime {
                fast_mode: crate::agent::config::FastModeSupport::default(),
                ..self.group_runtime(cfg)
            }
        } else {
            self.runtime.locked().get(&cfg.id).cloned().unwrap_or_default()
        };
        let last = self.store.last_message(&cfg.id);
        v["rev"] = row.rev.into();
        v["deleted"] = row.deleted.into();
        v["status"] = json!(rt.status);
        v["activity"] = rt.activity.into();
        v["startedAt"] = rt.started_at.into();
        v["workingChat"] = rt.lane.as_ref().map(|l| l.chat.clone()).into();
        v["fastModeSupport"] = rt.fast_mode.as_str().into();
        if !cfg.is_group() {
            v["agentStep"] = json!(rt.agent_step);
            v["resting"] = json!(crate::agent::fallback::resting(&self.store, cfg, crate::store::now_ms()));
        }
        v["fastModeIssue"] = rt.fast_mode.issue().into();
        v["unread"] = self.store.unread(&cfg.id, row.read_rev).into();
        v["readRev"] = row.read_rev.into();
        let preview = last.as_ref().map(|l| match &l.author {
            // A group names who spoke.
            Some(author) if cfg.is_group() => format!("{}: {}", self.bot_name(author), l.text),
            Some(_) => l.text.clone(),
            None => format!("You: {}", l.text),
        });
        v["lastMessage"] = preview.map(|p| crate::agent::acp::truncate(&p, 280)).into();
        v["lastAt"] = last.map_or(cfg.created_at, |l| l.at).into();
        v
    }

    /// Full refreshes retain cached client state, so they need deletion records too.
    pub fn bots_json(&self, since: i64) -> Result<Vec<Value>> {
        Ok(self.store.bots()?.iter().filter(|b| b.rev > since).map(|b| self.bot_json(b)).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::hub::Hub;
    use crate::remote::identity::Identity;
    use crate::store::Store;
    use serde_json::json;

    #[test]
    fn a_bot_offers_compaction_sizes_once_its_agent_tells_the_window() {
        let dir = std::env::temp_dir().join(format!("codync-compaction-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        for (id, backend) in [("c", "claude"), ("g", "gemini")] {
            let bot = json!({"id": id, "name": id, "backend": backend, "cwd": "/tmp", "compaction": 300_000});
            store.save_bot(&serde_json::from_value(bot).unwrap()).unwrap();
        }
        let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0);

        let claude = hub.bot_value_for_test("c");
        assert_eq!(claude["compaction"], 300_000);
        assert!(claude["compactionTokens"].is_null(), "no window reported yet");
        hub.store.kv_set("context.window.c", "1000000").unwrap();
        assert_eq!(
            hub.bot_value_for_test("c")["compactionTokens"],
            json!({"min": 50_000, "max": 960_000, "step": 10_000, "presets": (1..=9).map(|n| n * 100_000).collect::<Vec<u64>>()})
        );

        let gemini = hub.bot_value_for_test("g");
        assert!(gemini["compaction"].is_null() && gemini["compactionTokens"].is_null(), "Codync can't compact it");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_channel_hides_compact_context_unless_a_member_can_compact() {
        let dir = std::env::temp_dir().join(format!("codync-channel-compaction-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(&dir.join("t.db")).unwrap();
        for (id, backend) in [("a", "claude"), ("b", "gemini")] {
            let bot = json!({"id": id, "name": id, "backend": backend, "cwd": "/tmp"});
            store.save_bot(&serde_json::from_value(bot).unwrap()).unwrap();
        }
        let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0);
        let save = |members: &[&str]| {
            let group = json!({"id": "g", "name": "g", "kind": "group", "members": members, "compaction": 300_000});
            hub.store.save_bot(&serde_json::from_value(group).unwrap()).unwrap();
        };
        save(&["b"]);
        let card = hub.bot_value_for_test("g");
        assert!(card["compaction"].is_null(), "no member can compact this channel");

        save(&["b", "a"]);
        let card = hub.bot_value_for_test("g");
        assert!(card["compaction"].is_number(), "the Claude member makes Compact context available");
        assert!(card["compactionTokens"].is_null(), "no member reported a window yet");
        hub.store.kv_set("context.window.a", "1000000").unwrap();
        let card = hub.bot_value_for_test("g");
        assert!(card["compactionTokens"].is_object(), "the window comes from the member who can compact");
        hub.store.kv_set("context.window.b", "1000000").unwrap();
        assert_eq!(
            hub.bot_value_for_test("g")["compactionTokens"],
            json!({"min": 50_000, "max": 960_000, "step": 10_000, "presets": (1..=9).map(|n| n * 100_000).collect::<Vec<u64>>()})
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
