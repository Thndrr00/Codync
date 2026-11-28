#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/whatsapp.ts`: link states, message previews,
//! send errors, install steps and webhook words. Only `botInsights` calls this.

use serde_json::Value;

/// Integration id of WhatsApp (WHATSAPP_ID).
pub const WHATSAPP_ID: &str = "integration_whatsapp";

/// WhatsApp's own green (WHATSAPP_GREEN).
pub const WHATSAPP_GREEN: &str = "#25D366";

/// Default gateway URLs (DEFAULT_URLS).
pub const DEFAULT_URLS: [(&str, &str); 2] = [("gowa", "http://127.0.0.1:3000"), ("openwa", "http://127.0.0.1:2785")];

/// Default webhook port (DEFAULT_PORT).
pub const DEFAULT_PORT: u16 = 47_823;

/// Link states of the gateway.
pub const LINK_STATES: [&str; 8] =
    ["off", "badUrl", "connecting", "needsQr", "connected", "unreachable", "unauthorized", "failed"];

/// Send error codes.
pub const SEND_ERRORS: [&str; 6] = ["tooLong", "tooFast", "unreachable", "unauthorized", "needsQr", "refused"];

/// Install steps.
pub const INSTALL_STEPS: [&str; 5] = ["checking", "downloading", "verifying", "installing", "failed"];

/// Media kind labels.
pub const MEDIA_LABELS: [(&str, &str); 7] = [
    ("image", "Photo"),
    ("video", "Video"),
    ("voice", "Voice message"),
    ("document", "Document"),
    ("sticker", "Sticker"),
    ("location", "Location"),
    ("contact", "Contact"),
];

/// Initials of a name (initials).
pub fn initials(name: &str) -> String {
    let letters: Vec<String> = name
        .split(|c: char| c == ' ' || c == '-')
        .filter(|word| word.chars().next().is_some_and(char::is_alphabetic))
        .take(2)
        .filter_map(|word| word.chars().next())
        .map(|c| c.to_uppercase().to_string())
        .collect();
    if letters.is_empty() { "#".to_owned() } else { letters.concat() }
}

/// Seconds as m:ss (duration).
pub fn duration(seconds: i64) -> String {
    let total = seconds.max(0);
    format!("{}:{:02}", total / 60, total % 60)
}

/// Label and value of a message's preview (previewParts).
pub fn preview_parts(message: &Value) -> Value {
    let media = &message["media"];
    let kind = media["kind"].as_str().unwrap_or("none");
    let text = message["text"].as_str().unwrap_or("");
    let label = MEDIA_LABELS.iter().find(|(id, _)| *id == kind).map(|(_, label)| *label);
    let value = match kind {
        "none" | "image" | "video" => text.to_owned(),
        "audio" | "voice" => media["seconds"].as_i64().map_or(String::new(), duration),
        "document" => media["name"].as_str().unwrap_or(text).to_owned(),
        _ => String::new(),
    };
    json!({"label": label, "value": value})
}

/// One-line preview of a message (preview).
pub fn preview(message: &Value, is_group: bool) -> String {
    let parts = preview_parts(message);
    let value = parts["value"].as_str().unwrap_or("");
    let label = parts["label"].as_str();
    let mut line = match label {
        None => value.to_owned(),
        Some(label) if value.is_empty() => label.to_owned(),
        Some(label) => format!("{label} — {value}"),
    };
    line = line.replace('\n', " ");
    if message["fromMe"] == true {
        format!("You: {line}")
    } else if is_group && message["senderName"].as_str().is_some_and(|name| !name.is_empty()) {
        format!("{}: {line}", message["senderName"].as_str().unwrap_or(""))
    } else {
        line
    }
}

/// Time since a message (timeAgo).
pub fn time_ago(at_ms: i64, now_ms: i64) -> String {
    let diff = (now_ms - at_ms).max(0) / 1_000;
    if diff < 60 {
        "now".to_owned()
    } else if diff < 3_600 {
        format!("{}m", diff / 60)
    } else if diff < 86_400 {
        format!("{}h", diff / 3_600)
    } else {
        format!("{}d", diff / 86_400)
    }
}

/// Label of a link state (linkLabel).
pub fn link_label(state: &str, detail: Option<&str>) -> &str {
    match state {
        "off" => "Turned off in Settings",
        "badUrl" => "The gateway URL isn't valid",
        "connecting" => "Connecting to the gateway.",
        "needsQr" => "Scan the QR code in Settings",
        "connected" => "Connected",
        "unreachable" => "The gateway doesn't answer: is it running?",
        "unauthorized" => "The gateway refuses the credentials",
        "failed" => detail.unwrap_or("Failed"),
        _ => "Unknown",
    }
}

/// Color of a link state (linkColor).
pub fn link_color(state: &str) -> &'static str {
    match state {
        "connected" => "#22C55E",
        "connecting" | "needsQr" => "#F5B83D",
        "off" => "#6B7079",
        _ => "#F4505E",
    }
}

/// Label of a send error (sendErrorLabel).
pub fn send_error_label(code: &str, detail: Option<&str>) -> &str {
    match code {
        "tooLong" => "Too long for one message (4,096 characters at most).",
        "tooFast" => "A few replies a minute at most: wait a moment.",
        "unreachable" => "The gateway doesn't answer: is it running?",
        "unauthorized" => "The gateway refuses the credentials",
        "needsQr" => "Scan the QR code in Settings",
        "refused" => detail.unwrap_or("Refused"),
        _ => "Couldn't send",
    }
}

/// Last `count` messages of a conversation (lastMessages).
pub fn last_messages(conversation: &Value, count: usize) -> Vec<Value> {
    let messages = conversation["messages"].as_array().cloned().unwrap_or_default();
    let start = messages.len().saturating_sub(count);
    messages[start..].to_vec()
}

/// What a key press does in the composer (keyAction).
pub fn key_action(key: &str, shift: bool, composing: bool) -> Option<&'static str> {
    if key == "Escape" {
        Some("back")
    } else if key == "Enter" && !composing {
        Some(if shift { "newline" } else { "send" })
    } else {
        None
    }
}

/// Whether a draft is worth sending (canSend).
pub fn can_send(draft: &str, sending: bool) -> bool {
    !sending && !draft.trim().is_empty()
}

/// Label of a running install step (installStepLabel).
pub fn install_step_label(step: &str, megabytes: u64) -> String {
    match step {
        "checking" => "Looking for the latest GOWA.".to_owned(),
        "downloading" => format!("Downloading ({megabytes} MB)."),
        "verifying" => "Checking the download.".to_owned(),
        "installing" => "Installing and starting.".to_owned(),
        _ => String::new(),
    }
}

/// Why an install stopped (installFailure).
pub fn install_failure(reason: &str) -> &'static str {
    match reason {
        "noBuild" => "No GOWA build for this computer was found on GitHub.",
        "noChecksum" => "GOWA's release has no checksum: nothing was installed.",
        "badChecksum" => "The download doesn't match GOWA's file: nothing was installed.",
        "unzipFailed" => "The download couldn't be unzipped. Try again.",
        _ => "The download didn't go through. Check your connection and try again.",
    }
}

/// Error of an install step, when it failed (installError).
pub fn install_error(step: &str, reason: &str) -> Option<&'static str> {
    if step == "failed" { Some(install_failure(reason)) } else { None }
}

/// Whether an install is running (isInstalling).
pub fn is_installing(step: &str) -> bool {
    matches!(step, "checking" | "downloading" | "verifying" | "installing")
}

/// Line describing webhook delivery (webhookLine).
pub fn webhook_line(state: &str, port: u16, detail: Option<&str>) -> String {
    match state {
        "registered" => "Live: new messages arrive at once.".to_owned(),
        "portBusy" => format!("Port {port} is taken: pick another one. Checking every 20 s meanwhile."),
        "refused" => {
            format!("The gateway refused Coucou's address ({}). Checking every 20 s instead.", detail.unwrap_or(""))
        }
        _ => "Checking for new messages every 20 s.".to_owned(),
    }
}

/// Label of a connection test (testResultLabel).
pub fn test_result_label(result: &str, who: Option<&str>, detail: Option<&str>) -> String {
    match result {
        "connected" => format!("Connected as {}.", who.unwrap_or("")),
        "needsQr" => "The gateway answers. Scan the QR code below with your phone.".to_owned(),
        "connecting" => "The gateway answers and is connecting to WhatsApp.".to_owned(),
        "refused" => detail.unwrap_or("Refused").to_owned(),
        other => link_label(other, detail).to_owned(),
    }
}

/// Whether the link state is shown in Settings (showsLinkState).
pub fn shows_link_state(gateway: &str, installed: bool, url: &str, state: &str) -> bool {
    if gateway != "gowa" || installed || !url.is_empty() { true } else { state == "connected" || state == "needsQr" }
}

/// One line describing the WhatsApp card (whatsappLine).
pub fn whatsapp_line(state: &str, unread: usize, down: bool) -> String {
    format!("{state} · {unread} unread · {}", if down { "down" } else { "up" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(unread: usize, conversations: usize, connected: bool, down: bool) -> Vec<(&'static str, String)> {
    vec![
        ("unread", count_label(unread)),
        ("chats", count_label(conversations)),
        ("connected", if connected { "yes" } else { "no" }.to_owned()),
        ("down", if down { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names_and_previews() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials(""), "#");
        assert_eq!(duration(12), "0:12");
        assert_eq!(duration(75), "1:15");
        let message = json!({"text":"hi","media":{"kind":"none"},"fromMe":false,"senderName":""});
        assert_eq!(preview(&message, false), "hi");
        let voice = json!({"text":"","media":{"kind":"voice","seconds":42},"fromMe":true});
        assert_eq!(preview(&voice, false), "You: Voice message — 0:42");
        let group = json!({"text":"hello","media":{"kind":"none"},"fromMe":false,"senderName":"Ana"});
        assert_eq!(preview(&group, true), "Ana: hello");
        assert_eq!(preview_parts(&voice)["label"], "Voice message");
    }

    #[test]
    fn states_and_errors() {
        assert_eq!(link_label("connected", None), "Connected");
        assert_eq!(link_label("failed", Some("boom")), "boom");
        assert_eq!(link_color("connected"), "#22C55E");
        assert_eq!(link_color("off"), "#6B7079");
        assert_eq!(send_error_label("tooLong", None).contains("4,096"), true);
        assert_eq!(send_error_label("refused", Some("no")), "no");
        assert_eq!(time_ago(0, 30_000), "now");
        assert_eq!(time_ago(0, 300_000), "5m");
        assert_eq!(time_ago(0, 10_800_000), "3h");
        assert_eq!(time_ago(0, 172_800_000), "2d");
    }

    #[test]
    fn composer_and_install() {
        assert_eq!(key_action("Enter", false, false), Some("send"));
        assert_eq!(key_action("Enter", true, false), Some("newline"));
        assert_eq!(key_action("Enter", false, true), None);
        assert_eq!(key_action("Escape", false, false), Some("back"));
        assert!(can_send(" hi ", false));
        assert!(!can_send("  ", false));
        assert!(!can_send("hi", true));
        assert_eq!(install_step_label("downloading", 12), "Downloading (12 MB).");
        assert!(install_failure("noBuild").contains("GitHub"));
        assert_eq!(install_error("failed", "badChecksum").is_some(), true);
        assert!(is_installing("checking"));
        assert!(!is_installing("failed"));
    }

    #[test]
    fn settings_and_summary() {
        let conversation = json!({"messages":[{"id":"1"},{"id":"2"},{"id":"3"}]});
        assert_eq!(last_messages(&conversation, 2).len(), 2);
        assert!(webhook_line("registered", DEFAULT_PORT, None).contains("arrive at once"));
        assert!(webhook_line("portBusy", 1234, None).contains("1234"));
        assert!(test_result_label("connected", Some("+123"), None).contains("+123"));
        assert!(shows_link_state("gowa", false, "", "connected"));
        assert!(!shows_link_state("gowa", false, "", "off"));
        assert!(whatsapp_line("Connected", 2, false).contains("2 unread"));
        assert_eq!(WHATSAPP_ID, "integration_whatsapp");
        assert_eq!(WHATSAPP_GREEN, "#25D366");
        assert_eq!(DEFAULT_URLS.len(), 2);
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(2, 3, true, false).len(), 4);
    }
}
