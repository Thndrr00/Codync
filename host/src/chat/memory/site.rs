//! Site-scoped memories (OpenBot browser.md): a `MEMORY.md` line beginning `@site <host> ` is a hint
//! for that host and its subdomains. A browser snapshot adds up to five such lines for the page's
//! host and parent domains, once per host per turn. No second store: they live in the memory files
//! the bot already keeps, and the rule lives here.

use crate::hub::Hub;
use codync_mem::files as mf;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

/// The marker a fact needs to become a site memory: `@site example.com The checkout is slow`.
pub(crate) const SITE_PREFIX: &str = "@site ";
/// How many site memories one snapshot may add.
pub(crate) const MAX_SITE_MEMORIES: usize = 5;
/// How much of one memory's text the snapshot carries.
const MAX_TEXT: usize = 300;
/// An address longer than this is not a page worth a fact (the `navigate` tool's address cap).
const MAX_URL: usize = 2048;

/// Whether a memory stored for `site` applies to `host`. Both are lower-case hosts without a scheme,
/// port or path. A stored parent domain also matches its subdomains, but never a look-alike host.
pub(crate) fn site_memory_match(site: &str, host: &str) -> bool {
    let site = site.trim().trim_start_matches('.').to_ascii_lowercase();
    let host = host.trim().trim_start_matches('.').to_ascii_lowercase();
    let invalid = |text: &str| text.is_empty() || text.chars().any(|c| matches!(c, '/' | ':' | '?' | '#' | ' '));
    if invalid(&site) || invalid(&host) {
        return false;
    }
    site == host || host.strip_suffix(&site).is_some_and(|before| before.ends_with('.'))
}

/// The lower-case host of a page address, without credentials, port, path or fragment.
pub(crate) fn site_of(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else { return String::new() };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default().split('@').last().unwrap_or_default();
    host.split(':').next().unwrap_or_default().trim().to_ascii_lowercase()
}

/// The one fact a finished browser step may add: `@site <lower-case host> <note>`. The note is
/// whitespace-collapsed and cut at `MAX_TEXT`; an empty or oversized address, a non-http page or an
/// empty note gives nothing. Only the host and the step's own short description are kept — never a
/// query, a path, a field value or a secret.
pub(crate) fn site_fact(url: &str, text: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_URL {
        return None;
    }
    let (scheme, _) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let host = site_of(url);
    if !site_memory_match(&host, &host) {
        return None;
    }
    let note: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if note.is_empty() {
        return None;
    }
    let note: String = note.chars().take(MAX_TEXT).collect();
    Some(format!("{SITE_PREFIX}{host} {note}"))
}

/// One site memory, as it is written in a memory file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SiteMemory {
    pub(crate) site: String,
    pub(crate) text: String,
}

/// Parses a fact line into a site memory. Anything that is not a complete `@site host text` line is
/// left as an ordinary memory and never injected into a page.
pub(crate) fn parse_site_memory(fact: &str) -> Option<SiteMemory> {
    let rest = fact.strip_prefix(SITE_PREFIX)?.trim();
    let (site, text) = rest.split_once(char::is_whitespace)?;
    let text = text.trim();
    // A bare word is an ordinary fact, not a host: require a domain (or the local host).
    if text.is_empty()
        || !(site.contains('.') || site.eq_ignore_ascii_case("localhost"))
        || !site_memory_match(site, site)
    {
        return None;
    }
    Some(SiteMemory { site: site.to_ascii_lowercase(), text: text.chars().take(MAX_TEXT).collect() })
}

// In memory, like the turn ledger: a host is injected at most once in a turn, so later snapshots in
// that turn do not repeat the same hint. Pruned when it grows past a few hundred entries.
static INJECTED: LazyLock<Mutex<HashSet<(i64, String)>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Claims this host for the turn: true when this is its first claim, false for a repeat.
fn claim(turn: i64, host: &str) -> bool {
    let Ok(mut injected) = INJECTED.lock() else { return false };
    if injected.len() > 500 {
        injected.clear();
    }
    injected.insert((turn, host.to_owned()))
}

/// Up to five site memories for this page, as snapshot data. The read and the once-per-turn claim
/// happen only for a real host; a `about:blank` page or a memory-off bot gets nothing.
pub(crate) fn for_snapshot(hub: &Hub, bot: &str, url: &str) -> Vec<serde_json::Value> {
    let host = site_of(url);
    if !site_memory_match(&host, &host) {
        return Vec::new();
    }
    let Some(cfg) = hub.store.bot(bot).ok().flatten().filter(|row| !row.deleted).map(|row| row.config) else {
        return Vec::new();
    };
    if !super::switch::enabled(&cfg) {
        return Vec::new();
    }
    let turn = super::turns::current_turn(bot).unwrap_or_default();
    if !claim(turn, &host) {
        return Vec::new();
    }
    let found = hub.store.memory_read(|conn| {
        let mut found = Vec::new();
        for layer in super::files::layers(&cfg) {
            for fact in mf::facts_of(conn, layer.as_deref())? {
                if let Some(memory) = parse_site_memory(&fact.text)
                    && site_memory_match(&memory.site, &host)
                {
                    found.push(memory);
                }
            }
        }
        Ok(found)
    });
    let Ok(mut found) = found else { return Vec::new() };
    found.sort_by(|a, b| b.site.len().cmp(&a.site.len()).then_with(|| a.text.cmp(&b.text)));
    let mut seen = HashSet::new();
    found
        .into_iter()
        .filter(|memory| seen.insert(memory.text.clone()))
        .take(MAX_SITE_MEMORIES)
        .map(|memory| serde_json::json!({"site": memory.site, "text": memory.text}))
        .collect()
}

/// One line of snapshot text for a site memory, with the site as data the agent can check.
pub(crate) fn line(memory: &serde_json::Value) -> String {
    let site = memory["site"].as_str().unwrap_or_default();
    let text = memory["text"].as_str().unwrap_or_default();
    format!("site: {site} — {text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_memory_matches_its_host_and_subdomains_but_not_a_lookalike() {
        assert!(site_memory_match("example.com", "example.com"));
        assert!(site_memory_match("example.com", "shop.example.com"));
        assert!(site_memory_match("Example.COM", "SHOP.Example.com"), "hosts are case-insensitive");
        assert!(site_memory_match("shop.example.com", "shop.example.com"));
        assert!(!site_memory_match("example.com", "notexample.com"), "a dot boundary is required");
        assert!(!site_memory_match("example.com", "example.com.attacker.net"));
        assert!(!site_memory_match("shop.example.com", "example.com"), "a subdomain hint is narrower");
        assert!(!site_memory_match("", "example.com"));
        assert!(!site_memory_match("example.com", ""));
        for bad in ["https://example.com", "example.com:443", "example.com/path", "exa mple.com"] {
            assert!(!site_memory_match(bad, "example.com"), "{bad} is not a bare host");
            assert!(!site_memory_match("example.com", bad), "{bad} is not a bare host");
        }
    }

    #[test]
    fn a_page_address_gives_a_bare_lower_case_host() {
        assert_eq!(site_of("https://Shop.Example.com:8443/a?b=1#c"), "shop.example.com");
        assert_eq!(site_of("http://user:pass@example.com/"), "example.com");
        assert_eq!(site_of("about:blank"), "");
        assert_eq!(site_of(""), "");
    }

    #[test]
    fn a_host_is_claimed_once_per_turn() {
        let turn = -9_001;
        let host = "once-per-turn.example";
        assert!(claim(turn, host));
        assert!(!claim(turn, host), "a later snapshot in the same turn does not repeat it");
        assert!(claim(turn + 1, host), "the next turn sees it again");
        assert!(claim(turn, "another.example"), "a different host is its own claim");
    }

    #[test]
    fn a_browser_step_gives_one_bounded_site_fact() {
        let fact =
            site_fact("https://Shop.Example.com:8443/secret/reset?token=abc#x", "  Prefers\tconcise\n answers  ")
                .expect("a site fact");
        assert_eq!(fact, "@site shop.example.com Prefers concise answers");
        assert_eq!(parse_site_memory(&fact).map(|memory| memory.text), Some("Prefers concise answers".into()));

        let long = site_fact("https://example.com", &"x ".repeat(MAX_TEXT + 20)).expect("a bounded fact");
        let memory = parse_site_memory(&long).expect("the fact is parseable");
        assert_eq!(memory.text.chars().count(), MAX_TEXT);
    }

    #[test]
    fn an_empty_oversized_or_non_http_page_gives_no_site_fact() {
        assert_eq!(site_fact("", "A hint"), None);
        assert_eq!(site_fact("   ", "A hint"), None);
        assert_eq!(site_fact("https://example.com", "  \n "), None);
        assert_eq!(site_fact(&format!("https://example.com/{}", "x".repeat(MAX_URL)), "A hint"), None);
        for bad in ["about:blank", "ftp://example.com/a", "file:///C:/a", "example.com/a", "https:///a"] {
            assert_eq!(site_fact(bad, "A hint"), None, "{bad} is not an http page");
        }
    }

    #[test]
    fn only_a_complete_site_fact_is_a_site_memory() {
        let memory = parse_site_memory("@site example.com Prefers concise answers").expect("a site memory");
        assert_eq!(memory.site, "example.com");
        assert_eq!(memory.text, "Prefers concise answers");
        assert_eq!(parse_site_memory("Prefers concise answers"), None);
        assert_eq!(parse_site_memory("@site example.com"), None, "text is required");
        assert_eq!(parse_site_memory("@site  Prefers concise answers"), None, "a host is required");
        let long = format!("@site example.com {}", "x".repeat(MAX_TEXT + 20));
        assert_eq!(parse_site_memory(&long).unwrap().text.chars().count(), MAX_TEXT);
        assert!(
            line(&serde_json::json!({"site": "example.com", "text": "A hint"})).starts_with("site: example.com — ")
        );
    }
}
