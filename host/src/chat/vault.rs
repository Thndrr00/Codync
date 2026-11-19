//! A local vault for the secrets agents use (agentes.chat ADR 0144).
//!
//! Only the name, source and a masked hint are kept here — never the secret itself. Codync stores
//! the value in the OS Keychain (device-only work); this module is the host's index of what exists
//! so an agent can be told "there is a key named X" without ever seeing it.

use serde_json::{Value, json};

/// Masks a secret for display: only the first and last two characters survive.
pub fn mask(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 4 {
        return "*".repeat(chars.len());
    }
    let mut out = String::new();
    out.push(chars[0]);
    out.push(chars[1]);
    out.push_str(&"•".repeat(chars.len().saturating_sub(4).min(8)));
    out.push(chars[chars.len() - 2]);
    out.push(chars[chars.len() - 1]);
    out
}

/// An entry in the vault index; the value lives in the Keychain, not here.
pub fn entry(name: &str, source: &str, masked: &str) -> Value {
    json!({"name": name, "source": source, "masked": masked})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mask_shows_only_the_ends() {
        assert_eq!(mask("abcd"), "****");
        let m = mask("sk-1234567890xyz");
        assert!(m.starts_with("sk"));
        assert!(m.ends_with("yz"));
        assert!(!m.contains("1234567890"));
    }

    #[test]
    fn an_entry_never_holds_the_secret() {
        let e = entry("openai", "keychain", "sk-••xyz");
        assert_eq!(e["name"], "openai");
        assert!(e.get("value").is_none());
    }
}
