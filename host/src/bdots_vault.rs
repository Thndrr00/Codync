#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `bot-secrets.ts`: command variables, one-time codes, login
//! fields, authenticator keys, identifier labels and card shaping. Composes helpers already
//! ported into this host. Only `botInsights` calls this.

use serde_json::{Map, Value, json};

/// Kinds of bot secret authentication (BotSecretAuth).
pub const AUTH_TYPES: [&str; 7] = ["bearer", "header", "basic", "login", "command", "card", "contact"];

/// Shortest one-time code kept (ONE_TIME_CODE_MIN_LENGTH).
pub const ONE_TIME_CODE_MIN: usize = 4;

/// Longest one-time code kept (ONE_TIME_CODE_MAX_LENGTH).
pub const ONE_TIME_CODE_MAX: usize = 12;

/// Most fields a website login may ask for (LOGIN_FIELD_LIMIT).
pub const LOGIN_FIELD_LIMIT: usize = 8;

/// Default labels of a username/password login card.
pub const DEFAULT_LOGIN_FIELD_LABELS: [&str; 2] = ["Username", "Password"];

/// Policies for when the bot may use a vault item (VaultPolicy).
pub const VAULT_POLICIES: [&str; 3] = ["allow", "off", "ask"];

/// Kinds saved without a site: command variables, cards and contacts (isSitelessAuth).
pub fn is_siteless_auth(kind: &str) -> bool {
    matches!(kind, "command" | "card" | "contact")
}

/// Environment variable a command credential is exported as (commandVariableName).
pub fn command_variable_name(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
}

/// Whether a name is a syntactically valid environment variable (AGENT_SECRET_NAME_PATTERN).
pub fn valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && name.len() <= 64
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Whole families of variables a command credential must never override.
pub const RESERVED_PREFIXES: [&str; 5] = ["LD_", "DYLD_", "RAKAZO_", "GIT_CONFIG_", "NPM_CONFIG_"];

/// Reserved names that change shells, loaders or TLS trust.
pub fn is_reserved_variable(name: &str) -> bool {
    matches!(
        name,
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "PWD"
            | "IFS"
            | "ENV"
            | "BASH_ENV"
            | "NODE_OPTIONS"
            | "NODE_PATH"
            | "PYTHONPATH"
            | "TMPDIR"
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "ALL_PROXY"
            | "NO_PROXY"
            | "SSL_CERT_FILE"
            | "SSL_CERT_DIR"
            | "NODE_EXTRA_CA_CERTS"
            | "NODE_TLS_REJECT_UNAUTHORIZED"
            | "OPENSSL_CONF"
            | "OPENSSL_MODULES"
            | "GIT_SSH_COMMAND"
            | "GIT_ASKPASS"
            | "SSH_ASKPASS"
    ) || RESERVED_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// Why a variable cannot be exported (commandVariableProblem).
pub fn command_variable_problem(name: &str) -> Option<&'static str> {
    if !valid_env_name(name) {
        Some("invalid")
    } else if is_reserved_variable(name) {
        Some("reserved")
    } else {
        None
    }
}

/// One-time code as typed into a page (normalizeOneTimeCode).
pub fn normalize_one_time_code(answer: &str) -> Option<String> {
    let code: String = answer.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() > ONE_TIME_CODE_MAX {
        return None;
    }
    let no_dashes: String = code.chars().filter(|c| *c != '-').collect();
    if no_dashes.len() < ONE_TIME_CODE_MIN {
        return None;
    }
    let valid = code.split('-').all(|group| !group.is_empty() && group.chars().all(|c| c.is_ascii_alphanumeric()));
    valid.then_some(code)
}

/// Whether two field labels mean the same thing (sameLoginFieldLabel).
pub fn same_login_field_label(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

/// Whether every label is distinct, ignoring case and surrounding space (uniqueLabels).
pub fn unique_labels(labels: &[String]) -> bool {
    let mut seen = Vec::new();
    for label in labels {
        let key = label.trim().to_ascii_lowercase();
        if seen.contains(&key) {
            return false;
        }
        seen.push(key);
    }
    true
}

/// Normalize a pasted base32 authenticator secret to whole bytes.
pub fn base32_normalized(secret: &str) -> Option<String> {
    let clean: String =
        secret.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>().to_ascii_uppercase();
    let padding = clean.len() - clean.trim_end_matches('=').len();
    let core = &clean[..clean.len() - padding];
    let remainder = core.len() % 8;
    if [1, 3, 6].contains(&remainder) {
        return None;
    }
    if padding > 0 && padding != (8 - remainder) % 8 {
        return None;
    }
    if core.len() < 16 || core.len() > 256 || !core.chars().all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
    {
        return None;
    }
    Some(core.to_owned())
}

/// Parse a pasted authenticator key or otpauth:// link (authenticatorKey).
pub fn authenticator_key(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 1024 {
        return None;
    }
    let mut secret = trimmed.to_owned();
    let mut algorithm = "sha1";
    let mut digits = 6_u64;
    let mut period = 30_u64;
    if trimmed.to_ascii_lowercase().starts_with("otpauth:") {
        let rest = trimmed[8..].strip_prefix("//")?;
        let host = rest.split('/').next().unwrap_or("");
        if !host.eq_ignore_ascii_case("totp") {
            return None;
        }
        let query = rest.split_once('?').map_or("", |(_, query)| query);
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key.to_ascii_lowercase().as_str() {
                "secret" => secret = value.to_owned(),
                "algorithm" => {
                    algorithm = match value.to_ascii_lowercase().as_str() {
                        "sha256" => "sha256",
                        "sha512" => "sha512",
                        _ => "sha1",
                    }
                }
                "digits" => digits = value.parse().unwrap_or(6),
                "period" => period = value.parse().unwrap_or(30),
                _ => {}
            }
        }
        if ![6, 8].contains(&digits) || ![30, 60].contains(&period) {
            return None;
        }
    }
    let normalized = base32_normalized(&secret)?;
    Some(json!({"secret": normalized, "algorithm": algorithm, "digits": digits, "period": period}))
}

/// Words that mark a login field as an identifier.
const IDENTIFIER_WORDS: [&str; 14] = [
    "id",
    "user",
    "username",
    "login",
    "email",
    "mail",
    "phone",
    "mobile",
    "cell",
    "tel",
    "telephone",
    "account",
    "member",
    "customer",
];

/// Plain words that may accompany an identifier label.
const FILLER_WORDS: [&str; 18] = [
    "number", "no", "num", "name", "address", "or", "and", "of", "your", "the", "last", "first", "digit", "digits",
    "national", "tax", "employee", "e",
];

/// Words of a label, lowercased and stripped of punctuation.
fn label_words(label: &str) -> Vec<String> {
    label
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

/// Whether a label is made only of identifier and filler words, such as "Phone number".
pub fn is_identifier_label(label: &str) -> bool {
    let words = label_words(label);
    let has_identifier = words.iter().any(|word| IDENTIFIER_WORDS.contains(&word.as_str()));
    has_identifier
        && words.iter().all(|word| {
            word.chars().all(|c| c.is_ascii_digit())
                || IDENTIFIER_WORDS.contains(&word.as_str())
                || FILLER_WORDS.contains(&word.as_str())
        })
}

/// Empty fields a login card starts with (loginCardFields).
pub fn login_card_fields(requested: Option<&[String]>) -> Vec<Value> {
    match requested {
        None => DEFAULT_LOGIN_FIELD_LABELS
            .iter()
            .enumerate()
            .map(|(index, label)| json!({"label": label, "value": "", "hidden": index > 0}))
            .collect(),
        Some(labels) => labels
            .iter()
            .map(|label| json!({"label": label, "value": "", "hidden": !is_identifier_label(label)}))
            .collect(),
    }
}

/// Whether answered fields exactly match the requested labels, in order (loginAnswerValue).
pub fn login_answer_valid(requested: Option<&[String]>, fields: &[Value]) -> bool {
    let labels: Vec<String> = requested
        .map(|values| values.to_vec())
        .unwrap_or_else(|| DEFAULT_LOGIN_FIELD_LABELS.iter().map(|label| (*label).to_owned()).collect());
    fields.len() == labels.len()
        && fields.iter().zip(labels.iter()).all(|(field, label)| {
            field["label"].as_str().is_some_and(|value| value.trim() == label.trim())
                && field["value"].as_str().is_some_and(|value| !value.is_empty())
        })
}

/// Command variable holding the password of a login (loginCommandName).
pub fn login_command_name(login_name: &str) -> String {
    format!("{login_name}-password")
}

/// Digits only from a card number as people type it.
pub fn card_digits(number: &str) -> String {
    number.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Luhn check every issuer's card number passes (luhnValid).
pub fn luhn_valid(digits: &str) -> bool {
    let mut sum = 0_u32;
    for (index, ch) in digits.chars().rev().enumerate() {
        let Some(mut digit) = ch.to_digit(10) else {
            return false;
        };
        if index % 2 == 1 {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
    }
    !digits.is_empty() && sum % 10 == 0
}

/// Card network by first digits (cardBrand).
pub fn card_brand(number: &str) -> &'static str {
    let digits = card_digits(number);
    if digits.starts_with('4') {
        "Visa"
    } else if (digits.len() >= 2 && digits[..2].parse::<u32>().is_ok_and(|n| (51..=55).contains(&n)))
        || (digits.starts_with("222") && digits.as_bytes().get(3).is_some_and(|b| (b'1'..=b'9').contains(b)))
        || (digits.starts_with("22") && digits.as_bytes().get(2).is_some_and(|b| (b'3'..=b'9').contains(b)))
        || digits.starts_with("23")
        || digits.starts_with("24")
        || digits.starts_with("25")
        || digits.starts_with("26")
        || digits.starts_with("2701")
        || digits.starts_with("2720")
    {
        "Mastercard"
    } else if digits.starts_with("34") || digits.starts_with("37") {
        "Amex"
    } else if digits.starts_with("6011")
        || digits.starts_with("65")
        || digits.starts_with("64")
        || digits.starts_with("622")
    {
        "Discover"
    } else if digits.starts_with("30") || digits.starts_with("36") || digits.starts_with("38") {
        "Diners Club"
    } else if digits.starts_with("35") {
        "JCB"
    } else if digits.starts_with("62") {
        "UnionPay"
    } else {
        "Card"
    }
}

/// Owner-facing card summary, without the full number (cardSummary).
pub fn card_summary(number: &str, month: u64, year: u64, name: Option<&str>) -> Value {
    let digits = card_digits(number);
    let mut summary = Map::new();
    summary.insert("brand".to_owned(), Value::String(card_brand(number).to_owned()));
    summary.insert(
        "last4".to_owned(),
        Value::String(digits.chars().rev().take(4).collect::<String>().chars().rev().collect()),
    );
    summary.insert("expMonth".to_owned(), json!(month));
    summary.insert("expYear".to_owned(), json!(year));
    if let Some(name) = name.filter(|value| !value.is_empty()) {
        summary.insert("name".to_owned(), Value::String(name.to_owned()));
    }
    Value::Object(summary)
}

/// MM/YY as shown on a card (cardExpiry).
pub fn card_expiry(month: u64, year: u64) -> String {
    format!("{month:02}/{:02}", year % 100)
}

/// Default name of an untitled card (cardTitle).
pub fn card_title(brand: &str, last4: &str) -> String {
    format!("{brand} · {last4}")
}

/// Expiry as people type it, parsed to month and four-digit year (parseCardExpiry).
pub fn parse_card_expiry(text: &str) -> Option<(u64, u64)> {
    let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let (month_text, year_text) = if let Some(index) = clean.find(|c| matches!(c, '/' | '-' | '.')) {
        (&clean[..index], &clean[index + 1..])
    } else if clean.len() == 4 {
        (&clean[..2], &clean[2..])
    } else if clean.len() == 3 {
        (&clean[..1], &clean[1..])
    } else {
        return None;
    };
    if month_text.is_empty()
        || year_text.is_empty()
        || !month_text.chars().all(|c| c.is_ascii_digit())
        || !year_text.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let month: u64 = month_text.parse().ok()?;
    let year: u64 = year_text.parse().ok()?;
    if !(1..=12).contains(&month) || year_text.len() > 4 {
        return None;
    }
    Some((month, if year < 100 { 2000 + year } else { year }))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(auth: &str, fields: usize, cards: usize, replay: bool) -> Vec<(&'static str, String)> {
    vec![
        ("auth", crate::bdots_textfmt::upper_first(auth)),
        ("fields", count_label(fields)),
        ("cards", count_label(cards)),
        ("policy", if replay { "ask" } else { "allow" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_and_variables() {
        assert!(is_siteless_auth("command"));
        assert!(!is_siteless_auth("login"));
        assert_eq!(command_variable_name("netbird-setup-key"), "NETBIRD_SETUP_KEY");
        assert!(valid_env_name("netbird-setup-key"));
        assert!(!valid_env_name("BAD"));
        assert_eq!(command_variable_problem("PATH"), Some("reserved"));
        assert_eq!(command_variable_problem("BAD"), Some("invalid"));
        assert_eq!(command_variable_problem("my_key"), None);
        assert!(VAULT_POLICIES.contains(&"ask"));
    }

    #[test]
    fn codes_and_fields() {
        assert_eq!(normalize_one_time_code("123 456"), Some("123456".to_owned()));
        assert_eq!(normalize_one_time_code("12-3456"), Some("12-3456".to_owned()));
        assert_eq!(normalize_one_time_code("abc"), None);
        assert!(same_login_field_label(" EMAIL ", "email"));
        assert!(unique_labels(&["a".to_owned(), "b".to_owned()]));
        assert!(!unique_labels(&["a".to_owned(), "A".to_owned()]));
        assert_eq!(login_card_fields(None).len(), 2);
        assert_eq!(login_card_fields(None)[1]["hidden"], true);
        assert!(login_card_fields(Some(&["Phone number".to_owned()]))[0]["hidden"] == false);
        assert!(login_answer_valid(
            None,
            &[json!({"label":"Username","value":"a"}), json!({"label":"Password","value":"b"})]
        ));
        assert_eq!(login_command_name("work"), "work-password");
    }

    #[test]
    fn authenticator_keys() {
        let key = authenticator_key("JBSWY3DPEHPK3PXP");
        assert!(key.is_some());
        assert_eq!(key.unwrap()["digits"], 6);
        assert!(authenticator_key("A").is_none());
        let link = "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP&algorithm=SHA256&digits=8&period=60";
        let parsed = authenticator_key(link).unwrap();
        assert_eq!(parsed["algorithm"], "sha256");
        assert_eq!(parsed["digits"], 8);
        assert_eq!(parsed["period"], 60);
    }

    #[test]
    fn identifier_labels() {
        assert!(is_identifier_label("Phone number"));
        assert!(is_identifier_label("ID"));
        assert!(!is_identifier_label("Password"));
        assert!(!is_identifier_label("Phone number extra"));
    }

    #[test]
    fn cards_and_presentation() {
        assert!(luhn_valid("4242424242424242"));
        assert!(!luhn_valid("4242424242424241"));
        assert_eq!(card_digits("4242-4242 4242 4242").len(), 16);
        assert_eq!(card_brand("4242424242424242"), "Visa");
        assert_eq!(card_brand("5555555555554444"), "Mastercard");
        assert_eq!(card_brand("378282246310005"), "Amex");
        assert_eq!(card_brand("1234"), "Card");
        let card = card_summary("4242424242424242", 8, 2028, Some("Work"));
        assert_eq!(card["last4"], "4242");
        assert_eq!(card["name"], "Work");
        assert_eq!(card_expiry(8, 2028), "08/28");
        assert_eq!(card_title("Visa", "4242"), "Visa · 4242");
        assert_eq!(parse_card_expiry("8/28"), Some((8, 2028)));
        assert_eq!(parse_card_expiry("0828"), Some((8, 2028)));
        assert_eq!(parse_card_expiry("13/28"), None);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("card", 2, 1, false).len(), 4);
    }
}
