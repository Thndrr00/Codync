//! Time per ask, by kind (agentes.chat ADR 0254) and where it goes by stage (ADR 0256).
//!
//! An ask is a person's message and the stretch it starts. The kind comes from what the work did
//! when there is no judge: no tool and a short message is chat, no tool is a question, a few tools
//! is a lookup, more is a task. Codync already measures turns; this names the kind and its target
//! so the weekly view can show a meaningful speed, not a median that mixes greetings with reports.

use serde_json::{Value, json};

/// Targets for the first reply, in milliseconds (ADR 0254): chat 5s, question 10s, then longer.
pub fn target_ms(kind: &str) -> i64 {
    match kind {
        "chat" => 5_000,
        "question" => 10_000,
        "lookup" => 30_000,
        "action" => 60_000,
        "task" => 180_000,
        _ => 300_000, // project
    }
}

/// Reads the kind from what the work did, for hosts with no judge (ADR 0254 §2).
pub fn kind_from_work(tool_calls: usize, reply_chars: usize) -> &'static str {
    if tool_calls == 0 && reply_chars <= 15 {
        "chat"
    } else if tool_calls == 0 {
        "question"
    } else if tool_calls <= 3 {
        "lookup"
    } else {
        "task"
    }
}

/// An Apdex-style speed index: 1 when within target, 0.5 within twice it, else 0 (ADR 0254 §4).
pub fn speed_index(kind: &str, first_reply_ms: i64) -> f64 {
    let target = target_ms(kind);
    if first_reply_ms <= target {
        1.0
    } else if first_reply_ms <= target.saturating_mul(2) {
        0.5
    } else {
        0.0
    }
}

/// The stages a run's time is grouped into, for a chart (ADR 0256).
pub fn stages(wait_ms: i64, model_ms: i64, tools_ms: i64) -> Value {
    json!({"wait": wait_ms, "model": model_ms, "tools": tools_ms, "total": wait_ms + model_ms + tools_ms})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_has_a_target_and_a_speed() {
        assert_eq!(kind_from_work(0, 5), "chat");
        assert_eq!(kind_from_work(0, 60), "question");
        assert_eq!(kind_from_work(2, 100), "lookup");
        assert_eq!(kind_from_work(9, 100), "task");
        assert_eq!(speed_index("chat", 4_000), 1.0);
        assert_eq!(speed_index("chat", 8_000), 0.5);
        assert_eq!(speed_index("chat", 20_000), 0.0);
        assert_eq!(stages(10, 20, 30)["total"], 60);
    }
}
