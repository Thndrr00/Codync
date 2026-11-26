//! Whether each harness is signed in, as its own CLI tells it.

use super::{HARNESSES, Harness, on_path, which};
use crate::LockExt;
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;

/// `CODYNC_AGENT_CHECKS=off`, the tests' disposable hosts: nothing runs the computer's agent CLIs to check on their
/// sign-in or usage, so a test never starts one on the user's own login.
pub fn agent_checks_off() -> bool {
    std::env::var("CODYNC_AGENT_CHECKS").is_ok_and(|value| value == "off")
}

/// `<bin> <args>`, judged by exit code and stdout.
#[derive(Clone, Copy)]
pub struct SignInCheck {
    pub args: &'static str,
    pub ok: fn(bool, &str) -> bool,
}

pub(super) fn exit_ok(success: bool, _: &str) -> bool {
    success
}

pub(super) fn claude_signed_in(_: bool, out: &str) -> bool {
    serde_json::from_str::<Value>(out).is_ok_and(|v| v["loggedIn"] == true)
}

pub(super) fn qoder_signed_in(_: bool, out: &str) -> bool {
    serde_json::from_str::<Value>(out).is_ok_and(|v| v["logged_in"] == true)
}

pub(super) fn says_logged_in(_: bool, out: &str) -> bool {
    let out = out.to_lowercase();
    out.contains("logged in") && !out.contains("not logged in")
}

/// The sign-in page a CLI printed, when it is a plain https link on the provider's own domain.
/// OpenBot nightly `triage-15344`: on a Mac Claude's CLI can fail to open its page and then
/// only print the link, so the host opens it from the app, the way Codex's page is opened.
/// A link carrying a code, token or secret is never opened: this is not an OAuth client.
pub fn sign_in_page_available(backend: &str, output: &str) -> Option<String> {
    let domains: &[&str] = match backend {
        "claude" => &["claude.ai", "anthropic.com"],
        "codex" => &["chatgpt.com", "openai.com"],
        _ => return None,
    };
    output
        .split_whitespace()
        .filter_map(|word| {
            reqwest::Url::parse(word.trim_end_matches(|c: char| matches!(c, '.' | ',' | ')' | ']'))).ok()
        })
        .find(|url| {
            url.scheme() == "https"
                && domains.iter().any(|domain| on_domain(url.host_str(), domain))
                && !carries_secret(url)
        })
        .map(|url| url.into())
}

fn on_domain(host: Option<&str>, domain: &str) -> bool {
    host.is_some_and(|host| {
        host.eq_ignore_ascii_case(domain) || host.to_ascii_lowercase().ends_with(&format!(".{domain}"))
    })
}

fn carries_secret(url: &reqwest::Url) -> bool {
    url.query_pairs().any(|(name, _)| secret_name(&name)) || url.fragment().is_some_and(secret_name)
}

fn secret_name(name: &str) -> bool {
    const SECRET: &[&str] = &["token", "code", "secret", "key", "sig", "signature", "password", "session"];
    let name = name.to_ascii_lowercase().replace('_', "");
    SECRET.iter().any(|word| name.contains(word))
}

/// Last sign-in check per harness id; missing = can't tell.
static SIGNED_IN: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

pub fn signed_in(id: &str) -> Option<bool> {
    SIGNED_IN.locked().iter().find(|(h, _)| h == id).map(|(_, ok)| *ok)
}

/// A prompt was refused because the agent asked for auth. Only an agent whose new session does
/// not prove sign-in (`WorkBuddy`) can still report an account after that. Other agents' own
/// checks stand, so a setup terminal or saved keys can sign them in.
static PROMPT_REFUSED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The latest prompt for `id` was refused for auth. Stands until a prompt succeeds or the user signs in again.
/// An agent whose session proves sign-in is not recorded: its next check is the proof.
pub fn note_prompt_refused(id: &str) {
    if super::workbuddy::session_proves_sign_in(id) {
        return;
    }
    let mut refused = PROMPT_REFUSED.locked();
    if !refused.iter().any(|known| known == id) {
        refused.push(id.to_owned());
    }
}

pub fn clear_prompt_refused(id: &str) {
    PROMPT_REFUSED.locked().retain(|known| known != id);
}

/// An account check that says signed in loses to a prompt that was just refused for auth.
/// Other agents are unchanged: their check is what a finished terminal or saved keys report.
pub fn apply_prompt_refusal(id: &str, probe: Option<bool>) -> Option<bool> {
    if super::workbuddy::session_proves_sign_in(id) {
        return probe;
    }
    if probe == Some(true) && PROMPT_REFUSED.locked().iter().any(|known| known == id) { Some(false) } else { probe }
}

/// Records what a check found (`None`: can't tell).
pub fn set_signed_in(id: &str, state: Option<bool>) {
    let mut map = SIGNED_IN.locked();
    map.retain(|(h, _)| h != id);
    if let Some(ok) = state {
        map.push((id.to_owned(), ok));
    }
}

/// What the list shows when the agent's own CLI check applies: the live evidence (its report,
/// a session or a CLI check, all re-run on the last refresh) first, else its credential files —
/// recorded so the rest of the host agrees.
pub fn merged(id: &str, from_files: Option<bool>) -> Option<bool> {
    match signed_in(id) {
        Some(live) => Some(live),
        None => from_files.inspect(|&files| set_signed_in(id, Some(files))),
    }
}

/// What the list shows when the agent has no CLI check of its own: its credential files, read
/// just now, outrank a probe that is only as fresh as the last refresh; a probe result stands
/// when the files say nothing.
pub fn from_files_first(id: &str, from_files: Option<bool>) -> Option<bool> {
    match from_files {
        Some(files) => {
            set_signed_in(id, Some(files));
            Some(files)
        }
        None => signed_in(id),
    }
}

/// Re-checks every installed harness that can report its sign-in state.
pub async fn refresh_sign_in() {
    let installed = HARNESSES.iter().filter(|h| h.signed_in.is_some() && h.bins.iter().any(|b| on_path(b)));
    let results =
        futures::future::join_all(installed.map(|h| async move { (h.id, cli_signed_in(h, &[]).await) })).await;
    for (id, ok) in results {
        // A Mistral chat that was signed out when it opened cannot see a GitHub connection made
        // since; reopen it once rather than failing every message (OpenBot 0.70).
        if id == "vibe" && ok == Some(true) && signed_in(id) == Some(false) {
            super::mistral::note(super::mistral::ChatState { backend: id, connected: true, chat_open: true });
        }
        set_signed_in(id, ok);
    }
}

/// What `h`'s own CLI says about its sign-in, run with `env` (the keys its agent gets, so both
/// judge the same login). `None`: it has no such check, isn't installed or didn't answer.
pub async fn cli_signed_in(h: &Harness, env: &[(String, String)]) -> Option<bool> {
    if agent_checks_off() {
        return None;
    }
    let check = h.signed_in?;
    let bin = h.bins.iter().find_map(|b| which(b))?;
    let started = std::time::Instant::now();
    let run = tokio::process::Command::new(bin)
        .args(check.args.split_whitespace())
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let budget = if h.id == "claude" { super::claude::FALLBACK_AFTER } else { Duration::from_secs(15) };
    let (judged, answered) = match tokio::time::timeout(budget, run).await {
        Ok(Ok(out)) => {
            let text = String::from_utf8_lossy(&out.stdout);
            let answered = out.status.success() && !text.trim().is_empty();
            (Some((check.ok)(out.status.success(), &text)), answered)
        }
        Ok(Err(e)) => {
            tracing::info!(backend = h.id, error = %e, "sign-in check failed");
            (None, false)
        }
        Err(_) => (None, false),
    };
    let elapsed = started.elapsed();
    // A silent user copy is not a signed-out account: Claude's bundled copy answers within 20 s.
    if h.id == "claude"
        && super::claude::copy_fallback(elapsed, answered)
        && let Some(bundled) = super::claude::bundled()
    {
        if let Some(ok) = check_with(&bundled, &check, env, super::claude::BUNDLED_BUDGET).await {
            return Some(ok);
        }
    }
    judged
}

/// One bounded sign-in check against `bin`, used for the bundled copy when the user's was silent.
async fn check_with(
    bin: &std::path::Path,
    check: &SignInCheck,
    env: &[(String, String)],
    timeout: Duration,
) -> Option<bool> {
    let run = tokio::process::Command::new(bin)
        .args(check.args.split_whitespace())
        .envs(env.iter().map(|(k, v)| (k, v)))
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(timeout, run).await {
        Ok(Ok(out)) => Some((check.ok)(out.status.success(), &String::from_utf8_lossy(&out.stdout))),
        Ok(Err(e)) => {
            tracing::info!(error = %e, "bundled sign-in check failed");
            None
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_prompt_outranks_an_account_that_still_looks_signed_in() {
        let id = "workbuddy";
        clear_prompt_refused(id);
        assert_eq!(apply_prompt_refusal(id, Some(true)), Some(true));
        note_prompt_refused(id);
        assert_eq!(apply_prompt_refusal(id, Some(true)), Some(false), "an account object is not a working login");
        assert_eq!(apply_prompt_refusal(id, Some(false)), Some(false));
        assert_eq!(apply_prompt_refusal(id, None), None);
        clear_prompt_refused(id);
        assert_eq!(apply_prompt_refusal(id, Some(true)), Some(true));
    }

    #[test]
    fn only_a_plain_sign_in_page_from_the_provider_is_opened() {
        let out = "Opening https://claude.ai/login?state=abc in your browser. Try https://evil.test/signin";
        assert_eq!(sign_in_page_available("claude", out).as_deref(), Some("https://claude.ai/login?state=abc"));
        assert_eq!(
            sign_in_page_available("claude", "Open https://console.anthropic.com/oauth/authorize, then continue")
                .as_deref(),
            Some("https://console.anthropic.com/oauth/authorize")
        );
        assert_eq!(
            sign_in_page_available("codex", "Visit https://chatgpt.com/codex/login").as_deref(),
            Some("https://chatgpt.com/codex/login")
        );
        for refused in [
            "Open http://claude.ai/login",
            "Open https://claude.ai/login#token=one-time",
            "Open https://claude.ai/login?access_token=secret",
            "Open https://evil.test/login",
            "Open file:///etc/passwd",
            "nothing here",
        ] {
            assert_eq!(sign_in_page_available("claude", refused), None, "{refused}");
        }
        assert_eq!(sign_in_page_available("gemini", "Open https://claude.ai/login"), None);
    }

    #[test]
    fn another_agents_sign_in_check_is_not_held_back() {
        let id = "qwen";
        clear_prompt_refused(id);
        note_prompt_refused(id);
        assert_eq!(apply_prompt_refusal(id, Some(true)), Some(true), "a setup terminal or saved keys can sign it in");
        assert_eq!(apply_prompt_refusal(id, None), None);
    }
}
