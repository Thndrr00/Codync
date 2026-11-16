//! Calendar feeds: read-only `.ics` subscriptions (Google, Apple, Outlook, any `CalDAV` server's
//! export) that a bot can read and the agenda shows, without an OAuth app. The host fetches each
//! feed, keeps its parsed events for ten minutes, and never writes to the calendar.

use crate::hub::Hub;
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

pub const METHODS: &[&str] = &["calendarFeeds", "addCalendarFeed", "removeCalendarFeed", "calendarAgenda"];
const KEY: &str = "calendar-feeds.v1";
const CACHE: &str = "calendar-feed-events.v1";
const LIMIT: usize = 20;
const EVENTS_PER_FEED: usize = 500;
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
pub struct Event {
    pub feed: String,
    pub title: String,
    /// Unix milliseconds; all-day events start at local midnight.
    pub start: i64,
    pub end: Option<i64>,
    pub all_day: bool,
    pub location: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Cached {
    at: i64,
    events: Vec<Event>,
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
        "calendarFeeds" => {
            let mut listed = Vec::new();
            for feed in feeds(&hub.store) {
                let cached = cached(&hub.store, &feed.id);
                listed.push(json!({
                    "id": feed.id,
                    "name": feed.name,
                    "url": feed.url,
                    "events": cached.as_ref().map_or(Vec::new(), |c| c.events.iter().map(event_json).collect()),
                    "fetchedAt": cached.map(|c| c.at),
                }));
            }
            Ok(json!({"feeds": listed}))
        }
        "addCalendarFeed" => {
            let url = b["url"].as_str().unwrap_or_default().trim();
            ensure!(
                url.starts_with("https://") || url.starts_with("http://"),
                "a calendar feed is an http(s) address to an .ics file"
            );
            ensure!(url.len() <= 2_000, "that address is too long");
            let name = b["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Calendar");
            let mut feeds = feeds(&hub.store);
            ensure!(feeds.len() < LIMIT, "at most {LIMIT} calendar feeds");
            ensure!(!feeds.iter().any(|f| f.url == url), "that calendar is already added");
            let feed = Feed {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.chars().take(120).collect(),
                url: url.to_owned(),
            };
            feeds.push(feed.clone());
            save(&hub.store, &feeds)?;
            // Read it now so a bad link says so here, not on the next agenda.
            let events = match fetch(&feed.url).await {
                Ok(events) => stamp(events, &feed.name),
                Err(error) => {
                    save(&hub.store, &feeds[..feeds.len() - 1])?;
                    bail!("couldn't read that calendar: {error:#}");
                }
            };
            let events = stamp(events, &feed.name);
            store_cache(&hub.store, &feed.id, &events);
            Ok(json!({"feed": {"id": feed.id, "name": feed.name, "url": feed.url}, "events": events.len()}))
        }
        "removeCalendarFeed" => {
            let id = str_arg(b, "id")?;
            let mut feeds = feeds(&hub.store);
            let before = feeds.len();
            feeds.retain(|f| f.id != id);
            save(&hub.store, &feeds)?;
            let _ = hub.store.kv_set(&cache_key(id), "");
            Ok(json!({"removed": feeds.len() != before}))
        }
        "calendarAgenda" => Ok(agenda(hub, b).await?),
        other => bail!("unknown calendar method {other}"),
    }
}

/// The next `days` of events across every feed, newest fetch first when a feed is stale.
pub async fn agenda(hub: &Hub, b: &Value) -> Result<Value> {
    let days = b["days"].as_i64().unwrap_or(7).clamp(1, 90);
    let from = b["from"].as_i64().unwrap_or_else(crate::store::now_ms);
    let until = from + days * 86_400_000;
    let mut events = Vec::new();
    for feed in feeds(&hub.store) {
        let fresh = cached(&hub.store, &feed.id).filter(|c| crate::store::now_ms() - c.at < FRESH_MS);
        let cached = match fresh {
            Some(cached) => Some(cached),
            None => match fetch(&feed.url).await {
                Ok(events) => {
                    let events = stamp(events, &feed.name);
                    store_cache(&hub.store, &feed.id, &events);
                    cached(&hub.store, &feed.id)
                }
                Err(error) => {
                    tracing::info!(feed = %feed.name, error = format!("{error:#}"), "couldn't refresh a calendar feed");
                    cached(&hub.store, &feed.id)
                }
            },
        };
        if let Some(cached) = cached {
            events.extend(
                cached
                    .events
                    .into_iter()
                    .filter(|event| event.start < until && event.end.unwrap_or(event.start) >= from),
            );
        }
    }
    events.sort_by_key(|event| event.start);
    events.truncate(200);
    Ok(json!({"events": events.iter().map(event_json).collect::<Vec<_>>()}))
}

fn stamp(mut events: Vec<Event>, feed: &str) -> Vec<Event> {
    for event in &mut events {
        feed.clone_into(&mut event.feed);
    }
    events
}

fn event_json(event: &Event) -> Value {
    json!({
        "feed": event.feed,
        "title": event.title,
        "start": event.start,
        "end": event.end,
        "allDay": event.all_day,
        "location": event.location,
        "url": event.url,
    })
}

fn cached(store: &crate::store::Store, id: &str) -> Option<Cached> {
    store.kv_get(&cache_key(id)).and_then(|text| serde_json::from_str(&text).ok())
}

fn store_cache(store: &crate::store::Store, id: &str, events: &[Event]) {
    let cached = Cached { at: crate::store::now_ms(), events: events.to_vec() };
    if let Ok(text) = serde_json::to_string(&cached)
        && let Err(error) = store.kv_set(&cache_key(id), &text)
    {
        tracing::warn!(error = format!("{error:#}"), "couldn't cache a calendar feed");
    }
}

async fn fetch(url: &str) -> Result<Vec<Event>> {
    let response =
        crate::http().get(url).timeout(FETCH_TIMEOUT).send().await.with_context(|| format!("fetching {url}"))?;
    ensure!(response.status().is_success(), "{url} answered {}", response.status());
    let text = response.text().await.context("reading the calendar")?;
    ensure!(text.len() <= 8 * 1024 * 1024, "that calendar is larger than 8 MiB");
    Ok(parse(&text))
}

fn str_arg<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

/// Parses an iCalendar body into events, keeping at most [`EVENTS_PER_FEED`].
pub fn parse(body: &str) -> Vec<Event> {
    let lines = unfold(body);
    let mut events = Vec::new();
    let mut in_event = false;
    let mut fields: Vec<(String, String, String)> = Vec::new();
    for line in lines {
        let upper = line.to_ascii_uppercase();
        if upper == "BEGIN:VEVENT" {
            in_event = true;
            fields.clear();
            continue;
        }
        if upper == "END:VEVENT" {
            in_event = false;
            if let Some(event) = event_of(&fields) {
                events.push(event);
                if events.len() >= EVENTS_PER_FEED {
                    break;
                }
            }
            continue;
        }
        if !in_event {
            continue;
        }
        let Some((name, params, value)) = property(&line) else { continue };
        fields.push((name, params, value));
    }
    events
}

/// iCalendar folds long lines with a leading space or tab.
fn unfold(body: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in body.replace("\r\n", "\n").replace('\r', "\n").split('\n') {
        if raw.starts_with(' ') || raw.starts_with('\t') {
            if let Some(last) = lines.last_mut() {
                last.push_str(raw[1..].trim_end());
            }
        } else if !raw.is_empty() {
            lines.push(raw.to_owned());
        }
    }
    lines
}

/// `NAME;PARAM=value;...:VALUE`, with the name uppercased.
fn property(line: &str) -> Option<(String, String, String)> {
    let colon = line.find(':')?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let (name, params) = head.split_once(';').map_or((head, ""), |(name, params)| (name, params));
    Some((name.to_ascii_uppercase(), params.to_owned(), unescape(value)))
}

fn event_of(fields: &[(String, String, String)]) -> Option<Event> {
    let get = |name: &str| fields.iter().find(|(n, _, _)| n == name).map(|(_, p, v)| (p.as_str(), v.as_str()));
    let (_, start_raw) = get("DTSTART")?;
    let all_day = !start_raw.contains('T');
    let start = parse_time(start_raw)?;
    let end = get("DTEND").and_then(|(_, value)| parse_time(value));
    let title = get("SUMMARY").map_or_else(|| "Untitled".to_owned(), |(_, value)| value.trim().to_owned());
    Some(Event {
        feed: String::new(),
        title: if title.is_empty() { "Untitled".to_owned() } else { title },
        start,
        end,
        all_day,
        location: get("LOCATION").map(|(_, v)| v.trim().to_owned()).filter(|v| !v.is_empty()),
        url: get("URL").map(|(_, v)| v.trim().to_owned()).filter(|v| !v.is_empty()),
    })
}

fn parse_time(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y%m%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .and_then(|dt| Local.from_local_datetime(&dt).single())
            .map(|dt| dt.timestamp_millis());
    }
    let (body, utc) = raw.strip_suffix('Z').map_or((raw, false), |b| (b, true));
    let naive = NaiveDateTime::parse_from_str(body, "%Y%m%dT%H%M%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(body, "%Y%m%dT%H%M").ok())?;
    if utc {
        Some(Utc.from_utc_datetime(&naive).timestamp_millis())
    } else {
        Local.from_local_datetime(&naive).single().map(|dt| dt.timestamp_millis())
    }
}

fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(next) => out.push(next),
            None => out.push('\\'),
        }
    }
    out
}

/// The bot tool: a compact agenda a model can read without parsing JSON itself.
pub fn tools() -> Vec<Value> {
    vec![json!({
        "name": "calendar_agenda",
        "description": "Read the next days of the user's subscribed calendars (.ics feeds added on this computer). Read-only: it cannot change a calendar.",
        "inputSchema": {"type": "object", "properties": {
            "days": {"type": "integer", "description": "How many days ahead, 1 to 90. Default 7."},
        }},
        "annotations": {"readOnlyHint": true, "idempotentHint": true},
    })]
}

pub async fn tool_call(hub: &Hub, name: &str, args: &Value) -> Result<Value> {
    ensure!(name == "calendar_agenda", "unknown calendar tool: {name}");
    let agenda = agenda(hub, args).await?;
    let events = agenda["events"].as_array().cloned().unwrap_or_default();
    if events.is_empty() {
        return Ok(json!({"text": "Nothing is on the subscribed calendars in that window."}));
    }
    let lines: Vec<String> = events
        .iter()
        .map(|event| {
            let at = DateTime::<Utc>::from_timestamp_millis(event["start"].as_i64().unwrap_or_default())
                .map(|dt| dt.with_timezone(&Local).format("%a %d %b %H:%M").to_string())
                .unwrap_or_default();
            let place = event["location"].as_str().map(|p| format!(" ({p})")).unwrap_or_default();
            format!("- {at}: {}{place}", event["title"].as_str().unwrap_or("Untitled"))
        })
        .collect();
    Ok(json!({"text": lines.join("\n")}))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:1\r\nSUMMARY:Team sync\r\nDTSTART:20260105T100000Z\r\nDTEND:20260105T103000Z\r\nLOCATION:Room 2\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:2\r\nSUMMARY:A long title that is folded \r\n across two lines\r\nDTSTART;VALUE=DATE:20260106\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    #[test]
    fn events_keep_their_times_location_and_folded_title() {
        let events = parse(BODY);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Team sync");
        assert_eq!(events[0].location.as_deref(), Some("Room 2"));
        assert!(!events[0].all_day);
        assert_eq!(events[1].title, "A long title that is folded across two lines");
        assert!(events[1].all_day);
        assert!(events[1].end.is_none());
    }

    #[test]
    fn escaped_text_and_unknown_properties_are_safe() {
        let body = "BEGIN:VEVENT\nSUMMARY:Lunch\\, then a walk\\nwith the team\nDTSTART:20260107T120000\nX-CUSTOM:ignore me\nEND:VEVENT\n";
        let events = parse(body);
        assert_eq!(events[0].title, "Lunch, then a walk\nwith the team");
    }

    #[test]
    fn an_event_without_a_start_is_skipped() {
        assert!(parse("BEGIN:VEVENT\nSUMMARY:No date\nEND:VEVENT\n").is_empty());
    }
}
