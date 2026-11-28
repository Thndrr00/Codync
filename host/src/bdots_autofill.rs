#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `vault_login_classifier.py`: login/checkout control
//! classification, OTP detection and fill selection. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

use serde_json::Value;

/// Login autocomplete tokens (LOGIN_AUTOFILL_TOKENS).
pub const LOGIN_TOKENS: [&str; 4] = ["username", "email", "tel", "current-password"];

/// Payment autocomplete tokens (PAYMENT_AUTOFILL_TOKENS).
pub const PAYMENT_TOKENS: [&str; 6] = ["cc-number", "cc-name", "cc-exp", "cc-exp-month", "cc-exp-year", "cc-csc"];

/// Address autocomplete tokens (ADDRESS_AUTOFILL_TOKENS).
pub const ADDRESS_TOKENS: [&str; 7] =
    ["address-line1", "address-line2", "address-level2", "address-level1", "postal-code", "country-name", "country"];

/// Autocomplete tokens that exclude a control from login fill.
pub const EXCLUDED_TOKENS: [&str; 2] = ["new-password", "one-time-code"];

/// Score for an exact autocomplete token match.
pub const EXACT_SCORE: u32 = 100;

/// Score for a password/email/tel input type.
pub const TYPE_SCORE: u32 = 90;

/// Score for a label/name heuristic.
pub const HEURISTIC_SCORE: u32 = 70;

/// Normalize text for matching: lower, ASCII-fold, non-alphanumerics to spaces (_normalizeText).
pub fn normalize_text(text: &str) -> String {
    crate::bdots_textfmt::slugify(text).replace('-', " ")
}

/// Autocomplete tokens of a control, lower-cased.
pub fn autocomplete_tokens(value: &str) -> Vec<String> {
    value.to_ascii_lowercase().split_whitespace().map(str::to_owned).collect()
}

/// Whether a control carries an excluded autocomplete token.
pub fn is_excluded(autocomplete: &str) -> bool {
    autocomplete_tokens(autocomplete).iter().any(|token| EXCLUDED_TOKENS.contains(&token.as_str()))
}

/// Whether normalized text names a "new/confirm/create/repeat password".
pub fn is_excluded_password(text: &str) -> bool {
    let normalized = normalize_text(text);
    ["new password", "confirm password", "create password", "repeat password"]
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

/// Whether normalized text names email.
pub fn mentions_email(text: &str) -> bool {
    let normalized = normalize_text(text);
    normalized.contains("email") || normalized.contains("e mail")
}

/// Whether normalized text names a phone number.
pub fn mentions_phone(text: &str) -> bool {
    let normalized = normalize_text(text);
    ["phone", "telephone", "mobile"].iter().any(|word| normalized.contains(word))
}

/// Whether normalized text names a username.
pub fn mentions_username(text: &str) -> bool {
    let normalized = normalize_text(text);
    ["user name", "username", "login", "account", "member", "membership", "mileageplus"]
        .iter()
        .any(|word| normalized.contains(word))
}

/// Classify a login control as (score, token), or None (classifyLoginControl).
pub fn classify_login(autocomplete: &str, kind: &str, name: &str, label: &str) -> Option<(u32, &'static str)> {
    if is_excluded(autocomplete) {
        return None;
    }
    let tokens = autocomplete_tokens(autocomplete);
    for token in LOGIN_TOKENS {
        if tokens.iter().any(|value| value == token) {
            return Some((EXACT_SCORE, token));
        }
    }
    let searchable = normalize_text(&format!("{name} {label}"));
    if is_excluded_password(&searchable) {
        return None;
    }
    match kind.to_ascii_lowercase().as_str() {
        "password" => Some((TYPE_SCORE, "current-password")),
        "email" => Some((85, "email")),
        "tel" => Some((85, "tel")),
        _ if mentions_email(&searchable) => Some((HEURISTIC_SCORE + 5, "email")),
        _ if mentions_phone(&searchable) => Some((HEURISTIC_SCORE + 5, "tel")),
        _ if mentions_username(&searchable) => Some((HEURISTIC_SCORE, "username")),
        _ => None,
    }
}

/// Whether normalized text looks like a one-time code field (_RE_OTP).
pub fn mentions_otp(text: &str) -> bool {
    let normalized = normalize_text(text);
    let code_word = ["code", "pin", "token"].iter().any(|word| normalized.contains(word));
    let otp_word = ["otp", "totp", "2fa", "mfa", "passcode", "one time", "verification", "security"]
        .iter()
        .any(|word| normalized.contains(word));
    otp_word && (code_word || normalized.contains("otp") || normalized.contains("totp"))
}

/// Whether a control type can carry an OTP.
pub fn is_otp_kind(kind: &str) -> bool {
    matches!(kind.to_ascii_lowercase().as_str(), "" | "text" | "tel" | "number" | "password")
}

/// Classify an OTP control, returning its score (classifyOtpControls).
pub fn classify_otp(autocomplete: &str, kind: &str, name: &str, label: &str) -> Option<u32> {
    if autocomplete_tokens(autocomplete).iter().any(|token| token == "one-time-code") {
        return Some(EXACT_SCORE);
    }
    if is_otp_kind(kind) && mentions_otp(&format!("{name} {label}")) {
        return Some(HEURISTIC_SCORE);
    }
    None
}

/// Pick the best password control by (-score, index) (selectPasswordFill).
pub fn best_password<'a>(controls: &'a [(u32, &'static str, usize)]) -> Option<&'a (u32, &'static str, usize)> {
    controls
        .iter()
        .filter(|(_, token, _)| *token == "current-password")
        .min_by_key(|(score, _, index)| (std::cmp::Reverse(*score), *index))
}

/// Build a fill descriptor for a selected control.
pub fn fill_descriptor(index: usize, token: &str, value: &str) -> Value {
    serde_json::json!({"index": index, "token": token, "value": value})
}

/// Classify a checkout control from its autocomplete token (classifyCheckoutControl).
pub fn checkout_token(autocomplete: &str) -> Option<&'static str> {
    let tokens = autocomplete_tokens(autocomplete);
    PAYMENT_TOKENS.iter().chain(ADDRESS_TOKENS.iter()).copied().find(|token| tokens.iter().any(|value| value == *token))
}

/// Heuristic token from a label/name for checkout fields.
pub fn checkout_heuristic(name: &str, label: &str) -> Option<&'static str> {
    let text = normalize_text(&format!("{name} {label}"));
    if text.contains("card number") || text.contains("cc number") {
        Some("cc-number")
    } else if text.contains("name on card") || text.contains("cardholder") {
        Some("cc-name")
    } else if text.contains("cvc") || text.contains("cvv") || text.contains("security code") {
        Some("cc-csc")
    } else if text.contains("exp month") {
        Some("cc-exp-month")
    } else if text.contains("exp year") {
        Some("cc-exp-year")
    } else if text.contains("expiry") || text.contains("expiration") {
        Some("cc-exp")
    } else if text.contains("zip") || text.contains("postal") {
        Some("postal-code")
    } else if text.contains("city") || text.contains("town") {
        Some("address-level2")
    } else {
        None
    }
}

/// Whether a control is a password/email field, never a checkout target.
pub fn is_never_checkout(kind: &str) -> bool {
    matches!(kind.to_ascii_lowercase().as_str(), "password" | "email")
}

/// Human label of an autocomplete token.
pub fn token_label(token: &str) -> String {
    crate::bdots_textfmt::upper_first(token.replace('-', " ").as_str())
}

/// SF Symbol for a fill token.
pub fn token_icon(token: &str) -> &'static str {
    match token {
        "current-password" => "lock.fill",
        "email" => "envelope.fill",
        "tel" => "phone.fill",
        "one-time-code" | "cc-csc" => "number",
        "cc-number" => "creditcard.fill",
        _ => "rectangle.and.pencil.and.ellipsis",
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(login: usize, otp: usize, checkout: usize, best: bool) -> Vec<(&'static str, String)> {
    vec![
        ("login", count_label(login)),
        ("otp", count_label(otp)),
        ("checkout", count_label(checkout)),
        ("password", if best { "found" } else { "none" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization() {
        assert_eq!(normalize_text("E-Mail Address"), "e mail address");
        assert_eq!(autocomplete_tokens("current-password webauthn"), vec!["current-password", "webauthn"]);
        assert!(is_excluded("new-password"));
        assert!(!is_excluded("current-password"));
        assert!(is_excluded_password("Confirm Password"));
        assert!(mentions_email("e mail"));
        assert!(mentions_phone("mobile"));
        assert!(mentions_username("user name"));
    }

    #[test]
    fn login_classification() {
        assert_eq!(classify_login("current-password", "text", "", ""), Some((100, "current-password")));
        assert_eq!(classify_login("", "password", "", ""), Some((90, "current-password")));
        assert_eq!(classify_login("", "email", "", ""), Some((85, "email")));
        assert_eq!(classify_login("", "text", "email", ""), Some((75, "email")));
        assert_eq!(classify_login("", "text", "username", ""), Some((70, "username")));
        assert_eq!(classify_login("new-password", "password", "", ""), None);
        assert_eq!(classify_login("", "text", "confirm password", ""), None);
    }

    #[test]
    fn otp_and_checkout() {
        assert!(mentions_otp("verification code"));
        assert!(mentions_otp("otp"));
        assert!(!mentions_otp("hello"));
        assert!(is_otp_kind("text"));
        assert!(!is_otp_kind("checkbox"));
        assert_eq!(classify_otp("one-time-code", "text", "", ""), Some(100));
        assert_eq!(classify_otp("", "text", "otp", ""), Some(70));
        assert_eq!(checkout_token("cc-number"), Some("cc-number"));
        assert_eq!(checkout_heuristic("card number", ""), Some("cc-number"));
        assert_eq!(checkout_heuristic("zip code", "postal"), Some("postal-code"));
        assert!(is_never_checkout("password"));
    }

    #[test]
    fn selection_and_presentation() {
        let controls = vec![(90, "current-password", 7), (100, "current-password", 2)];
        let best = best_password(&controls).unwrap();
        assert_eq!(best.2, 2);
        let fill = fill_descriptor(2, "current-password", "secret");
        assert_eq!(fill["token"], "current-password");
        assert_eq!(token_label("current-password"), "Current password");
        assert_eq!(token_icon("email"), "envelope.fill");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 1, 2, true).len(), 4);
    }
}
