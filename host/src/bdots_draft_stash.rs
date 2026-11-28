#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `composer-draft-stash.ts` and `draft-title.ts`:
//! timestamped draft merging, stash compaction, legacy adoption and derived titles. Composes
//! helpers already ported into this host. Only `botInsights` calls this.

use serde_json::{Map, Value, json};

/// Authoritative stash key (DRAFT_STASH_STORAGE_KEY).
pub const DRAFT_STASH_STORAGE_KEY: &str = "hermes:composer-drafts:v4";

/// Key of the new-session draft (NEW_SESSION_DRAFT_KEY).
pub const NEW_SESSION_DRAFT_KEY: &str = "__new__";

/// How long a cleared-draft marker lives (CLEARED_MARKER_TTL_MS).
pub const CLEARED_MARKER_TTL_MS: u64 = 10 * 60_000;

/// Most cleared markers kept (MAX_CLEARED_MARKERS).
pub const MAX_CLEARED_MARKERS: usize = 200;

/// Prefix of mirror receipts (MIRROR_RECEIPT_PREFIX).
pub const MIRROR_RECEIPT_PREFIX: &str = "hermes:composer-drafts:v3-mirror:";

/// Most mirror receipts kept (MAX_MIRROR_RECEIPTS).
pub const MAX_MIRROR_RECEIPTS: usize = 32;

/// Whether entry `a` wins over `b` (newerEntry).
pub fn newer_entry(a: Option<&Value>, b: Option<&Value>) -> bool {
    let Some(a) = a else {
        return false;
    };
    let Some(b) = b else {
        return true;
    };
    let at_a = a["at"].as_u64().unwrap_or(0);
    let at_b = b["at"].as_u64().unwrap_or(0);
    if at_a != at_b {
        return at_a > at_b;
    }
    text_of(a) > text_of(b)
}

/// Text of an entry, where null reads as absent (a.text ?? '').
fn text_of(entry: &Value) -> String {
    entry["text"].as_str().unwrap_or("").to_owned()
}

/// Later than every entry and no earlier than the wall clock (nextStashStamp).
pub fn next_stash_stamp(entries: &Value, now: u64) -> u64 {
    let highest = entries
        .as_object()
        .map(|map| map.values().filter_map(|entry| entry["at"].as_u64()).max().unwrap_or(0))
        .unwrap_or(0);
    now.max(highest + 1)
}

/// Convert a number to a base-36 string.
fn to_base36(value: u32) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rest = value;
    let mut out = Vec::new();
    while rest > 0 {
        out.push(digits[(rest % 36) as usize]);
        rest /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// Two-hash fingerprint of a legacy list (hashText).
pub fn hash_text(text: &str) -> String {
    let mut a = 0x811c9dc5_u32;
    let mut b = 0x9e3779b9_u32;
    for byte in text.bytes() {
        a = a.wrapping_add(u32::from(byte)).wrapping_mul(0x01000193);
        b = b.wrapping_add(u32::from(byte)).wrapping_mul(0x5bd1e995);
    }
    format!("{}{}", to_base36(a), to_base36(b))
}

/// Receipt key for a legacy list (mirrorReceiptKey).
pub fn mirror_receipt_key(raw: Option<&str>) -> String {
    format!("{MIRROR_RECEIPT_PREFIX}{}", hash_text(raw.unwrap_or("")))
}

/// The pre-v4 list: non-empty string texts with no timestamps (parseLegacyDrafts).
pub fn parse_legacy_drafts(raw: Option<&str>) -> Option<Value> {
    let parsed: Value = serde_json::from_str(raw?).ok()?;
    let entries = parsed.as_object()?;
    let mut out = Map::new();
    for (key, value) in entries {
        if let Some(text) = value.as_str().filter(|text| !text.is_empty()) {
            out.insert(key.clone(), json!({"at": 0, "text": text}));
        }
    }
    Some(Value::Object(out))
}

/// Keys whose legacy text differs from the stash (legacyChanges).
pub fn legacy_changes(entries: &Value, legacy: &Value) -> Option<Value> {
    let legacy_map = legacy.as_object()?;
    let entries_map = entries.as_object()?;
    let mut keys: Vec<&String> = entries_map.keys().chain(legacy_map.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut changes = Map::new();
    for key in keys {
        let theirs = legacy_map.get(key).and_then(|entry| entry["text"].as_str());
        let ours = entries_map.get(key).and_then(|entry| entry["text"].as_str());
        if theirs != ours {
            changes.insert(key.clone(), theirs.map_or(Value::Null, |text| Value::String(text.to_owned())));
        }
    }
    Some(Value::Object(changes))
}

/// Keep the newest drafts and only recent cleared markers (compactDraftStash).
pub fn compact_draft_stash(entries: &Value, max_drafts: usize, now: u64) -> Value {
    let mut items: Vec<(&String, &Value)> = entries.as_object().map(|map| map.iter().collect()).unwrap_or_default();
    items.sort_by(|(ka, a), (kb, b)| {
        b["at"].as_u64().unwrap_or(0).cmp(&a["at"].as_u64().unwrap_or(0)).then_with(|| ka.cmp(kb))
    });
    let mut kept = Map::new();
    let mut drafts = 0;
    let mut markers = 0;
    for (key, entry) in items {
        if entry["text"].is_null() {
            let age = now.saturating_sub(entry["at"].as_u64().unwrap_or(0));
            if age < CLEARED_MARKER_TTL_MS && markers < MAX_CLEARED_MARKERS {
                kept.insert(key.clone(), entry.clone());
                markers += 1;
            }
        } else if drafts < max_drafts {
            kept.insert(key.clone(), entry.clone());
            drafts += 1;
        }
    }
    Value::Object(kept)
}

/// Merge two stashes, keeping the newer entry for each key.
pub fn merge_stashes(local: &Value, remote: &Value) -> Value {
    let mut out = local.as_object().cloned().unwrap_or_default();
    if let Some(remote) = remote.as_object() {
        for (key, entry) in remote {
            if newer_entry(Some(entry), out.get(key)) {
                out.insert(key.clone(), entry.clone());
            }
        }
    }
    Value::Object(out)
}

/// Stash entries from a stored `{cleared, drafts}` value (readDraftStash).
pub fn stash_from_stored(stored: &Value) -> Value {
    let mut out = Map::new();
    if let Some(cleared) = stored["cleared"].as_object() {
        for (key, at) in cleared {
            out.insert(key.clone(), json!({"at": at.as_u64().unwrap_or(0), "text": Value::Null}));
        }
    }
    if let Some(drafts) = stored["drafts"].as_object() {
        for (key, draft) in drafts {
            if let Some(text) = draft["text"].as_str().filter(|text| !text.is_empty()) {
                out.insert(key.clone(), json!({"at": draft["at"].as_u64().unwrap_or(0), "text": text}));
            }
        }
    }
    Value::Object(out)
}

/// Stored `{cleared, drafts, mirrored}` value from stash entries.
pub fn stored_from_stash(entries: &Value) -> Value {
    let mut cleared = Map::new();
    let mut drafts = Map::new();
    if let Some(map) = entries.as_object() {
        for (key, entry) in map {
            if entry["text"].is_null() {
                cleared.insert(key.clone(), json!(entry["at"].as_u64().unwrap_or(0)));
            } else {
                drafts.insert(key.clone(), json!({"at": entry["at"].as_u64().unwrap_or(0), "text": entry["text"]}));
            }
        }
    }
    json!({"cleared": Value::Object(cleared), "drafts": Value::Object(drafts), "mirrored": true})
}

/// Whether a draft scope is the new-session draft (isFreshDraftScope).
pub fn is_fresh_draft_scope(scope: Option<&str>) -> bool {
    scope == Some(NEW_SESSION_DRAFT_KEY)
}

/// Scope of a draft, defaulting to the new-session key (draftKey).
pub fn draft_key(scope: Option<&str>) -> String {
    scope.map(str::trim).filter(|value| !value.is_empty()).unwrap_or(NEW_SESSION_DRAFT_KEY).to_owned()
}

/// Title of a draft in a titles map (draftTitleIn).
pub fn draft_title_in(titles: &Value, scope: Option<&str>) -> String {
    titles[draft_key(scope)].as_str().unwrap_or("").to_owned()
}

/// Whether a draft contains a terminal chip (draftHasTerminalChips).
pub fn draft_has_terminal_chips(draft: &str) -> bool {
    draft.split_whitespace().any(|word| word.starts_with("terminal:"))
}

/// Name a draft after its first meaningful line (deriveDraftTitle).
pub fn derive_draft_title(text: &str) -> String {
    let line = text.lines().find(|line| !line.trim().is_empty()).map(str::trim).unwrap_or("");
    if line.is_empty() {
        return String::new();
    }
    let body =
        if line.starts_with('/') { line.split_once(char::is_whitespace).map_or("", |(_, rest)| rest) } else { line };
    let collapsed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= 48 {
        return collapsed;
    }
    let cut: String = collapsed.chars().take(48).collect();
    let space = cut.rfind(' ').unwrap_or(0);
    let kept = if space > 24 { &cut[..space] } else { &cut };
    format!("{}…", kept.trim_end_matches(|c: char| matches!(c, ' ' | ',' | '.' | ';' | ':' | '?' | '-')))
}

/// Saved-at timestamp of a draft scope (savedDraftAt).
pub fn saved_draft_at(entries: &Value, scope: Option<&str>) -> Option<u64> {
    entries.get(draft_key(scope)).and_then(|entry| entry["at"].as_u64())
}

/// Whether a preview URL is a blob URL that must be revoked.
pub fn is_blob_url(url: Option<&str>) -> bool {
    url.is_some_and(|value| value.starts_with("blob:"))
}

/// Blob previews in `previous` that `next` no longer retains.
pub fn discarded_blob_urls(previous: &[Value], next: &[Value]) -> Vec<String> {
    let retained: Vec<&str> = next
        .iter()
        .filter_map(|attachment| attachment["previewUrl"].as_str().filter(|url| url.starts_with("blob:")))
        .collect();
    previous
        .iter()
        .filter_map(|attachment| attachment["previewUrl"].as_str())
        .filter(|url| url.starts_with("blob:") && !retained.contains(url))
        .map(str::to_owned)
        .collect()
}

/// One line describing a stash entry (stashLine).
pub fn stash_line(scope: &str, entry: &Value) -> String {
    let title = if entry["text"].is_null() { "cleared" } else { "draft" };
    format!("{scope} · {title} · {}", entry["at"].as_u64().unwrap_or(0))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(entries: usize, drafts: usize, cleared: usize, legacy: bool) -> Vec<(&'static str, String)> {
    vec![
        ("entries", count_label(entries)),
        ("drafts", count_label(drafts)),
        ("cleared", count_label(cleared)),
        ("legacy", if legacy { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_and_hashes() {
        let entries = json!({"a":{"at":100,"text":"x"},"b":{"at":90,"text":null}});
        assert_eq!(next_stash_stamp(&entries, 50), 101);
        assert_eq!(next_stash_stamp(&entries, 200), 200);
        assert!(newer_entry(Some(&entries["a"]), Some(&entries["b"])));
        assert!(!newer_entry(None, Some(&entries["b"])));
        assert!(newer_entry(Some(&entries["b"]), None));
        assert!(!hash_text("a").is_empty());
        assert_eq!(hash_text("a"), hash_text("a"));
        assert!(mirror_receipt_key(Some("a")).starts_with(MIRROR_RECEIPT_PREFIX));
    }

    #[test]
    fn stored_and_legacy() {
        let stored = json!({"cleared":{"gone":100},"drafts":{"a":{"at":200,"text":"hello"}}});
        let stash = stash_from_stored(&stored);
        assert_eq!(stash["a"]["text"], "hello");
        assert!(stash["gone"]["text"].is_null());
        let restored = stored_from_stash(&stash);
        assert_eq!(restored["mirrored"], true);
        let legacy = parse_legacy_drafts(Some("{\"a\":\"old\",\"empty\":\"\"}")).unwrap();
        assert_eq!(legacy["a"]["at"], 0);
        assert!(legacy.get("empty").is_none());
        let changes = legacy_changes(&stash, &legacy).unwrap();
        assert_eq!(changes["a"], "old");
    }

    #[test]
    fn merge_and_compact() {
        let local = json!({"a":{"at":100,"text":"new"},"b":{"at":50,"text":"old"}});
        let remote = json!({"a":{"at":90,"text":"old"},"c":{"at":110,"text":"fresh"}});
        let merged = merge_stashes(&local, &remote);
        assert_eq!(merged["a"]["text"], "new");
        assert_eq!(merged["c"]["text"], "fresh");
        let entries = json!({
            "d1":{"at":100,"text":"a"},"d2":{"at":90,"text":"b"},"d3":{"at":80,"text":"c"},
            "m1":{"at":600_000,"text":null},"old":{"at":1,"text":null},
        });
        let compacted = compact_draft_stash(&entries, 2, 600_100);
        assert_eq!(compacted.as_object().map_or(0, |map| map.len()), 3);
        assert!(compacted.get("d3").is_none());
        assert!(compacted.get("old").is_none());
    }

    #[test]
    fn scopes_and_titles() {
        assert!(is_fresh_draft_scope(Some("__new__")));
        assert_eq!(draft_key(Some("  s1  ")), "s1");
        assert_eq!(draft_key(None), NEW_SESSION_DRAFT_KEY);
        assert_eq!(draft_title_in(&json!({"s1":"Title"}), Some("s1")), "Title");
        assert!(draft_has_terminal_chips("fix terminal:build now"));
        assert!(!draft_has_terminal_chips("plain text"));
        assert_eq!(derive_draft_title("\n  Hello   world \nnext"), "Hello world");
        assert_eq!(derive_draft_title(""), "");
        assert_eq!(derive_draft_title("/skin review this draft"), "review this draft");
        assert!(derive_draft_title(&"word ".repeat(30)).ends_with('…'));
        assert_eq!(saved_draft_at(&json!({"s1":{"at":42,"text":"x"}}), Some("s1")), Some(42));
    }

    #[test]
    fn attachments_and_summary() {
        assert!(is_blob_url(Some("blob:abc")));
        assert!(!is_blob_url(Some("https://x")));
        let previous = vec![json!({"previewUrl":"blob:one"}), json!({"previewUrl":"blob:two"})];
        let next = vec![json!({"previewUrl":"blob:one"})];
        assert_eq!(discarded_blob_urls(&previous, &next), vec!["blob:two"]);
        assert!(stash_line("s1", &json!({"at":1,"text":"x"})).contains("draft"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 1, false).len(), 4);
    }
}
