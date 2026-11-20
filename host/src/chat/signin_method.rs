//! How each connection signs in, chosen by the operator (agentes.chat ADR 0150).
//!
//! A Claude (or other) connection can sign in more than one way; the choice is kept per connection,
//! not globally. Codync already stores a credential per bot; this is the rule that reports which
//! method a connection uses without ever echoing the secret.

use serde_json::{Value, json};

/// The sign-in methods a connection may use, in a fixed order for UI.
pub const METHODS: [&str; 3] = ["browser", "api_key", "device_code"];

/// The method a connection is using; an unknown value falls back to browser.
pub fn method(saved: Option<&str>) -> Value {
    let chosen = saved.filter(|m| METHODS.contains(m)).unwrap_or(METHODS[0]);
    json!({"method": chosen, "available": METHODS})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_method_falls_back_to_browser() {
        assert_eq!(method(Some("api_key"))["method"], "api_key");
        assert_eq!(method(Some("something"))["method"], "browser");
        assert_eq!(method(None)["method"], "browser");
        assert_eq!(method(None)["available"].as_array().unwrap().len(), 3);
    }
}
