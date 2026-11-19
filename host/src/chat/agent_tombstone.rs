//! Deleting an agent leaves a tombstone that names what it left (agentes.chat ADR 0082).
//!
//! A deleted bot is never silently forgotten: the record says what it owned — memory, files,
//! routines — so the person knows what remains. Nothing is removed by this module; it only says.

use serde_json::{Value, json};

/// What a deleted agent leaves behind, from what the host already knows about it.
pub fn tombstone(id: &str, name: &str, memory: usize, files: usize, routines: usize) -> Value {
    let mut left = Vec::new();
    if memory > 0 {
        left.push(json!({"kind": "memory", "count": memory}));
    }
    if files > 0 {
        left.push(json!({"kind": "files", "count": files}));
    }
    if routines > 0 {
        left.push(json!({"kind": "routines", "count": routines}));
    }
    json!({
        "type": "agent_tombstone",
        "id": id,
        "name": name,
        "deleted": true,
        "left": left,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tombstone_names_only_what_the_agent_left() {
        let t = tombstone("b1", "Ada", 3, 0, 2);
        assert_eq!(t["type"], "agent_tombstone");
        assert_eq!(t["left"].as_array().unwrap().len(), 2);
        assert_eq!(tombstone("b2", "Theo", 0, 0, 0)["left"].as_array().unwrap().len(), 0);
    }
}
