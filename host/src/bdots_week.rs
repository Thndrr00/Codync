#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers for a bot's weekly summary: turn daily activity counts into trends, peaks,
//! streaks and a plain-language headline. Composes `bdots_activity`, `bdots_text` and
//! `bdots_profile` instead of reimplementing them. Only the `botInsights` endpoint calls this,
//! so every helper is live; clients just render the result.

/// One day's activity, oldest first.
#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    pub key: String,
    pub label: String,
    pub count: usize,
}

/// A trend over the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

/// SF Symbol for a trend, shared by the four clients.
pub fn trend_icon(trend: Trend) -> &'static str {
    match trend {
        Trend::Up => "arrow.up.right",
        Trend::Down => "arrow.down.right",
        Trend::Flat => "equal",
    }
}

/// Semantic tone for a trend.
pub fn trend_tone(trend: Trend) -> &'static str {
    match trend {
        Trend::Up => "good",
        Trend::Down => "attention",
        Trend::Flat => "secondary",
    }
}

/// Short label for a trend.
pub fn trend_label(trend: Trend) -> &'static str {
    match trend {
        Trend::Up => "More active",
        Trend::Down => "Less active",
        Trend::Flat => "About the same",
    }
}

/// Compare the first and second half of the week; a small difference is flat.
pub fn trend(days: &[Day]) -> Trend {
    if days.len() < 2 {
        return Trend::Flat;
    }
    let mid = days.len() / 2;
    let first: usize = days[..mid].iter().map(|d| d.count).sum();
    let second: usize = days[mid..].iter().map(|d| d.count).sum();
    if second > first + 1 {
        Trend::Up
    } else if first > second + 1 {
        Trend::Down
    } else {
        Trend::Flat
    }
}

/// Total activity across the week.
pub fn total(days: &[Day]) -> usize {
    days.iter().map(|d| d.count).sum()
}

/// Average per active day; zero for an empty week.
pub fn daily_average(days: &[Day]) -> u32 {
    if days.is_empty() {
        return 0;
    }
    (total(days) as f64 / days.len() as f64).round() as u32
}

/// The busiest day, or `None` for an empty week.
pub fn peak_day(days: &[Day]) -> Option<&Day> {
    days.iter().max_by_key(|d| d.count).filter(|d| d.count > 0)
}

/// Days with no activity.
pub fn quiet_days(days: &[Day]) -> usize {
    days.iter().filter(|d| d.count == 0).count()
}

/// Longest run of consecutive active days.
pub fn active_streak(days: &[Day]) -> usize {
    let mut best = 0;
    let mut current = 0;
    for day in days {
        if day.count > 0 {
            current += 1;
            best = best.max(current);
        } else {
            current = 0;
        }
    }
    best
}

/// Longest run of consecutive quiet days.
pub fn quiet_streak(days: &[Day]) -> usize {
    let mut best = 0;
    let mut current = 0;
    for day in days {
        if day.count == 0 {
            current += 1;
            best = best.max(current);
        } else {
            current = 0;
        }
    }
    best
}

/// How many days are in the window.
pub fn span_days(days: &[Day]) -> usize {
    days.len()
}

/// Percent change between halves, clamped to ±100.
pub fn change_percent(days: &[Day]) -> i32 {
    if days.len() < 2 {
        return 0;
    }
    let mid = days.len() / 2;
    let first: usize = days[..mid].iter().map(|d| d.count).sum();
    let second: usize = days[mid..].iter().map(|d| d.count).sum();
    if first == 0 {
        return i32::from(second > 0) * 100;
    }
    let delta = (second as i64 - first as i64) * 100 / first as i64;
    delta.clamp(-100, 100) as i32
}

/// A bar of filled cells for one day's count against the week's peak.
pub fn day_bar(day: &Day, peak: usize, width: usize) -> String {
    "▮".repeat(crate::bdots_activity::bar_width(day.count, peak, width).max(if day.count > 0 { 1 } else { 0 }))
}

/// The five busiest days, in descending order, with bars.
pub fn top_days(days: &[Day], width: usize) -> Vec<(String, usize, String)> {
    let peak = peak_day(days).map_or(0, |d| d.count);
    let mut ranked: Vec<&Day> = days.iter().collect();
    ranked.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    ranked.into_iter().take(5).map(|d| (d.label.clone(), d.count, day_bar(d, peak, width))).collect()
}

/// The least active days, in ascending order, for a quiet-days strip.
pub fn bottom_days(days: &[Day], width: usize) -> Vec<(String, usize, String)> {
    let peak = peak_day(days).map_or(0, |d| d.count);
    let mut ranked: Vec<&Day> = days.iter().collect();
    ranked.sort_by(|a, b| a.count.cmp(&b.count).then_with(|| a.key.cmp(&b.key)));
    ranked.into_iter().take(3).map(|d| (d.label.clone(), d.count, day_bar(d, peak, width))).collect()
}

/// One plain sentence describing the week.
pub fn week_headline(days: &[Day]) -> String {
    if total(days) == 0 {
        return "No activity this week yet.".to_owned();
    }
    let trend = trend(days);
    let avg = daily_average(days);
    format!("{} · {} steps a day on average", trend_label(trend), avg)
}

/// Semantic tone for the whole week.
pub fn week_tone(days: &[Day]) -> &'static str {
    if total(days) == 0 { "secondary" } else { trend_tone(trend(days)) }
}

/// Whether the bot worked every day of the window.
pub fn perfect_attendance(days: &[Day]) -> bool {
    !days.is_empty() && quiet_days(days) == 0
}

/// Whether activity grew from the first day to the last.
pub fn grew(days: &[Day]) -> bool {
    days.first().zip(days.last()).is_some_and(|(a, b)| b.count > a.count)
}

/// Number of days that beat the weekly average.
pub fn above_average_days(days: &[Day]) -> usize {
    let avg = daily_average(days) as usize;
    days.iter().filter(|d| d.count > avg).count()
}

/// Compact label for the weekly total, reusing the shared formatter.
pub fn total_label(days: &[Day]) -> String {
    crate::bdots_desk::compact_count(total(days) as u64)
}

/// A one-word verdict for the week, for a small badge.
pub fn week_verdict(days: &[Day]) -> &'static str {
    match trend(days) {
        Trend::Up => "Growing",
        Trend::Down => "Slowing",
        Trend::Flat if total(days) == 0 => "Quiet",
        Trend::Flat => "Steady",
    }
}

/// Build the seven `Day` rows from a day→count map, oldest first, filling quiet days with zero.
pub fn build_days(counts: &[(&str, usize)], today: &str, now: i64) -> Vec<Day> {
    let mut out = Vec::new();
    for back in (0..7).rev() {
        let ms = now - back * 86_400_000;
        let key = crate::bdots_activity::bucket_day(ms);
        let count = counts.iter().find(|(k, _)| *k == key).map_or(0, |(_, n)| *n);
        out.push(Day { label: crate::bdots_activity::day_label(&key, today), key, count });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn days(counts: &[usize]) -> Vec<Day> {
        counts
            .iter()
            .enumerate()
            .map(|(i, n)| Day { key: format!("d{i}"), label: format!("D{i}"), count: *n })
            .collect()
    }

    #[test]
    fn trend_compares_halves_with_a_deadband() {
        assert_eq!(trend(&days(&[1, 1, 5, 6])), Trend::Up);
        assert_eq!(trend(&days(&[6, 5, 1, 1])), Trend::Down);
        assert_eq!(trend(&days(&[2, 2, 2, 2])), Trend::Flat);
        assert_eq!(trend(&[]), Trend::Flat);
        assert_eq!(trend_icon(Trend::Up), "arrow.up.right");
        assert_eq!(trend_tone(Trend::Down), "attention");
        assert_eq!(trend_label(Trend::Flat), "About the same");
    }

    #[test]
    fn totals_averages_and_peaks() {
        let d = days(&[2, 0, 5, 3]);
        assert_eq!(total(&d), 10);
        assert_eq!(daily_average(&d), 3);
        assert_eq!(peak_day(&d).map(|x| x.count), Some(5));
        assert_eq!(quiet_days(&d), 1);
        assert_eq!(total_label(&d), "10");
    }

    #[test]
    fn streaks_and_attendance() {
        assert_eq!(active_streak(&days(&[1, 1, 0, 1])), 2);
        assert_eq!(quiet_streak(&days(&[0, 0, 1, 0])), 2);
        assert!(!perfect_attendance(&days(&[1, 0])));
        assert!(perfect_attendance(&days(&[1, 2])));
        assert!(grew(&days(&[1, 2, 3])));
        assert_eq!(above_average_days(&days(&[1, 1, 1, 9])), 1);
    }

    #[test]
    fn change_and_bars() {
        assert_eq!(change_percent(&days(&[1, 1, 2, 2])), 100);
        assert_eq!(change_percent(&days(&[2, 2, 1, 1])), -50);
        assert_eq!(change_percent(&days(&[0, 0])), 0);
        let d = days(&[3, 0, 6]);
        assert!(!day_bar(&d[1], 6, 8).is_empty());
        assert_eq!(top_days(&d, 8).len(), 3);
        assert_eq!(bottom_days(&d, 8)[0].1, 0);
    }

    #[test]
    fn headlines_and_verdicts() {
        assert!(week_headline(&[]).contains("No activity"));
        assert!(week_headline(&days(&[1, 1, 4, 5])).contains("steps a day"));
        assert_eq!(week_tone(&[]), "secondary");
        assert_eq!(week_verdict(&days(&[1, 1, 5, 6])), "Growing");
        assert_eq!(week_verdict(&[]), "Quiet");
        assert_eq!(span_days(&days(&[1, 2, 3])), 3);
    }

    #[test]
    fn build_days_fills_the_window_oldest_first() {
        let today = crate::bdots_activity::bucket_day(86_400_000 * 6);
        let d = build_days(&[("1970-01-07", 4)], "1970-01-07", 86_400_000 * 6);
        assert_eq!(d.len(), 7);
        assert_eq!(d.last().map(|x| x.count), Some(4));
        assert_eq!(d[0].count, 0);
    }
}
