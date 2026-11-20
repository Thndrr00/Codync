//! One setup on many agents (agentes.chat ADR 0094).
//!
//! A setup (instructions, tools, memory) can be applied to several bots. This is the rule for
//! cloning a setup onto a target without touching the source: the target keeps its own id, name
//! and session; only the shared parts are copied.

use serde_json::Value;

/// The fields a setup carries from one bot to another.
pub const SHARED: [&str; 4] = ["instructions", "approvalRules", "connectors", "recallChars"];

/// Builds the patch applied to a target, copying only the shared fields present in the source.
pub fn patch(source: &Value) -> Value {
    let mut out = serde_json::Map::new();
    for field in SHARED {
        if let Some(value) = source.get(field) {
            out.insert(field.to_owned(), value.clone());
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_setup_copies_only_the_shared_fields() {
        let source = json!({"id": "a", "name": "Ada", "instructions": "helps", "connectors": ["gmail"]});
        let p = patch(&source);
        assert_eq!(p["instructions"], "helps");
        assert_eq!(p["connectors"][0], "gmail");
        assert!(p.get("id").is_none(), "identity is not shared");
        assert!(p.get("name").is_none());
    }
}
