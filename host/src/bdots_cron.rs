#![allow(dead_code)]

//! Minimal 5-field UTC cron, ported from bdots' `lib/cron.ts`. Enough to describe a routine's
//! schedule and find its next runs; the host's scheduler remains the source of truth for firing.

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Field {
    any: bool,
    values: BTreeSet<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cron {
    minute: Field,
    hour: Field,
    dom: Field,
    month: Field,
    dow: Field,
    raw: [String; 5],
}

fn parse_field(spec: &str, min: u32, max: u32) -> Option<Field> {
    let mut values = BTreeSet::new();
    for part in spec.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => (r, s.parse::<u32>().ok()?),
            None => (part, 1),
        };
        if step < 1 {
            return None;
        }
        let (from, to) = if range == "*" {
            (min, max)
        } else {
            let mut nums = range.split('-').map(|n| n.parse::<u32>().ok());
            let a = nums.next()??;
            let b = match nums.next() {
                Some(v) => v?,
                None if part.contains('/') => max,
                None => a,
            };
            (a, b)
        };
        if from > to {
            return None;
        }
        let mut v = from;
        while v <= to {
            values.insert(v);
            v += step;
        }
    }
    Some(Field { any: spec == "*", values })
}

/// Parse a five-field UTC cron, or `None` when it is malformed (`7` in dow becomes `0`).
pub fn parse_cron(expr: &str) -> Option<Cron> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }
    let dow = parts[4].replace('7', "0");
    Some(Cron {
        minute: parse_field(parts[0], 0, 59)?,
        hour: parse_field(parts[1], 0, 23)?,
        dom: parse_field(parts[2], 1, 31)?,
        month: parse_field(parts[3], 1, 12)?,
        dow: parse_field(&dow, 0, 6)?,
        raw: [parts[0].into(), parts[1].into(), parts[2].into(), parts[3].into(), parts[4].into()],
    })
}

fn matches_at(cron: &Cron, minute: u32, hour: u32, dom: u32, month: u32, dow: u32) -> bool {
    if !cron.minute.values.contains(&minute) || !cron.hour.values.contains(&hour) || !cron.month.values.contains(&month)
    {
        return false;
    }
    let dom_ok = cron.dom.values.contains(&dom);
    let weekday_ok = cron.dow.values.contains(&dow);
    if !cron.dom.any && !cron.dow.any { dom_ok || weekday_ok } else { dom_ok && weekday_ok }
}

/// UTC fields for an epoch-millisecond timestamp (civil-from-days, no timezone dependency).
fn utc_fields(ms: i64) -> (u32, u32, u32, u32, u32) {
    let days = ms.div_euclid(86_400_000);
    let secs = ms.rem_euclid(86_400_000) / 1000;
    let hour = u32::try_from(secs / 3600).unwrap_or(0);
    let minute = u32::try_from(secs % 3600 / 60).unwrap_or(0);
    // 1970-01-01 was a Thursday (dow 4).
    let dow = u32::try_from((days + 4).rem_euclid(7)).unwrap_or(0);
    let (year, month, dom) = civil_from_days(days);
    let _ = year;
    (minute, hour, dom, month, dow)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), u32::try_from(m).unwrap_or(0), u32::try_from(d).unwrap_or(0))
}

/// Run times in `[from, from + window)` at minute resolution, at most `limit` (`runsBetween`).
pub fn runs_between(cron: &Cron, from: i64, window: i64, limit: usize) -> Vec<i64> {
    let mut out = Vec::new();
    let mut t = from.saturating_add(59_999) / 60_000 * 60_000;
    while t < from + window && out.len() < limit {
        let (m, h, dom, mon, dow) = utc_fields(t);
        if matches_at(cron, m, h, dom, mon, dow) {
            out.push(t);
        }
        t += 60_000;
    }
    out
}

/// First matching run within a year, or `None` (`nextRun`).
pub fn next_run(cron: &Cron, from: i64) -> Option<i64> {
    let day = 86_400_000;
    for i in 0..366 {
        if let Some(first) = runs_between(cron, from + i * day, day, 1).into_iter().next() {
            return Some(first);
        }
    }
    None
}

const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// "Every 10 minutes", "Weekdays at 08:00 UTC", or `None` when unusual (`describeCron`).
pub fn describe_cron(cron: &Cron) -> Option<String> {
    let [m, h, dom, mon, dow] = &cron.raw;
    if dom != "*" || mon != "*" {
        return None;
    }
    if dow == "*" {
        if m == "*" && h == "*" {
            return Some("Every minute".into());
        }
        if let Some(step) = m.strip_prefix("*/")
            && h == "*"
            && step.chars().all(|c| c.is_ascii_digit())
        {
            return Some(format!("Every {step} minutes"));
        }
        if cron.minute.values.len() == 1 {
            let minute = *cron.minute.values.iter().next()?;
            if h == "*" {
                return Some(if minute == 0 { "Every hour".into() } else { format!("Every hour at :{minute:02}") });
            }
            if let Some(step) = h.strip_prefix("*/")
                && step.chars().all(|c| c.is_ascii_digit())
            {
                return Some(format!("Every {step} hours"));
            }
        }
    }
    if cron.minute.values.len() != 1 || cron.hour.any || cron.hour.values.len() > 3 {
        return None;
    }
    let minute = *cron.minute.values.iter().next()?;
    let times = cron.hour.values.iter().map(|hour| format!("{hour:02}:{minute:02}")).collect::<Vec<_>>().join(", ");
    let days: Vec<u32> = cron.dow.values.iter().copied().collect();
    let when = if cron.dow.any {
        "Every day".to_owned()
    } else if days == [1, 2, 3, 4, 5] {
        "Weekdays".to_owned()
    } else if days == [0, 6] {
        "Weekends".to_owned()
    } else if days.len() == 1 {
        format!("Every {}", DAYS[days[0] as usize])
    } else {
        days.iter().map(|d| &DAYS[*d as usize][..3]).collect::<Vec<_>>().join(", ")
    };
    Some(format!("{when} at {times} UTC"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ten_minutes_and_weekdays() {
        let c = parse_cron("*/10 * * * *").unwrap();
        assert_eq!(describe_cron(&c).as_deref(), Some("Every 10 minutes"));
        let c = parse_cron("0 8 * * 1-5").unwrap();
        assert_eq!(describe_cron(&c).as_deref(), Some("Weekdays at 08:00 UTC"));
        assert!(parse_cron("nope").is_none());
    }

    #[test]
    fn next_run_is_a_matching_minute() {
        // 2024-01-01 00:00:00 UTC.
        let from = 1_704_067_200_000;
        let c = parse_cron("30 9 * * *").unwrap();
        let next = next_run(&c, from).unwrap();
        let (m, h, _, _, _) = utc_fields(next);
        assert_eq!((m, h), (30, 9));
        assert!(next > from);
    }

    #[test]
    fn day_of_week_seven_is_sunday() {
        let c = parse_cron("0 0 * * 7").unwrap();
        let runs = runs_between(&c, 1_704_067_200_000, 8 * 86_400_000, 10);
        assert!(!runs.is_empty());
        assert_eq!(describe_cron(&c).as_deref(), Some("Every Sunday at 00:00 UTC"));
    }
}
