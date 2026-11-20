//! Agents start what the UI does (agentes.chat ADR 0129).
//!
//! Every action the app can take (save a routine, publish a file, join a network) is also a tool the
//! agent can call, through the same host method. This is the rule that names such an action and
//! marks it as going through the host, never reimplemented in the client.

use serde_json::{Value, json};

/// Wraps a UI action as a host call: the name is the host method, arguments are passed through.
pub fn action(method: &str, args: &Value) -> Value {
    json!({"type": "ui_action", "method": method, "arguments": args, "viaHost": true})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ui_action_goes_through_the_host() {
        let a = action("saveRoutine", &json!({"name": "daily"}));
        assert_eq!(a["viaHost"], true);
        assert_eq!(a["method"], "saveRoutine");
        assert_eq!(a["arguments"]["name"], "daily");
    }
}
