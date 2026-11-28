#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat 0.7.5 (`client/lib/format.ts`): locale-aware numbers,
//! money, token counts, calendar distances and day labels, without any date crate. Composes the
//! helpers already ported into this host instead of duplicating them. Only `botInsights` calls this.

/// Decimal sign the locale writes (`,` in pt/es/fr/de/it, `.` elsewhere).
pub fn decimal_sign(locale: &str) -> char {
    if matches!(
        locale.split(|c| c == '-' || c == '_').next().unwrap_or("").to_ascii_lowercase().as_str(),
        "pt" | "es" | "fr" | "de" | "it" | "nl" | "ru" | "pl" | "tr" | "id"
    ) {
        ','
    } else {
        '.'
    }
}

/// Whether the locale writes 12-hour clock times (uses12h).
pub fn uses_12h(locale: &str) -> bool {
    locale.split(|c| c == '-' || c == '_').next().unwrap_or("").eq_ignore_ascii_case("en")
}

/// The `hour` option for a clock time: `numeric` in 12-hour locales, `2-digit` in 24-hour ones.
pub fn clock_hour(locale: &str) -> &'static str {
    if uses_12h(locale) { "numeric" } else { "2-digit" }
}

/// Grouping separator that goes with the locale's decimal sign.
fn grouping_sign(locale: &str) -> char {
    if decimal_sign(locale) == ',' { '.' } else { ',' }
}

/// Insert a grouping separator every three digits (Intl.NumberFormat's grouping).
fn group_digits(digits: &str, sep: char) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

/// Dollars as the language writes them, with a floor for tiny amounts (formatUsd).
pub fn format_usd(locale: &str, value: f64) -> String {
    if value > 0.0 && value < 0.00005 {
        return format!(
            "<{}",
            crate::bdots_extra::format_money(crate::bdots_extra::to_minor_units(0.0001, "USD"), "USD")
        );
    }
    crate::bdots_extra::format_money(crate::bdots_extra::to_minor_units(value, "USD"), "USD")
}

/// Budget use as a percentage: `37,5%`, `12%`, `<1%` (formatPercent).
pub fn format_percent(locale: &str, value: f64) -> String {
    if value > 0.0 && value < 1.0 {
        return format!("<1%");
    }
    let sign = decimal_sign(locale);
    let text =
        if value < 10.0 { format!("{}{}1%", value.trunc() as i64, sign) } else { format!("{}%", value.round() as i64) };
    if value < 10.0 {
        // keep the first decimal digit, dropping a trailing zero
        let frac = ((value - value.trunc()) * 10.0).round() as i64;
        if frac == 0 { format!("{}%", value.trunc() as i64) } else { text }
    } else {
        text
    }
}

/// A grouped integer with the locale's separators (formatInt).
pub fn format_int(locale: &str, value: f64) -> String {
    let rounded = value.round().abs() as i64;
    let grouped = group_digits(&rounded.to_string(), grouping_sign(locale));
    if value < 0.0 { format!("-{grouped}") } else { grouped }
}

/// A fraction of a cent, with the locale's decimal sign (formatDecimal).
pub fn format_decimal(locale: &str, value: f64) -> String {
    let sign = decimal_sign(locale);
    let whole = value.trunc().abs() as i64;
    let frac = ((value - value.trunc()).abs() * 10_000.0).round() as i64;
    let mut digits = format!("{frac:04}").trim_end_matches('0').to_owned();
    let prefix = if value < 0.0 { "-" } else { "" };
    if digits.is_empty() {
        format!("{prefix}{whole}")
    } else {
        digits.insert(0, sign);
        format!("{prefix}{whole}{digits}")
    }
}

/// Token counts in short form: `3,1M`, `12K`, `940` (formatTokensShort).
pub fn format_tokens_short(locale: &str, value: f64) -> String {
    if value >= 1_000_000.0 {
        let suffix = if decimal_sign(locale) == ',' { "mi" } else { "M" };
        format!("{}{suffix}", format_decimal(locale, value / 1_000_000.0))
    } else if value >= 10_000.0 {
        format!(
            "{}K",
            format_decimal(locale, value / 1_000.0).trim_end_matches(|c: char| c == '0' || c == decimal_sign(locale))
        )
    } else {
        format_int(locale, value)
    }
}

/// Days from the civil epoch, for calendar arithmetic without a date crate.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year - i32::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 } as i32;
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i64::from(era * 146097 + doe - 719468)
}

/// The civil date of an ISO timestamp, when it is well formed.
fn iso_date(iso: &str) -> Option<(i32, u32, u32)> {
    let d = iso.get(..10)?;
    let y = d.get(..4)?.parse().ok()?;
    let m = d.get(5..7)?.parse().ok()?;
    let day = d.get(8..10)?.parse().ok()?;
    Some((y, m, day))
}

/// Milliseconds since the epoch from an ISO timestamp, assuming UTC.
fn iso_ms(iso: &str) -> Option<i64> {
    let (y, m, d) = iso_date(iso)?;
    let hour: i64 = iso.get(11..13).and_then(|h| h.parse().ok()).unwrap_or(0);
    let minute: i64 = iso.get(14..16).and_then(|v| v.parse().ok()).unwrap_or(0);
    Some(days_from_civil(y, m, d) * 86_400_000 + hour * 3_600_000 + minute * 60_000)
}

/// Inverse of `days_from_civil`, for rendering an epoch day back to a civil date.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((y + i64::from(m <= 2)) as i32, m as u32, d as u32)
}

/// An epoch-millisecond stamp as an ISO timestamp, for rendering host dates without a crate.
pub fn ms_to_iso(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let hour = rem / 3_600_000;
    let minute = (rem % 3_600_000) / 60_000;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:00Z")
}

/// Calendar distance in days: 0 today, 1 yesterday (daysBetween).
pub fn days_between(iso: &str, now: &str) -> i64 {
    let (Some((y, m, d)), Some((ny, nm, nd))) = (iso_date(iso), iso_date(now)) else {
        return 0;
    };
    days_from_civil(ny, nm, nd) - days_from_civil(y, m, d)
}

/// Day of week, 0 Sunday through 6 Saturday.
pub fn day_of_week(iso: &str) -> u32 {
    iso_date(iso).map_or(0, |(y, m, d)| (days_from_civil(y, m, d) + 4).rem_euclid(7) as u32)
}

/// Weekday names, Sunday first.
pub const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// Short month name in a few supported languages.
pub fn month_short(locale: &str, month: u32) -> &'static str {
    let lang = locale.split(|c| c == '-' || c == '_').next().unwrap_or("").to_ascii_lowercase();
    let en = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let pt = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
    let es = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
    let fr = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
    let table: &[&str; 12] = match lang.as_str() {
        "pt" => &pt,
        "es" => &es,
        "fr" => &fr,
        _ => &en,
    };
    table.get(month.saturating_sub(1) as usize).copied().unwrap_or("")
}

/// Long month name in a few supported languages.
pub fn month_long(locale: &str, month: u32) -> &'static str {
    let lang = locale.split(|c| c == '-' || c == '_').next().unwrap_or("").to_ascii_lowercase();
    let en = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let pt = [
        "janeiro",
        "fevereiro",
        "março",
        "abril",
        "maio",
        "junho",
        "julho",
        "agosto",
        "setembro",
        "outubro",
        "novembro",
        "dezembro",
    ];
    let es = [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ];
    let fr = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    let table: &[&str; 12] = match lang.as_str() {
        "pt" => &pt,
        "es" => &es,
        "fr" => &fr,
        _ => &en,
    };
    table.get(month.saturating_sub(1) as usize).copied().unwrap_or("")
}

/// A short date in the locale's own order (formatShortDate).
pub fn format_short_date(locale: &str, iso: &str, year: bool) -> String {
    let Some((y, m, d)) = iso_date(iso) else {
        return String::new();
    };
    let month = month_short(locale, m);
    let yr = if year { format!(" {y}") } else { String::new() };
    if decimal_sign(locale) == ',' { format!("{d} {month}{yr}") } else { format!("{month} {d}{yr}") }
}

/// A long month with its year (formatMonthYear).
pub fn format_month_year(locale: &str, iso: &str) -> String {
    iso_date(iso).map_or_else(String::new, |(y, m, _)| format!("{} {y}", month_long(locale, m)))
}

/// A clock time, 12-hour with AM/PM or zero-padded 24-hour (formatTime).
pub fn format_time(locale: &str, iso: &str) -> String {
    let hour: u32 = iso.get(11..13).and_then(|h| h.parse().ok()).unwrap_or(0);
    let minute: u32 = iso.get(14..16).and_then(|v| v.parse().ok()).unwrap_or(0);
    if uses_12h(locale) {
        let suffix = if hour < 12 { "AM" } else { "PM" };
        let h = if hour % 12 == 0 { 12 } else { hour % 12 };
        format!("{h}:{minute:02} {suffix}")
    } else {
        format!("{hour:02}:{minute:02}")
    }
}

/// A clock time today, or a short date with the time on another day (clockAt).
pub fn clock_at(locale: &str, iso: &str, now: &str) -> String {
    if days_between(iso, now) == 0 {
        format_time(locale, iso)
    } else {
        format!("{} {}", format_short_date(locale, iso, false), format_time(locale, iso))
    }
}

/// A weekday name with its first letter up (formatWeekday), reusing the shared upperFirst.
pub fn format_weekday(locale: &str, iso: &str) -> String {
    let _ = locale;
    crate::bdots_textfmt::upper_first(WEEKDAYS[day_of_week(iso) as usize])
}

/// Yesterday/today/weekday/short date divider (dayLabel).
pub fn day_label(locale: &str, iso: &str, now: &str) -> String {
    let days = days_between(iso, now);
    match days {
        0 => relative_day(locale, 0).to_owned(),
        1 => relative_day(locale, -1).to_owned(),
        2..=6 => format_weekday(locale, iso),
        _ => {
            let year = iso_date(iso).zip(iso_date(now)).is_some_and(|((y, _, _), (ny, _, _))| y != ny);
            format_short_date(locale, iso, year)
        }
    }
}

/// The locale's word for a day offset of -1, 0 or +1 (relativeDay).
pub fn relative_day(locale: &str, offset: i8) -> &'static str {
    let lang = locale.split(|c| c == '-' || c == '_').next().unwrap_or("").to_ascii_lowercase();
    match (lang.as_str(), offset) {
        ("pt", -1) => "ontem",
        ("pt", 0) => "hoje",
        ("pt", 1) => "amanhã",
        ("es", -1) => "ayer",
        ("es", 0) => "hoy",
        ("es", 1) => "mañana",
        ("fr", -1) => "hier",
        ("fr", 0) => "aujourd'hui",
        ("fr", 1) => "demain",
        ("de", -1) => "gestern",
        ("de", 0) => "heute",
        ("de", 1) => "morgen",
        (_, -1) => "yesterday",
        (_, 1) => "tomorrow",
        _ => "today",
    }
}

/// Ordinal plural form of a number ("one" for 1st, "few" for 3rd), with en/fr rules.
pub fn ordinal_form(locale: &str, n: u64) -> &'static str {
    let lang = locale.split(|c| c == '-' || c == '_').next().unwrap_or("").to_ascii_lowercase();
    if lang == "fr" {
        return if n == 1 { "one" } else { "other" };
    }
    if lang != "en" || (n % 100 >= 11 && n % 100 <= 13) {
        return "other";
    }
    match n % 10 {
        1 => "one",
        2 => "two",
        3 => "few",
        _ => "other",
    }
}

/// A relative time label: now, minutes/hours ago, yesterday or a short date (relativeTime).
pub fn relative_time(locale: &str, iso: &str, now: &str) -> String {
    let (Some(at), Some(now_ms)) = (iso_ms(iso), iso_ms(now)) else {
        return String::new();
    };
    let diff = now_ms - at;
    if diff < 60_000 {
        return (if locale.to_ascii_lowercase().starts_with("pt") { "agora" } else { "now" }).to_owned();
    }
    if diff < 3_600_000 {
        let minutes = diff / 60_000;
        return format!("{} min {}", minutes, ago_word(locale));
    }
    let days = days_between(iso, now);
    if days == 0 {
        return format!("{} h {}", diff / 3_600_000, ago_word(locale));
    }
    if days == 1 {
        return relative_day(locale, -1).to_owned();
    }
    format_short_date(locale, iso, false)
}

/// Word for "ago" in the supported languages.
fn ago_word(locale: &str) -> &'static str {
    let lang = locale.split(|c| c == '-' || c == '_').next().unwrap_or("").to_ascii_lowercase();
    match lang.as_str() {
        "pt" => "atrás",
        "es" => "atrás",
        "fr" => "il y a",
        "de" => "vor",
        _ => "ago",
    }
}

/// A human span since a timestamp, reusing the shared span formatter (timeAgo).
pub fn time_ago(iso: &str, now: &str) -> String {
    match (iso_ms(iso), iso_ms(now)) {
        (Some(at), Some(now_ms)) => crate::bdots_text::format_span(now_ms - at),
        _ => String::new(),
    }
}

/// A day label followed by its clock time (whenAt).
pub fn when_at(locale: &str, iso: &str, now: &str) -> String {
    format!("{} · {}", day_label(locale, iso, now), format_time(locale, iso))
}

/// Keep a fact on one line, reusing the shared non-breaking-space helper (keepWhole).
pub fn keep_fact(fact: &str) -> String {
    crate::bdots_textfmt::keep_whole(fact)
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(locale: &str, iso: &str, now: &str) -> Vec<(&'static str, String)> {
    vec![
        ("day", day_label(locale, iso, now)),
        ("time", format_time(locale, iso)),
        ("weekday", format_weekday(locale, iso)),
        ("relative", relative_time(locale, iso, now)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_rules() {
        assert_eq!(decimal_sign("pt-BR"), ',');
        assert_eq!(decimal_sign("en-US"), '.');
        assert!(uses_12h("en-US"));
        assert!(!uses_12h("pt-BR"));
        assert_eq!(clock_hour("pt-BR"), "2-digit");
        assert_eq!(format_int("en-US", 1234.0), "1,234");
        assert_eq!(format_int("pt-BR", 1234.0), "1.234");
        assert_eq!(format_decimal("pt-BR", 0.042), "0,042");
    }

    #[test]
    fn money_and_tokens() {
        assert!(format_usd("en-US", 0.42).contains('$'));
        assert!(format_usd("en-US", 0.00001).starts_with('<'));
        assert_eq!(format_percent("pt-BR", 0.5), "<1%");
        assert_eq!(format_percent("en-US", 12.0), "12%");
        assert!(format_tokens_short("en-US", 3_100_000.0).contains('M'));
        assert!(format_tokens_short("pt-BR", 3_100_000.0).contains("mi"));
        assert!(format_tokens_short("en-US", 12_000.0).contains('K'));
        assert_eq!(format_tokens_short("en-US", 940.0), "940");
    }

    #[test]
    fn calendar_math() {
        assert_eq!(days_between("2026-11-25T10:00:00Z", "2026-11-25T23:00:00Z"), 0);
        assert_eq!(days_between("2026-11-24T10:00:00Z", "2026-11-25T10:00:00Z"), 1);
        assert_eq!(day_of_week("2026-11-25T10:00:00Z"), 3);
        assert_eq!(ms_to_iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(days_between(&ms_to_iso(0), &ms_to_iso(86_400_000)), 1);
        assert_eq!(month_short("pt-BR", 10), "out");
        assert_eq!(month_long("en-US", 10), "October");
        assert!(format_short_date("pt-BR", "2026-10-12T10:00:00Z", true).starts_with("12 out"));
        assert!(format_short_date("en-US", "2026-10-12T10:00:00Z", false).starts_with("Oct"));
        assert!(format_month_year("pt-BR", "2026-03-01T10:00:00Z").starts_with("março"));
    }

    #[test]
    fn clocks_and_labels() {
        assert_eq!(format_time("en-US", "2026-11-25T20:05:00Z"), "8:05 PM");
        assert_eq!(format_time("pt-BR", "2026-11-25T20:05:00Z"), "20:05");
        assert_eq!(clock_at("pt-BR", "2026-11-25T20:05:00Z", "2026-11-25T08:00:00Z"), "20:05");
        assert!(clock_at("pt-BR", "2026-11-24T20:05:00Z", "2026-11-25T08:00:00Z").contains("24"));
        assert_eq!(format_weekday("en-US", "2026-11-25T10:00:00Z"), "Wednesday");
        assert_eq!(relative_day("pt-BR", -1), "ontem");
        assert_eq!(relative_day("en-US", 0), "today");
        assert_eq!(ordinal_form("en-US", 1), "one");
        assert_eq!(ordinal_form("en-US", 3), "few");
        assert_eq!(ordinal_form("en-US", 11), "other");
        assert_eq!(ordinal_form("fr-FR", 2), "other");
    }

    #[test]
    fn relatives_and_summary() {
        assert_eq!(relative_time("en-US", "2026-11-25T10:00:00Z", "2026-11-25T10:00:10Z"), "now");
        assert!(relative_time("en-US", "2026-11-25T09:58:00Z", "2026-11-25T10:00:00Z").contains("min"));
        assert_eq!(relative_time("pt-BR", "2026-11-24T10:00:00Z", "2026-11-25T10:00:00Z"), "ontem");
        assert!(!time_ago("2026-11-25T09:00:00Z", "2026-11-25T10:00:00Z").is_empty());
        assert!(when_at("pt-BR", "2026-11-25T20:05:00Z", "2026-11-25T08:00:00Z").contains("20:05"));
        assert_eq!(keep_fact("a b"), "a\u{00A0}b");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary("pt-BR", "2026-11-25T10:00:00Z", "2026-11-25T10:00:00Z").len(), 4);
    }
}
