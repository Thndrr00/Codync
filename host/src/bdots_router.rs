#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `router.ts`: thresholds, addressee shaping, policy
//! selection, social reactions and router JSON parsing. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

use serde_json::{Map, Value, json};

/// Above this "already answered" probability nobody is picked.
pub const ALREADY_ANSWERED_THRESHOLD: f64 = 0.7;

/// From this "for everyone" probability several agents may answer.
pub const FOR_EVERYONE_THRESHOLD: f64 = 0.5;

/// From this probability a message speaks to one agent, that agent is its addressee.
pub const SPOKEN_TO_THRESHOLD: f64 = 0.6;

/// Longest router message slice, in characters.
pub const ROUTER_MESSAGE_CHARS: usize = 4_000;

/// Default strategy name (DEFAULT_STRATEGY).
pub const DEFAULT_STRATEGY: &str = "rule:default";

/// Social reactions a step may choose, with when each fits.
pub const SOCIAL_REACTIONS: [(&str, &str); 7] = [
    ("🌙", "Says good night, or that they are off to sleep."),
    ("☀", "Says good morning."),
    ("👋", "Says hi, good afternoon, or bye."),
    ("🎉", "Congratulates or celebrates."),
    ("🙏", "Thanks."),
    ("❤", "Shows affection."),
    ("👌", "An ok, an acknowledgement, or cheers."),
];

/// Clamp a score into [0, 1] (clamp01).
pub fn clamp01(value: f64) -> f64 {
    if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) }
}

/// Whether steps ask who a message speaks to (asksAddressee).
pub fn asks_addressee(candidates: usize, author_is_person: bool) -> bool {
    author_is_person && candidates >= 2
}

/// Emoji without variation selectors or surrounding space (bareEmoji).
pub fn bare_emoji(emoji: &str) -> String {
    emoji.trim().chars().filter(|c| *c != '\u{fe0f}').collect()
}

/// One of the social reactions, however the variation selector was written (socialChoice).
pub fn social_choice(emoji: &str) -> Option<&'static str> {
    let bare = bare_emoji(emoji);
    SOCIAL_REACTIONS.iter().find(|(candidate, _)| bare_emoji(candidate) == bare).map(|(_, description)| *description)
}

/// A short follow-up needing no new routing (isShortFollowUp).
pub fn is_short_follow_up(body: &str) -> bool {
    let text = body.trim();
    !text.is_empty() && !text.contains('@') && text.chars().count() <= 60 && text.split_whitespace().count() <= 8
}

/// Reaction a social message gets from its emoji alone (socialReaction).
pub fn social_reaction(text: &str) -> Option<&'static str> {
    let bare = text.replace("<3", "❤");
    if bare.contains('?') || bare.chars().any(|c| c.is_alphanumeric()) {
        return None;
    }
    let first = SOCIAL_REACTIONS.iter().find(|(emoji, _)| bare.contains(emoji) || bare.contains(&bare_emoji(emoji)))?;
    Some(first.1)
}

/// Need-any-agent gate, lower for a single-agent channel (needGateOf).
pub fn need_gate(min_score: f64, single_agent_min: Option<f64>, single: bool) -> f64 {
    if single { min_score.min(single_agent_min.unwrap_or(min_score)) } else { min_score }
}

/// Score threshold for one candidate, lowered by continuity and a single agent.
pub fn min_score_for(policy: &Value, agent_id: &str, continuity: Option<&str>, single: bool) -> f64 {
    let mut threshold = policy["minScore"].as_f64().unwrap_or(0.5);
    if single {
        threshold = threshold.min(policy["singleAgentMinScore"].as_f64().unwrap_or(threshold));
    }
    if continuity == Some(agent_id) {
        threshold = threshold.min(policy["continuityMinScore"].as_f64().unwrap_or(threshold));
    }
    threshold
}

/// Whether another agent already answered (ALREADY_ANSWERED_THRESHOLD).
pub fn is_answered_enough(probability: Option<f64>) -> bool {
    probability.is_some_and(|value| value >= ALREADY_ANSWERED_THRESHOLD)
}

/// Whether several agents may answer (FOR_EVERYONE_THRESHOLD).
pub fn is_for_everyone(probability: Option<f64>) -> bool {
    probability.is_some_and(|value| value >= FOR_EVERYONE_THRESHOLD)
}

/// Highest-scoring agent of a scored result.
pub fn top_agent(result: &Value) -> Option<String> {
    let mut best: Option<(String, f64)> = None;
    if let Some(agents) = result["agents"].as_array() {
        for entry in agents {
            let id = entry["agentId"].as_str().unwrap_or("");
            let score = entry["score"].as_f64().unwrap_or(0.0);
            if id.is_empty() {
                continue;
            }
            if best.as_ref().is_none_or(|(_, value)| score > *value) {
                best = Some((id.to_owned(), score));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Highest score of one agent in a result (scoreOf).
pub fn score_of(result: &Value, agent_id: &str) -> f64 {
    result["agents"]
        .as_array()
        .map(|agents| {
            agents
                .iter()
                .filter(|entry| entry["agentId"].as_str() == Some(agent_id))
                .filter_map(|entry| entry["score"].as_f64())
                .fold(0.0_f64, f64::max)
        })
        .unwrap_or(0.0)
}

/// Handle to agent id lookup (idOf).
pub fn id_of_handle(candidates: &[Value], handle: &str) -> Option<String> {
    let bare = handle.trim_start_matches('@').to_ascii_lowercase();
    candidates
        .iter()
        .find(|candidate| candidate["handle"].as_str().unwrap_or("").eq_ignore_ascii_case(&bare))
        .and_then(|candidate| candidate["agentId"].as_str())
        .map(str::to_owned)
}

/// Handles of all candidates, for the router prompt.
pub fn handles_of(candidates: &[Value]) -> Vec<String> {
    candidates.iter().filter_map(|candidate| candidate["handle"].as_str().map(str::to_owned)).collect()
}

/// The agent whose message is a clear pick (CLEAR_CHOICE).
pub fn clear_pick(result: &Value, clear_score: f64) -> Option<String> {
    result["agents"]
        .as_array()
        .and_then(|agents| {
            agents
                .iter()
                .filter(|entry| entry["score"].as_f64().unwrap_or(0.0) >= clear_score)
                .max_by(|a, b| a["score"].as_f64().unwrap_or(0.0).total_cmp(&b["score"].as_f64().unwrap_or(0.0)))
        })
        .and_then(|entry| entry["agentId"].as_str().map(str::to_owned))
}

/// Shape scores with a spoken-to addressee (withAddressee).
pub fn with_addressee(result: &Value, spoken_to: &[(String, f64)]) -> Value {
    let mut sorted: Vec<&(String, f64)> = spoken_to.iter().collect();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    let Some((top_id, top_score)) = sorted.first().map(|(id, score)| ((*id).clone(), *score)) else {
        return result.clone();
    };
    if top_score < SPOKEN_TO_THRESHOLD || is_for_everyone(result["forEveryone"].as_f64()) {
        return result.clone();
    }
    let to_any: f64 = spoken_to.iter().map(|(_, score)| score).sum();
    let needs = result["needsAgent"].as_f64().unwrap_or(0.0);
    let mut ids: Vec<String> = result["agents"]
        .as_array()
        .map(|agents| agents.iter().filter_map(|entry| entry["agentId"].as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    for (id, _) in spoken_to {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    let agents: Vec<Value> = ids
        .into_iter()
        .map(|id| {
            let p = spoken_to.iter().find(|(name, _)| *name == id).map_or(0.0, |(_, score)| *score);
            let score = score_of(result, &id).min(1.0 - to_any + p).max(needs * p);
            json!({"agentId": id, "score": clamp01(score)})
        })
        .collect();
    let mut out = result.clone();
    out["agents"] = Value::Array(agents);
    out["addressee"] = Value::String(top_id);
    out
}

/// Apply policy to scores and pick agent ids (selectAgents).
pub fn select_agents(candidates: &[Value], result: &Value, policy: &Value, continuity: Option<&str>) -> Vec<String> {
    let allowed: Vec<&str> = candidates.iter().filter_map(|c| c["agentId"].as_str()).collect();
    let single = candidates.len() == 1;
    let mut best: Vec<(String, f64)> = Vec::new();
    if let Some(agents) = result["agents"].as_array() {
        for entry in agents {
            let id = entry["agentId"].as_str().unwrap_or("");
            let score = entry["score"].as_f64().unwrap_or(0.0);
            if !allowed.contains(&id) || !score.is_finite() {
                continue;
            }
            if let Some(found) = best.iter_mut().find(|(name, _)| name == id) {
                found.1 = found.1.max(score);
            } else {
                best.push((id.to_owned(), score));
            }
        }
    }
    let continuing = continuity
        .filter(|id| {
            best.iter()
                .find(|(name, _)| name == *id)
                .is_some_and(|(_, score)| *score >= min_score_for(policy, id, continuity, single))
        })
        .is_some();
    let clear_score = policy["clearPickScore"].as_f64().unwrap_or(f64::INFINITY);
    let clear = best.iter().any(|(_, score)| *score >= clear_score);
    if let Some(needs) = result["needsAgent"].as_f64() {
        if needs < need_gate(policy["minScore"].as_f64().unwrap_or(0.5), policy["singleAgentMinScore"].as_f64(), single)
            && !continuing
            && !clear
        {
            return Vec::new();
        }
    }
    let mut picked: Vec<(String, f64)> =
        best.into_iter().filter(|(id, score)| *score >= min_score_for(policy, id, continuity, single)).collect();
    picked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let limit = if is_for_everyone(result["forEveryone"].as_f64()) {
        policy["maxAgents"].as_u64().unwrap_or(4) as usize
    } else {
        1
    };
    picked.into_iter().take(limit).map(|(id, _)| id).collect()
}

/// Strip code fences from a router answer.
pub fn strip_fences(text: &str) -> String {
    let trimmed = text.trim();
    let without_open =
        trimmed.strip_prefix("```json").or_else(|| trimmed.strip_prefix("```")).unwrap_or(trimmed).trim_start();
    without_open.strip_suffix("```").unwrap_or(without_open).trim().to_owned()
}

/// Validate and normalize parsed router JSON (parseRouterJson).
pub fn parse_router_json(raw: &Value) -> Option<Value> {
    if !raw.is_object() {
        return None;
    }
    let scores = raw["scores"].as_array().map(|values| {
        values
            .iter()
            .filter_map(|entry| {
                let handle = entry["handle"].as_str()?;
                Some(json!({"handle": handle, "score": clamp01(entry["score"].as_f64().unwrap_or(0.0))}))
            })
            .collect::<Vec<_>>()
    });
    let mut out = Map::new();
    out.insert("addressee".to_owned(), Value::String(raw["addressee"].as_str().unwrap_or("none").to_owned()));
    out.insert("needs_agent".to_owned(), json!(clamp01(raw["needs_agent"].as_f64().unwrap_or(0.0))));
    out.insert("for_everyone".to_owned(), json!(clamp01(raw["for_everyone"].as_f64().unwrap_or(0.0))));
    out.insert("already_answered".to_owned(), json!(clamp01(raw["already_answered"].as_f64().unwrap_or(0.0))));
    out.insert("scores".to_owned(), Value::Array(scores.unwrap_or_default()));
    out.insert("reason".to_owned(), Value::String(raw["reason"].as_str().unwrap_or("").to_owned()));
    if raw["social"].is_object() {
        out.insert(
            "social".to_owned(),
            json!({"react": raw["social"]["react"] == true, "emoji": raw["social"]["emoji"].as_str().unwrap_or("")}),
        );
    }
    Some(Value::Object(out))
}

/// Parse a router answer, with or without code fences (parseRouterOutput).
pub fn parse_router_output(text: &str) -> Option<Value> {
    let clean = strip_fences(text);
    serde_json::from_str::<Value>(&clean).ok().and_then(|value| parse_router_json(&value))
}

/// Turn parsed router JSON into scores (scoresOf).
pub fn scores_of(candidates: &[Value], parsed: &Value) -> Value {
    let to = parsed["addressee"]
        .as_str()
        .filter(|value| *value != "none")
        .and_then(|handle| id_of_handle(candidates, handle))
        .filter(|_| asks_addressee(candidates.len(), true));
    let agents: Vec<Value> = parsed["scores"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|entry| {
                    let id = id_of_handle(candidates, entry["handle"].as_str().unwrap_or(""))?;
                    Some(json!({"agentId": id, "score": clamp01(entry["score"].as_f64().unwrap_or(0.0))}))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut result = json!({
        "agents": agents,
        "needsAgent": parsed["needs_agent"].as_f64().unwrap_or(0.0),
        "forEveryone": parsed["for_everyone"].as_f64().unwrap_or(0.0),
        "alreadyAnswered": parsed["already_answered"].as_f64().unwrap_or(0.0),
    });
    if let Some(id) = to {
        result = with_addressee(&result, &[(id, 1.0)]);
    }
    if parsed["social"].is_object() {
        result["social"] = parsed["social"].clone();
    }
    result
}

/// Name of the strategy that decided, with a default (strategyName).
pub fn strategy_name(value: Option<&str>) -> &'static str {
    match value.unwrap_or(DEFAULT_STRATEGY) {
        "jev" => "jev",
        other if other.starts_with("llm:") => "llm",
        other if other.starts_with("plan:") => "plan",
        _ => DEFAULT_STRATEGY,
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(candidates: usize, picked: usize, everyone: bool, answered: bool) -> Vec<(&'static str, String)> {
    vec![
        ("candidates", count_label(candidates)),
        ("picked", count_label(picked)),
        ("everyone", if everyone { "yes" } else { "no" }.to_owned()),
        ("answered", if answered { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_and_reactions() {
        assert_eq!(clamp01(1.5), 1.0);
        assert_eq!(clamp01(-0.2), 0.0);
        assert_eq!(clamp01(f64::NAN), 0.0);
        assert!(asks_addressee(2, true));
        assert!(!asks_addressee(1, true));
        assert!(!asks_addressee(2, false));
        assert_eq!(bare_emoji("👌\u{fe0f}"), "👌");
        assert!(social_choice("👌").is_some());
        assert!(social_choice("x").is_none());
        assert!(is_short_follow_up("ok thanks"));
        assert!(!is_short_follow_up("@ada what do you think"));
        assert!(social_reaction("👋").is_some());
        assert!(social_reaction("thanks").is_none());
    }

    #[test]
    fn gates_and_picks() {
        assert_eq!(need_gate(0.5, Some(0.3), true), 0.3);
        assert_eq!(need_gate(0.5, Some(0.3), false), 0.5);
        let policy = json!({"minScore":0.5,"maxAgents":4,"clearPickScore":0.8,"continuityMinScore":0.3,"singleAgentMinScore":0.35});
        assert_eq!(min_score_for(&policy, "a", Some("a"), false), 0.3);
        assert!(is_answered_enough(Some(0.8)));
        assert!(is_for_everyone(Some(0.6)));
        let result = json!({"agents":[{"agentId":"a","score":0.9},{"agentId":"b","score":0.4}],"needsAgent":0.3});
        assert_eq!(top_agent(&result).as_deref(), Some("a"));
        assert_eq!(score_of(&result, "b"), 0.4);
        assert_eq!(clear_pick(&result, 0.8).as_deref(), Some("a"));
        let candidates = vec![json!({"agentId":"a","handle":"Ada"}), json!({"agentId":"b","handle":"Theo"})];
        assert_eq!(id_of_handle(&candidates, "@ada").as_deref(), Some("a"));
        assert_eq!(handles_of(&candidates), vec!["Ada", "Theo"]);
    }

    #[test]
    fn selection() {
        let policy = json!({"minScore":0.5,"maxAgents":4,"clearPickScore":0.8,"continuityMinScore":0.3});
        let candidates = vec![json!({"agentId":"a","handle":"Ada"}), json!({"agentId":"b","handle":"Theo"})];
        let result = json!({"agents":[{"agentId":"a","score":0.9},{"agentId":"b","score":0.6}],"needsAgent":0.8,"forEveryone":0.0});
        assert_eq!(select_agents(&candidates, &result, &policy, None), vec!["a"]);
        let everyone = json!({"agents":[{"agentId":"a","score":0.9},{"agentId":"b","score":0.6}],"needsAgent":0.8,"forEveryone":0.9});
        assert_eq!(select_agents(&candidates, &everyone, &policy, None).len(), 2);
        let low = json!({"agents":[{"agentId":"a","score":0.2}],"needsAgent":0.1});
        assert!(select_agents(&candidates, &low, &policy, None).is_empty());
        let shaped = with_addressee(&result, &[("b".to_owned(), 0.9)]);
        assert_eq!(shaped["addressee"], "b");
    }

    #[test]
    fn parsing_and_presentation() {
        let text = "```json\n{\"needs_agent\":0.8,\"for_everyone\":0.2,\"already_answered\":0.1,\"scores\":[{\"handle\":\"ada\",\"score\":1.2}],\"reason\":\"ok\"}\n```";
        let parsed = parse_router_output(text).unwrap();
        assert_eq!(parsed["needs_agent"], 0.8);
        assert_eq!(parsed["scores"][0]["score"], 1.0);
        assert!(parse_router_output("not json").is_none());
        assert!(parse_router_json(&json!([1, 2])).is_none());
        let candidates = vec![json!({"agentId":"a","handle":"Ada"})];
        let scored = scores_of(&candidates, &parsed);
        assert_eq!(scored["agents"][0]["agentId"], "a");
        assert_eq!(strategy_name(Some("llm:gpt")), "llm");
        assert_eq!(strategy_name(None), DEFAULT_STRATEGY);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(2, 1, false, false).len(), 4);
    }
}
