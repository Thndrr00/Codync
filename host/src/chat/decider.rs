//! Fast Decision Model (`server/decider/`, `docs/decision-model.md` from OpenMausBot)
//! for low-latency group chat speaker routing.
//!
//! When a user writes in a group without explicit `@bot` mentions, the decider
//! evaluates whether a single specialist bot ($\ge 0.6$ probability), all members,
//! or the group's lead/Chief bot should answer, with a strict 1200ms timeout.
//! If no fast decision endpoint is configured, Codync preserves the classic
//! Grok-Bot multi-round behavior (`AllMembers`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub const CONFIDENCE_THRESHOLD: f64 = 0.6;
pub const DECIDER_TIMEOUT: Duration = Duration::from_millis(1200);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCandidate {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeciderRequest {
    pub group_name: String,
    pub recent_transcript: Vec<String>,
    pub user_message: String,
    pub candidates: Vec<BotCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingDecision {
    SingleBot(String),
    AllMembers,
    FallbackToLead,
}

/// Evaluates raw probability scores returned by a fast decision model endpoint.
/// Applies the $\ge 0.6$ threshold from OpenMausBot's decision model contract.
pub fn evaluate_probabilities(
    scores: &[(String, f64)],
    all_members_prob: f64,
    candidates: &[BotCandidate],
) -> RoutingDecision {
    if all_members_prob >= CONFIDENCE_THRESHOLD {
        return RoutingDecision::AllMembers;
    }
    let best = scores
        .iter()
        .filter(|(id, _)| candidates.iter().any(|c| &c.id == id))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((bot_id, prob)) = best
        && *prob >= CONFIDENCE_THRESHOLD
    {
        return RoutingDecision::SingleBot(bot_id.clone());
    }
    RoutingDecision::FallbackToLead
}

/// Queries the configured fast decision endpoint (`CODYNC_DECIDER_URL`) with a
/// hard 1200ms timeout. When unconfigured, returns `RoutingDecision::AllMembers`
/// so classic round-robin group turns run unchanged.
pub async fn resolve_speaker(req: &DeciderRequest) -> RoutingDecision {
    let Ok(endpoint) = std::env::var("CODYNC_DECIDER_URL") else {
        return RoutingDecision::AllMembers;
    };
    if endpoint.trim().is_empty() {
        return RoutingDecision::AllMembers;
    }

    let mut http_req = crate::http().post(endpoint.trim()).timeout(DECIDER_TIMEOUT).json(req);
    if let Ok(key) = std::env::var("CODYNC_DECIDER_API_KEY")
        && !key.trim().is_empty()
    {
        http_req = http_req.bearer_auth(key.trim());
    }

    let Ok(Ok(resp)) = tokio::time::timeout(DECIDER_TIMEOUT, http_req.send()).await else {
        return RoutingDecision::FallbackToLead;
    };
    let Ok(body) = resp.json::<Value>().await else {
        return RoutingDecision::FallbackToLead;
    };

    let all_prob = body["allMembersProbability"].as_f64().unwrap_or(0.0);
    let mut scores = Vec::new();
    if let Some(obj) = body["probabilities"].as_object() {
        for (k, v) in obj {
            if let Some(p) = v.as_f64() {
                scores.push((k.clone(), p));
            }
        }
    }
    evaluate_probabilities(&scores, all_prob, &req.candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidates() -> Vec<BotCandidate> {
        vec![
            BotCandidate {
                id: "rust-bot".into(),
                name: "Rustacean".into(),
                description: "Rust backend specialist".into(),
            },
            BotCandidate {
                id: "ui-bot".into(),
                name: "Designer".into(),
                description: "SwiftUI and Compose specialist".into(),
            },
        ]
    }

    #[test]
    fn selects_single_bot_when_probability_meets_threshold() {
        let candidates = sample_candidates();
        let scores = vec![("rust-bot".into(), 0.78), ("ui-bot".into(), 0.12)];
        assert_eq!(evaluate_probabilities(&scores, 0.10, &candidates), RoutingDecision::SingleBot("rust-bot".into()));
    }

    #[test]
    fn selects_all_members_when_multi_member_score_is_high() {
        let candidates = sample_candidates();
        let scores = vec![("rust-bot".into(), 0.45), ("ui-bot".into(), 0.45)];
        assert_eq!(evaluate_probabilities(&scores, 0.65, &candidates), RoutingDecision::AllMembers);
    }

    #[test]
    fn falls_back_to_lead_when_uncertain() {
        let candidates = sample_candidates();
        let scores = vec![("rust-bot".into(), 0.42), ("ui-bot".into(), 0.39)];
        assert_eq!(evaluate_probabilities(&scores, 0.19, &candidates), RoutingDecision::FallbackToLead);
    }
}
