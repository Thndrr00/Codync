//! A routine comes on with connecting an app (agentes.chat ADR 0242).
//!
//! One fixed routine per app, offered inside the connect flow. Codync already has connectors and
//! routines; this is the mapping from an app to the routine template it gets, so the offer is made
//! at the moment of connecting, not in a later step people skip.

use serde_json::{Value, json};

/// The app → template mapping (ADR 0242 §1), limited to apps Codync connects.
pub fn template(app: &str) -> Option<&'static str> {
    match app {
        "gmail" | "outlook" | "imap" | "email" => Some("Unanswered email"),
        "calendar" | "google_calendar" => Some("Today's plan"),
        "github" => Some("Pull requests to review"),
        "slack" | "telegram" | "whatsapp" => Some("Messages that need a reply"),
        _ => None,
    }
}

/// The offer shown at the end of a connect flow.
pub fn offer(app: &str, bot: &str) -> Value {
    match template(app) {
        Some(name) => json!({"type": "routine_offer", "app": app, "template": name, "bot": bot, "enabled": true}),
        None => json!({"type": "routine_offer", "app": app, "template": Value::Null, "bot": bot, "enabled": false}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_app_gets_one_fixed_routine() {
        assert_eq!(template("gmail"), Some("Unanswered email"));
        assert_eq!(template("github"), Some("Pull requests to review"));
        assert_eq!(template("unknown"), None);
        assert_eq!(offer("gmail", "b1")["enabled"], true);
        assert_eq!(offer("unknown", "b1")["enabled"], false);
    }
}
