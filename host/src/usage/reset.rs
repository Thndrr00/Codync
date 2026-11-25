//! Spending a provider's extra credits to reset a usage hold (OpenBot providers.md). The hold is
//! the host's fallback rest; the credit balance comes from the usage report, never from a client
//! credential. Pure decisions here are shared by the refresh and the reset method.

use super::{Usage, Window};
use crate::LockExt;
use crate::agent::fallback::{Reason, rest};
use crate::hub::Hub;
use crate::store::{Store, now_ms};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;

/// An account with no reported reset time spends credits until the provider's next read.
const DEFAULT_WINDOW: i64 = 60 * 60 * 1000;

/// A provider row that can offer Reset usage.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreditOffer {
    pub id: String,
    /// Extra credits left after earlier resets.
    pub credits: u32,
    pub resets_at: Option<i64>,
}

/// A provider row spending extra credits after a reset, until its window ended.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsingCredits {
    pub id: String,
    pub until: i64,
    pub credits_left: u32,
}

/// The one fact about an account the decision needs: the hold that stops its bots now.
pub struct Account<'a> {
    pub id: &'a str,
    pub hold: Option<Reason>,
}

/// Whether a "Reset usage" action may be offered: the account is held by a spent plan window and
/// the provider reported at least one extra credit to spend. Pure, so the API and tests agree.
pub fn reset_usage_offer(account: &Account, credits: u32) -> bool {
    account.hold == Some(Reason::UsageLimit) && credits > 0
}

fn credit_key(id: &str) -> String {
    format!("usage.credits_spent.{id}")
}

/// Resets already spent here, kept in the host's kv so a restart doesn't give them back.
fn spent(store: &Store, id: &str) -> u32 {
    store.kv_get(&credit_key(id)).and_then(|value| value.parse().ok()).unwrap_or(0)
}

fn spend(store: &Store, id: &str) -> Result<u32> {
    let next = spent(store, id).saturating_add(1);
    store.kv_set(&credit_key(id), &next.to_string())?;
    Ok(next)
}

/// The count in a note such as "250 credits" or "20/100 left". A note without a count is none.
fn credit_count(text: &str) -> Option<u32> {
    text.split(|c: char| !c.is_ascii_digit()).find(|part| !part.is_empty())?.parse().ok()
}

/// Extra credits a provider reported as a note window. A quota percent or an unknown balance is
/// never treated as credits.
fn credits(usage: &Usage, id: &str) -> u32 {
    let Some(provider) = usage.providers.iter().find(|p| p.id == id) else { return 0 };
    let total = provider
        .windows
        .iter()
        .filter(|w: &&Window| w.text.is_some())
        .filter(|w| w.id == "credits" || w.id == "bonus" || w.label.to_ascii_lowercase().contains("credit"))
        .filter_map(|w| w.text.as_deref())
        .filter_map(credit_count)
        .fold(0u64, |sum, n| sum.saturating_add(u64::from(n)));
    u32::try_from(total).unwrap_or(u32::MAX)
}

fn split_id(id: &str) -> (&str, Option<&str>) {
    id.split_once(':').map_or((id, None), |(backend, account)| (backend, Some(account)))
}

fn offers(usage: &Usage, store: &Store, now: i64) -> Vec<CreditOffer> {
    usage
        .providers
        .iter()
        .filter_map(|provider| {
            let (backend, account) = split_id(&provider.id);
            let held = rest::get_scoped(store, backend, account, now)?;
            let left = credits(usage, &provider.id).saturating_sub(spent(store, &provider.id));
            if !reset_usage_offer(&Account { id: &provider.id, hold: Some(held.reason) }, left) {
                return None;
            }
            Some(CreditOffer { id: provider.id.clone(), credits: left, resets_at: held.reset_at.or(Some(held.until)) })
        })
        .collect()
}

/// Recomputes the offers after a reading lands, and forgets "Using credits" once its window passed.
pub fn refresh_offers(hub: &Arc<Hub>) {
    let now = now_ms();
    let next = {
        let usage = hub.usage.locked();
        offers(&usage, &hub.store, now)
    };
    hub.update_usage(|usage| {
        usage.credit_offers = next;
        usage.using_credits.retain(|using| using.until > now);
    });
}

/// Consumes one of the provider's extra credits, ends its usage-limit hold and marks the account as
/// spending credits. Returns the state the apps need; no credential is read or returned here.
pub fn reset_usage(hub: &Arc<Hub>, id: &str) -> Result<Value> {
    let now = now_ms();
    let offer = {
        let usage = hub.usage.locked();
        usage.credit_offers.iter().find(|o| o.id == id).cloned()
    }
    .context("no usage-limit reset is available for this provider")?;
    let (backend, account) = split_id(id);
    let mut cleared = false;
    if let Some(account) = account {
        cleared = rest::clear_scoped(&hub.store, backend, account);
    }
    if rest::get(&hub.store, backend, now).is_some_and(|held| held.reason == Reason::UsageLimit) {
        cleared |= rest::clear(&hub.store, backend);
    }
    if !cleared {
        bail!("the usage limit has already ended");
    }
    let credits_spent = spend(&hub.store, id)?;
    let until = offer.resets_at.unwrap_or_else(|| now + DEFAULT_WINDOW);
    let credits_left = offer.credits.saturating_sub(1);
    crate::agent::fallback::tell_bots(hub, id);
    hub.update_usage(|usage| {
        usage.credit_offers.retain(|o| o.id != id);
        usage.using_credits.retain(|using| using.id != id);
        usage.using_credits.push(UsingCredits { id: id.to_owned(), until, credits_left });
    });
    hub.usage_wake.notify_one();
    Ok(
        json!({"id": id, "usingCredits": true, "until": until, "creditsLeft": credits_left, "creditsSpent": credits_spent}),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::hub::Hub;
    use crate::remote::identity::Identity;
    use crate::usage::{Provider, Window};

    fn hub_with(provider: Provider) -> Arc<Hub> {
        let dir = std::env::temp_dir().join(format!("codync-reset-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let hub = Hub::new(store, "reset".into(), Identity::load_or_create(&dir).unwrap(), "t".into(), 0);
        hub.update_usage(|usage| usage.providers.push(provider));
        Arc::new(hub)
    }

    #[test]
    fn a_reset_is_offered_only_for_a_spent_window_with_credits_left() {
        let account = |hold| Account { id: "codex", hold };
        assert!(reset_usage_offer(&account(Some(Reason::UsageLimit)), 1));
        assert!(!reset_usage_offer(&account(Some(Reason::UsageLimit)), 0), "no credit to spend");
        assert!(!reset_usage_offer(&account(Some(Reason::OutOfCredits)), 3), "that needs a top-up");
        assert!(!reset_usage_offer(&account(None), 3), "a provider that can answer needs no reset");
        assert!(!reset_usage_offer(&account(Some(Reason::RateLimited)), 3));
    }

    #[test]
    fn only_reported_credit_notes_count_and_every_reset_spends_one() {
        let mut usage = Usage::default();
        usage.providers.push(Provider::new(
            "gemini",
            "Gemini",
            "test",
            vec![
                Window::new("weekly", "Weekly", 100.0, Some(9_000)),
                Window::note("credits", "Credit balance", "250 credits"),
            ],
        ));
        assert_eq!(credits(&usage, "gemini"), 250, "a quota bar is not a credit balance");
        assert_eq!(credits(&usage, "nobody"), 0);

        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(spent(&store, "gemini"), 0);
        assert_eq!(spend(&store, "gemini").unwrap(), 1);
        assert_eq!(spend(&store, "gemini").unwrap(), 2);
        let store2 = Store::open(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(spent(&store2, "gemini"), 0, "a different host has no shared budget");
        assert_eq!(credit_count("Unlimited"), None);
        assert_eq!(credit_count("$4.20"), Some(4));
    }

    #[test]
    fn a_usage_hold_with_credits_becomes_an_offer() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let mut usage = Usage::default();
        usage.providers.push(Provider::new(
            "claude:acc_1",
            "Claude 2",
            "account",
            vec![Window::note("credits", "Credit balance", "5 credits")],
        ));
        assert!(offers(&usage, &store, 1_000).is_empty(), "no hold yet");
        rest::record_scoped(&store, "claude", Some("acc_1"), Reason::UsageLimit, 1_000, Some(9_000));
        let list = offers(&usage, &store, 1_000);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].credits, 5);
        assert_eq!(list[0].resets_at, Some(9_000));
        spend(&store, "claude:acc_1").unwrap();
        assert_eq!(offers(&usage, &store, 1_000)[0].credits, 4, "an earlier reset used one");
        rest::clear_scoped(&store, "claude", "acc_1");
        assert!(offers(&usage, &store, 1_000).is_empty());
    }

    #[test]
    fn resetting_usage_ends_the_hold_and_reports_a_credit_was_spent() {
        let provider =
            Provider::new("codex", "Codex", "test", vec![Window::note("credits", "Credit balance", "3 credits")]);
        let hub = hub_with(provider);
        let now = now_ms();
        rest::record(&hub.store, "codex", Reason::UsageLimit, now, Some(now + 90_000));
        refresh_offers(&hub);
        assert_eq!(hub.usage.locked().credit_offers.len(), 1);

        let answer = reset_usage(&hub, "codex").unwrap();
        assert_eq!(answer["usingCredits"], true);
        assert_eq!(answer["creditsSpent"], 1);
        assert_eq!(answer["creditsLeft"], 2);
        assert!(rest::get(&hub.store, "codex", now).is_none(), "the hold ended at once");
        {
            let usage = hub.usage.locked();
            assert!(usage.credit_offers.is_empty());
            assert_eq!(usage.using_credits.len(), 1);
            assert_eq!(usage.using_credits[0].id, "codex");
        }
        assert!(reset_usage(&hub, "codex").is_err(), "a second reset has no hold to clear");
    }
}
