//! Card-facing helpers for the browser's sign-in and takeover flow (OpenBot 0.77.0).
//!
//! The decisions live in `screen::browser_cards`; these thin public functions shape them for the
//! chat and approval-card code. Nothing here reads a page or logs an address: only fixed labels and
//! the `state`/`code` of a pasted sign-in return, which the existing connector OAuth path consumes.
//! Register in `api/mod.rs` only where the parent dispatches it; this file is otherwise inert.

use crate::screen::browser_cards;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// 0.77.0: the refused-server-sign-in card keeps only one action and never opens the server's page.
/// Merge `label` and `showOpen` into the card; `action` is a fixed id, not page content.
pub fn sign_in_card_action(server_refused: bool, web: bool) -> Value {
    let action = browser_cards::sign_in_card_action(server_refused, web);
    match action {
        Some(action) => {
            json!({"action": if action == browser_cards::SignInAction::OnThisComputer { "signInOnThisComputer" } else { "signInFromDesktop" }, "label": action.label(), "showOpen": action.show_open()})
        }
        None => json!({"action": null, "showOpen": true}),
    }
}

/// 0.77.0: "Keep a browser takeover card open when the agent stops waiting for it." `expired` is
/// the card's own deadline; a wait that ended is reported as `answerAsNextMessage` instead of
/// closing the card. Callers pass the current `requestedAt`/deadline fields of the card data.
pub fn takeover_card(data: &Value, waiting: bool, now_ms: i64) -> Value {
    let request = &data["takeoverRequest"];
    let expired = request["expiresAt"].as_i64().is_some_and(|at| at <= now_ms);
    let answered = matches!(request["status"].as_str(), Some("answered" | "resolved" | "cancelled"));
    let life = browser_cards::card_stays_open(waiting, expired, answered);
    let mut next = data.clone();
    next["takeoverRequest"]["status"] = json!(if life.stays_open() { "waiting" } else { "expired" });
    next["takeoverRequest"]["answerAsNextMessage"] = json!(life == browser_cards::CardLife::AnswerAsNextMessage);
    next
}

/// Parses the address a phone or another computer's browser shows at the end of a sign-in, so the
/// existing `connectorSignInFinish` path can finish it (0.77.0: "paste the address ... and OpenBot
/// finishes the sign-in"). Accepts http(s) and the app callback scheme, with the result in the query
/// or the fragment. Bound the input; return only `state`, `code` and `error`, never the whole URL.
pub fn finish_sign_in_address(url: &str) -> Result<Value> {
    let raw = url.trim();
    ensure!(
        !raw.is_empty() && raw.len() <= browser_cards::MAX_PASTED_URL,
        "Paste the address the other browser showed."
    );
    let parsed = url::Url::parse(raw).map_err(|_| anyhow::anyhow!("That is not a web address."))?;
    ensure!(
        matches!(parsed.scheme(), "https" | "http" | "codync"),
        "Paste the http or app address the other browser showed."
    );
    let mut state = String::new();
    let mut code = None;
    let mut error = None;
    let mut read = |key_value: (std::borrow::Cow<'_, str>, std::borrow::Cow<'_, str>)| match key_value.0.as_ref() {
        "state" if state.is_empty() => state = key_value.1.into_owned(),
        "code" if code.is_none() => code = Some(key_value.1.into_owned()),
        "error" | "error_description" if error.is_none() => error = Some(key_value.1.into_owned()),
        _ => {}
    };
    if let Some(query) = parsed.query() {
        for pair in url::form_urlencoded::parse(query.as_bytes()) {
            read(pair);
        }
    }
    if let Some(fragment) = parsed.fragment().filter(|fragment| fragment.contains('=')) {
        for pair in url::form_urlencoded::parse(fragment.as_bytes()) {
            read(pair);
        }
    }
    ensure!(!state.is_empty(), "That address has no sign-in state. Start the sign-in again.");
    ensure!(
        code.is_some() || error.is_some(),
        "That address has no sign-in result. Open it at the end of the sign-in."
    );
    Ok(json!({"state": state, "code": code, "error": error}))
}

/// The words for the tab's current sign-in state, so the desk never composes card copy itself.
pub fn browser_notice(url: &str, active: bool) -> Value {
    json!({"text": browser_cards::sign_in_notice(url, active).text()})
}

/// Whether a tool call that just timed out should report a stalled page. Kept public so a new call
/// site can share the bounded decision without reaching into `screen`.
pub fn page_stalled(last_response: Instant, window: Duration) -> bool {
    browser_cards::browser_page_stalled(last_response, Instant::now(), window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_sign_in_card_has_one_action_and_no_open() {
        let desktop = sign_in_card_action(true, false);
        assert_eq!(desktop["label"], browser_cards::SIGN_IN_ON_THIS_COMPUTER);
        assert_eq!(desktop["showOpen"], false);
        assert_eq!(desktop["action"], "signInOnThisComputer");
        let web = sign_in_card_action(true, true);
        assert_eq!(web["label"], browser_cards::SIGN_IN_FROM_DESKTOP);
        assert_eq!(web["showOpen"], false);
        assert_eq!(sign_in_card_action(false, false)["showOpen"], true);
    }

    #[test]
    fn a_takeover_card_stays_answerable_after_the_agent_stops_waiting() {
        let card = json!({"takeoverRequest": {"status": "waiting", "expiresAt": 10_000}});
        let next = takeover_card(&card, false, 5_000);
        assert_eq!(next["takeoverRequest"]["status"], "waiting");
        assert_eq!(next["takeoverRequest"]["answerAsNextMessage"], true);
        let still = takeover_card(&card, true, 5_000);
        assert_eq!(still["takeoverRequest"]["answerAsNextMessage"], false);
        assert_eq!(takeover_card(&card, false, 10_000)["takeoverRequest"]["status"], "expired");
        assert_eq!(takeover_card(&card, false, 20_000)["takeoverRequest"]["status"], "expired");
    }

    #[test]
    fn a_pasted_address_finishes_the_sign_in_without_keeping_the_url() {
        let value = finish_sign_in_address("https://other.example/callback?state=abc&code=xyz").expect("parsed");
        assert_eq!(value["state"], "abc");
        assert_eq!(value["code"], "xyz");
        assert!(value["error"].is_null());
        let fragment = finish_sign_in_address("codync://oauth#state=abc&code=xyz").expect("fragment");
        assert_eq!(fragment["code"], "xyz");
        let failed = finish_sign_in_address("https://other.example/callback?state=abc&error=denied").expect("failed");
        assert_eq!(failed["error"], "denied");
        assert!(failed["code"].is_null());
    }

    #[test]
    fn a_bad_pasted_address_is_refused_plainly() {
        assert!(finish_sign_in_address("").is_err());
        assert!(finish_sign_in_address("not an address").is_err());
        assert!(finish_sign_in_address("javascript:alert(1)").is_err());
        assert!(finish_sign_in_address("https://other.example/callback?code=xyz").is_err(), "no state");
        assert!(finish_sign_in_address("https://other.example/callback?state=abc").is_err(), "no result");
        assert!(finish_sign_in_address(&format!("https://x.example/{}", "a".repeat(3_000))).is_err());
    }

    #[test]
    fn the_notice_and_stall_decisions_are_reachable_from_the_api() {
        assert_eq!(
            browser_notice("https://site.example/signin/callback", false)["text"],
            browser_cards::START_NEW_SIGN_IN
        );
        assert_eq!(browser_notice("https://site.example/inbox", false)["text"], "");
        assert!(!page_stalled(Instant::now(), browser_cards::STALL_AFTER));
        assert!(page_stalled(Instant::now() - browser_cards::STALL_AFTER * 2, browser_cards::STALL_AFTER));
    }
}
