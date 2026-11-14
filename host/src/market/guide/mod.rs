//! How to connect an app: what to do at the vendor, the page to do it on, and the form's boxes in
//! words a person can follow.
//!
//! WHY THIS EXISTS. Most of the catalogues have no sign-in a person can be sent to: an app that only
//! takes a key, or a token, opens a form of the vendor's own field names, and the vendor writes those
//! for a developer ("Bearer Token", "Service Account Token (ops_* prefix)…"), saying nothing about
//! where to go. Somebody who expected a sign-in met a form and no way forward.
//!
//! WHERE THE LINK COMES FROM. The catalogue's own guide first, then the vendor's published hint, then
//! the map in `key_guides.json` (ported from `CopilotKit/OpenBot`; see `NOTICE.md`). A link that is
//! not the vendor's — `http`, or Composio's own site — is dropped rather than shown: a guessed link
//! is worse than none. An app missing from the map keeps the vendor's own help and gets no button.
//!
//! This module only supplies text and links. Nothing here talks to the network.

use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// What connecting an app asks of a person.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The vendor's own sign-in page.
    SignIn,
    /// A key or token typed into a form.
    Key,
    /// Nothing: the server answers without an account.
    Nothing,
    /// Nothing to connect: the server runs on this computer.
    Install,
}

impl Kind {
    fn wire(self) -> &'static str {
        match self {
            Self::SignIn => "signIn",
            Self::Key => "key",
            Self::Nothing => "nothing",
            Self::Install => "install",
        }
    }

    /// The step somebody reads when the catalogue published none.
    fn fallback(self, name: &str) -> String {
        match self {
            Self::SignIn => {
                format!("{name} opens its own sign-in page. Sign in with the account bots should use.")
            }
            Self::Key => format!("Sign in to {name} and look for API keys in its settings or developer pages."),
            Self::Nothing => format!("Nothing to fill in: {name} answers without an account."),
            Self::Install => format!("Nothing to connect: {name} runs on this computer."),
        }
    }
}

/// An app as its catalogue published it, before this module adds the guide.
pub struct Published<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub kind: Kind,
    /// The steps the catalogue wrote, if any.
    pub steps: &'a [String],
    /// The link the catalogue published for this app.
    pub docs: Option<&'a str>,
    /// The vendor's own key hint, when the broker published one.
    pub hint: Option<&'a str>,
}

#[derive(Deserialize, Clone, Default)]
struct KeyGuide {
    #[serde(default)]
    steps: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    fields: BTreeMap<String, FieldRewrite>,
}

#[derive(Deserialize, Clone, Default)]
struct FieldRewrite {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    help: Option<String>,
}

#[derive(Deserialize)]
struct Guides {
    apps: BTreeMap<String, KeyGuide>,
}

static GUIDES: LazyLock<Guides> =
    LazyLock::new(|| serde_json::from_str(include_str!("key_guides.json")).expect("the bundled key guides are valid"));

fn mapped(slug: &str) -> Option<&'static KeyGuide> {
    GUIDES.apps.get(&slug.to_lowercase())
}

/// A link only when it is the vendor's own page: `https`, and not the broker's site, which would put
/// Composio's name in front of somebody who is on their way to make a key.
fn provider_link(url: Option<&str>) -> Option<String> {
    let parsed = reqwest::Url::parse(url?.trim()).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    if host == "composio.dev" || host.ends_with(".composio.dev") {
        return None;
    }
    Some(parsed.to_string())
}

/// How to connect one app: the steps, and the page to do them on when there is a vendor page.
pub fn guide(p: &Published) -> Value {
    let map = mapped(p.slug);
    let mut steps: Vec<String> = match map {
        Some(g) if !g.steps.trim().is_empty() => vec![g.steps.trim().to_owned()],
        _ => p.steps.iter().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect(),
    };
    if steps.is_empty() {
        steps.push(p.kind.fallback(p.name));
    }
    json!({
        "kind": p.kind.wire(),
        "steps": steps,
        "url": provider_link(p.docs).or_else(|| provider_link(p.hint))
            .or_else(|| map.and_then(|g| provider_link(g.url.as_deref()))),
    })
}

/// The guide for a reviewed catalogue app.
pub fn catalog_app(a: &super::copilotkit::App) -> Value {
    let kind = match a.setup.as_str() {
        "key" => Kind::Key,
        "link" => Kind::SignIn,
        "local" => Kind::Install,
        _ => Kind::Nothing,
    };
    guide(&Published { slug: &a.id, name: &a.name, kind, steps: &a.steps, docs: a.docs_url.as_deref(), hint: None })
}

/// The guide for an app from the nightly list.
pub fn nightly_app(a: &super::openbot::App) -> Value {
    let kind = match a.setup.as_str() {
        "key" => Kind::Key,
        "link" => Kind::SignIn,
        "local" => Kind::Install,
        _ => Kind::Nothing,
    };
    guide(&Published { slug: &a.id, name: &a.name, kind, steps: &a.steps, docs: a.docs_url.as_deref(), hint: None })
}

/// The guide for a Composio app, from the links Composio published for it.
pub fn toolkit(slug: &str, name: &str, hint: Option<&str>, docs: Option<&str>, needs_key: Option<bool>) -> Value {
    let kind = if needs_key == Some(false) { Kind::SignIn } else { Kind::Key };
    guide(&Published { slug, name, kind, steps: &[], docs, hint })
}

/// Labels the catalogues publish again and again, in the words the rest of Codync uses.
///
/// Only the generic ones: a label naming the app ("Postmark Server API Token") already says what it
/// is and is left alone.
const PLAIN_LABELS: &[(&str, &str)] = &[
    ("API Key", "API key"),
    ("Api Key", "API key"),
    ("API Token", "API token"),
    ("API Secret", "API secret"),
    ("Bearer Token", "Access token"),
    ("Base URL", "Web address"),
    ("API URL", "API web address"),
    ("Sub Domain", "Subdomain"),
    ("Username", "Username"),
    ("Password", "Password"),
];

fn plain_label(slug: &str, name: &str, label: &str) -> String {
    if let Some(rewrite) = mapped(slug).and_then(|g| g.fields.get(name)).and_then(|r| r.label.as_deref()) {
        return rewrite.trim().to_owned();
    }
    PLAIN_LABELS
        .iter()
        .find(|(published, _)| *published == label.trim())
        .map_or_else(|| label.trim().to_owned(), |(_, plain)| (*plain).to_owned())
}

fn plain_help(slug: &str, name: &str, help: Option<&str>) -> Option<String> {
    let rewritten = mapped(slug).and_then(|g| g.fields.get(name)).and_then(|r| r.help.as_deref());
    rewritten.or(help).map(|h| h.trim().to_owned()).filter(|h| !h.is_empty())
}

/// The keys a form's help travels under, so a rewrite lands where the client reads it.
const HELP_KEYS: [&str; 3] = ["help", "description", "hint"];

/// A form's boxes, each labelled and explained for a person. Names and order are untouched.
pub fn plain_fields(slug: &str, fields: &[Value]) -> Vec<Value> {
    fields.iter().map(|field| plain_field(slug, field)).collect()
}

fn plain_field(slug: &str, field: &Value) -> Value {
    let name = field["name"].as_str().or(field["id"].as_str()).unwrap_or_default();
    let label = field["label"].as_str().unwrap_or_default();
    let help_key = HELP_KEYS.iter().find(|k| field[k].is_string());
    let mut out = field.clone();
    out["label"] = json!(plain_label(slug, name, label));
    if let Some(help) = plain_help(slug, name, help_key.and_then(|k| field[k].as_str())) {
        if let Some(key) = help_key {
            out[*key] = json!(help);
        }
    } else if let Some(key) = help_key {
        out[*key] = Value::Null;
    }
    out
}

/// An app's name as a person reads it.
///
/// The catalogues usually carry the brand's own spelling; a handful arrive all lowercase. A name with
/// a dot is left alone (domain-style brands write it lowercase on purpose), and a name starting with
/// a digit has no first letter to capitalise, which is why 1Password is in the map.
pub fn display_name(slug: &str, published: &str) -> String {
    if slug.eq_ignore_ascii_case("_1password") {
        return "1Password".to_owned();
    }
    if published != published.to_lowercase() || published.contains('.') {
        return published.to_owned();
    }
    let mut chars = published.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The line a card says before anybody adds the app.
pub fn needs_key_line(name: &str) -> String {
    format!("Needs a key from {name}")
}

/// Reads a toolkit's published auth details: `Some(true)` when a person types a key, `Some(false)`
/// when they sign in, `None` when Composio publishes nothing this can tell it from.
pub fn needs_key(detail: &Value) -> Option<bool> {
    if detail["composio_managed_auth_schemes"].as_array().is_some_and(|a| !a.is_empty()) {
        return Some(false);
    }
    let schemes = detail["auth_config_details"].as_array()?;
    if schemes.is_empty() {
        return None;
    }
    Some(schemes.iter().any(|s| {
        let mode = s["mode"].as_str().unwrap_or_default().to_uppercase();
        !mode.starts_with("OAUTH")
    }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn published<'a>(slug: &'a str, docs: Option<&'a str>) -> Published<'a> {
        Published { slug, name: "Acme", kind: Kind::Key, steps: &[], docs, hint: None }
    }

    #[test]
    fn every_mapped_app_carries_steps_and_a_provider_link() {
        assert!(GUIDES.apps.len() >= 30);
        for (slug, g) in &GUIDES.apps {
            assert!(!g.steps.trim().is_empty(), "{slug}");
            assert!(provider_link(g.url.as_deref()).is_some(), "{slug} links {}", g.url.as_deref().unwrap_or("-"));
        }
    }

    #[test]
    fn a_known_app_gets_the_maps_steps_and_page() {
        let g = guide(&published("openai", None));
        assert_eq!(g["url"], "https://platform.openai.com/api-keys");
        assert!(g["steps"][0].as_str().unwrap().contains("API keys page"));
        assert_eq!(g["kind"], "key");
    }

    #[test]
    fn an_unknown_app_says_where_to_look_and_gets_no_invented_link() {
        let g = guide(&published("zzz", None));
        assert!(g["url"].is_null());
        assert!(g["steps"][0].as_str().unwrap().contains("Acme"));
    }

    #[test]
    fn the_brokers_own_site_and_plain_http_are_not_provider_pages() {
        assert!(provider_link(Some("https://composio.dev/x")).is_none());
        assert!(provider_link(Some("https://backend.composio.dev/x")).is_none());
        assert!(provider_link(Some("http://app.example.com")).is_none());
        assert!(provider_link(Some("https://app.example.com/k")).is_some());
    }

    #[test]
    fn a_published_vendor_page_wins_over_the_map() {
        let g = guide(&published("openai", Some("https://acme.example.com/keys")));
        assert_eq!(g["url"], "https://acme.example.com/keys");
    }

    #[test]
    fn generic_labels_become_plain_and_a_specific_rewrite_wins() {
        assert_eq!(plain_label("zzz", "key", "Bearer Token"), "Access token");
        assert_eq!(plain_label("_1password", "generic_api_key", "API Key"), "Access token");
        assert_eq!(plain_label("zzz", "key", "Postmark Server API Token"), "Postmark Server API Token");
    }

    #[test]
    fn a_fields_help_travels_under_the_key_it_came_in() {
        let fields = vec![json!({"name":"generic_api_key","label":"API Key","description":"From the app."})];
        let plain = plain_fields("_1password", &fields);
        assert_eq!(plain[0]["label"], "Access token");
        assert!(plain[0]["description"].as_str().unwrap().contains("Connect server"));
        let empty = plain_fields("zzz", &[json!({"name":"key","label":"API key","hint":"Keep it."})]);
        assert_eq!(empty[0]["hint"], "Keep it.");
    }

    #[test]
    fn display_names_keep_a_brand_spelling() {
        assert_eq!(display_name("_1password", "1password"), "1Password");
        assert_eq!(display_name("mailfloss", "mailfloss"), "Mailfloss");
        assert_eq!(display_name("incident", "incident.io"), "incident.io");
        assert_eq!(display_name("posthog", "PostHog"), "PostHog");
    }

    #[test]
    fn auth_details_say_a_key_or_a_sign_in_and_nothing_when_unpublished() {
        assert_eq!(needs_key(&json!({"composio_managed_auth_schemes": ["OAUTH2"]})), Some(false));
        assert_eq!(needs_key(&json!({"auth_config_details": [{"mode": "API_KEY"}]})), Some(true));
        assert_eq!(needs_key(&json!({"auth_config_details": [{"mode": "OAUTH2"}]})), Some(false));
        assert_eq!(needs_key(&json!({})), None);
        assert_eq!(needs_key_line("Acme"), "Needs a key from Acme");
    }
}
