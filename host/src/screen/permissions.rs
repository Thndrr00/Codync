//! Explicit, local-only permission requests from the computer access setup flow.

use super::Screen;
use crate::LockExt as _;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// triage-15679: macOS can list a connected display while Codync has no Screen Recording grant, so
/// a capture comes back with no image. Fixed words name the pane and the restart; this only maps a
/// message, it never opens a system prompt or carries a display name into logs or analytics.
pub(crate) const MAC_CAPTURE_GUIDANCE: &str = "No image came from this display. In System Settings › Screen & System Audio Recording, turn on Codync, then reopen Codync and try again.";

/// Which fixed message a capture failure uses (triage-15679).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureFailure {
    /// A connected display returned no image; tell the person how to grant screen recording.
    NoImage,
    /// The existing freshness error when a capture is missing for another reason.
    StaleSnapshot,
}

/// The words shown to the agent or the person, chosen by kind alone (triage-15679).
pub(crate) fn capture_failure_message(kind: CaptureFailure) -> &'static str {
    match kind {
        CaptureFailure::NoImage => MAC_CAPTURE_GUIDANCE,
        CaptureFailure::StaleSnapshot => {
            "The computer snapshot is stale or unknown. Capture a fresh snapshot before acting."
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Permission {
    Capture,
    Input,
    Portal,
}

impl Screen {
    pub(crate) async fn request_permission(&self, body: &Value) -> Result<Value> {
        let permission: Permission = serde_json::from_value(body["permission"].clone())
            .context("permission must be capture, input or portal")?;
        if cfg!(windows) {
            bail!("Windows does not require these computer access permissions");
        }
        let is_portal = matches!(permission, Permission::Portal);
        if is_portal != cfg!(target_os = "linux") {
            bail!("this permission is not supported on this platform");
        }
        // triage-15679: a connected display with no capture grant is the no-image case, not a
        // generic helper error. Read only status metadata here; no display name or image is logged.
        let connected_without_capture = cfg!(target_os = "macos") && matches!(permission, Permission::Capture) && {
            let status = self.status.locked();
            !status.capture && !status.displays.is_empty()
        };
        match self.link()?.request("requestPermission", json!({"permission": permission})).await {
            Ok(_) => Ok(self.state()),
            Err(_) if connected_without_capture => {
                bail!("{}", capture_failure_message(CaptureFailure::NoImage))
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_display_with_no_image_names_the_screen_recording_setting_and_restart() {
        let message = capture_failure_message(CaptureFailure::NoImage);
        assert!(message.contains("System Settings › Screen & System Audio Recording"), "{message}");
        assert!(message.contains("turn on Codync"), "{message}");
        assert!(message.contains("reopen Codync"), "{message}");
        assert!(!message.contains("http"), "{message}");
        let stale = capture_failure_message(CaptureFailure::StaleSnapshot);
        assert!(stale.contains("stale or unknown"), "{stale}");
        assert_ne!(stale, message);
    }

    #[tokio::test]
    async fn invalid_permission_never_reaches_a_helper() {
        let screen = Screen::new(Some(true), tokio::sync::broadcast::channel(1).0);
        let error = screen.request_permission(&json!({"permission": "all"})).await.unwrap_err();
        assert!(error.to_string().contains("permission must be"));
    }

    #[test]
    fn remote_devices_cannot_request_system_permissions() {
        use crate::api::devices::{Caller, permit};
        assert!(permit(&Caller::Local, "requestScreenPermission").is_ok());
        let device = Caller::Pairing { key: "test".into() };
        assert!(permit(&device, "requestScreenPermission").is_err());
    }
}
