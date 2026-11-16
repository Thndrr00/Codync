//! RSS and Atom subscriptions: a bot can read the news feeds the person follows, and the agenda
//! keeps the newest items. The host fetches each feed, caches it for ten minutes and never writes
//! to the site.

use crate::hub::Hub;
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

pub const METHODS: &[&str] = &["feedSubscriptions", "addFeed", "removeFeed", "feedItems"];
const KEY: &str = "feed-subscriptions.v1";
const CACHE: &str = "feed-items.v1";
const LIMIT: usize = 30;
const ITEMS_PER_FEED: usize = 100;
const FRESH_MS: i64 = 10 * 60 * 1000;
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Feed {
    id: String,
    name: String,
    url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub feed: String,
    pub title: String,
    pub url: Option<String>,
    pub published: Option<i64>,
    pub summary: String,
}

#[derive(Serialize, Deserialize)]
struct Cached {
    at: i64,
    items: Vec<Item>,
}

fn feeds(store: &crate::store::Store) -> Vec<Feed> {
    store.kv_read(KEY).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn save(store: &crate::store::Store, feeds: &[Feed]) -> Result<()> {
    store.kv_set(KEY, &serde_json::to_string(feeds)?)
}

fn cache_key(id: &str) -> String {
    format!("{CACHE}.{id}")
}

pub async fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    match method {
        "feedSubscriptions" => {
            let feeds: Vec<Value> = feeds(&hub.store)
                .into_iter()
                .map(|feed| {
                    let cached = cached(&hub.store, &feed.id);
                    json!({
                        "id": feed.id,
                        "name": feed.name,
                        "url": feed.url,
                        "items": cached.as_ref().map_or(0, |c| c.items.len()),
                        "fetchedAt": cached.map(|c| c.at),
                    })
                })
                .collect();
            Ok(json!({"feeds": feeds}))
        }
        "addFeed" => {
            let url = b["url"].as_str().unwrap_or_default().trim();
            ensure!(url.starts_with("https://") || url.starts_with("http://"), "a feed is an http(s) address");
            ensure!(url.len() <= 2_000, "that address is too long");
            let name = b["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Feed");
            let mut feeds = feeds(&hub.store);
            ensure!(feeds.len() < LIMIT, "at most {LIMIT} feeds");
            ensure!(!feeds.iter().any(|f| f.url == url), "that feed is already added");
            let feed = Feed {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.chars().take(120).collect(),
                url: url.to_owned(),
            };
            feeds.push(feed.clone());
            save(&hub.store, &feeds)?;
            let items = match fetch(&feed.url).await {
                Ok(items) => stamp(items, &feed.name),
                Err(error) => {
                    save(&hub.store, &feeds[..feeds.len() - 1])?;
                    bail!("couldn't read that feed: {error:#}");
                }
            };
            store_cache(&hub.store, &feed.id, &items);
            Ok(json!({"feed": {"id": feed.id, "name": feed.name, "url": feed.url}, "items": items.len()}))
        }
        "removeFeed" => {
            let id = str_arg(b, "id")?;
            let mut feeds = feeds(&hub.store);
            let before = feeds.len();
            feeds.retain(|f| f.id != id);
            save(&hub.store, &feeds)?;
            let _ = hub.store.kv_set(&cache_key(id), "");
            Ok(json!({"removed": feeds.len() != before}))
        }
        "feedItems" => Ok(json!({"items": items(hub, b).await?})),
        other => bail!("unknown feed method {other}"),
    }
}

/// The newest items across every feed, or one feed when `feed` names it.
pub async fn items(hub: &Hub, b: &Value) -> Result<Vec<Value>> {
    let limit = usize::try_from(b["limit"].as_u64().unwrap_or(30).clamp(1, 200)).unwrap_or(30);
    let only = b["feed"].as_str();
    let mut items = Vec::new();
    for feed in feeds(&hub.store) {
        if only.is_some_and(|wanted| wanted != feed.id && wanted != feed.name) {
            continue;
        }
        let cached = match cached(&hub.store, &feed.id).filter(|c| crate::store::now_ms() - c.at < FRESH_MS) {
            Some(cached) => Some(cached),
            None => match fetch(&feed.url).await {
                Ok(items) => {
                    let items = stamp(items, &feed.name);
                    store_cache(&hub.store, &feed.id, &items);
                    cached(&hub.store, &feed.id)
                }
                Err(error) => {
                    tracing::info!(feed = %feed.name, error = format!("{error:#}"), "couldn't refresh a feed");
                    cached(&hub.store, &feed.id)
                }
            },
        };
        if let Some(cached) = cached {
            items.extend(cached.items);
        }
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.published.unwrap_or_default()));
    items.truncate(limit);
    Ok(items.iter().map(item_json).collect())
}

fn item_json(item: &Item) -> Value {
    json!({
        "feed": item.feed,
        "title": item.title,
        "url": item.url,
        "published": item.published,
        "summary": item.summary,
    })
}

fn stamp(mut items: Vec<Item>, feed: &str) -> Vec<Item> {
    for item in &mut items {
        feed.clone_into(&mut item.feed);
    }
    items
}

fn cached(store: &crate::store::Store, id: &str) -> Option<Cached> {
    store.kv_get(&cache_key(id)).and_then(|text| serde_json::from_str(&text).ok())
}

fn store_cache(store: &crate::store::Store, id: &str, items: &[Item]) {
    let cached = Cached { at: crate::store::now_ms(), items: items.to_vec() };
    if let Ok(text) = serde_json::to_string(&cached)
        && let Err(error) = store.kv_set(&cache_key(id), &text)
    {
        tracing::warn!(error = format!("{error:#}"), "couldn't cache a feed");
    }
}

async fn fetch(url: &str) -> Result<Vec<Item>> {
    let response =
        crate::http().get(url).timeout(FETCH_TIMEOUT).send().await.with_context(|| format!("fetching {url}"))?;
    ensure!(response.status().is_success(), "{url} answered {}", response.status());
    let text = response.text().await.context("reading the feed")?;
    ensure!(text.len() <= 4 * 1024 * 1024, "that feed is larger than 4 MiB");
    Ok(parse(&text))
}

fn str_arg<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

/// Parses RSS 2.0 (`item`) or Atom (`entry`) into at most [`ITEMS_PER_FEED`] items.
pub fn parse(body: &str) -> Vec<Item> {
    let (open, close) =
        if body.to_ascii_lowercase().contains("<entry") { ("<entry", "</entry>") } else { ("<item", "</item>") };
    let atom = open == "<entry";
    let mut items = Vec::new();
    for block in blocks(body, open, close) {
        let title = tag(&block, "title").map(|t| crate::chat::clip::plain_text(&t)).unwrap_or_default();
        if title.trim().is_empty() {
            continue;
        }
        let url = if atom { atom_link(&block) } else { tag(&block, "link") };
        let published = tag(&block, if atom { "published" } else { "pubDate" })
            .or_else(|| tag(&block, "updated"))
            .and_then(|t| time_of(t.trim()));
        let summary = tag(&block, "description")
            .or_else(|| tag(&block, "summary"))
            .or_else(|| tag(&block, "content"))
            .map(|t| crate::chat::clip::plain_text(&t))
            .unwrap_or_default();
        items.push(Item {
            feed: String::new(),
            title: title.trim().chars().take(300).collect(),
            url: url.map(|u| u.trim().to_owned()).filter(|u| !u.is_empty()),
            published,
            summary: summary.chars().take(1_200).collect(),
        });
        if items.len() >= ITEMS_PER_FEED {
            break;
        }
    }
    items
}

fn blocks<'a>(body: &'a str, open: &str, close: &str) -> Vec<String> {
    let lower = body.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(start) = lower[at..].find(open) {
        let start = at + start;
        let Some(end) = lower[start..].find(close) else { break };
        out.push(body[start..start + end + close.len()].to_owned());
        at = start + end + close.len();
    }
    out
}

/// The inner text of the first `<name ...>…</name>`.
fn tag(block: &str, name: &str) -> Option<String> {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let lower = block.to_ascii_lowercase();
    let start = lower.find(&open)?;
    let start = lower[start..].find('>').map(|at| start + at + 1)?;
    let end = lower[start..].find(&close).map(|at| start + at)?;
    Some(block[start..end].to_owned())
}

fn atom_link(block: &str) -> Option<String> {
    let lower = block.to_ascii_lowercase();
    let start = lower.find("<link")?;
    let tag = &block[start..start + lower[start..].find('>')? + 1];
    let href = tag.split_once("href=")?.1.trim_start_matches(['"', '\'']);
    let end = href.find(['"', '\'', ' ', '>'])?;
    Some(href[..end].to_owned())
}

fn time_of(text: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(text)
        .or_else(|_| DateTime::parse_from_rfc2822(text))
        .ok()
        .map(|at| at.with_timezone(&Utc).timestamp_millis())
        .or_else(|| {
            // A date without a time ("Mon, 05 Jan 2026") is noon UTC, so the day is still right.
            DateTime::parse_from_str(&format!("{text} 12:00:00 +0000"), "%a, %d %b %Y %H:%M:%S %z")
                .ok()
                .map(|at| at.timestamp_millis())
        })
}

/// The bot tool: the newest headlines, one per line.
pub fn tools() -> Value {
    json!([{
        "name": "news_feed",
        "description": "Read the newest items from the RSS/Atom feeds the user follows on this computer. Read-only; use the item URLs when the user wants the full text (clip_page reads it).",
        "inputSchema": {"type": "object", "properties": {
            "limit": {"type": "integer", "description": "How many items, 1 to 200. Default 20."},
            "feed": {"type": "string", "description": "Optional feed id or name to read only that one."},
        }},
        "annotations": {"readOnlyHint": true, "idempotentHint": true},
    }])
}

pub async fn tool_call(hub: &Hub, name: &str, args: &Value) -> Result<Value> {
    ensure!(name == "news_feed", "unknown feed tool: {name}");
    let items = items(hub, args).await?;
    if items.is_empty() {
        return Ok(json!({"text": "No feeds are added, or none has items yet."}));
    }
    let lines: Vec<String> = items
        .iter()
        .map(|item| {
            let when = item["published"]
                .as_i64()
                .and_then(DateTime::<Utc>::from_timestamp_millis)
                .map(|at| at.format("%d %b").to_string())
                .unwrap_or_default();
            let url = item["url"].as_str().map(|u| format!(" — {u}")).unwrap_or_default();
            format!(
                "- {when} [{}] {}{url}",
                item["feed"].as_str().unwrap_or("Feed"),
                item["title"].as_str().unwrap_or("Untitled")
            )
        })
        .collect();
    Ok(json!({"text": lines.join("\n")}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_items_keep_title_link_date_and_text_summary() {
        let xml = r"<rss><channel><item><title>Hello &amp; welcome</title><link>https://example.com/a</link>
            <pubDate>Mon, 05 Jan 2026 10:00:00 +0000</pubDate><description>&lt;p&gt;First &lt;b&gt;note&lt;/b&gt;&lt;/p&gt;</description></item>
            <item><title>Second</title><link>https://example.com/b</link></item></channel></rss>";
        let items = parse(xml);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Hello & welcome");
        assert_eq!(items[0].url.as_deref(), Some("https://example.com/a"));
        assert!(items[0].summary.contains("First note"));
        assert!(items[0].published.is_some());
    }

    #[test]
    fn atom_entries_use_the_link_href_and_iso_dates() {
        let xml = r#"<feed><entry><title>Atom one</title><link href="https://example.com/atom"/>
            <published>2026-01-05T10:00:00Z</published><summary>Short text</summary></entry></feed>"#;
        let items = parse(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url.as_deref(), Some("https://example.com/atom"));
        assert_eq!(items[0].summary, "Short text");
        assert!(items[0].published.is_some());
    }

    #[test]
    fn an_item_without_a_title_is_skipped() {
        assert!(parse("<rss><item><link>x</link></item></rss>").is_empty());
    }
}
