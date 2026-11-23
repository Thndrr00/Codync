//! Bounded batches, per-result status and availability for flow media steps, ported from OpenBot
//! nightly's `MEDIA_BATCH_SIZES = [1, 2, 3, 4]` and `mediaTakes`, and from its "model not available"
//! and three-day reference retention changes. Everything here is data and decisions: the provider
//! calls stay in `flow` and `sunra`, so one batch takes the same path as a single request.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// The batch sizes a step may ask for, as OpenBot shows them.
pub const BATCH_SIZES: [u32; 4] = [1, 2, 3, 4];
pub const DEFAULT_BATCH: u32 = 1;
pub const MAX_BATCH: u32 = 4;
/// The most files one step keeps, whatever the batch or how many a provider returned.
pub const MAX_STEP_FILES: usize = 4;
/// The one error a run gives when its step's model cannot be made now. Kept fixed so clients can
/// show one clear message and never start a chargeable call.
pub const MODEL_UNAVAILABLE: &str = "The model of this step is not available";
/// OpenBot nightly keeps uploaded reference pictures for three days (it was one). The service owns
/// the upload; the host only publishes the window so a client can say how long a picture is kept.
pub const REFERENCE_RETENTION_DAYS: u64 = 3;
pub const REFERENCE_RETENTION: Duration = Duration::from_secs(3 * 24 * 60 * 60);

/// The serde default for older settings and steps, which predate a batch field.
#[must_use]
pub fn default_batch() -> u32 {
    DEFAULT_BATCH
}

/// A batch in `1..=4`: an absent or out-of-range value falls back to the bound, as an unknown media
/// setting falls back to a model's first choice.
#[must_use]
pub fn media_batch(value: u32) -> u32 {
    value.clamp(DEFAULT_BATCH, MAX_BATCH)
}

/// What a client's Generate button shows for a batch: the price of one result times the batch.
/// `price` is a count of credits, never a token, address or description, so it never reaches logs.
#[must_use]
pub fn batch_credits(price: u64, batch: u32) -> u64 {
    price.saturating_mul(u64::from(media_batch(batch)))
}

/// The batch choices and their totals in a catalog entry. `None` is a subscription model with no
/// price to multiply; its Generate button shows no credit amount.
#[must_use]
pub fn batch_credits_view(price: Option<u64>) -> Value {
    match price {
        None => json!({"batchSizes": BATCH_SIZES, "credits": Value::Null, "totals": Value::Null}),
        Some(price) => json!({
            "batchSizes": BATCH_SIZES,
            "credits": price,
            "totals": BATCH_SIZES.map(|batch| json!({"batch": batch, "credits": batch_credits(price, batch)})),
        }),
    }
}

/// Whether `model` is in the catalog of models the service can make now. Pure so a run can check
/// before it starts a provider call, and a catalog can be filtered before it reaches a client.
#[must_use]
pub fn model_available<'a>(catalog: impl IntoIterator<Item = &'a str>, model: &str) -> bool {
    catalog.into_iter().any(|id| id == model)
}

/// Whether an uploaded reference picture has passed its three-day window.
#[must_use]
pub fn reference_expired(uploaded_at_ms: i64, now_ms: i64) -> bool {
    let window = i64::try_from(REFERENCE_RETENTION.as_millis()).unwrap_or(i64::MAX);
    now_ms.saturating_sub(uploaded_at_ms) >= window
}

/// The state of one result in a batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BatchStatus {
    /// Not made yet: still running, or the flow stopped before it started.
    #[default]
    Pending,
    Succeeded,
    Failed,
}

impl BatchStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

/// One result's status on a step's saved settings. A failure carries the text the client shows with
/// its Retry button; no price is stored here, the client reads it from the catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub index: u32,
    pub status: BatchStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The whole batch's status, with the results not yet finished left pending. Pure: the engine
/// records this after every attempt so a client can follow progress result by result.
#[must_use]
pub fn batch_results(batch: u32, succeeded: &BTreeSet<u32>, failed: &BTreeMap<u32, String>) -> Vec<BatchResult> {
    (0..media_batch(batch))
        .map(|index| {
            let (status, error) = if let Some(reason) = failed.get(&index) {
                (BatchStatus::Failed, Some(reason.clone()))
            } else if succeeded.contains(&index) {
                (BatchStatus::Succeeded, None)
            } else {
                (BatchStatus::Pending, None)
            };
            BatchResult { index, status, error }
        })
        .collect()
}

/// Bounds the files a step keeps, in the order they were made.
pub fn kept_files<T>(files: Vec<T>) -> Vec<T> {
    files.into_iter().take(MAX_STEP_FILES).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_is_one_to_four_and_an_out_of_range_value_falls_back() {
        assert_eq!(media_batch(0), 1);
        assert_eq!(media_batch(1), 1);
        assert_eq!(media_batch(3), 3);
        assert_eq!(media_batch(4), 4);
        assert_eq!(media_batch(9), 4);
        assert_eq!(BATCH_SIZES, [1, 2, 3, 4]);
        assert_eq!(default_batch(), 1);
    }

    #[test]
    fn credits_are_the_price_times_the_batch_without_overflow() {
        assert_eq!(batch_credits(7, 1), 7);
        assert_eq!(batch_credits(7, 4), 28);
        assert_eq!(batch_credits(7, 99), 28, "a batch is bounded before it is priced");
        assert_eq!(batch_credits(u64::MAX, 4), u64::MAX);
    }

    #[test]
    fn a_catalog_shows_each_batch_total_and_no_price_for_subscription_models() {
        let priced = batch_credits_view(Some(5));
        assert_eq!(priced["credits"], json!(5));
        assert_eq!(priced["batchSizes"], json!([1, 2, 3, 4]));
        assert_eq!(priced["totals"][3]["credits"], json!(20));
        assert_eq!(batch_credits_view(None)["totals"], Value::Null);
    }

    #[test]
    fn availability_is_a_decision_about_the_catalog_given_to_it() {
        let offered = ["sunra-a", "sunra-b"];
        assert!(model_available(offered, "sunra-b"));
        assert!(!model_available(offered, "sunra-c"));
        assert!(!model_available([], "sunra-a"));
    }

    #[test]
    fn a_reference_picture_expires_after_three_days_not_one() {
        assert_eq!(REFERENCE_RETENTION_DAYS, 3);
        let at = 10_000_000_000;
        assert!(!reference_expired(at, at + 86_400_000));
        assert!(!reference_expired(at, at + 3 * 86_400_000 - 1));
        assert!(reference_expired(at, at + 3 * 86_400_000));
        assert!(!reference_expired(at + 1, at), "a clock going back does not expire it");
    }

    #[test]
    fn a_partly_finished_batch_keeps_the_other_results_pending() {
        let ok = BTreeSet::from([0u32]);
        let bad = BTreeMap::from([(1u32, "no file".to_owned())]);
        let results = batch_results(3, &ok, &bad);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].status, BatchStatus::Succeeded);
        assert_eq!(results[1].status, BatchStatus::Failed);
        assert_eq!(results[1].error.as_deref(), Some("no file"));
        assert_eq!(results[2].status, BatchStatus::Pending);
        assert_eq!(batch_results(99, &ok, &bad).len(), 4);
    }

    #[test]
    fn a_step_keeps_at_most_four_files_in_the_order_they_were_made() {
        assert_eq!(kept_files(vec![1, 2, 3, 4, 5, 6]), vec![1, 2, 3, 4]);
        assert_eq!(kept_files(vec![9, 8]), vec![9, 8]);
    }

    #[test]
    fn older_settings_read_with_one_result_and_no_results() {
        #[derive(Deserialize)]
        struct Saved {
            #[serde(default = "default_batch")]
            batch: u32,
            #[serde(default)]
            results: Vec<BatchResult>,
        }
        let saved: Saved = serde_json::from_value(json!({})).unwrap();
        assert_eq!(saved.batch, 1);
        assert!(saved.results.is_empty());
        let written = serde_json::to_value(BatchResult { index: 0, status: BatchStatus::Failed, error: None }).unwrap();
        assert_eq!(written["status"], json!("failed"));
        assert!(written.get("error").is_none());
    }
}
