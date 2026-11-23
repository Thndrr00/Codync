//! The per-bot "OpenBot memory" switch (OpenBot nightly `t3-session-memory-providers`).
//!
//! A bot stores only `memory: false`; an absent field reads as "on", so saved bots need no
//! migration. Switching memory off takes away the saved-memory opening and the built-in memory
//! tools, but leaves the files on disk and the independent skill learning untouched.

use crate::store::BotConfig;

/// The built-in memory tools that stop working while the switch is off. Chat-history tools
/// (`search_history`, `read_history`) read the transcript rather than saved memory and stay.
pub const MEMORY_TOOLS: [&str; 7] = [
    "search_memory",
    "read_memory",
    "edit_memory",
    "remember_for_all_bots",
    "confirm_memory",
    "correct_memory",
    "forget_memory",
];

/// The block a chat that once held saved memories hears when the switch is turned off, so an
/// agent can't keep following an earlier `<openbot_saved_memories>` block as if it were current.
pub const OFF_UPDATE: &str = "<openbot_saved_memories_update>
The user turned OpenBot memory off for you. The saved memories in earlier <openbot_saved_memories> blocks are no longer current: do not use them.
</openbot_saved_memories_update>";

/// The memory MCP server's instructions while the switch is off: memory apps connected as MCP
/// servers are still available, and skill learning is a separate setting that keeps working.
pub const OFF_INSTRUCTIONS: &str = "OpenBot memory is off for you: you have no saved OpenBot memories, and the \
built-in memory tools do not work. When a memory app is connected as an MCP server, use its tools to recall and \
save durable facts. Learning skills is a separate setting and is unaffected by this switch.";

/// The refusal returned by a gated tool (OpenBot nightly's user-facing label, adapted to Codync).
pub const REFUSAL: &str = "OpenBot memory is off for you, so this tool does not work. Use a connected memory app \
instead, or tell the user they can turn on OpenBot memory in your settings.";

/// Whether this bot has OpenBot memory on. Only an explicit `false` turns it off.
pub fn enabled(cfg: &BotConfig) -> bool {
    cfg.memory != Some(false)
}

/// Whether `tool` is a built-in memory tool gated by the switch.
pub fn gates(tool: &str) -> bool {
    MEMORY_TOOLS.contains(&tool)
}

/// Whether the agent may use `tool` with this bot's settings. Only the built-in memory tools go away
/// when the switch is off: chat history and a connected memory app's MCP tools (for example Mem0's)
/// are not in [`MEMORY_TOOLS`], so the agent sees those and only those for memory.
pub fn tool_visible(cfg: &BotConfig, tool: &str) -> bool {
    enabled(cfg) || !gates(tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(memory: Option<bool>) -> BotConfig {
        serde_json::from_value(serde_json::json!({"id": "b", "name": "Bot", "memory": memory})).expect("a bot")
    }

    #[test]
    fn memory_is_on_by_default_and_when_explicitly_on() {
        assert!(enabled(&bot(None)), "bots saved before the switch keep their memories");
        assert!(enabled(&bot(Some(true))));
        for tool in MEMORY_TOOLS {
            assert!(tool_visible(&bot(None), tool), "{tool} works with the switch on");
        }
    }

    #[test]
    fn an_explicit_false_turns_memory_off_and_hides_only_the_built_in_tools() {
        let off = bot(Some(false));
        assert!(!enabled(&off));
        for tool in MEMORY_TOOLS {
            assert!(!tool_visible(&off, tool), "{tool} must refuse while memory is off");
        }
        assert!(tool_visible(&off, "search_history"));
        assert!(tool_visible(&off, "read_history"));
        // A memory app connected as an MCP server keeps its tools, even with the switch off.
        for app_tool in ["mcp__mem0__add_memory", "mcp__mem0__search_memories", "mcp__mem0__update_memory"] {
            assert!(tool_visible(&off, app_tool), "{app_tool} belongs to the external app");
        }
    }

    #[test]
    fn skills_still_learn_while_memory_is_off() {
        let mut off = bot(Some(false));
        off.learn_skills = true;
        assert!(!enabled(&off), "the memory switch stays off");
        assert!(off.learn_skills, "skill learning is a separate setting and is untouched");
        assert!(tool_visible(&off, "save_skill"), "learning skills does not use a gated memory tool");
    }

    #[test]
    fn memory_tools_are_gated_but_history_tools_are_not() {
        for tool in MEMORY_TOOLS {
            assert!(gates(tool), "{tool} must refuse while memory is off");
        }
        assert!(!gates("search_history"));
        assert!(!gates("read_history"));
    }

    #[test]
    fn the_switch_only_ever_stores_false_on_disk() {
        let on = serde_json::to_value(&bot(None)).expect("serialize");
        assert!(on.get("memory").is_none(), "on is the default and is not stored");
        let off = serde_json::to_value(&bot(Some(false))).expect("serialize");
        assert_eq!(off["memory"], false);
    }

    #[test]
    fn the_off_block_points_past_earlier_saved_memories() {
        assert!(OFF_UPDATE.contains("no longer current"));
        assert!(OFF_INSTRUCTIONS.contains("memory app"));
        assert!(REFUSAL.contains("turn on OpenBot memory"));
    }
}
