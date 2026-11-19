//! Memory that proves itself: no result claimed that no call produced (agentes.chat ADR 0167).
//!
//! A fact learned from a tool's output carries where it came from; a fact with no source is not
//! written. This is the check the host applies before it records what an agent says it found.

use serde_json::{Value, json};

/// A claim is admissible only when it names a source that actually produced it.
pub fn admissible(claim: &str, produced: bool, source: Option<&str>) -> Value {
    let ok = produced && source.is_some_and(|s| !s.is_empty()) && !claim.trim().is_empty();
    json!({
        "admitted": ok,
        "claim": claim.trim(),
        "source": source,
        "reason": if ok { "none" } else { "unproven" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unproven_claim_is_not_recorded() {
        assert_eq!(admissible("found X", true, Some("tool-1"))["admitted"], true);
        assert_eq!(admissible("found X", false, Some("tool-1"))["admitted"], false);
        assert_eq!(admissible("found X", true, None)["admitted"], false);
        assert_eq!(admissible("", true, Some("tool-1"))["admitted"], false);
    }
}
