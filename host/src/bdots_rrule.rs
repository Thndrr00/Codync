#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `rrule.ts`: RRULE-like recurrence validation,
//! cron conversion and descriptions. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::Value;

/// Supported frequencies (FREQS).
pub const FREQS: [&str; 5] = ["hourly", "daily", "weekly", "monthly", "yearly"];

/// Weekday codes in UTC order (WEEKDAYS).
pub const WEEKDAYS: [&str; 7] = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

/// Workday codes (WORKDAYS).
pub const WORKDAYS: [&str; 5] = ["MO", "TU", "WE", "TH", "FR"];

/// English weekday names, UTC order.
pub const EN_DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// Maximum interval allowed in a rule.
pub const MAX_INTERVAL: u32 = 1000;

/// Whether a value is a supported frequency.
pub fn is_freq(value: &str) -> bool {
    FREQS.contains(&value.to_ascii_lowercase().as_str())
}

/// Whether a weekday code is valid (`MO` or `2TU`, `-1FR`).
pub fn valid_weekday(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    let (pos, code) = match upper.len() {
        2 => (None, upper.as_str()),
        3 | 4 => (Some(&upper[..upper.len() - 2]), &upper[upper.len() - 2..]),
        _ => return false,
    };
    if !WEEKDAYS.contains(&code) {
        return false;
    }
    pos.is_none_or(|digits| digits.parse::<i32>().is_ok_and(|n| (-5..=5).contains(&n) && n != 0))
}

/// Whether a wall-clock time string is `HH:MM`.
pub fn valid_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 5
        && bytes[2] == b':'
        && value[..2].parse::<u8>().is_ok_and(|h| h <= 23)
        && value[3..].parse::<u8>().is_ok_and(|m| m <= 59)
        && value[..2].bytes().all(|b| b.is_ascii_digit())
        && value[3..].bytes().all(|b| b.is_ascii_digit())
}

/// Whether a date string is `YYYY-MM-DD`.
pub fn valid_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|part| part.bytes().all(|b| b.is_ascii_digit()))
}

/// Normalized interval of a rule, defaulting to 1.
pub fn interval_of(rule: &Value) -> u32 {
    rule["interval"].as_u64().map_or(1, |value| value as u32).clamp(1, MAX_INTERVAL)
}

/// Times of a rule, defaulting to `09:00`.
pub fn times_of(rule: &Value) -> Vec<String> {
    if let Some(times) = rule["times"].as_array() {
        let mut out: Vec<String> = times.iter().filter_map(Value::as_str).map(str::to_owned).collect();
        out.sort();
        out.dedup();
        return out;
    }
    rule["time"].as_str().map_or_else(|| vec!["09:00".to_owned()], |time| vec![time.to_owned()])
}

/// Set positions of a rule, normalized to a vector.
pub fn set_positions(rule: &Value) -> Vec<i32> {
    match &rule["bySetPos"] {
        Value::Number(number) => number.as_i64().map(|n| vec![n as i32]).unwrap_or_default(),
        Value::Array(values) => values.iter().filter_map(Value::as_i64).map(|n| n as i32).collect(),
        _ => Vec::new(),
    }
}

/// Whether a rule uses a working-day set.
pub fn is_workdays(rule: &Value) -> bool {
    let days: Vec<String> = rule["byWeekday"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    days.len() == WORKDAYS.len() && WORKDAYS.iter().all(|day| days.iter().any(|value| value == day))
}

/// Validation error for a rule, empty when valid.
pub fn validation_error(rule: &Value) -> String {
    let freq = rule["freq"].as_str().unwrap_or("");
    if !is_freq(freq) {
        return "schedule.unknown_freq".to_owned();
    }
    if !valid_time("00:00") || times_of(rule).iter().any(|time| !valid_time(time)) {
        return "schedule.invalid_time".to_owned();
    }
    let days: Vec<String> = rule["byWeekday"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    if days.iter().any(|day| !valid_weekday(day)) {
        return "schedule.invalid_weekday".to_owned();
    }
    if days.iter().any(|day| day.len() > 2) && !matches!(freq, "monthly" | "yearly") {
        return "schedule.position_needs_month".to_owned();
    }
    String::new()
}

/// Whether a rule can be expressed as a cron expression.
pub fn is_cron_compatible(rule: &Value) -> bool {
    if rule["until"].is_string() || rule["count"].is_number() || !set_positions(rule).is_empty() {
        return false;
    }
    let days: Vec<String> = rule["byWeekday"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    if days.iter().any(|day| day.len() != 2) {
        return false;
    }
    let month_days: Vec<i32> = rule["byMonthDay"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_i64).map(|n| n as i32).collect())
        .unwrap_or_default();
    !month_days.iter().any(|day| *day < 0)
}

/// Cron day-of-week list for a rule's weekdays.
pub fn cron_dows(rule: &Value) -> String {
    let days: Vec<String> = rule["byWeekday"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    if days.is_empty() {
        return "*".to_owned();
    }
    days.iter()
        .filter_map(|day| WEEKDAYS.iter().position(|value| value == day))
        .map(|index| index.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Convert a simple rule to cron, when possible (ruleToCron).
pub fn rule_to_cron(rule: &Value) -> Option<String> {
    if !is_cron_compatible(rule) {
        return None;
    }
    let times = times_of(rule);
    let interval = interval_of(rule);
    let freq = rule["freq"].as_str()?.to_ascii_lowercase();
    let dows = cron_dows(rule);
    if freq == "hourly" {
        if 24 % interval != 0 {
            return None;
        }
        let minute = times.first().and_then(|time| time.get(3..5))?;
        let hour = if interval == 1 { "*".to_owned() } else { format!("*/{interval}") };
        return Some(format!("{minute} {hour} * * {dows}"));
    }
    if interval != 1 || times.is_empty() {
        return None;
    }
    let minutes: Vec<&str> = times.iter().filter_map(|time| time.get(3..5)).collect();
    if minutes.iter().any(|minute| *minute != minutes[0]) {
        return None;
    }
    let hours: Vec<&str> = times.iter().filter_map(|time| time.get(..2)).collect();
    let minute = minutes[0];
    let hours = hours.join(",");
    let days = if rule["byMonthDay"].is_array() {
        rule["byMonthDay"]
            .as_array()
            .map(|values| values.iter().filter_map(Value::as_i64).map(|n| n.to_string()).collect::<Vec<_>>().join(","))
            .unwrap_or_default()
    } else {
        "*".to_owned()
    };
    match freq.as_str() {
        "daily" => Some(format!("{minute} {hours} {days} * {dows}")),
        "weekly" => Some(format!("{minute} {hours} * * {dows}")),
        _ => None,
    }
}

/// English names for a rule's weekdays, reusing the i18n locale helper.
pub fn weekday_names(rule: &Value) -> Vec<String> {
    rule["byWeekday"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|day| WEEKDAYS.iter().position(|value| value == day))
                .map(|index| EN_DAYS[index].to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Human description of a rule (describeRule).
pub fn describe_rule(rule: &Value) -> String {
    let freq = rule["freq"].as_str().unwrap_or("daily");
    let interval = interval_of(rule);
    let unit = if interval == 1 { freq.to_owned() } else { format!("{freq}s") };
    let every = if interval == 1 { format!("Every {unit}") } else { format!("Every {interval} {unit}") };
    let names = weekday_names(rule);
    if names.is_empty() {
        format!("{every} at {}", times_of(rule).first().map_or("09:00", String::as_str))
    } else {
        format!("{every} on {}", names.join(", "))
    }
}

/// Human name of a timezone, reusing the locale helper (timezoneName).
pub fn timezone_name(zone: &str) -> String {
    if zone == "UTC" || zone == "Etc/UTC" { "UTC".to_owned() } else { crate::bdots_i18n::locale_label(zone) }
}

/// Human label of a frequency.
pub fn freq_label(freq: &str) -> String {
    crate::bdots_textfmt::upper_first(freq)
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(freq: &str, interval: u32, times: usize, cron: bool) -> Vec<(&'static str, String)> {
    vec![
        ("freq", freq_label(freq)),
        ("interval", interval.to_string()),
        ("times", count_label(times)),
        ("cron", if cron { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn daily() -> Value {
        json!({"freq":"daily","times":["09:00"],"timezone":"UTC"})
    }

    #[test]
    fn validation() {
        assert!(is_freq("weekly"));
        assert!(!is_freq("yearlyx"));
        assert!(valid_weekday("MO"));
        assert!(valid_weekday("2TU"));
        assert!(valid_weekday("-1FR"));
        assert!(!valid_weekday("XX"));
        assert!(valid_time("09:00"));
        assert!(!valid_time("24:00"));
        assert!(valid_date("2026-11-26"));
        assert!(!valid_date("26-11-2026"));
        assert!(validation_error(&daily()).is_empty());
        assert!(!validation_error(&json!({"freq":"bad"})).is_empty());
    }

    #[test]
    fn defaults() {
        assert_eq!(interval_of(&daily()), 1);
        assert_eq!(interval_of(&json!({"freq":"daily","interval":2000})), MAX_INTERVAL);
        assert_eq!(times_of(&daily()), vec!["09:00"]);
        assert_eq!(times_of(&json!({"freq":"daily","time":"08:30"})), vec!["08:30"]);
        assert_eq!(set_positions(&json!({"bySetPos":-1})), vec![-1]);
        assert!(is_workdays(&json!({"byWeekday":WORKDAYS})));
    }

    #[test]
    fn cron_conversion() {
        assert!(is_cron_compatible(&daily()));
        assert!(!is_cron_compatible(&json!({"freq":"daily","count":3})));
        assert_eq!(cron_dows(&daily()), "*");
        assert_eq!(rule_to_cron(&daily()).as_deref(), Some("00 09 * * *"));
        let hourly = json!({"freq":"hourly","time":"00:30","timezone":"UTC"});
        assert_eq!(rule_to_cron(&hourly).as_deref(), Some("30 * * * *"));
    }

    #[test]
    fn descriptions() {
        assert_eq!(weekday_names(&json!({"byWeekday":["MO","FR"]})), vec!["Monday", "Friday"]);
        assert_eq!(describe_rule(&daily()), "Every daily at 09:00");
        assert!(describe_rule(&json!({"freq":"weekly","byWeekday":["MO"],"timezone":"UTC"})).contains("Monday"));
        assert_eq!(timezone_name("UTC"), "UTC");
        assert_eq!(freq_label("daily"), "Daily");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("daily", 1, 1, true).len(), 4);
    }
}
