//! Per-provider account rotation: one stamped kv blob (`account.rotation.<backend>`) holds the
//! mode, the manually chosen account and the cursor. Picking an account is pure — callers supply
//! the rest each account has — so it can be tested without a host, a store or analytics.

use super::{AgentAccount, checked_id};
use crate::agent::fallback::{Reason, rest};
use crate::store::{Store, now_ms};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Most isolated accounts one provider may keep on a computer.
pub const MAX_ACCOUNTS_PER_PROVIDER: usize = 20;

const PREFIX: &str = "account.rotation.";

/// When the host moves a bot to another account of the same provider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Always use the chosen account (or the computer's own sign-in). Rotation is off.
    #[default]
    Manual,
    /// Use the next eligible account on every turn.
    RoundRobin,
    /// Keep the active account until it runs out or is rate limited, then move on.
    OnRateLimit,
}

impl Mode {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "manual" => Ok(Self::Manual),
            "roundRobin" => Ok(Self::RoundRobin),
            "onRateLimit" => Ok(Self::OnRateLimit),
            _ => bail!("Unknown account rotation mode"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::RoundRobin => "roundRobin",
            Self::OnRateLimit => "onRateLimit",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    /// The sync revision stamped on the last change.
    #[serde(default)]
    pub rev: i64,
    #[serde(default)]
    pub mode: Mode,
    /// The account `Mode::Manual` uses.
    #[serde(default)]
    pub account_id: Option<String>,
    /// The next account in round-robin, or the active one when rotating only on a limit.
    #[serde(default)]
    pub cursor: usize,
}

fn key(backend: &str) -> String {
    format!("{PREFIX}{backend}")
}

/// A damaged or older blob reads as the default (rotation off), like the other stamped blobs.
pub fn load(store: &Store, backend: &str) -> Result<Policy> {
    match store.kv_read(&key(backend))? {
        Some(raw) => serde_json::from_str(&raw).context("Invalid account rotation policy"),
        None => Ok(Policy::default()),
    }
}

/// Stamps one global rev for the change and saves the blob in the same transaction.
pub fn save(store: &Store, backend: &str, policy: &mut Policy) -> Result<()> {
    store.kv_set_stamped(&key(backend), |rev| {
        policy.rev = rev;
        Ok(serde_json::to_string(policy)?)
    })
}

/// A policy a client can render, without any credential data.
pub fn wire(policy: &Policy) -> Value {
    json!({
        "mode": policy.mode.as_str(),
        "accountId": policy.account_id,
        "cursor": policy.cursor,
        "rev": policy.rev,
    })
}

/// Saves a mode and a manually chosen account. The id must belong to this provider.
pub fn set(store: &Store, backend: &str, mode: Mode, account_id: Option<&str>) -> Result<Policy> {
    if let Some(id) = account_id {
        checked_id(id)?;
        super::validate(id, backend)?;
    }
    let mut policy = load(store, backend)?;
    policy.mode = mode;
    policy.account_id = account_id.map(str::to_owned);
    policy.cursor = 0;
    save(store, backend, &mut policy)?;
    Ok(policy)
}

/// One turn's pick: `None` means the computer's own sign-in. The cursor indexes the accounts in
/// host order; it advances after a round-robin turn and sticks to the active account otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub id: Option<String>,
    pub cursor: usize,
}

fn limited(reason: Reason) -> bool {
    matches!(
        reason,
        Reason::UsageLimit | Reason::OutOfCredits | Reason::RateLimited | Reason::Overloaded | Reason::SignedOut
    )
}

/// Picks an account from `accounts` in host order. `resting(id)` reports why one cannot answer.
/// A bot's pin is applied by the caller and always wins. With fewer than two accounts nothing
/// rotates, so a computer with one extra account keeps its old behaviour.
pub fn choose(
    accounts: &[AgentAccount],
    mode: Mode,
    chosen: Option<&str>,
    cursor: usize,
    resting: impl Fn(&str) -> Option<Reason>,
) -> Choice {
    if accounts.len() < 2 {
        let id = chosen.filter(|id| accounts.iter().any(|a| a.id == *id) && resting(id).is_none()).map(str::to_owned);
        return Choice { id, cursor: 0 };
    }
    match mode {
        Mode::Manual => Choice {
            id: chosen.filter(|id| accounts.iter().any(|a| a.id == *id) && resting(id).is_none()).map(str::to_owned),
            cursor,
        },
        Mode::RoundRobin => {
            let start = cursor % accounts.len();
            let next = (0..accounts.len())
                .map(|step| (start + step) % accounts.len())
                .find(|&i| resting(&accounts[i].id).is_none());
            match next {
                Some(i) => Choice { id: Some(accounts[i].id.clone()), cursor: (i + 1) % accounts.len() },
                None => Choice { id: None, cursor },
            }
        }
        Mode::OnRateLimit => {
            let index = accounts.iter().position(|a| Some(a.id.as_str()) == chosen).unwrap_or(cursor % accounts.len());
            if resting(&accounts[index].id).is_none_or(|reason| !limited(reason)) {
                return Choice { id: Some(accounts[index].id.clone()), cursor: index };
            }
            let next = (1..=accounts.len())
                .map(|step| (index + step) % accounts.len())
                .find(|&i| resting(&accounts[i].id).is_none());
            match next {
                Some(i) => Choice { id: Some(accounts[i].id.clone()), cursor: i },
                None => Choice { id: None, cursor },
            }
        }
    }
}

/// An account's rest at `now`. Rests the host records against the provider (it cannot yet tell
/// which isolated profile failed) belong to the account currently in use; other accounts stay
/// eligible so rotation can carry on.
fn resting(store: &Store, backend: &str, account: &str, active: Option<&str>, now: i64) -> Option<Reason> {
    // A usage or rate limit of one isolated profile now rests only that profile; provider-wide
    // reasons (signed out, out of credits) still fall through to the provider key (OpenBot nightly).
    if let Some(rest) = rest::get_scoped(store, backend, Some(account), now) {
        return Some(rest.reason);
    }
    if rest::signed_out(store, backend).iter().any(|id| id.as_str() == account) {
        return Some(Reason::SignedOut);
    }
    if active == Some(account) {
        match rest::get_scoped(store, backend, None, now) {
            Some(rest) if limited(rest.reason) => return Some(rest.reason),
            _ => {}
        }
    }
    None
}

/// The account a bot's turn uses. A pin wins; otherwise the provider's policy picks one, resting
/// accounts are skipped and the cursor is stamped only when it changes.
pub fn select(store: &Store, backend: &str, pinned: Option<&str>) -> Result<Option<String>> {
    if let Some(id) = pinned {
        checked_id(id)?;
        return Ok(Some(id.to_owned()));
    }
    let accounts = super::list(Some(backend))?;
    let mut policy = load(store, backend)?;
    let now = now_ms();
    let active = policy
        .account_id
        .as_deref()
        .or_else(|| accounts.get(policy.cursor % accounts.len().max(1)).map(|a| a.id.as_str()));
    let choice = choose(&accounts, policy.mode, policy.account_id.as_deref(), policy.cursor, |id| {
        resting(store, backend, id, active, now)
    });
    if choice.cursor != policy.cursor {
        policy.cursor = choice.cursor;
        save(store, backend, &mut policy)?;
    }
    Ok(choice.id)
}

/// Accounts and their current state for the host API; never tokens or profile paths.
pub fn states(store: &Store, backend: &str) -> Result<Vec<Value>> {
    let now = now_ms();
    let accounts = super::list(Some(backend))?;
    let active = accounts.first().map(|a| a.id.as_str());
    Ok(accounts
        .iter()
        .map(|account| {
            let reason = resting(store, backend, &account.id, active, now);
            json!({
                "id": account.id,
                "name": account.name,
                "email": account.email,
                "state": match reason {
                    Some(Reason::SignedOut) => "signedOut",
                    Some(_) => "resting",
                    None => "ready",
                },
                "restingReason": reason.and_then(|r| serde_json::to_value(r).ok()),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str) -> AgentAccount {
        AgentAccount { id: id.into(), name: id.into(), backend: "claude".into(), created_at: 0, email: None }
    }

    fn accounts() -> Vec<AgentAccount> {
        ["acc_00000001", "acc_00000002", "acc_00000003"].map(account).to_vec()
    }

    #[test]
    fn round_robin_visits_each_account_and_skips_a_resting_one() {
        let all = accounts();
        let resting = |id: &str| (id == "acc_00000002").then_some(Reason::RateLimited);
        let first = choose(&all, Mode::RoundRobin, None, 0, resting);
        assert_eq!(first.id.as_deref(), Some("acc_00000001"));
        let second = choose(&all, Mode::RoundRobin, None, first.cursor, resting);
        assert_eq!(second.id.as_deref(), Some("acc_00000003"), "the limited account is skipped");
        let third = choose(&all, Mode::RoundRobin, None, second.cursor, resting);
        assert_eq!(third.id.as_deref(), Some("acc_00000001"), "the cursor keeps cycling past the resting one");
        assert_eq!(
            choose(&all, Mode::RoundRobin, None, 0, |_| Some(Reason::Overloaded)).id,
            None,
            "no healthy account means the default sign-in"
        );
    }

    #[test]
    fn rate_limit_mode_stays_put_until_the_active_account_runs_out() {
        let all = accounts();
        let healthy = |_: &str| None;
        let first = choose(&all, Mode::OnRateLimit, None, 0, healthy);
        assert_eq!(first.id.as_deref(), Some("acc_00000001"));
        let still = choose(&all, Mode::OnRateLimit, None, first.cursor, healthy);
        assert_eq!(still.id.as_deref(), Some("acc_00000001"), "it doesn't rotate every turn");
        let limited = |id: &str| (id == "acc_00000001").then_some(Reason::UsageLimit);
        let moved = choose(&all, Mode::OnRateLimit, None, still.cursor, limited);
        assert_eq!(moved.id.as_deref(), Some("acc_00000002"));
        let settled = choose(&all, Mode::OnRateLimit, None, moved.cursor, limited);
        assert_eq!(settled.id.as_deref(), Some("acc_00000002"), "it sticks to the new account");
        let not_a_limit = |id: &str| (id == "acc_00000002").then_some(Reason::NotStarting);
        assert_eq!(
            choose(&all, Mode::OnRateLimit, None, settled.cursor, not_a_limit).id.as_deref(),
            Some("acc_00000002"),
            "a start failure is not a reason to change accounts"
        );
    }

    #[test]
    fn a_pin_or_a_single_account_keeps_the_old_behaviour() {
        let all = accounts();
        let pin = choose(&all, Mode::RoundRobin, Some("acc_00000003"), 0, |_| None);
        assert_eq!(pin.id.as_deref(), Some("acc_00000003"));
        let one = [account("acc_00000001")];
        for mode in [Mode::Manual, Mode::RoundRobin, Mode::OnRateLimit] {
            assert_eq!(choose(&one, mode, None, 0, |_| None).id, None, "one account never rotates");
        }
        assert_eq!(choose(&one, Mode::Manual, Some("acc_00000001"), 0, |_| None).id.as_deref(), Some("acc_00000001"));
        assert_eq!(
            choose(&all, Mode::Manual, Some("acc_00000001"), 0, |id| (id == "acc_00000001")
                .then_some(Reason::SignedOut))
            .id,
            None,
            "a resting chosen account falls back to the default"
        );
    }
}
