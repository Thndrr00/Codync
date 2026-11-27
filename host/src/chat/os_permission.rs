//! OS permissions agents' work waits on: one wait, told everywhere, asked in the app (ADR 0255).
//!
//! Codync's `screen::permissions` already tracks grants and revocations for screen/computer use.
//! This is the shared vocabulary for a wait: what is needed, who asked, since when, and the action
//! the person takes. A wait is a single record, not an error guessed at by each caller.

use serde_json::{Value, json};

/// The permissions an agent's work may need on a computer.
pub const PERMISSIONS: [&str; 6] = ["files", "screen", "accessibility", "automation", "camera", "microphone"];

/// One wait, with who asked and since when.
pub fn wait(permission: &str, agent: &str, what: &str, since: i64) -> Result<Value, &'static str> {
    if !PERMISSIONS.contains(&permission) {
        return Err("unknown permission");
    }
    Ok(json!({
        "permission": permission,
        "agent": agent,
        "what": what,
        "since": since,
        "status": "waiting",
    }))
}

/// The line an agent's command result ends with while a wait is open.
pub fn reply_line(w: &Value) -> String {
    format!(
        "Waiting for {} permission on this computer; {} asked to {}. I'll run it again once you allow it.",
        w["permission"].as_str().unwrap_or(""),
        w["agent"].as_str().unwrap_or("an agent"),
        w["what"].as_str().unwrap_or("do the work"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_is_one_record_with_a_clear_line() {
        let w = wait("screen", "Ada", "read the screen", 1000).unwrap();
        assert_eq!(w["status"], "waiting");
        assert!(reply_line(&w).contains("screen"));
        assert!(wait("bogus", "Ada", "x", 1000).is_err());
    }
}
