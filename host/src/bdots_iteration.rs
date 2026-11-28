#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `turn_iteration_prep.py`: iteration budgeting, turn
//! anchoring, replay ghost neutralisation and tool-round pairing. Composes helpers already ported
//! into this host. Only `botInsights` calls this.

use serde_json::Value;

/// Current wording of the interrupt placeholder.
pub const INTERRUPT_PLACEHOLDER: &str = "[interrupt: no assistant output for this turn]";

/// Legacy spellings that models reproduce verbatim.
pub const LEGACY_PLACEHOLDERS: [&str; 2] = ["[response interrupted]", "[interrupt scaffold]"];

/// Ratio of iterations after which a one-shot budget warning fires.
pub const BUDGET_WARNING_RATIO: f64 = 0.9;

/// Ratio used for kanban workers with no explicit setting.
pub const KANBAN_WARNING_RATIO: f64 = 0.9;

/// Warning template for an iteration checkpoint.
pub const WARNING_TEMPLATE: &str = "[SYSTEM NOTICE — iteration budget checkpoint] You have used {used} of {maximum} iterations. Checkpoint durable progress now, then continue the task; do not stop solely because of this warning.";

/// Whether a row anchors the current turn's user message (_anchorsCurrentTurn).
pub fn anchors_turn(messages: &[Value], index: usize, user_message: &str) -> bool {
    let Some(row) = messages.get(index) else {
        return false;
    };
    if row["role"] != "user" {
        return false;
    }
    let content = row["content"].as_str().unwrap_or("");
    content == user_message || row["api_content"].as_str() == Some(user_message)
}

/// Whether a budget warning should fire once (maybeInjectIterationBudgetWarning).
pub fn should_warn(used: u32, maximum: u32, ratio: Option<f64>, already_injected: bool) -> bool {
    if already_injected || maximum <= 1 {
        return false;
    }
    let ratio = ratio.unwrap_or(BUDGET_WARNING_RATIO);
    let threshold = (ratio * maximum as f64).min((maximum - 1) as f64);
    used >= threshold as u32
}

/// Render the one-shot warning text.
pub fn warning_text(used: u32, maximum: u32) -> String {
    WARNING_TEMPLATE.replace("{used}", &used.to_string()).replace("{maximum}", &maximum.to_string())
}

/// Whether a row is a hidden interrupt ghost to neutralise.
pub fn is_ghost(row: &Value) -> bool {
    if row["display_kind"] != "hidden" || row["role"] != "assistant" {
        return false;
    }
    ["content", "api_content"]
        .iter()
        .any(|key| row[*key].as_str().map(|text| LEGACY_PLACEHOLDERS.contains(&text.trim())).unwrap_or(false))
}

/// Neutralise replay ghosts, returning count without mutating input (_neutraliseReplayEchoGhosts).
pub fn neutralise_ghosts(rows: &[Value]) -> (Vec<Value>, usize) {
    let mut out = Vec::with_capacity(rows.len());
    let mut count = 0;
    for row in rows {
        if is_ghost(row) {
            let mut clean = row.clone();
            clean["content"] = serde_json::Value::String(String::new());
            clean["api_content"] = serde_json::Value::String(INTERRUPT_PLACEHOLDER.to_owned());
            out.push(clean);
            count += 1;
        } else {
            out.push(row.clone());
        }
    }
    (out, count)
}

/// Whether a message is the last tool result, eligible for a warning tail.
pub fn is_tool_tail(messages: &[Value]) -> bool {
    messages.last().is_some_and(|row| row["role"] == "tool")
}

/// Append a notice to a message's textual content, reusing the shared formatter.
pub fn append_notice(content: &str, notice: &str) -> String {
    if notice.is_empty() {
        return content.to_owned();
    }
    format!("{}\n\n{}", content.trim_end(), crate::bdots_more::shorten(notice, 400))
}

/// Pair the newest assistant tool_calls batch with following results (_previousToolRound).
pub fn previous_tool_round(messages: &[Value]) -> Vec<Value> {
    let Some(index) = messages.iter().rposition(|row| row["role"] == "assistant" && row["tool_calls"].is_array())
    else {
        return Vec::new();
    };
    let calls = messages[index]["tool_calls"].as_array().map_or(Vec::new(), |array| array.clone());
    let mut results: Vec<Value> = Vec::new();
    for row in messages.iter().skip(index + 1) {
        if row["role"] != "tool" {
            break;
        }
        results.push(row.clone());
    }
    calls
        .iter()
        .filter_map(|call| {
            let id = call["id"].as_str()?;
            let function = call["function"].as_object()?;
            let result = results
                .iter()
                .find(|row| row["tool_call_id"].as_str() == Some(id))
                .and_then(|row| row.get("content"))
                .cloned()
                .unwrap_or(Value::Null);
            Some(serde_json::json!({
                "name": function.get("name").cloned().unwrap_or(Value::Null),
                "arguments": function.get("arguments").cloned().unwrap_or(Value::Null),
                "result": result,
            }))
        })
        .collect()
}

/// Whether a steer row should be inserted after the newest tool result.
pub fn steer_after_tool(messages: &[Value]) -> Option<usize> {
    messages.iter().rposition(|row| row["role"] == "tool")
}

/// Build a standalone steer user row.
pub fn steer_row(text: &str) -> Value {
    serde_json::json!({"role":"user","content":crate::bdots_more::shorten(text.trim(), 400),"display_kind":"steer"})
}

/// Whether a restart exceeds the retry bound.
pub fn restart_exceeded(restart_count: u32, max_retries: u32) -> bool {
    restart_count > max_retries
}

/// Refund a used iteration for a bounded restart.
pub fn refund_iteration(used: u32, restart_count: u32, max_retries: u32) -> u32 {
    if restart_exceeded(restart_count, max_retries) { used } else { used.saturating_sub(1) }
}

/// Human label for a restart action.
pub fn restart_label(action: &str) -> String {
    crate::bdots_textfmt::upper_first(action.replace('_', " ").as_str())
}

/// SF Symbol for a restart action.
pub fn restart_icon(action: &str) -> &'static str {
    if action.contains("redirect") {
        "arrow.uturn.right.circle"
    } else if action.contains("rebuild") {
        "arrow.triangle.2.circlepath"
    } else {
        "arrow.clockwise"
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(warn: bool, ghosts: usize, tools: usize, restarts: usize) -> Vec<(&'static str, String)> {
    vec![
        ("warning", if warn { "on" } else { "off" }.to_owned()),
        ("ghosts", count_label(ghosts)),
        ("tools", count_label(tools)),
        ("restarts", count_label(restarts)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn anchoring_and_budgets() {
        let rows = vec![json!({"role":"user","content":"go"}), json!({"role":"assistant","content":"ok"})];
        assert!(anchors_turn(&rows, 0, "go"));
        assert!(!anchors_turn(&rows, 1, "go"));
        assert!(!anchors_turn(&rows, 5, "go"));
        assert!(!should_warn(5, 10, None, true));
        assert!(should_warn(9, 10, Some(0.9), false));
        assert!(!should_warn(5, 10, Some(0.9), false));
        assert!(warning_text(9, 10).contains("9"));
    }

    #[test]
    fn ghosts_and_tails() {
        let rows = vec![
            json!({"role":"assistant","display_kind":"hidden","content":"[response interrupted]"}),
            json!({"role":"assistant","display_kind":"visible","content":"ok"}),
        ];
        assert!(is_ghost(&rows[0]));
        assert!(!is_ghost(&rows[1]));
        let (clean, count) = neutralise_ghosts(&rows);
        assert_eq!(count, 1);
        assert_eq!(clean[0]["api_content"], INTERRUPT_PLACEHOLDER);
        assert_eq!(rows[0]["content"], "[response interrupted]");
        assert!(is_tool_tail(&[json!({"role":"tool"})]));
        assert!(!is_tool_tail(&[]));
        assert!(append_notice("a", "b").contains("b"));
    }

    #[test]
    fn tool_rounds_and_steer() {
        let rows = vec![
            json!({"role":"assistant","tool_calls":[{"id":"1","function":{"name":"run","arguments":"{}"}}]}),
            json!({"role":"tool","tool_call_id":"1","content":"done"}),
        ];
        let round = previous_tool_round(&rows);
        assert_eq!(round.len(), 1);
        assert_eq!(round[0]["result"], "done");
        assert_eq!(steer_after_tool(&rows), Some(1));
        assert_eq!(steer_row("  hi  ")["content"], "hi");
    }

    #[test]
    fn restarts_and_presentation() {
        assert!(restart_exceeded(4, 3));
        assert_eq!(refund_iteration(5, 1, 3), 4);
        assert_eq!(refund_iteration(5, 4, 3), 5);
        assert_eq!(restart_label("restart_with_redirect"), "Restart with redirect");
        assert_eq!(restart_icon("restart_with_redirect"), "arrow.uturn.right.circle");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(true, 1, 2, 0).len(), 4);
    }
}
