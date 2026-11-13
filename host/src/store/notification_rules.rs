//! Which alerts reach the phone, and when. The host decides before it asks the relay to push:
//! a rule here saves the person a buzz instead of the app hiding it afterwards.
//!
//! An approval is never held back by quiet hours: a bot waiting for an answer is stuck until the
//! person sees it, so `needs_input` only obeys its own switch.

use super::Store;
use anyhow::{Result, ensure};
use chrono::{DateTime, Local, Timelike, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

const KEY: &str = "notification-rules.v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationRules {
    /// A turn finished.
    pub done: bool,
    /// A bot needs an answer before it can go on.
    pub needs_input: bool,
    /// A turn failed.
    pub failed: bool,
    /// A turn paused before it was done.
    pub paused: bool,
    /// Start of the quiet window, a local hour (0–23). Equal bounds mean no quiet hours.
    pub quiet_start: u8,
    pub quiet_end: u8,
    /// The zone the quiet hours are read in; unset uses this computer's zone.
    pub timezone: Option<String>,
}

impl Default for NotificationRules {
    fn default() -> Self {
        Self { done: true, needs_input: true, failed: true, paused: true, quiet_start: 0, quiet_end: 0, timezone: None }
    }
}

/// The local hour at `now` in `zone` (this computer's zone when unset or unknown).
pub fn local_hour(now: i64, zone: Option<&str>) -> u32 {
    let at = DateTime::<Utc>::from_timestamp_millis(now).unwrap_or_default();
    match zone.and_then(|z| z.parse::<Tz>().ok()) {
        Some(tz) => at.with_timezone(&tz).hour(),
        None => at.with_timezone(&Local).hour(),
    }
}

/// Whether `hour` falls in `start..end`, which may wrap past midnight. Equal bounds mean never.
pub fn is_quiet(hour: u32, start: u8, end: u8) -> bool {
    let (start, end) = (u32::from(start), u32::from(end));
    match start.cmp(&end) {
        std::cmp::Ordering::Equal => false,
        std::cmp::Ordering::Less => (start..end).contains(&hour),
        std::cmp::Ordering::Greater => hour >= start || hour < end,
    }
}

impl NotificationRules {
    /// Whether an alert of this kind may be pushed at `now`.
    pub fn allows(&self, kind: NotificationKind, now: i64) -> bool {
        let switched_on = match kind {
            NotificationKind::Done => self.done,
            NotificationKind::NeedsInput => self.needs_input,
            NotificationKind::Failed => self.failed,
            NotificationKind::Paused => self.paused,
        };
        if !switched_on {
            return false;
        }
        if kind == NotificationKind::NeedsInput {
            return true;
        }
        !is_quiet(local_hour(now, self.timezone.as_deref()), self.quiet_start, self.quiet_end)
    }

    fn check(&self) -> Result<()> {
        ensure!(self.quiet_start <= 23 && self.quiet_end <= 23, "quiet hours are local hours, from 0 to 23");
        if let Some(zone) = self.timezone.as_deref().filter(|z| !z.is_empty()) {
            ensure!(zone.parse::<Tz>().is_ok(), "unknown time zone: {zone}");
        }
        Ok(())
    }
}

/// The four alerts, named here so the store does not depend on the push layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    Done,
    NeedsInput,
    Failed,
    Paused,
}

impl Store {
    pub fn notification_rules(&self) -> Result<NotificationRules> {
        match self.kv_read(KEY)? {
            Some(value) => Ok(serde_json::from_str(&value)?),
            None => Ok(NotificationRules::default()),
        }
    }

    /// Saves the rules after checking them; the caller keeps the old ones when this fails.
    pub fn set_notification_rules(&self, rules: &NotificationRules) -> Result<()> {
        rules.check()?;
        self.kv_set(KEY, &serde_json::to_string(rules)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-01-01 at `hour`:00 UTC.
    const DAY: i64 = 1_767_225_600_000;
    fn at(hour: i64) -> i64 {
        DAY + hour * 3_600_000
    }

    fn rules() -> NotificationRules {
        NotificationRules {
            timezone: Some("UTC".into()),
            quiet_start: 22,
            quiet_end: 8,
            ..NotificationRules::default()
        }
    }

    #[test]
    fn quiet_hours_wrap_past_midnight() {
        assert!(is_quiet(23, 22, 8) && is_quiet(3, 22, 8));
        assert!(!is_quiet(8, 22, 8) && !is_quiet(12, 22, 8));
        assert!(!is_quiet(3, 5, 5), "equal bounds mean no quiet hours");
    }

    #[test]
    fn quiet_hours_hold_back_everything_but_an_approval() {
        let r = rules();
        assert!(!r.allows(NotificationKind::Done, at(23)));
        assert!(!r.allows(NotificationKind::Failed, at(2)));
        assert!(r.allows(NotificationKind::NeedsInput, at(2)), "a blocked bot still reaches the person");
        assert!(r.allows(NotificationKind::Done, at(12)));
    }

    #[test]
    fn a_switched_off_kind_never_reaches_the_phone() {
        let r = NotificationRules { done: false, needs_input: false, ..rules() };
        assert!(!r.allows(NotificationKind::Done, at(12)));
        assert!(!r.allows(NotificationKind::NeedsInput, at(12)));
        assert!(r.allows(NotificationKind::Paused, at(12)));
    }

    #[test]
    fn rules_are_saved_checked_and_an_unreadable_save_changes_nothing() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(store.notification_rules().unwrap(), NotificationRules::default());
        let quiet = rules();
        store.set_notification_rules(&quiet).unwrap();
        assert_eq!(store.notification_rules().unwrap(), quiet);
        let broken = NotificationRules { quiet_start: 24, ..quiet.clone() };
        assert!(store.set_notification_rules(&broken).is_err());
        let bad_zone = NotificationRules { timezone: Some("Mars/Olympus".into()), ..quiet.clone() };
        assert!(store.set_notification_rules(&bad_zone).is_err());
        assert_eq!(store.notification_rules().unwrap(), quiet, "the old rules stay");
    }
}
