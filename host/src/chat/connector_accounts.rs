//! Several accounts per connector (agentes.chat ADR 0099).
//!
//! A connector (Gmail, Slack, GitHub) may have more than one account; a message names which one it
//! came from and a reply uses that same account. Codync keeps one account per connector today; this
//! is the selection rule, so adding a second account never misroutes a reply.

use serde_json::{Value, json};

/// Picks the account a reply must use: the one named by the message, or the default.
pub fn account_for(message: &Value, accounts: &[Value], default: Option<&str>) -> Value {
    let named = message["accountId"].as_str().or(default);
    let found = named.and_then(|id| accounts.iter().find(|a| a["id"] == id));
    json!({
        "accountId": found.and_then(|a| a["id"].as_str()),
        "known": found.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_uses_the_account_the_message_named() {
        let accounts = vec![json!({"id": "work"}), json!({"id": "home"})];
        let msg = json!({"accountId": "home"});
        assert_eq!(account_for(&msg, &accounts, None)["accountId"], "home");
        assert_eq!(account_for(&json!({}), &accounts, Some("work"))["accountId"], "work");
        assert_eq!(account_for(&json!({"accountId": "missing"}), &accounts, None)["known"], false);
    }
}
