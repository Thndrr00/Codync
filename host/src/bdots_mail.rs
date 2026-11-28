#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `client/lib/mail.ts`: addresses, mailbox choices,
//! levels and policies. Composes helpers already ported into this host. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Custom preset name (CUSTOM_PRESET).
pub const CUSTOM_PRESET: &str = "custom";

/// "Send as" value when the agent is not pinned to one address (AUTO_FROM).
pub const AUTO_FROM: &str = "auto";

/// Default daily send cap for a new mailbox policy (DEFAULT_DAILY_CAP).
pub const DEFAULT_DAILY_CAP: u64 = 25;

/// Known mail connect errors (MAIL_ERRORS).
pub const MAIL_ERRORS: [&str; 6] = ["auth", "imap", "smtp", "tls", "provider", "rate_limit"];

/// Mailbox choices offered in the picker.
pub const MAIL_CHOICES: [&str; 4] = ["gmail", "outlook", "icloud", "other"];

/// Access levels a person may grant on a mailbox.
pub const MAIL_LEVELS: [&str; 4] = ["none", "read", "write", "read_write"];

/// Mail provider brands and their names.
pub const MAIL_BRANDS: [(&str, &str); 6] = [
    ("gmail", "Gmail"),
    ("outlook", "Outlook"),
    ("icloud", "iCloud Mail"),
    ("yahoo", "Yahoo Mail"),
    ("proton", "Proton Mail"),
    ("custom", "Custom"),
];

/// Normalize a mail connect error code (mailErrorOf).
pub fn mail_error_of(code: Option<&str>) -> &str {
    code.filter(|value| MAIL_ERRORS.contains(value)).unwrap_or("unknown")
}

/// Catalog choice of a connected mailbox (choiceOf).
pub fn choice_of(account: &Value) -> &'static str {
    match account["provider"].as_str().unwrap_or("") {
        "gmail" => "gmail",
        "outlook" => "outlook",
        _ if account["preset"].as_str() == Some("icloud") => "icloud",
        _ => "other",
    }
}

/// Login to use for a preset: the address or only its local part (loginOf).
pub fn login_of(preset: Option<&Value>, address: &str) -> String {
    let trimmed = address.trim();
    if preset.and_then(|value| value["username"].as_str()) == Some("local") {
        trimmed.split('@').next().unwrap_or(trimmed).to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Whether a value is an email address (isAddress).
pub fn is_address(value: &str) -> bool {
    let trimmed = value.trim();
    let mut parts = trimmed.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    parts.next().is_none()
        && !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !trimmed.chars().any(char::is_whitespace)
}

/// Whether a value is an address or a whole domain (isAddressOrDomain).
pub fn is_address_or_domain(value: &str) -> bool {
    let trimmed = value.trim().to_ascii_lowercase();
    is_address(&trimmed)
        || (trimmed.starts_with('@')
            && trimmed.len() > 3
            && trimmed[1..].contains('.')
            && !trimmed.chars().any(char::is_whitespace))
}

/// Webmail hint when an address cannot sign in to iCloud Mail (notIcloudMail).
pub fn not_icloud_mail(address: &str) -> Option<&'static str> {
    let lower = address.to_ascii_lowercase();
    if lower.contains("icloud.com")
        || lower.contains("me.com")
        || lower.contains("mac.com")
        || lower.contains('.') && !lower.contains("gmail")
    {
        None
    } else if lower.contains("gmail") {
        Some("Gmail")
    } else {
        None
    }
}

/// Split typed text into valid and invalid addresses/domains (splitAddresses).
pub fn split_addresses(text: &str) -> Value {
    let parts: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let valid: Vec<String> = parts.iter().filter(|part| is_address_or_domain(part)).cloned().collect();
    let invalid: Vec<String> = parts.into_iter().filter(|part| !is_address_or_domain(part)).collect();
    json!({"valid": valid, "invalid": invalid})
}

/// Split typed text into email addresses only (splitEmails).
pub fn split_emails(text: &str) -> Value {
    let parts = split_addresses(text);
    let valid: Vec<String> = parts["valid"]
        .as_array()
        .map(|values| {
            values.iter().filter_map(Value::as_str).filter(|value| is_address(value)).map(str::to_owned).collect()
        })
        .unwrap_or_default();
    let mut invalid: Vec<String> = parts["invalid"]
        .as_array()
        .map_or(Vec::new(), |values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect());
    if let Some(values) = parts["valid"].as_array() {
        invalid.extend(values.iter().filter_map(Value::as_str).filter(|value| !is_address(value)).map(str::to_owned));
    }
    json!({"valid": valid, "invalid": invalid})
}

/// Every address a mailbox sends as, own first (mailboxAddresses).
pub fn mailbox_addresses(account: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(address) = account["address"].as_str() {
        out.push(address.to_owned());
    }
    if let Some(aliases) = account["aliases"].as_array() {
        for alias in aliases.iter().filter_map(Value::as_str) {
            if !out.iter().any(|value| value == alias) {
                out.push(alias.to_owned());
            }
        }
    }
    out
}

/// Connector level as the mail API says it (toMailLevel).
pub fn to_mail_level(level: &str) -> &'static str {
    if level == "write" { "read_write" } else { level }
}

/// Mail API level as the connector says it (fromMailLevel).
pub fn from_mail_level(level: Option<&str>) -> &'static str {
    match level.unwrap_or("") {
        "read_write" => "write",
        other if MAIL_LEVELS.contains(&other) => other,
        _ => "none",
    }
}

/// Whether a viewer may give or remove a mailbox (mailAccessOf).
pub fn mail_access_of(account: &Value, viewer: &Value) -> Value {
    let owns =
        viewer["userId"].as_str().is_some_and(|id| !id.is_empty() && account["ownerUserId"].as_str() == Some(id));
    json!({
        "gives": owns && viewer["talks"] == true,
        "removes": owns || viewer["managesAgent"] == true,
    })
}

/// Default policy of a new mailbox (defaultPolicy).
pub fn default_policy() -> Value {
    json!({
        "folders": [],
        "senders": [],
        "windowDays": Value::Null,
        "allowedRecipients": [],
        "dailySendCap": DEFAULT_DAILY_CAP,
        "sendWithoutAsking": false,
        "fromAddress": Value::Null,
    })
}

/// Enforce that sending without asking needs allowed recipients (normalizePolicy).
pub fn normalize_policy(policy: &Value) -> Value {
    let mut next = policy.clone();
    let allowed = next["allowedRecipients"].as_array().map_or(0, Vec::len);
    if allowed == 0 && next["sendWithoutAsking"] == true {
        next["sendWithoutAsking"] = Value::Bool(false);
    }
    next
}

/// Sending policy level (canSendWithoutAsking).
pub fn can_send_without_asking(policy: &Value) -> bool {
    policy["sendWithoutAsking"] == true
        && policy["allowedRecipients"].as_array().is_some_and(|values| !values.is_empty())
}

/// Newest account by creation time (newestAccount).
pub fn newest_account(accounts: &[Value]) -> Value {
    accounts
        .iter()
        .max_by(|a, b| a["createdAt"].as_str().unwrap_or("").cmp(b["createdAt"].as_str().unwrap_or("")))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Brand name of a provider.
pub fn brand_name(provider: &str) -> &'static str {
    MAIL_BRANDS.iter().find(|(id, _)| *id == provider).map_or("Mail", |(_, name)| *name)
}

/// One line describing a mailbox (mailLine).
pub fn mail_line(provider: &str, address: &str, level: &str, cap: u64) -> String {
    format!("{provider} · {address} · {level} · {cap}/day")
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(accounts: usize, aliases: usize, allowed: usize, auto: bool) -> Vec<(&'static str, String)> {
    vec![
        ("accounts", count_label(accounts)),
        ("aliases", count_label(aliases)),
        ("allowed", count_label(allowed)),
        ("auto", if auto { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert!(is_address("ana@example.com"));
        assert!(!is_address("@example.com"));
        assert!(is_address_or_domain("@example.com"));
        assert_eq!(split_addresses("ana@x.com, bad, @y.com")["valid"].as_array().map_or(0, Vec::len), 2);
        assert_eq!(split_emails("ana@x.com @y.com")["valid"].as_array().map_or(0, Vec::len), 1);
        assert_eq!(login_of(Some(&json!({"username":"local"})), " ana@x.com "), "ana");
        assert_eq!(login_of(None, " ana@x.com "), "ana@x.com");
        assert_eq!(not_icloud_mail("a@gmail.com"), Some("Gmail"));
        assert_eq!(mail_error_of(Some("imap")), "imap");
        assert_eq!(mail_error_of(None), "unknown");
    }

    #[test]
    fn accounts_and_levels() {
        assert_eq!(choice_of(&json!({"provider":"gmail"})), "gmail");
        assert_eq!(choice_of(&json!({"preset":"icloud"})), "icloud");
        assert_eq!(mailbox_addresses(&json!({"address":"a@x.com","aliases":["b@x.com","a@x.com"]})).len(), 2);
        assert_eq!(to_mail_level("write"), "read_write");
        assert_eq!(from_mail_level(Some("read_write")), "write");
        assert_eq!(from_mail_level(None), "none");
        let access = mail_access_of(&json!({"ownerUserId":"u1"}), &json!({"userId":"u1","talks":true}));
        assert_eq!(access["gives"], true);
        assert_eq!(access["removes"], true);
        assert_eq!(brand_name("gmail"), "Gmail");
    }

    #[test]
    fn policies_and_presentation() {
        let policy = default_policy();
        assert_eq!(policy["dailySendCap"], 25);
        let dirty = json!({"allowedRecipients":[],"sendWithoutAsking":true});
        assert_eq!(normalize_policy(&dirty)["sendWithoutAsking"], false);
        let good = json!({"allowedRecipients":["a@x.com"],"sendWithoutAsking":true});
        assert!(can_send_without_asking(&good));
        let accounts = vec![
            json!({"createdAt":"2026-01-01","address":"a@x.com"}),
            json!({"createdAt":"2026-02-01","address":"b@x.com"}),
        ];
        assert_eq!(newest_account(&accounts)["address"], "b@x.com");
        assert!(mail_line("gmail", "a@x.com", "read", 25).contains("gmail"));
        assert_eq!(MAIL_CHOICES.len(), 4);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(2, 1, 1, false).len(), 4);
    }
}
