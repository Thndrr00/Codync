//! The Schedule view's read and create methods. The calendar expands the same triggers the
//! scheduler runs; it never parses a second cron language or writes routine state of its own.
use super::{Routines, Trigger, local_zone, public};
use crate::{hub::Hub, store::now_ms};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;
use croner::Cron;
use serde_json::{Value, json};
use std::str::FromStr;

/// The most occurrences one trigger may contribute to a single range.
const MAX_OCCURRENCES: usize = 500;
/// The largest window the view asks for (two months), so a long interval cannot flood it.
const MAX_WINDOW_MS: i64 = 62 * 86_400_000;

/// The concrete UTC-millisecond occurrences of `trigger` in `[from, to]`.
///
/// `zone` is the calendar's IANA zone (used when a trigger has none of its own). Cron and once
/// triggers are absolute instants, so drawing them in another zone keeps the same set. Intervals
/// have no stored phase: their first tick is `from`, then one `seconds` step at a time. Event
/// triggers have no clock and contribute nothing.
pub fn occurrences_in_range(trigger: &Trigger, from: i64, to: i64, zone: &str) -> Vec<i64> {
    if to < from {
        return Vec::new();
    }
    match trigger {
        Trigger::Once { at } => (*at >= from && *at <= to).then_some(*at).into_iter().collect(),
        Trigger::Interval { seconds } => {
            let Some(step) = seconds.checked_mul(1000).filter(|step| *step > 0) else {
                return Vec::new();
            };
            let mut at = from;
            let mut out = Vec::new();
            while at <= to && out.len() < MAX_OCCURRENCES {
                out.push(at);
                match at.checked_add(step) {
                    Some(next) => at = next,
                    None => break,
                }
            }
            out
        }
        Trigger::Cron { expression, time_zone } => {
            let Ok(zone) = time_zone.parse::<Tz>().or_else(|_| zone.parse::<Tz>()) else {
                return Vec::new();
            };
            let Ok(cron) = Cron::from_str(expression) else {
                return Vec::new();
            };
            // `find_next_occurrence` is exclusive; starting one millisecond earlier keeps an
            // occurrence exactly at `from` (the window is inclusive at both ends).
            let Some(start) = from
                .checked_sub(1)
                .and_then(|at| DateTime::<Utc>::from_timestamp_millis(at))
                .map(|date| date.with_timezone(&zone))
            else {
                return Vec::new();
            };
            let mut cursor = start;
            let mut out = Vec::new();
            while out.len() < MAX_OCCURRENCES {
                let Ok(next) = cron.find_next_occurrence(&cursor, false) else {
                    break;
                };
                let at = next.timestamp_millis();
                if at > to {
                    break;
                }
                out.push(at);
                cursor = next;
            }
            out
        }
        Trigger::Webhook | Trigger::Event { .. } | Trigger::Watch { .. } => Vec::new(),
    }
}

/// Converts a wall-clock date and time in `zone` to an absolute instant. A local time skipped or
/// repeated by a DST change is refused rather than guessed, so the routine runs when the person
/// asked it to (or tells them to pick another time).
fn local_millis(date: &str, hour: u32, minute: u32, zone: &str) -> Result<i64> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").context("date must be YYYY-MM-DD")?;
    let time = NaiveTime::from_hms_opt(hour, minute, 0).context("invalid hour or minute")?;
    let zone: Tz = zone.parse().context("invalid IANA time zone")?;
    date.and_time(time)
        .and_local_timezone(zone)
        .single()
        .context("that local time is skipped or repeated by a daylight saving change")
        .map(|at| at.timestamp_millis())
}

impl Routines {
    /// Every scheduled routine's occurrences in `[from, to]`, for the Schedule calendar. Each
    /// item is one concrete moment of one routine; `trigger` names the clock that produced it and
    /// `kind` separates routine occurrences from other calendar items.
    pub fn schedule_calendar(&self, from: i64, to: i64) -> Result<Value> {
        if to.saturating_sub(from) > MAX_WINDOW_MS {
            bail!("the schedule window must be at most 62 days");
        }
        let state = self.state.locked();
        let mut items: Vec<Value> = Vec::new();
        for routine in state.routines.iter().filter(|r| !r.deleted && r.enabled) {
            for trigger in &routine.triggers {
                let (kind, zone) = match trigger {
                    Trigger::Cron { time_zone, .. } => ("cron", time_zone.to_string()),
                    Trigger::Once { .. } => ("once", local_zone()),
                    Trigger::Interval { .. } => ("interval", local_zone()),
                    Trigger::Webhook | Trigger::Event { .. } | Trigger::Watch { .. } => continue,
                };
                for at in occurrences_in_range(trigger, from, to, &zone) {
                    items.push(json!({
                        "id": routine.id,
                        "name": routine.name,
                        "at": at,
                        "timezone": zone,
                        "trigger": kind,
                        "botId": routine.bot_id,
                        "kind": "routine",
                        "description": trigger.description(),
                        "instruction": routine.instruction,
                    }));
                }
            }
        }
        items.sort_by_key(|item| item["at"].as_i64().unwrap_or(0));
        Ok(json!({"items": items}))
    }

    /// Creates a one-off routine at a local date/time in `timeZone`. The host converts the
    /// wall-clock time and hands a normal `once` schedule draft to `Routines::save`, so validation,
    /// persistence and the chat notice are exactly the ones a form or an agent would get.
    pub fn schedule_create_at(&self, hub: &Hub, bot: &str, body: &Value) -> Result<Value> {
        let date = body["date"].as_str().context("date is required")?;
        let time_zone = body["timeZone"].as_str().context("timeZone is required")?;
        let hour = body["hour"].as_u64().context("hour is required")?;
        let minute = body["minute"].as_u64().context("minute is required")?;
        let (hour, minute) =
            (u32::try_from(hour).context("invalid hour")?, u32::try_from(minute).context("invalid minute")?);
        let at = local_millis(date, hour, minute, time_zone)?;
        if at <= now_ms() {
            bail!("choose a time in the future");
        }
        let mut draft = body.clone();
        draft["schedule"] = json!({"kind": "once", "at": at, "zone": time_zone});
        let saved = self.save(hub, bot, &draft)?;
        Ok(json!({"routine": saved["routine"].clone(), "at": at}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rfc3339: &str) -> i64 {
        DateTime::parse_from_rfc3339(rfc3339).unwrap().timestamp_millis()
    }

    #[test]
    fn cron_expands_each_occurrence_and_includes_the_window_start() {
        // 09:00 UTC on three days; the first is exactly `from`, the last exactly `to`.
        let trigger = Trigger::Cron { expression: "0 9 * * *".into(), time_zone: "UTC".into() };
        let from = at("2026-09-27T09:00:00Z");
        let to = at("2026-09-29T09:00:00Z");
        assert_eq!(occurrences_in_range(&trigger, from, to, "UTC"), vec![from, at("2026-09-28T09:00:00Z"), to]);
    }

    #[test]
    fn cron_uses_the_trigger_zone_and_stays_inside_a_dst_free_window() {
        // Taipei has no DST: each 09:00 local is 01:00 UTC, and only two days are in the window.
        let trigger = Trigger::Cron { expression: "0 9 * * 0".into(), time_zone: "Asia/Taipei".into() };
        let from = at("2026-09-27T00:00:00Z");
        let to = at("2026-10-10T00:00:00Z");
        assert_eq!(
            occurrences_in_range(&trigger, from, to, "UTC"),
            vec![at("2026-09-27T01:00:00Z"), at("2026-10-04T01:00:00Z")]
        );
    }

    #[test]
    fn cron_stops_at_the_bound_limit() {
        let trigger = Trigger::Cron { expression: "* * * * *".into(), time_zone: "UTC".into() };
        let from = at("2026-09-27T00:00:00Z");
        let to = from + 86_400_000;
        let expanded = occurrences_in_range(&trigger, from, to, "UTC");
        assert_eq!(expanded.len(), MAX_OCCURRENCES);
        assert_eq!(expanded[0], from, "the window start is an occurrence and is included");
        assert!(expanded.iter().all(|at| *at >= from && *at <= to));
    }

    #[test]
    fn once_appears_only_when_its_timestamp_is_inside_the_window() {
        let inside = at("2026-09-28T12:00:00Z");
        let trigger = Trigger::Once { at: inside };
        let from = at("2026-09-27T00:00:00Z");
        let to = at("2026-09-29T00:00:00Z");
        assert_eq!(occurrences_in_range(&trigger, from, to, "UTC"), vec![inside]);
        assert!(occurrences_in_range(&trigger, inside + 1, to, "UTC").is_empty());
        assert!(occurrences_in_range(&trigger, from, inside - 1, "UTC").is_empty());
    }

    #[test]
    fn interval_steps_from_the_window_start_without_replaying_past_ticks() {
        let trigger = Trigger::Interval { seconds: 3600 };
        let from = at("2026-09-27T08:00:00Z");
        let to = at("2026-09-27T11:00:00Z");
        assert_eq!(occurrences_in_range(&trigger, from, to, "UTC"), vec![from, from + 3_600_000, from + 7_200_000, to]);
    }

    #[test]
    fn event_triggers_have_no_calendar_occurrences() {
        let trigger = Trigger::Event { source: "github".into(), event: "*".into(), filters: Default::default() };
        let from = at("2026-09-27T00:00:00Z");
        assert!(occurrences_in_range(&trigger, from, from + 86_400_000, "UTC").is_empty());
    }

    #[test]
    fn local_wall_clock_uses_the_zone_without_a_dst_jump() {
        // 09:00 in Taipei is 01:00 UTC in September; 09:00 in New York is 13:00 UTC (EDT).
        let taipei = local_millis("2026-09-27", 9, 0, "Asia/Taipei").unwrap();
        let new_york = local_millis("2026-09-27", 9, 0, "America/New_York").unwrap();
        assert_eq!(taipei, at("2026-09-27T01:00:00Z"));
        assert_eq!(new_york, at("2026-09-27T13:00:00Z"));
        // 02:30 does not exist on the spring-forward day in New York.
        assert!(local_millis("2026-03-08", 2, 30, "America/New_York").is_err());
    }
}
