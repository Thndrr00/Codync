//! Stretches: agents keep going until the work is done (agentes.chat ADR 0089).
//!
//! A stretch is the work agents do on their own since a person last acted on it. It is bounded by
//! a run count and a budget share; on a stop, one card with `Continuar` lets the person let it go on.
//! Codync keeps its own hop and budget limits — this adds the "one stretch, one card" view.

use serde_json::{Value, json};

/// Default bounds for an unattended stretch (ADR 0089): 50 self-started runs, 20% of a monthly budget.
pub const MAX_STRETCH_RUNS: usize = 50;
pub const MAX_STRETCH_UNATTENDED_RUNS: usize = 10;
pub const MAX_BUDGET_SHARE_PERCENT: u8 = 20;

/// Whether another self-started run fits in this stretch, and why not if it does not.
pub fn may_continue(runs: usize, unattended: bool, budget_share_percent: u8) -> Result<(), &'static str> {
    let cap = if unattended { MAX_STRETCH_UNATTENDED_RUNS } else { MAX_STRETCH_RUNS };
    if runs >= cap {
        return Err("runs");
    }
    if budget_share_percent > MAX_BUDGET_SHARE_PERCENT {
        return Err("budget");
    }
    Ok(())
}

/// The card shown once when a stretch is paused, with the reason in the agent's words.
pub fn paused_card(agent: &str, reason: &str) -> Value {
    json!({
        "type": "stretch_paused",
        "agent": agent,
        "reason": reason,
        "action": "continuar",
        "bodyCode": if reason == "budget" { "stretch_paused_spend" } else { "stretch_paused_runs" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stretch_stops_at_its_bounds() {
        assert!(may_continue(0, false, 0).is_ok());
        assert_eq!(may_continue(MAX_STRETCH_RUNS, false, 0), Err("runs"));
        assert_eq!(may_continue(0, true, 0), Err("runs")); // unattended cap is lower
        assert_eq!(may_continue(0, false, MAX_BUDGET_SHARE_PERCENT + 1), Err("budget"));
    }

    #[test]
    fn a_paused_card_says_why_and_offers_continuar() {
        let card = paused_card("Ada", "budget");
        assert_eq!(card["type"], "stretch_paused");
        assert_eq!(card["action"], "continuar");
        assert_eq!(card["bodyCode"], "stretch_paused_spend");
    }
}
