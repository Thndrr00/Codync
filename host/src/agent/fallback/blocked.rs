//! When every automatic backup is at its usage limit or on hold after a recent failure, the chat says so
//! instead of showing only the provider's limit text (OpenBot nightly `triage-14070`). Models of one account
//! can share a limit; a backup on another provider or another account lets the agent continue.

use super::Reason;
use super::rest::Rest;

/// Why a backup step cannot take the turn now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupStatus {
    /// No rest: it can answer.
    Available,
    /// Its plan's limit is used up.
    Limited,
    /// It failed a short time ago and is resting for it.
    Failed,
    /// Signed out (or its model missing): not a limit, so it must not count as one.
    SignedOut,
}

impl BackupStatus {
    /// The status of a step from the rest it is under at `now` (active rests only).
    pub fn of(rest: Option<&Rest>) -> Self {
        match rest.map(|r| r.reason) {
            None => Self::Available,
            Some(Reason::UsageLimit | Reason::OutOfCredits) => Self::Limited,
            Some(
                Reason::RateLimited | Reason::Overloaded | Reason::NotStarting | Reason::Network | Reason::Timeout,
            ) => Self::Failed,
            Some(Reason::SignedOut | Reason::RefusedKey | Reason::ModelMissing) => Self::SignedOut,
        }
    }
}

/// Only when there is at least one automatic backup and each one is limited or failed does the chat explain
/// the shared limit. A signed-out or available backup keeps the ordinary failure text.
pub fn all_backups_blocked(statuses: &[BackupStatus]) -> bool {
    !statuses.is_empty() && statuses.iter().all(|s| matches!(s, BackupStatus::Limited | BackupStatus::Failed))
}

/// A short sentence that fits in a chat banner, with the provider's redacted detail kept in the log.
pub const NOTE: &str = "Every backup model is also at its usage limit or failed a short time ago. Models on one account can share a limit; a backup on another provider or another account lets the agent continue.";

#[cfg(test)]
mod tests {
    use super::*;

    fn rest(reason: Reason) -> Rest {
        Rest { until: i64::MAX, reason, failures: 1, reset_at: None }
    }

    #[test]
    fn the_shared_limit_note_needs_every_backup_limited_or_failed() {
        let limited = BackupStatus::of(Some(&rest(Reason::UsageLimit)));
        let failed = BackupStatus::of(Some(&rest(Reason::RateLimited)));
        assert!(all_backups_blocked(&[limited, failed]));
        assert!(!all_backups_blocked(&[]), "no backups keep the ordinary text");
        assert!(!all_backups_blocked(&[BackupStatus::Available]));
        assert!(!all_backups_blocked(&[limited, BackupStatus::SignedOut]), "signed out is not a limit");
        assert!(NOTE.len() < 300, "the banner shows all of it");
    }
}
