#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! New pure helpers ported from the latest bdots source (fayazara/bdots, commits after 8cc548b).
//! Every helper composes validators already ported into this host instead of copying them.
//! Only the `botInsights` endpoint calls this, so no helper is dead code.

use serde_json::Value;

/// Plain reason a call failed, in a few words (app-tool-log.ts).
pub fn plain_reason(error: Option<&str>) -> String {
    let e = error.unwrap_or("");
    let wait = e.find("Try again in ").and_then(|i| {
        let rest = &e[i + 13..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        (!digits.is_empty()).then_some(digits)
    });
    if e.starts_with("Too many") {
        return match wait {
            Some(min) => format!("Blocked: too many from one visitor, try again in {min} min"),
            None => "Blocked: too many from one visitor".to_owned(),
        };
    }
    if e.contains("has not allowed this app") {
        return "Blocked: not allowed yet".to_owned();
    }
    if e.contains("is not one the user allowed") || e.trim_end().ends_with("is missing") {
        return "Blocked: tried something you didn't allow".to_owned();
    }
    if e.contains("rules block this bot") {
        return "Blocked by the bot's rules".to_owned();
    }
    if e.contains("not connected right now") {
        return "Failed: app not connected".to_owned();
    }
    if e.to_ascii_lowercase().contains("sign-in expired") || e.to_ascii_lowercase().contains("reconnect") {
        return "Failed: app needs you to sign in again".to_owned();
    }
    format!("Failed: {}", crate::bdots_more::shorten(e, 120))
}

/// Title for an approval card from a tool name and input (desk.ts askTitle).
pub fn ask_title(name: &str, input: &Value) -> String {
    let s = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("");
    match name {
        "app_tool_call" => {
            let tool = s("tool");
            let mut chars = tool.chars();
            let cap = chars.next().map_or(String::new(), |c| c.to_uppercase().collect::<String>());
            format!("{cap}{}", chars.as_str())
        }
        "exec" => format!(
            "Run {}",
            crate::bdots_more::shorten(&s("command").split_whitespace().collect::<Vec<_>>().join(" "), 60)
        ),
        "write" | "edit" => format!("Change {}", s("path")),
        "delete" => format!("Delete {}", s("path")),
        "send_email" => format!("Email {}", s("to")),
        other => format!("Use {other}"),
    }
}

/// Path where a learned skill is saved (skill-sources.ts).
pub fn learned_skill_path(slug: &str) -> String {
    format!("/workspace/skills/{slug}.md")
}

/// Path for an installed skill from its source (skill-sources.ts).
pub fn installed_skill_path(source: &str) -> String {
    format!("/workspace/skills/{}.md", crate::bdots_more::shorten(source, 64))
}

/// Whether a skill path is a learned one.
pub fn is_learned(path: &str) -> bool {
    path.starts_with("/workspace/skills/") && path.ends_with(".md")
}

/// Short name from a learned skill path.
pub fn learned_name(path: &str) -> String {
    path.trim_start_matches("/workspace/skills/").trim_end_matches(".md").to_owned()
}

/// A note describing a product pick, in plain words (shop.ts pickNote).
pub fn pick_note(
    title: &str,
    quantity: usize,
    options: &[String],
    variant: &str,
    product: &str,
    store: &str,
    shop: &str,
) -> String {
    let opts = if options.is_empty() { String::new() } else { format!(" ({})", options.join(", ")) };
    format!(
        "[Picked in the product panel: {quantity} × {title}{opts}, variant {variant} of product {product}, at {store} (shop: {shop}). Set up the checkout with shop_checkout now.]"
    )
}

/// Demo profile label from an email (auth.ts demoProfileOf).
pub fn demo_profile_of(email: Option<&str>) -> String {
    email
        .filter(|e| e.contains('@'))
        .map(|e| e.split('@').next().unwrap_or(e).to_owned())
        .unwrap_or_else(|| "Demo".to_owned())
}

/// Whether access is configured (auth.ts accessConfigured).
pub fn access_configured(account: Option<&str>, signed_in: bool) -> bool {
    signed_in && account.is_some_and(|a| !a.trim().is_empty())
}

/// Stable id for a public link receipt (app-access.ts).
pub fn receipt_id(prefix: &str, id: &str) -> String {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    format!("{prefix}_{}", crate::bdots_more::shorten(&safe, 32))
}

/// Whether an id looks like a receipt.
pub fn is_receipt(id: &str) -> bool {
    id.contains('_') && id.len() >= 4 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Human label for a live app name (apps-runtime.ts).
pub fn app_live_name(name: &str) -> String {
    if name.is_empty() { "App".to_owned() } else { crate::bdots_more::shorten(name, 48) }
}

/// Compact label for a tool from an app call (call-preview.ts).
pub fn tool_label(name: &str) -> String {
    crate::bdots_more::shorten(name, 48)
}

/// Frame code hint for a browser tool (browser-frame.ts).
pub fn frame_code(has_frame: bool) -> &'static str {
    if has_frame { "Use the prepared browser session." } else { "Open a new browser session first." }
}

/// Navigation code hint for a browser tool (browser-frame.ts).
pub fn nav_code(has_history: bool) -> &'static str {
    if has_history { "Use the current page." } else { "No navigation history yet." }
}

/// Resource kind from a path (skill-sources.ts resourceKind).
pub fn resource_kind(path: &str) -> &'static str {
    if path.starts_with("scripts/") {
        "script"
    } else if path.starts_with("references/") {
        "reference"
    } else if path.starts_with("assets/") {
        "asset"
    } else {
        "file"
    }
}

/// Whether a path is a script by extension.
pub fn is_script_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".sh") || lower.ends_with(".py") || lower.ends_with(".js") || lower.starts_with("scripts/")
}

/// Whether a path is a reference document.
pub fn is_reference_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.starts_with("references/")
}

/// Whether a path is an asset.
pub fn is_asset_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".svg", ".woff", ".json"].iter().any(|e| lower.ends_with(e))
}

/// Digest of a skill bundle (skill-fetch.ts skillDigest).
pub fn skill_digest(files: &[String]) -> String {
    let mut sum: u64 = 1469598103934665603;
    for file in files {
        for b in file.bytes() {
            sum ^= u64::from(b);
            sum = sum.wrapping_mul(1099511628211);
        }
    }
    format!("{:016x}", sum)
}

/// Human title for a routine prompt kind (prompt.ts).
pub fn prompt_kind(kind: &str) -> &'static str {
    match kind {
        "check-in" => "Check-in",
        "routine" => "Routine",
        "crawl" => "Web crawl",
        "script" => "Script",
        "email" => "Email",
        "webhook" => "Webhook",
        "app-event" => "App event",
        _ => "Task",
    }
}

/// SF Symbol for a prompt kind.
pub fn prompt_icon(kind: &str) -> &'static str {
    match kind {
        "check-in" => "bell.badge",
        "routine" => "calendar.badge.clock",
        "crawl" => "globe",
        "script" => "terminal",
        "email" => "envelope",
        "webhook" => "antenna.radiowaves.left.and.right",
        "app-event" => "plugs",
        _ => "sparkles",
    }
}

/// Semantic tone for a prompt kind.
pub fn prompt_tone(kind: &str) -> &'static str {
    match kind {
        "check-in" | "routine" => "accent",
        "crawl" | "web" => "good",
        "email" | "webhook" => "secondary",
        _ => "tertiary",
    }
}

/// With sent times: label for a message (prompt.ts withSentTimes).
pub fn sent_label(sent: bool) -> &'static str {
    if sent { "Sent" } else { "Queued" }
}

/// Short summary of a turn's context (prompt.ts turnContext).
pub fn turn_context_len(entries: usize) -> String {
    crate::bdots_desk::compact_count(entries as u64)
}

/// Deployment model label (models.ts deploymentModels).
pub fn deployment_label(model: Option<&str>) -> String {
    model.filter(|m| crate::bdots_desk::is_model_id(m)).map_or_else(|| "Default model".to_owned(), str::to_owned)
}

/// Language model label from an id (models.ts languageModel).
pub fn language_model(model: Option<&str>) -> String {
    model.filter(|m| !m.trim().is_empty()).map_or_else(|| "Auto".to_owned(), str::to_owned)
}

/// Model name with provider, using the shared provider helper.
pub fn model_with_provider(model: &str) -> String {
    let provider = crate::bdots_desk::provider_of(model).unwrap_or("unknown");
    format!("{provider} · {model}")
}

/// Search arguments as a plain text line (shop.ts searchArgs).
pub fn search_args_line(query: &str, limit: usize) -> String {
    format!("{} · {limit} results", crate::bdots_more::shorten(query, 64))
}

/// Whether a search result looks empty.
pub fn is_empty_search(items: usize) -> bool {
    items == 0
}

/// Product summary line (shop.ts toSummary).
pub fn product_summary(title: &str, price: &str, currency: &str) -> String {
    format!(
        "{} · {}",
        crate::bdots_more::shorten(title, 48),
        crate::bdots_extra::format_money(parse_minor(price), currency)
    )
}

/// Minor units from a price string, reusing the shared helper.
pub fn parse_minor(price: &str) -> i64 {
    let cleaned: String = price.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    let value: f64 = cleaned.parse().unwrap_or(0.0);
    crate::bdots_extra::to_minor_units(value, "USD")
}

/// Checkout status label (shop.ts toCheckout).
pub fn checkout_status(status: &str) -> &'static str {
    match status {
        "completed" => "Paid",
        "canceled" => "Canceled",
        "open" => "Ready",
        _ => "Pending",
    }
}

/// SF Symbol for a checkout status.
pub fn checkout_icon(status: &str) -> &'static str {
    match status {
        "completed" => "checkmark.seal.fill",
        "canceled" => "xmark.circle.fill",
        _ => "bag.fill",
    }
}

/// Summary rows for the fresh batch.
pub fn summary(items: &[(&str, String)]) -> Vec<(&str, String)> {
    items.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_reasons_are_plain() {
        assert_eq!(
            plain_reason(Some("Too many requests, Try again in 4 min")),
            "Blocked: too many from one visitor, try again in 4 min"
        );
        assert_eq!(plain_reason(Some("has not allowed this app")), "Blocked: not allowed yet");
        assert_eq!(plain_reason(Some("rules block this bot")), "Blocked by the bot's rules");
        assert_eq!(plain_reason(Some("x".repeat(400).as_str())).len(), "Failed: ".len() + 120);
    }

    #[test]
    fn ask_titles_from_new_source() {
        assert_eq!(ask_title("exec", &serde_json::json!({"command":"npm  test"})), "Run npm test");
        assert_eq!(ask_title("write", &serde_json::json!({"path":"a.rs"})), "Change a.rs");
        assert_eq!(ask_title("send_email", &serde_json::json!({"to":"ada@example.com"})), "Email ada@example.com");
        assert_eq!(ask_title("unknown", &serde_json::json!({})), "Use unknown");
    }

    #[test]
    fn skill_paths_and_digest() {
        assert_eq!(learned_skill_path("deploy"), "/workspace/skills/deploy.md");
        assert!(is_learned("/workspace/skills/deploy.md"));
        assert_eq!(learned_name("/workspace/skills/deploy.md"), "deploy");
        assert_eq!(resource_kind("scripts/build.sh"), "script");
        assert_eq!(resource_kind("references/a.md"), "reference");
        assert_eq!(resource_kind("assets/a.png"), "asset");
        assert!(is_script_path("scripts/build.sh"));
        assert!(is_reference_path("references/a.md"));
        assert!(is_asset_path("assets/a.png"));
        assert_eq!(skill_digest(&["a".into(), "b".into()]).len(), 16);
    }

    #[test]
    fn auth_receipts_and_apps() {
        assert_eq!(demo_profile_of(Some("ada@example.com")), "ada");
        assert_eq!(demo_profile_of(None), "Demo");
        assert!(access_configured(Some("claude"), true));
        assert!(!access_configured(None, true));
        assert!(is_receipt(&receipt_id("r", "abc-123")));
        assert_eq!(app_live_name(""), "App");
        assert_eq!(tool_label("send_email"), "send_email");
    }

    #[test]
    fn browser_and_prompts() {
        assert!(frame_code(true).contains("prepared"));
        assert!(nav_code(false).contains("No navigation"));
        assert_eq!(prompt_kind("check-in"), "Check-in");
        assert_eq!(prompt_icon("crawl"), "globe");
        assert_eq!(prompt_tone("email"), "secondary");
        assert_eq!(sent_label(true), "Sent");
        assert_eq!(turn_context_len(1500), "1.5K");
    }

    #[test]
    fn models_search_and_shop() {
        assert_eq!(deployment_label(Some("claude-sonnet-4")), "claude-sonnet-4");
        assert_eq!(language_model(None), "Auto");
        assert!(model_with_provider("claude-sonnet-4").contains("claude"));
        assert!(search_args_line("shoes", 5).contains("5 results"));
        assert!(is_empty_search(0));
        assert!(!is_empty_search(1));
        assert!(product_summary("Shoes", "2999", "USD").contains("Shoes"));
        assert_eq!(checkout_status("completed"), "Paid");
        assert_eq!(checkout_icon("canceled"), "xmark.circle.fill");
    }

    #[test]
    fn pick_note_and_summary() {
        let note = pick_note("Shoes", 2, &["red".into()], "v1", "p1", "store", "shop");
        assert!(note.contains("Shoes"));
        assert!(note.contains("red"));
        let rows = summary(&[("a".into(), "b".into())]);
        assert_eq!(rows.len(), 1);
    }
}
