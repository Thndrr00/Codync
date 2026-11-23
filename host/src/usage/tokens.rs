//! One definition of a turn's token counts: new tokens and cache reads, apart.
//!
//! Ported from OpenBot nightly triage-7101. Each model call in a turn reads the
//! saved conversation again, so a "Tokens" total that included cache reads could
//! show more than a million tokens for a short task. New tokens are the ones the
//! Usage tab spends (`inputTokens` + `outputTokens` + `cachedWriteTokens`);
//! `cachedReadTokens` is reported separately as "Cached input". Cost is taken
//! from the provider's own report and never changes here.

use serde_json::Value;

/// The two counts a turn's usage contributes to its trace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TurnTokens {
    /// Fresh input, output and cache writes: the "Tokens" number.
    pub new: u64,
    /// Cache reads: the "Cached input" number.
    pub cached_input: u64,
}

/// Split a provider's usage report into new tokens and cache reads. Missing or
/// non-numeric fields count as zero; older reports with no `cachedReadTokens` get
/// no cached input, so their trace keeps the total it had before this change.
pub fn split(usage: &Value) -> TurnTokens {
    let new =
        ["inputTokens", "outputTokens", "cachedWriteTokens"].iter().map(|key| usage[*key].as_u64().unwrap_or(0)).sum();
    TurnTokens { new, cached_input: usage["cachedReadTokens"].as_u64().unwrap_or(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cache_reads_are_cached_input_not_tokens() {
        let usage = json!({
            "inputTokens": 12,
            "outputTokens": 5,
            "cachedWriteTokens": 3,
            "cachedReadTokens": 1_200_000
        });
        assert_eq!(split(&usage), TurnTokens { new: 20, cached_input: 1_200_000 });
    }

    #[test]
    fn an_old_report_has_no_cached_input_to_show() {
        let usage = json!({"inputTokens": 7, "outputTokens": 4});
        assert_eq!(split(&usage), TurnTokens { new: 11, cached_input: 0 });
        assert_eq!(split(&Value::Null), TurnTokens::default());
    }
}
