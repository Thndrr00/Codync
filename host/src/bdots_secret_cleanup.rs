#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo's `secret-cleanup.ts`: secret reference ids, cleanup job keys,
//! timing windows and holder-column validation. Composes helpers already ported into this host.
//! Only `botInsights` calls this.

/// Retry attempt window in ms (ATTEMPT_MS).
pub const ATTEMPT_MS: u64 = 10_000;

/// Backup window in ms (BACKUP_MS).
pub const BACKUP_MS: u64 = 15_000;

/// Store write deadline in ms (STORE_WRITE_MS).
pub const STORE_WRITE_MS: u64 = 30_000;

/// Save margin in ms (SAVE_MARGIN_MS).
pub const SAVE_MARGIN_MS: u64 = 300_000;

/// Provider-call window before a row is saved (PROVIDER_BEFORE_SAVE_MS).
pub const PROVIDER_BEFORE_SAVE_MS: u64 = SAVE_MARGIN_MS / 2;

/// Prefixes of secret references.
pub const PREFIXES: [&str; 4] = ["purchase-request:", "purchase-card:", "purchase-token:", "payment-wallet:"];

/// Request record id (purchaseRequestRecordId).
pub fn purchase_request_record_id(operation_key: &str) -> String {
    format!("purchase-request:{operation_key}")
}

/// Card record id (purchaseCardRecordId).
pub fn purchase_card_record_id(purchase_id: &str) -> String {
    format!("purchase-card:{purchase_id}")
}

/// Token record id (purchaseTokenRecordId).
pub fn purchase_token_record_id(purchase_id: &str) -> String {
    format!("purchase-token:{purchase_id}")
}

/// Wallet record id (paymentWalletRecordId).
pub fn payment_wallet_record_id(user_id: &str) -> String {
    format!("payment-wallet:{user_id}")
}

/// Cleanup job key for a reference (secretCleanupJobKey).
pub fn cleanup_job_key(reference: &str) -> String {
    crate::bdots_textfmt::slugify(&format!("secret cleanup {reference}"))
}

/// Prefix of a reference, when known.
pub fn reference_prefix(reference: &str) -> Option<&'static str> {
    PREFIXES.iter().copied().find(|prefix| reference.starts_with(prefix))
}

/// Bare id from a reference, without its prefix.
pub fn reference_id(reference: &str) -> String {
    reference_prefix(reference).map_or_else(|| reference.to_owned(), |prefix| reference[prefix.len()..].to_owned())
}

/// Whether a reference is one of the four known kinds.
pub fn is_known_reference(reference: &str) -> bool {
    reference_prefix(reference).is_some_and(|prefix| reference.len() > prefix.len())
}

/// Whether a holder column name is valid (table.column with word chars only).
pub fn valid_holder_column(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() == 2
        && parts.iter().all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
}

/// Table part of a holder column.
pub fn holder_table(name: &str) -> &str {
    name.split_once('.').map_or(name, |(table, _)| table)
}

/// Column part of a holder column.
pub fn holder_column(name: &str) -> &str {
    name.split_once('.').map_or("", |(_, column)| column)
}

/// Cleanup kind of a reference.
pub fn cleanup_kind(reference: &str) -> &'static str {
    match reference_prefix(reference) {
        Some("purchase-request:") => "request",
        Some("purchase-card:") => "card",
        Some("purchase-token:") => "token",
        Some("payment-wallet:") => "wallet",
        _ => "unknown",
    }
}

/// SF Symbol for a cleanup kind.
pub fn cleanup_icon(kind: &str) -> &'static str {
    match kind {
        "request" => "doc.text.magnifyingglass",
        "card" => "creditcard",
        "token" => "key",
        "wallet" => "wallet.pass",
        _ => "questionmark.circle",
    }
}

/// Semantic tone for a cleanup kind.
pub fn cleanup_tone(kind: &str) -> &'static str {
    if kind == "unknown" { "secondary" } else { "attention" }
}

/// Backup due time after a write started.
pub fn backup_due(started_ms: u64) -> u64 {
    started_ms + BACKUP_MS
}

/// Retry due time after a failed attempt.
pub fn retry_due(started_ms: u64) -> u64 {
    started_ms + ATTEMPT_MS
}

/// Latest safe provider call before a row is saved.
pub fn provider_deadline(saved_ms: u64) -> u64 {
    saved_ms.saturating_sub(PROVIDER_BEFORE_SAVE_MS)
}

/// Whether a write exceeded its store deadline.
pub fn write_expired(duration_ms: u64) -> bool {
    duration_ms > STORE_WRITE_MS
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(references: usize, kinds: usize, holders: usize, expired: bool) -> Vec<(&'static str, String)> {
    vec![
        ("refs", count_label(references)),
        ("kinds", count_label(kinds)),
        ("holders", count_label(holders)),
        ("expired", if expired { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_ids() {
        assert_eq!(purchase_request_record_id("op"), "purchase-request:op");
        assert_eq!(purchase_card_record_id("p1"), "purchase-card:p1");
        assert_eq!(purchase_token_record_id("p1"), "purchase-token:p1");
        assert_eq!(payment_wallet_record_id("u1"), "payment-wallet:u1");
        assert_eq!(PROVIDER_BEFORE_SAVE_MS, 150_000);
    }

    #[test]
    fn references() {
        let reference = purchase_card_record_id("p1");
        assert!(is_known_reference(&reference));
        assert_eq!(reference_prefix(&reference), Some("purchase-card:"));
        assert_eq!(reference_id(&reference), "p1");
        assert_eq!(cleanup_kind(&reference), "card");
        assert_eq!(cleanup_kind("x"), "unknown");
        assert!(!is_known_reference("purchase-card:"));
        assert!(!cleanup_job_key(&reference).is_empty());
    }

    #[test]
    fn holders_and_timing() {
        assert!(valid_holder_column("secret.ciphertext"));
        assert!(!valid_holder_column("secret.ciphertext.extra"));
        assert!(!valid_holder_column("bad-name.col"));
        assert_eq!(holder_table("secret.ciphertext"), "secret");
        assert_eq!(holder_column("secret.ciphertext"), "ciphertext");
        assert_eq!(backup_due(1000), 16_000);
        assert_eq!(retry_due(1000), 11_000);
        assert_eq!(provider_deadline(300_000), 150_000);
        assert!(write_expired(31_000));
    }

    #[test]
    fn presentation() {
        assert_eq!(cleanup_icon("token"), "key");
        assert_eq!(cleanup_tone("card"), "attention");
        assert_eq!(cleanup_tone("unknown"), "secondary");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 2, 4, false).len(), 4);
    }
}
