#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! A second batch of pure helpers from bdots' current source: tool-call descriptions, error
//! reasons, safety validation, money/format and skill parsing. All deterministic, no client/DB.

use serde_json::Value;

// `shorten` lives in bdots_activity; keep the historical bdots_more path working for callers.
pub use crate::bdots_activity::shorten;

/// The first string field of a call's input (`describeCall` family).
fn str_field<'a>(input: &'a Value, key: &str) -> &'a str {
    input.get(key).and_then(Value::as_str).unwrap_or("")
}

/// One-line past-tense description of a tool call for a folded step list (`message-parts.ts`).
pub fn describe_call(name: &str, input: &Value) -> String {
    let s = |key: &str| str_field(input, key);
    let clean = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    match name {
        "read" => format!("Read {}", s("path")),
        "ls" => format!("Listed {}", if s("path").is_empty() { "/workspace" } else { s("path") }),
        "find" => format!("Searched files for {}", s("pattern")),
        "grep" => format!("Searched for \"{}\"", s("pattern")),
        "write" => format!("Wrote {}", s("path")),
        "edit" => format!("Edited {}", s("path")),
        "delete" => format!("Deleted {}", s("path")),
        "exec" => format!("Ran {}", clean(s("command"))),
        "browser_markdown" => format!("Read {}", s("url")),
        "browser_extract" => format!("Extracted data from {}", s("url")),
        "browser_links" => format!("Listed links on {}", s("url")),
        "browser_scrape" => format!("Scraped {}", if s("url").is_empty() { "a page" } else { s("url") }),
        "browser_screenshot" => {
            format!("Took a screenshot of {}", if s("url").is_empty() { "a page" } else { s("url") })
        }
        "browser_pdf" => format!("Saved {} as PDF", if s("url").is_empty() { "a page" } else { s("url") }),
        "browser_crawl" => format!("Started crawling {}", s("url")),
        "message_bot" => format!("Messaged {}", s("to")),
        "send_email" => format!("Emailed {}", s("to")),
        "reply_email" => "Replied to an email".into(),
        "app_tool_call" => {
            let tool = s("tool");
            let first = tool.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
            format!("{first}{}", tool.chars().skip(1).collect::<String>())
        }
        other => format!("Used {other}"),
    }
}

/// A tool call's category for icons and grouping (`stepKind`).
pub fn step_kind(name: &str) -> &'static str {
    match name {
        "exec" => "shell",
        "read" | "ls" | "write" | "edit" | "delete" => "files",
        "find" | "grep" => "search",
        "search_history" | "read_history" => "history",
        "search_context" => "memory",
        "schedule_routine" | "update_routine" | "list_routines" | "cancel_routine" => "routine",
        "message_bot" | "delegate_task" | "attach_result_files" => "bot",
        "save_app" | "list_apps" | "delete_app" | "app_data_list" | "app_data_put" | "app_data_delete" => "app",
        _ if name.starts_with("shop_") => "shop",
        _ if name.starts_with("browser_") => "web",
        _ if name.starts_with("tool_") => "app",
        _ => "other",
    }
}

/// Header for a run of same-kind steps (`describeGroup`): "Ran 4 commands", "Changed 2 files".
pub fn describe_group(kind: &str, calls: &[String], has_write: bool) -> String {
    let n = calls.len();
    match kind {
        "shell" => format!("Ran {n} commands"),
        "files" if has_write => format!("Changed {n} files"),
        "files" => format!("Read {n} files"),
        "search" => format!("Searched {n} times"),
        "history" => format!("Looked back through history {n} times"),
        "web" => format!("Visited {n} pages"),
        _ => format!("{n} steps"),
    }
}

/// The reason an app call failed, in plain words, clipped at 120 (`app-tool-log.ts`).
pub fn plain_reason(error: Option<&str>) -> String {
    let e = error.unwrap_or("");
    let wait = e.find("Try again in ").and_then(|i| {
        let rest = &e[i + 13..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() { None } else { Some(digits) }
    });
    if e.starts_with("Too many") {
        return match wait {
            Some(min) => format!("Blocked: too many from one visitor, try again in {min} min"),
            None => "Blocked: too many from one visitor".into(),
        };
    }
    if e.contains("has not allowed this app") {
        return "Blocked: not allowed yet".into();
    }
    if e.contains("is not one the user allowed") || e.trim_end().ends_with("is missing") {
        return "Blocked: tried something you didn't allow".into();
    }
    if e.contains("rules block this bot") {
        return "Blocked by the bot's rules".into();
    }
    if e.contains("not connected right now") {
        return "Failed: app not connected".into();
    }
    if e.contains("sign-in expired") || e.to_lowercase().contains("reconnect") {
        return "Failed: app needs you to sign in again".into();
    }
    let short: String = e.chars().take(120).collect();
    format!("Failed: {}", if short.is_empty() { "unknown error" } else { &short })
}

/// A secret name the model may refer to: starts with a letter, then letters/digits/_/-, ≤64 (`secrets.ts`).
pub fn valid_secret_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else { return false };
    first.is_ascii_alphabetic() && name.len() <= 64 && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Remove every form of a secret a response could echo back: raw, URL-encoded and base64 (`redact`).
pub fn redact(text: &str, value: &str) -> String {
    if value.len() < 4 {
        return text.to_owned();
    }
    let mut forms = vec![value.to_owned(), url_encode(value)];
    if let Some(b64) = base64_latin1(value) {
        forms.push(b64);
    }
    let mut out = text.to_owned();
    for form in forms {
        out = out.replace(&form, "[redacted]");
    }
    out
}

fn url_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn base64_latin1(value: &str) -> Option<String> {
    if !value.chars().all(|c| (c as u32) <= 255) {
        return None;
    }
    // Reuse the crate already in the host rather than writing an encoder.
    use base64::Engine;
    Some(base64::engine::general_purpose::STANDARD.encode(value.as_bytes()))
}

/// The plain HTTPS origin of a URL, or `None` for anything else (`httpsOrigin`). Credentials in
/// the URL are refused; a path, query or fragment is dropped from the returned origin.
pub fn https_origin(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let rest = trimmed.strip_prefix("https://")?;
    if rest.is_empty() {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() || authority.contains('@') || authority.contains(' ') {
        return None;
    }
    Some(format!("https://{authority}"))
}

/// A stable id for a granted tool, including its limits, so widening re-asks (`approvalKey`).
pub fn approval_key(name: &str, read_only: bool, only: &[String]) -> String {
    let mut sorted = only.to_vec();
    sorted.sort();
    let suffix = if sorted.is_empty() { String::new() } else { format!(":{}", sorted.join(",")) };
    format!("{}:{}:{}", name, if read_only { "read" } else { "write" }, suffix).trim_end_matches(':').to_owned()
}

/// A mini-app slug: lowercase alphanumeric and dashes, 1–48 chars (`isAppSlug`).
pub fn is_app_slug(slug: &str) -> bool {
    (1..=48).contains(&slug.len())
        && slug.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Whole percent off a list price, or `None` when there is no real sale (`discountPercent`).
pub fn discount_percent(amount: f64, currency: &str, list: Option<(f64, &str)>) -> Option<u32> {
    let (list_amount, list_currency) = list?;
    if list_currency != currency || list_amount <= amount {
        return None;
    }
    let off = ((1.0 - amount / list_amount) * 100.0).round() as i32;
    (off >= 1).then_some(off as u32)
}

/// Add `?width=` to a Shopify CDN image; other hosts pass through (`sizedImage`).
pub fn sized_image(url: &str, width: u32) -> String {
    if !url.starts_with("https://cdn.shopify.com/") && !url.starts_with("http://cdn.shopify.com/") {
        return url.to_owned();
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    let kept = url.split('?').next().unwrap_or(url);
    format!("{kept}{separator}width={width}")
}

/// A store's hostname without `www.`, falling back to its domain (`storeHost`).
pub fn store_host(url: &str, domain: &str) -> String {
    let host = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))
        .unwrap_or(url.trim())
        .split('/')
        .next()
        .unwrap_or("")
        .split('@')
        .last()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host.is_empty() { domain.to_owned() } else { host.to_owned() }
}

/// A checkout the person can still act on: not completed/canceled, not expired (`checkoutOpen`).
pub fn checkout_open(status: &str, expires_at: Option<&str>, now_ms: i64) -> bool {
    if status == "completed" || status == "canceled" {
        return false;
    }
    let Some(expires) = expires_at else { return true };
    // An unparseable expiry is treated as no expiry, as bdots does with `NaN`.
    match chrono::DateTime::parse_from_rfc3339(expires) {
        Ok(at) => at.timestamp_millis() > now_ms,
        Err(_) => true,
    }
}

/// Compact context-window label (`formatContext`): 1_000_000 → "1M", 262_144 → "262K".
pub fn format_context(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let m = tokens / 1_000_000;
        let rem = (tokens % 1_000_000) / 100_000;
        if rem == 0 { format!("{m}M") } else { format!("{m}.{rem}M") }
    } else if tokens >= 1_000 {
        format!("{}K", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}

/// Model price line (`formatPrice`): [2, 10] → "$2 in · $10 out per 1M".
pub fn format_price(input_per_m: f64, output_per_m: f64) -> String {
    let money = |v: f64| if v.fract() == 0.0 { format!("${v:.0}") } else { format!("${v}") };
    format!("{} in · {} out per 1M", money(input_per_m), money(output_per_m))
}

/// A skill reference from a pasted install command or URL (`parseSkillRef`).
#[derive(Debug, PartialEq, Eq)]
pub struct SkillRef {
    pub owner: String,
    pub repo: String,
    pub global: bool,
    pub skill: Option<String>,
}

pub fn parse_skill_ref(text: &str) -> Option<SkillRef> {
    let trimmed = text.trim();
    let global = trimmed.contains(" -g") || trimmed.starts_with("-g ");
    let mut candidate = trimmed
        .strip_prefix("npx skills add ")
        .or_else(|| trimmed.strip_prefix("npx -y skills add "))
        .unwrap_or(trimmed)
        .trim()
        .trim_matches(|c| c == '\'' || c == '"')
        .to_owned();
    for flag in [" -g", " --global", " -s", " --skill"] {
        candidate = candidate.replace(flag, "");
    }
    let (repo, skill) = match candidate.split_once(" --skill ") {
        Some((r, s)) => (r.trim().to_owned(), Some(s.trim().to_owned())),
        None => (candidate.trim().to_owned(), None),
    };
    let repo = repo
        .trim_start_matches("https://github.com/")
        .trim_start_matches("git@github.com:")
        .trim_start_matches("github.com/")
        .trim_end_matches(".git")
        .trim_end_matches('/');
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some(SkillRef { owner: parts[0].to_owned(), repo: parts[1].to_owned(), global, skill })
}

/// Display string for a skill reference (`skillSource`).
pub fn skill_source(reference: &SkillRef) -> String {
    let base = format!("{}/{}", reference.owner, reference.repo);
    match &reference.skill {
        Some(skill) if !skill.is_empty() => format!("{base}#{skill}"),
        _ => base,
    }
}

/// The `name`/`description` of a SKILL.md, from its frontmatter (`parseSkillMd`).
pub fn parse_skill_md(text: &str) -> Option<(String, String, String)> {
    let trimmed = text.trim_start_matches('\u{feff}');
    let rest = trimmed.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let front = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_owned();
    let mut name = String::new();
    let mut description = String::new();
    for line in front.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = unquote(value.trim());
        } else if let Some(value) = line.strip_prefix("description:") {
            description = unquote(value.trim());
        }
    }
    if name.is_empty() {
        return None;
    }
    Some((name, description, body))
}

fn unquote(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

/// Filter found skills by wanted names, or name what is available (`pickSkills`).
pub fn pick_skills<'a>(found: &'a [String], wanted: &[String]) -> Result<Vec<&'a String>, Vec<&'a String>> {
    if wanted.is_empty() {
        return Ok(found.iter().collect());
    }
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for want in wanted {
        if let Some(skill) = found.iter().find(|s| *s == want) {
            out.push(skill);
        } else {
            missing.push(want);
        }
    }
    if missing.is_empty() { Ok(out) } else { Err(found.iter().collect()) }
}

/// Resolve a chat or background model with fallbacks (`pickModel`).
pub fn pick_model<'a>(configured: Option<&'a str>, background: Option<&'a str>, is_background: bool) -> &'a str {
    if is_background { background.or(configured).unwrap_or("default") } else { configured.unwrap_or("default") }
}

/// A gateway id, or "default" when unset (`gatewayId`).
pub fn gateway_id(value: Option<&str>) -> String {
    let trimmed = value.unwrap_or("").trim();
    if trimmed.is_empty() { "default".into() } else { trimmed.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn calls_describe_and_group() {
        assert_eq!(describe_call("read", &json!({"path": "a.rs"})), "Read a.rs");
        assert_eq!(describe_call("exec", &json!({"command": "npm  test"})), "Ran npm test");
        assert_eq!(describe_call("ls", &json!({})), "Listed /workspace");
        assert_eq!(step_kind("exec"), "shell");
        assert_eq!(step_kind("browser_page"), "web");
        assert_eq!(describe_group("shell", &["a".into(), "b".into()], false), "Ran 2 commands");
        assert_eq!(describe_group("files", &["a".into()], true), "Changed 1 files");
    }

    #[test]
    fn failures_read_plain_and_stay_short() {
        assert_eq!(
            plain_reason(Some("Too many from one visitor. Try again in 4 min")),
            "Blocked: too many from one visitor, try again in 4 min"
        );
        assert_eq!(plain_reason(Some("rules block this bot")), "Blocked by the bot's rules");
        assert_eq!(plain_reason(Some("sign-in expired")), "Failed: app needs you to sign in again");
        assert!(plain_reason(Some(&"x".repeat(400))).len() <= 126);
    }

    #[test]
    fn secrets_and_origins_are_strict() {
        assert!(valid_secret_name("stripe_key"));
        assert!(!valid_secret_name("1key"));
        assert_eq!(https_origin("https://api.test/v1").as_deref(), Some("https://api.test"));
        assert_eq!(https_origin("https://user@api.test"), None);
        assert_eq!(https_origin("http://api.test"), None);
        let out = redact("token=abc12345 and abc12345", "abc12345");
        assert!(!out.contains("abc12345"));
        assert_eq!(approval_key("call", true, &["b".to_owned(), "a".to_owned()]), "call:read:a,b");
    }

    #[test]
    fn shop_and_model_helpers() {
        assert!(is_app_slug("split-bill"));
        assert!(!is_app_slug("Bad Slug"));
        assert_eq!(discount_percent(75.0, "USD", Some((100.0, "USD"))), Some(25));
        assert_eq!(sized_image("https://cdn.shopify.com/p.png", 320), "https://cdn.shopify.com/p.png?width=320");
        assert_eq!(store_host("https://www.myshop.com/x", "fallback.com"), "myshop.com");
        assert!(!checkout_open("completed", None, 0));
        assert_eq!(format_context(262_144), "262K");
        assert_eq!(format_price(2.0, 10.0), "$2 in · $10 out per 1M");
    }

    #[test]
    fn skill_and_model_selection() {
        let reference = parse_skill_ref("npx skills add owner/repo --skill deep -g").unwrap();
        assert_eq!(reference.owner, "owner");
        assert_eq!(reference.repo, "repo");
        assert!(reference.global);
        assert_eq!(skill_source(&reference), "owner/repo#deep");
        let (name, description, _) = parse_skill_md("---\nname: Deploy\ndescription: \"A skill\"\n---\nBody").unwrap();
        assert_eq!(name, "Deploy");
        assert_eq!(description, "A skill");
        assert_eq!(pick_model(Some("chat"), Some("bg"), true), "bg");
        assert_eq!(gateway_id(None), "default");
    }
}
