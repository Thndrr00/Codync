#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for the time zones a bot's routines use. Reuses `bdots_cron`, `bdots_text` and
//! `bdots_activity` instead of duplicating their formatting. Only `botInsights` calls this.

/// One time zone reduced to what the card shows.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneLine {
    pub name: String,
    pub label: String,
    pub icon: &'static str,
    pub tone: &'static str,
    pub count: usize,
    pub local: bool,
}

/// SF Symbol for a zone relative to the computer.
pub fn zone_icon(local: bool) -> &'static str {
    if local { "globe.badge.chevron.backward" } else { "globe" }
}

/// Semantic tone for a zone relative to the computer.
pub fn zone_tone(local: bool) -> &'static str {
    if local { "good" } else { "accent" }
}

/// Human label for a zone id (last segment, spaces for underscores).
pub fn zone_label(zone: &str) -> String {
    if zone.trim().is_empty() {
        return "Computer time".to_owned();
    }
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

/// Compact display for a zone id, reusing the shared shortener.
pub fn short_zone(zone: &str) -> String {
    crate::bdots_more::shorten(zone_label(zone), 48)
}

/// Whether a zone matches the computer's own zone.
pub fn is_local(zone: &str, local_zone: &str) -> bool {
    zone.trim().is_empty() || zone.eq_ignore_ascii_case(local_zone)
}

/// Count how many routines use each zone, in first-seen order.
pub fn count_zones(zones: &[String]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for zone in zones {
        let key = if zone.trim().is_empty() { "local".to_owned() } else { zone.to_owned() };
        if let Some(entry) = out.iter_mut().find(|(z, _)| z.eq_ignore_ascii_case(&key)) {
            entry.1 += 1;
        } else {
            out.push((key, 1));
        }
    }
    out
}

/// Number of distinct zones.
pub fn distinct_count(zones: &[String]) -> usize {
    count_zones(zones).len()
}

/// Whether the bot uses more than one zone.
pub fn is_multi_zone(zones: &[String]) -> bool {
    distinct_count(zones) > 1
}

/// The most used zone, or "local".
pub fn primary_zone(zones: &[String]) -> String {
    count_zones(zones).into_iter().max_by_key(|(_, n)| *n).map_or_else(|| "local".to_owned(), |(z, _)| z)
}

/// Number of routines using a given zone.
pub fn zone_count(zones: &[String], zone: &str) -> usize {
    count_zones(zones).into_iter().find(|(z, _)| z.eq_ignore_ascii_case(zone)).map_or(0, |(_, n)| n)
}

/// Percent of routines in a zone.
pub fn zone_percent(zones: &[String], zone: &str) -> u32 {
    crate::bdots_activity::percent(zone_count(zones, zone), zones.len())
}

/// Bar of filled cells for a zone's share.
pub fn zone_bar(zones: &[String], zone: &str, width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(zone_count(zones, zone), zones.len(), width))
}

/// Build a display line for one zone.
pub fn zone_line(zone: &str, count: usize, local_zone: &str) -> ZoneLine {
    let local = is_local(zone, local_zone);
    ZoneLine {
        name: zone.to_owned(),
        label: zone_label(zone),
        icon: zone_icon(local),
        tone: zone_tone(local),
        count,
        local,
    }
}

/// All zones as display lines, sorted by count then name.
pub fn sorted_lines(zones: &[String], local_zone: &str) -> Vec<ZoneLine> {
    let mut lines: Vec<ZoneLine> = count_zones(zones).into_iter().map(|(z, n)| zone_line(&z, n, local_zone)).collect();
    lines.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    lines
}

/// Whether any routine uses a non-local zone.
pub fn has_remote(zones: &[String], local_zone: &str) -> bool {
    zones.iter().any(|z| !is_local(z, local_zone))
}

/// Human label for a remote zone, or the computer's own.
pub fn local_label(zones: &[String], local_zone: &str) -> String {
    if has_remote(zones, local_zone) { format!("{} + remote", zone_label(local_zone)) } else { zone_label(local_zone) }
}

/// One plain sentence describing the zone mix.
pub fn zone_headline(zones: &[String], local_zone: &str) -> String {
    if zones.is_empty() {
        return "No scheduled times yet.".to_owned();
    }
    let primary = primary_zone(zones);
    format!(
        "{} · {} zone{}",
        zone_label(&primary),
        distinct_count(zones),
        if distinct_count(zones) == 1 { "" } else { "s" }
    )
}

/// Semantic tone for the zone card.
pub fn zone_tone_headline(zones: &[String]) -> &'static str {
    if zones.is_empty() { "secondary" } else { "good" }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(zones: &[String]) -> String {
    crate::bdots_desk::compact_count(zones.len() as u64)
}

/// Remote zones only, for a "different from this computer" strip.
pub fn remote_zones(zones: &[String], local_zone: &str) -> Vec<ZoneLine> {
    sorted_lines(zones, local_zone).into_iter().filter(|l| !l.local).collect()
}

/// Summary rows for the card.
pub fn summary(zones: &[String], local_zone: &str) -> Vec<(&'static str, String)> {
    vec![
        ("schedules", count_label(zones)),
        ("zones", distinct_count(zones).to_string()),
        ("primary", zone_label(&primary_zone(zones))),
        ("headline", zone_headline(zones, local_zone)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zones() -> Vec<String> {
        vec!["America/Mexico_City".into(), "America/Mexico_City".into(), "Europe/Madrid".into(), String::new()]
    }

    #[test]
    fn labels_and_icons() {
        assert_eq!(zone_label("America/Mexico_City"), "Mexico City");
        assert_eq!(zone_label(""), "Computer time");
        assert_eq!(short_zone("Europe/Madrid"), "Madrid");
        assert_eq!(zone_icon(true), "globe.badge.chevron.backward");
        assert_eq!(zone_tone(false), "accent");
    }

    #[test]
    fn local_detection() {
        assert!(is_local("", "America/Mexico_City"));
        assert!(is_local("america/mexico_city", "America/Mexico_City"));
        assert!(!is_local("Europe/Madrid", "America/Mexico_City"));
        assert!(has_remote(&zones(), "America/Mexico_City"));
        assert_eq!(local_label(&zones(), "America/Mexico_City"), "Mexico City + remote");
        assert_eq!(local_label(&[], "America/Mexico_City"), "Mexico City");
    }

    #[test]
    fn counts_and_shares() {
        let z = zones();
        assert_eq!(distinct_count(&z), 3);
        assert!(is_multi_zone(&z));
        assert!(!is_multi_zone(&[]));
        assert_eq!(primary_zone(&z), "America/Mexico_City");
        assert_eq!(zone_count(&z, "Europe/Madrid"), 1);
        assert_eq!(zone_percent(&z, "Europe/Madrid"), 25);
        assert!(!zone_bar(&z, "Europe/Madrid", 8).is_empty());
    }

    #[test]
    fn lines_and_strips() {
        let z = zones();
        let lines = sorted_lines(&z, "America/Mexico_City");
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().any(|l| l.local));
        assert_eq!(remote_zones(&z, "America/Mexico_City").len(), 1);
        assert_eq!(count_label(&z), "4");
    }

    #[test]
    fn headlines_and_summary() {
        assert!(zone_headline(&[], "x").contains("No scheduled"));
        assert!(zone_headline(&zones(), "America/Mexico_City").contains("zone"));
        assert_eq!(zone_tone_headline(&[]), "secondary");
        assert_eq!(zone_tone_headline(&zones()), "good");
        assert_eq!(summary(&zones(), "America/Mexico_City").len(), 4);
    }
}
