//! Pure decisions behind the browser's sign-in and takeover cards (OpenBot 0.77.0).
//!
//! A card's words are fixed labels here: the host decides *what* to say, the apps only render it.
//! An address is read to classify a page, never logged or sent to analytics. The time bounds are
//! short so a page that stops answering, or a wait that ended, never leaves a card looking alive.

// The Windows browser panel and the card API (registered by the parent) call into these; other
// hosts keep the decisions and their tests without a browser to drive.
#![allow(dead_code, reason = "the browser panel and the card API are the only callers, off Windows neither runs")]

use std::time::{Duration, Instant};

/// 0.77.0: a sign-in page that is no longer active says to go back and start again, not that no
/// sign-in is waiting.
pub(crate) const START_NEW_SIGN_IN: &str =
    "This sign-in is no longer active. Go back to Codync and start a new sign-in.";
/// 0.77.0: a page that stops answering names a never-ending script and the fix, in plain words.
pub(crate) const PAGE_NOT_ANSWERING: &str =
    "This page is not answering; a script may still be running. Reload it to continue.";
/// 0.77.0: a sign-in window whose page cannot load is closed, with a check-the-connection message.
pub(crate) const SIGN_IN_PAGE_UNAVAILABLE: &str =
    "The sign-in page could not load. Check the internet connection and open the sign-in window again.";
/// 0.77.0: when a site refuses a server sign-in, the desktop card has one action.
pub(crate) const SIGN_IN_ON_THIS_COMPUTER: &str = "Sign in on this computer";
/// 0.77.0: the web card drops Open and points to the desktop app instead.
pub(crate) const SIGN_IN_FROM_DESKTOP: &str = "Sign in from the desktop app";

/// How long a tab may be silent before a tool call treats it as stalled. Below the 30 s a chat
/// tool call has, so the agent hears about it in this turn (0.77.0: "a page that stops answering").
pub(crate) const STALL_AFTER: Duration = Duration::from_secs(20);
/// Upper bound for a pasted sign-in address, so a stray paste cannot grow a card or a query.
pub(crate) const MAX_PASTED_URL: usize = 2048;
/// OpenBot nightly: how long a saved-password submit waits for a slow site to reach the signed-in page. Well
/// under a tool call's own deadline, and only a page that is still a login form waits at all.
pub(crate) const SIGN_IN_GRACE: Duration = Duration::from_secs(3);

/// The notice a returned sign-in page shows. A fixed label is returned so callers can count the
/// outcome without carrying the address into analytics.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SignInNotice {
    None,
    Waiting,
    StartAgain,
}

impl SignInNotice {
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Waiting => "Waiting for the sign-in to finish in the browser.",
            Self::StartAgain => START_NEW_SIGN_IN,
        }
    }
}

/// Whether an address is a sign-in page or its return: only the scheme and path are read, never a
/// token in the query or fragment.
pub(crate) fn is_sign_in_page(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_PASTED_URL {
        return false;
    }
    let path = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = path.split(['?', '#']).next().unwrap_or(path).to_ascii_lowercase();
    ["signin", "sign-in", "login", "oauth", "auth", "callback"].iter().any(|needle| path.contains(needle))
}

/// 0.77.0: "When the browser comes back from a sign-in page that is no longer active, the page now
/// tells you to go back to OpenBot and start the sign-in again."
pub(crate) fn sign_in_notice(url: &str, active: bool) -> SignInNotice {
    if !is_sign_in_page(url) {
        return SignInNotice::None;
    }
    if active { SignInNotice::Waiting } else { SignInNotice::StartAgain }
}

/// What a saved-password submit does after the Enter press (OpenBot nightly: a slow site must not hand the
/// browser to the user when the sign-in already finished).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SignInProgress {
    /// The site moved on, or the form is gone: the agent reads the signed-in page and continues.
    Continue,
    /// Still the same login form, but the grace period has not run out: keep polling.
    Waiting,
    /// No navigation and still the login form after the bound: keep the existing handoff path.
    Handoff,
}

/// `url_before` is the address at submit time, `url_after` the address now; `still_on_form` comes
/// from the page (a visible password field) and the address. Addresses are only compared and
/// classified here, never logged or sent anywhere. This never touches a password or payment
/// approval card: those are a separate, explicit handoff.
pub(crate) fn sign_in_complete(
    url_before: &str,
    url_after: &str,
    still_on_form: bool,
    waited: Duration,
) -> SignInProgress {
    if !still_on_form || url_after != url_before {
        SignInProgress::Continue
    } else if waited >= SIGN_IN_GRACE {
        SignInProgress::Handoff
    } else {
        SignInProgress::Waiting
    }
}

/// What a takeover or ask-help card does once the agent's wait has ended (0.77.0: "Keep a browser
/// takeover card open when the agent stops waiting for it", and the question-card counterpart:
/// "the agent gets your answer as your next message").
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CardLife {
    /// The card's own deadline passed or it was answered: no longer answerable.
    Close,
    /// The agent still waits on this card; answer it as before.
    KeepOpen,
    /// The agent stopped waiting; keep the card, and forward the answer as the next message.
    AnswerAsNextMessage,
}

impl CardLife {
    pub(crate) fn stays_open(self) -> bool {
        self != Self::Close
    }
}

/// `expired` is the card's own deadline (for example a routine's), not the agent's wait: a wait
/// that ended is exactly when the card must stay open and answerable.
pub(crate) fn card_stays_open(waiting: bool, expired: bool, answered: bool) -> CardLife {
    if expired || answered {
        CardLife::Close
    } else if waiting {
        CardLife::KeepOpen
    } else {
        CardLife::AnswerAsNextMessage
    }
}

/// A page is stalled only after a whole silent window. `saturating_duration_since` keeps a clock
/// that moved backwards from reporting a stall (0.77.0: "a page ... that stops answering").
pub(crate) fn browser_page_stalled(last_activity: Instant, now: Instant, window: Duration) -> bool {
    now.saturating_duration_since(last_activity) >= window
}

/// The single action a refused-server-sign-in card offers. In a browser there is no Open action,
/// so the card tells the person to use the desktop app (0.77.0: "the sign-in card no longer opens
/// the server's page").
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SignInAction {
    OnThisComputer,
    FromDesktop,
}

impl SignInAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OnThisComputer => SIGN_IN_ON_THIS_COMPUTER,
            Self::FromDesktop => SIGN_IN_FROM_DESKTOP,
        }
    }

    /// Never open a remote browser preview from a card that a site refused.
    pub(crate) fn show_open(self) -> bool {
        false
    }
}

pub(crate) fn sign_in_card_action(server_refused: bool, web: bool) -> Option<SignInAction> {
    if !server_refused {
        return None;
    }
    Some(if web { SignInAction::FromDesktop } else { SignInAction::OnThisComputer })
}

/// Whether a sign-in window's failed page should close and report the connection message.
pub(crate) fn unavailable_sign_in_page(load_failed: bool) -> Option<&'static str> {
    load_failed.then_some(SIGN_IN_PAGE_UNAVAILABLE)
}

/// 0.77.0: "A page that an agent shows in a long chat now keeps what you did on it." A page whose
/// saved tab is still open is activated, not loaded again, so its clicks and selections survive.
pub(crate) fn page_reuses_tab<'a>(saved: Option<&str>, tabs: impl Iterator<Item = &'a str>) -> bool {
    saved.is_some_and(|saved| tabs.any(|tab| tab == saved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_returned_inactive_sign_in_says_to_start_again() {
        assert_eq!(sign_in_notice("https://site.example/signin/callback", false), SignInNotice::StartAgain);
        assert_eq!(sign_in_notice("https://site.example/signin/callback", false).text(), START_NEW_SIGN_IN);
        assert_eq!(sign_in_notice("https://site.example/login?code=secret", true), SignInNotice::Waiting);
        assert_eq!(sign_in_notice("https://site.example/inbox", false), SignInNotice::None);
        assert_eq!(sign_in_notice("", false), SignInNotice::None);
        assert_eq!(
            sign_in_notice(&format!("https://x.example/oauth/{}", "a".repeat(MAX_PASTED_URL)), false),
            SignInNotice::None
        );
    }

    #[test]
    fn a_slow_sign_in_continues_once_the_site_moves() {
        let login = "https://site.example/login";
        let signed_in = "https://site.example/inbox";
        assert_eq!(sign_in_complete(login, signed_in, true, Duration::ZERO), SignInProgress::Continue);
        assert_eq!(sign_in_complete(login, signed_in, false, Duration::ZERO), SignInProgress::Continue);
        assert_eq!(sign_in_complete(login, login, false, Duration::ZERO), SignInProgress::Continue, "in-page sign-in");
    }

    #[test]
    fn a_stuck_sign_in_waits_only_for_the_bounded_grace() {
        let login = "https://site.example/login";
        assert_eq!(
            sign_in_complete(login, login, true, SIGN_IN_GRACE - Duration::from_millis(1)),
            SignInProgress::Waiting
        );
        assert_eq!(sign_in_complete(login, login, true, SIGN_IN_GRACE), SignInProgress::Handoff);
        assert_eq!(sign_in_complete(login, login, true, SIGN_IN_GRACE * 2), SignInProgress::Handoff);
        assert!(SIGN_IN_GRACE < Duration::from_secs(20), "the wait stays inside one browser tool call");
    }

    #[test]
    fn an_answered_or_lapsed_card_closes_but_a_stopped_wait_stays_answerable() {
        assert_eq!(card_stays_open(true, false, false), CardLife::KeepOpen);
        assert_eq!(card_stays_open(false, false, false), CardLife::AnswerAsNextMessage);
        assert!(card_stays_open(false, false, false).stays_open(), "the answer becomes the next message");
        assert_eq!(card_stays_open(false, false, true), CardLife::Close);
        assert_eq!(card_stays_open(false, true, false), CardLife::Close, "the card's own deadline still ends it");
        assert!(!CardLife::Close.stays_open());
    }

    #[test]
    fn a_page_is_stalled_only_after_the_whole_window() {
        let now = Instant::now();
        assert!(!browser_page_stalled(now - STALL_AFTER + Duration::from_secs(1), now, STALL_AFTER));
        assert!(browser_page_stalled(now - STALL_AFTER, now, STALL_AFTER));
        assert!(browser_page_stalled(now - STALL_AFTER * 2, now, STALL_AFTER));
        assert!(
            !browser_page_stalled(now, now - Duration::from_secs(1), STALL_AFTER),
            "a backwards clock is not a stall"
        );
        assert!(STALL_AFTER < Duration::from_secs(30), "the agent hears about it within a chat tool call");
    }

    #[test]
    fn a_refused_sign_in_has_one_action_and_never_opens_the_server_page() {
        assert_eq!(sign_in_card_action(false, false), None, "a normal card keeps its own actions");
        let desktop = sign_in_card_action(true, false).expect("refused on desktop");
        assert_eq!(desktop.label(), SIGN_IN_ON_THIS_COMPUTER);
        assert!(!desktop.show_open());
        let web = sign_in_card_action(true, true).expect("refused in a browser");
        assert_eq!(web.label(), SIGN_IN_FROM_DESKTOP);
        assert!(!web.show_open());
    }

    #[test]
    fn a_failed_sign_in_page_reports_the_connection_message() {
        assert_eq!(unavailable_sign_in_page(false), None);
        assert_eq!(unavailable_sign_in_page(true), Some(SIGN_IN_PAGE_UNAVAILABLE));
    }

    #[test]
    fn a_page_keeps_its_tab_when_the_page_is_still_open() {
        let tabs = ["a", "b"];
        assert!(page_reuses_tab(Some("a"), tabs.iter().copied()));
        assert!(!page_reuses_tab(Some("gone"), tabs.iter().copied()));
        assert!(!page_reuses_tab(None, tabs.iter().copied()));
    }
}
