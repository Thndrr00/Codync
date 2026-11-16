//! Three rules to start from: read-only, no shell, no sends. They are ordinary rules the person
//! copies into their own list and edits — nothing is enforced until one is saved, and none is hidden.

use crate::store::PolicyEffect;
use serde::Serialize;

/// One starting point, as the API hands it to an editor.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    /// What it does, for the person choosing it.
    pub about: &'static str,
    pub expression: &'static str,
    pub effect: PolicyEffect,
    pub message: &'static str,
}

pub const PRESETS: [Preset; 3] = [
    Preset {
        id: "readOnly",
        name: "Read-only",
        about: "Only reading, searching and fetching go through by itself; everything else is refused.",
        expression: r#"tool.kind in ["read", "search", "fetch", "think"]"#,
        effect: PolicyEffect::Allow,
        message: "Only reading, searching and fetching are allowed on this computer.",
    },
    Preset {
        id: "noShell",
        name: "No shell",
        about: "Shell commands are refused, whatever the bot's own rules or Approvals setting say.",
        expression: r#"tool.kind == "execute""#,
        effect: PolicyEffect::Deny,
        message: "Shell commands are off on this computer.",
    },
    Preset {
        id: "noSends",
        name: "No sends",
        about: "Sending a message, an email or a post is refused: reading and drafting still work.",
        expression: r#"intent.matches("(?i)\\b(send|sent|post|publish|email|mail|message|tweet|reply)\\b") || tool.name.matches("(?i)(send|post|publish|email|tweet)")"#,
        effect: PolicyEffect::Deny,
        message: "Sending messages, email or posts is off on this computer.",
    },
];

/// The presets as the API returns them.
pub fn list() -> serde_json::Value {
    serde_json::to_value(PRESETS).unwrap_or_else(|_| serde_json::Value::Array(Vec::new()))
}
