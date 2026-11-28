#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from hermes-agent's `use-prompt-actions/utils.ts`: session ids, gateway error
//! classification, submit guards and image/receipt helpers. Composes helpers already ported into
//! this host. Only `botInsights` calls this.

use serde_json::Value;

/// Bounded window for retrying a busy session (SESSION_BUSY_RETRY_TIMEOUT_MS).
pub const SESSION_BUSY_RETRY_TIMEOUT_MS: u64 = 6_000;

/// Poll interval while a session is busy (SESSION_BUSY_RETRY_INTERVAL_MS).
pub const SESSION_BUSY_RETRY_INTERVAL_MS: u64 = 150;

/// Gateway code for a refused deep truncate (GATEWAY_DEEP_TRUNCATE_REFUSED_CODE).
pub const GATEWAY_DEEP_TRUNCATE_REFUSED_CODE: u64 = 4_033;

/// Cooldown after an interrupt (RECENT_INTERRUPT_COOLDOWN_MS).
pub const RECENT_INTERRUPT_COOLDOWN_MS: u64 = 3_000;

/// TTL of a submit-in-flight guard (SUBMIT_IN_FLIGHT_TTL_MS).
pub const SUBMIT_IN_FLIGHT_TTL_MS: u64 = 30_000;

/// Gateway code for a session owned by another surface.
pub const SESSION_NOT_OWNED_CODE: u64 = 4_090;

/// Message of an error value, or an empty string.
fn error_message(error: &Value) -> &str {
    error.as_str().unwrap_or_else(|| error["message"].as_str().unwrap_or(""))
}

/// Reason stamped in a gateway error's data.
fn gateway_reason(error: &Value) -> &str {
    error["data"]["reason"].as_str().unwrap_or("")
}

/// Field stamped in a gateway error's data.
fn gateway_field(error: &Value) -> &str {
    error["data"]["field"].as_str().unwrap_or("")
}

/// Whether a stored session id has one of the known shapes (isSessionIdCandidate).
pub fn is_session_id_candidate(value: &str) -> bool {
    let text = value.trim();
    let chars: Vec<char> = text.chars().collect();
    let hex = |slice: &[char]| slice.iter().all(|c| c.is_ascii_hexdigit());
    if chars.len() == 32 {
        return hex(&chars);
    }
    chars.len() == 21
        && chars[8] == '_'
        && chars[15] == '_'
        && chars[..8].iter().all(|c| c.is_ascii_digit())
        && hex(&chars[9..15])
        && hex(&chars[16..])
}

/// Clean the message of a wrapped remote error (inlineErrorMessage).
pub fn inline_error_message(error: &Value, fallback: &str) -> String {
    let raw = if error.is_string() || error["message"].is_string() { error_message(error) } else { fallback };
    let stripped = raw.split("': Error: ").last().unwrap_or(raw).strip_prefix("Error: ").unwrap_or(raw);
    stripped.trim().to_owned()
}

/// Whether an error is about provider setup (isProviderSetupError).
pub fn is_provider_setup_error(error: &Value) -> bool {
    let lower = error_message(error).to_ascii_lowercase();
    lower.contains("provider") && (lower.contains("setup") || lower.contains("configur") || lower.contains("api key"))
}

/// Whether the session no longer exists (isSessionNotFoundError).
pub fn is_session_not_found_error(error: &Value) -> bool {
    error_message(error).to_ascii_lowercase().contains("session not found")
}

/// Whether the gateway call timed out (isGatewayTimeoutError).
pub fn is_gateway_timeout_error(error: &Value) -> bool {
    error_message(error).to_ascii_lowercase().contains("request timed out")
}

/// Whether the gateway refused a deep truncate (isDeepTruncateRefusal).
pub fn is_deep_truncate_refusal(error: &Value) -> bool {
    error["code"].as_u64() == Some(GATEWAY_DEEP_TRUNCATE_REFUSED_CODE)
}

/// Whether a turn is already running (isSessionBusyError).
pub fn is_session_busy_error(error: &Value) -> bool {
    let lower = error_message(error).to_ascii_lowercase();
    lower.contains("session busy") || lower.contains("session_busy") || gateway_reason(error) == "session_busy"
}

/// Whether the compression lock is held by another writer (isCompressionBusyError).
pub fn is_compression_busy_error(error: &Value) -> bool {
    gateway_reason(error) == "session_busy" && gateway_field(error) == "compression"
}

/// Whether a submission id was already admitted (isAdmittedSubmissionConflict).
pub fn is_admitted_submission_conflict(error: &Value) -> bool {
    gateway_reason(error) == "admission_conflict" && gateway_field(error) == "request_id"
}

/// Whether another surface holds the session (isSessionNotOwnedError).
pub fn is_session_not_owned_error(error: &Value) -> bool {
    gateway_reason(error) == "SESSION_NOT_OWNED"
        || error_message(error).to_ascii_lowercase().contains("already has a live owner")
}

/// Whether the owning gateway is mid-turn (isSessionOwnerBusyError).
pub fn is_session_owner_busy_error(error: &Value) -> bool {
    gateway_reason(error) == "SESSION_OWNER_BUSY"
}

/// Whether a refusal unstaged the images (refusalDroppedImages).
pub fn refusal_dropped_images(error: &Value) -> bool {
    let lower = error_message(error).to_ascii_lowercase();
    is_session_owner_busy_error(error) || (lower.contains("image") && lower.contains("removed"))
}

/// Error card surface of an ownership refusal (ownershipErrorSurface).
pub fn ownership_error_surface(error: &Value) -> Value {
    let code = if is_session_not_owned_error(error) {
        Some("SESSION_NOT_OWNED")
    } else {
        match gateway_reason(error) {
            "session_read_only" => Some("session_read_only"),
            "SESSION_OWNER_UNCONFIRMED" => Some("SESSION_OWNER_UNCONFIRMED"),
            _ => None,
        }
    };
    code.map_or(Value::Null, |code| serde_json::json!({"layer": "gateway", "code": code, "retryable": false}))
}

/// Whether a legacy accepted send was actually forwarded (isForwardedReceipt).
pub fn is_forwarded_receipt(legacy_accepted: bool, result: &Value) -> bool {
    legacy_accepted && result["forwarded"] == true && result["status"].as_str() == Some("queued")
}

/// Whether the voice chat was stopped by a typed phrase (isVoiceStoppedReceipt).
pub fn is_voice_stopped_receipt(result: &Value) -> bool {
    result["voice_stopped"] == true
}

/// Whether a target session, or the foreground, is busy (isTargetSessionBusy).
pub fn is_target_session_busy(states: &Value, session_id: Option<&str>, foreground_busy: bool) -> bool {
    match session_id.filter(|id| !id.is_empty()) {
        None => foreground_busy,
        Some(id) => states[id]["busy"] == true,
    }
}

/// Whether an interrupt must precede a rewind (shouldInterruptBeforeRewind).
pub fn should_interrupt_before_rewind(busy: bool, recently_interrupted: bool) -> bool {
    busy || recently_interrupted
}

/// Whether a submit is still in flight, expiring stale guards (isSubmitInFlight).
pub fn is_submit_in_flight(acquired_at_ms: Option<u64>, now_ms: u64) -> bool {
    acquired_at_ms.is_some_and(|at| now_ms.saturating_sub(at) < SUBMIT_IN_FLIGHT_TTL_MS)
}

/// Base64 payload of a data URL (base64FromDataUrl).
pub fn base64_from_data_url(data_url: &str) -> String {
    data_url.split_once(',').map_or(String::new(), |(_, payload)| payload.to_owned())
}

/// Filename of an attachment path (imageFilenameFromPath).
pub fn image_filename_from_path(path: &str) -> String {
    path.split(|c| c == '/' || c == '\\').filter(|part| !part.is_empty()).last().unwrap_or("image.png").to_owned()
}

/// Status text of a slash command (slashStatusText).
pub fn slash_status_text(command: &str, output: &str) -> String {
    format!("slash:{command}\n{}", output.trim())
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(errors: usize, retryable: usize, busy: bool, forwarded: bool) -> Vec<(&'static str, String)> {
    vec![
        ("errors", count_label(errors)),
        ("retryable", count_label(retryable)),
        ("busy", if busy { "yes" } else { "no" }.to_owned()),
        ("forwarded", if forwarded { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn session_ids_and_messages() {
        assert!(is_session_id_candidate("20261010_143012_a1b2c3"));
        assert!(is_session_id_candidate("0123456789abcdef0123456789abcdef"));
        assert!(!is_session_id_candidate("short"));
        let wrapped = json!({"message":"Error invoking remote method 'x': Error: Session not found"});
        assert_eq!(inline_error_message(&wrapped, "fallback"), "Session not found");
        assert_eq!(inline_error_message(&json!({}), "fallback"), "fallback");
        assert!(is_provider_setup_error(&json!({"message":"Provider setup required"})));
        assert!(is_session_not_found_error(&json!({"message":"session not found"})));
        assert!(is_gateway_timeout_error(&json!({"message":"request timed out: prompt.submit"})));
        assert!(is_deep_truncate_refusal(&json!({"code": 4033})));
    }

    #[test]
    fn busy_and_ownership() {
        assert!(is_session_busy_error(&json!({"message":"session_busy"})));
        assert!(is_compression_busy_error(&json!({"data":{"reason":"session_busy","field":"compression"}})));
        assert!(is_admitted_submission_conflict(&json!({"data":{"reason":"admission_conflict","field":"request_id"}})));
        assert!(is_session_not_owned_error(&json!({"data":{"reason":"SESSION_NOT_OWNED"}})));
        assert!(is_session_owner_busy_error(&json!({"data":{"reason":"SESSION_OWNER_BUSY"}})));
        assert!(refusal_dropped_images(&json!({"message":"staged images were removed"})));
        let surface = ownership_error_surface(&json!({"data":{"reason":"SESSION_NOT_OWNED"}}));
        assert_eq!(surface["code"], "SESSION_NOT_OWNED");
        assert_eq!(surface["retryable"], false);
        assert_eq!(ownership_error_surface(&json!({})), Value::Null);
        assert_eq!(SESSION_NOT_OWNED_CODE, 4090);
    }

    #[test]
    fn receipts_and_guards() {
        assert!(is_forwarded_receipt(true, &json!({"forwarded":true,"status":"queued"})));
        assert!(!is_forwarded_receipt(false, &json!({"forwarded":true,"status":"queued"})));
        assert!(is_voice_stopped_receipt(&json!({"voice_stopped":true})));
        let states = json!({"s1":{"busy":true}});
        assert!(is_target_session_busy(&states, Some("s1"), false));
        assert!(!is_target_session_busy(&states, Some("s2"), false));
        assert!(is_target_session_busy(&states, None, true));
        assert!(should_interrupt_before_rewind(false, true));
        assert!(is_submit_in_flight(Some(1000), 2000));
        assert!(!is_submit_in_flight(Some(1000), 31_000));
    }

    #[test]
    fn attachments_and_summary() {
        assert_eq!(base64_from_data_url("data:image/png;base64,AAAA"), "AAAA");
        assert_eq!(base64_from_data_url("nope"), "");
        assert_eq!(image_filename_from_path("C:\\tmp\\shot.png"), "shot.png");
        assert_eq!(image_filename_from_path(""), "image.png");
        assert_eq!(slash_status_text("help", "  done  "), "slash:help\ndone");
        assert_eq!(SESSION_BUSY_RETRY_TIMEOUT_MS, 6000);
        assert_eq!(RECENT_INTERRUPT_COOLDOWN_MS, 3000);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(3, 1, true, false).len(), 4);
    }
}
