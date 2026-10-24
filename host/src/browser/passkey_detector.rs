//! Passkey and security-key wall detection (OpenMausBot `shared/passkey-wall.ts`
//! and `server/browser/passkey-notice.ts`).
//!
//! Detects when a sign-in step stops at a passkey or security key challenge
//! that a headless/cloud bot browser cannot satisfy, and formats actionable
//! guidance or user intervention cards.

use serde::{Deserialize, Serialize};

/// Whose account sign-in stopped at a passkey challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PasskeyWall {
    Google,
    Microsoft,
}

/// Where the bot's browser runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserPlace {
    Cloud,
    Desktop,
    Server,
}

/// Checks if a path matches Google's passkey (`pk`) or security key (`sk`) challenge:
/// `/v3/signin/challenge/pk`, `/signin/v2/challenge/sk`, etc.
fn is_google_challenge_path(path: &str) -> bool {
    let trimmed = path.trim_start_matches('/');
    let parts: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    // Match optional `v<digits>` followed by `signin`, optional `v<digits>`, `challenge`, and `pk` or `sk`
    let mut i = 0;
    if i < parts.len() && is_version_segment(parts[i]) {
        i += 1;
    }
    if parts.get(i) != Some(&"signin") {
        return false;
    }
    i += 1;
    if i < parts.len() && is_version_segment(parts[i]) {
        i += 1;
    }
    if parts.get(i) != Some(&"challenge") {
        return false;
    }
    i += 1;
    matches!(parts.get(i), Some(&"pk" | &"sk"))
}

fn is_version_segment(seg: &str) -> bool {
    seg.strip_prefix('v').is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

/// Detects if a URL corresponds to Google's passkey (`pk`) or security key (`sk`) challenge.
pub fn passkey_wall_at(address: &str) -> Option<PasskeyWall> {
    let url = reqwest::Url::parse(address).ok()?;
    if url.host_str() == Some("accounts.google.com") && is_google_challenge_path(url.path()) {
        Some(PasskeyWall::Google)
    } else {
        None
    }
}

/// Inspects page text or tool output for a passkey wall URL or provider-specific passkey wording.
pub fn passkey_wall_in(text: &str) -> Option<PasskeyWall> {
    // 1. Check embedded https://accounts.google.com/... URLs
    let needle = "https://accounts.google.com/";
    let mut rest = text;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + "https://accounts.google.com".len()..];
        let end =
            after.find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '?' | '#' | ')')).unwrap_or(after.len());
        if is_google_challenge_path(&after[..end]) {
            return Some(PasskeyWall::Google);
        }
        rest = &after[end..];
    }

    let lower = text.to_lowercase();
    let has_google_page = text.contains("Google") || lower.contains("accounts.google.com");
    if has_google_page && has_google_words(&lower) {
        return Some(PasskeyWall::Google);
    }

    let has_ms_page =
        text.contains("Microsoft") || lower.contains("login.microsoftonline.com") || lower.contains("login.live.com");
    if has_ms_page && has_microsoft_words(&lower) {
        return Some(PasskeyWall::Microsoft);
    }

    None
}

fn has_google_words(lower: &str) -> bool {
    const PHRASES: &[&str] = &[
        "use your passkey",
        "using your passkey",
        "use sua chave de acesso",
        "usando sua chave de acesso",
        "usa tu llave de acceso",
        "usando tu llave de acceso",
    ];
    if PHRASES.iter().any(|p| lower.contains(p)) {
        return true;
    }
    for verify in ["verifying it's you", "verifying it\u{2019}s you"] {
        if let Some(idx) = lower.find(verify) {
            let window_end = (idx + verify.len() + 400).min(lower.len());
            if lower[idx..window_end].contains("passkey") {
                return true;
            }
        }
    }
    false
}

fn has_microsoft_words(lower: &str) -> bool {
    const PHRASES: &[&str] = &[
        "sign in with your passkey",
        "use your passkey",
        "sign in with your face, fingerprint, pin, or security key",
        "sign in with your face, fingerprint, pin or security key",
    ];
    PHRASES.iter().any(|p| lower.contains(p))
}

/// Actionable notice added when a browser tool encounters a passkey wall.
pub fn passkey_notice(wall: PasskeyWall, place: BrowserPlace) -> String {
    let (name, apps, other_way) = match wall {
        PasskeyWall::Google => ("Google", "Gmail, Calendar or Drive", "Try another way"),
        PasskeyWall::Microsoft => ("Microsoft", "Outlook, Calendar, OneDrive or Teams", "Other ways to sign in"),
    };
    let mut msg = format!(
        "Passkey step, from Codync: this {name} account wants a passkey or security key, which lives on the person's own phone or computer. \
        This browser has none, so it turns the request down at once and {name} may say something went wrong: do not retry, wait, or press Continue or Try again. \
        Choose \"{other_way}\" on this page (the options can take a moment: take a snapshot to see them), or use the person's own computer's browser. \
        What works, best first: 1) For {apps}, use connected apps instead of this browser; if {name} is not connected, call COMPOSIO_MANAGE_CONNECTIONS so the person can connect it. \
        2) If this browser must be signed in: with a saved login for this account, click \"{other_way}\", choose the password and call fill_login; \
        otherwise call browser_wait_for_user and tell them to choose \"{other_way}\" on this page, then use their password and a code (authenticator app, text message or backup code)."
    );
    if place == BrowserPlace::Cloud {
        msg.push_str(" 3) If they lend a PC to Codync, their own browser on it can use the passkey while they approve it at that PC.");
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_google_passkey_challenge_urls() {
        assert_eq!(
            passkey_wall_at("https://accounts.google.com/v3/signin/challenge/pk?TL=abc"),
            Some(PasskeyWall::Google)
        );
        assert_eq!(
            passkey_wall_at("https://accounts.google.com/signin/v2/challenge/sk/presend"),
            Some(PasskeyWall::Google)
        );
        assert_eq!(passkey_wall_at("https://accounts.google.com/v3/signin/challenge/pwd"), None);
    }

    #[test]
    fn detects_google_and_microsoft_passkey_wording() {
        let google_text = "Sign in with Google\nVerifying it's you\nUse your passkey to continue";
        assert_eq!(passkey_wall_in(google_text), Some(PasskeyWall::Google));

        let ms_text = "Microsoft\nSign in with your face, fingerprint, PIN, or security key";
        assert_eq!(passkey_wall_in(ms_text), Some(PasskeyWall::Microsoft));

        let benign = "You can sign in with a passkey or password.";
        assert_eq!(passkey_wall_in(benign), None);
    }
}
