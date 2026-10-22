//! Browser automation subsystem (CDP engine, isolated profiles, tab ownership
//! guard, and Passkey wall detection).

pub mod cdp_client;
pub mod passkey_detector;
pub mod profile_manager;
pub mod tab_guard;

use serde_json::{Value, json};

/// MCP tool definitions for semantic browser control and passkey assistance.
pub fn tools() -> Value {
    json!([
        {
            "name": "browser_open",
            "description": "Open a URL in a bot-controlled browser tab over CDP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "HTTPS/HTTP URL to open (chrome:// and edge:// are forbidden)." }
                },
                "required": ["url"]
            }
        },
        {
            "name": "browser_click",
            "description": "Click an element matching a CSS selector in an owned browser tab.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "targetId": { "type": "string" },
                    "selector": { "type": "string" }
                },
                "required": ["targetId", "selector"]
            }
        },
        {
            "name": "browser_type",
            "description": "Type text directly into an element matching a CSS selector in an owned browser tab.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "targetId": { "type": "string" },
                    "selector": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["targetId", "selector", "text"]
            }
        },
        {
            "name": "browser_snapshot",
            "description": "Capture the structured DOM/accessibility text and detect any Passkey/WebAuthn walls.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "targetId": { "type": "string" }
                },
                "required": ["targetId"]
            }
        },
        {
            "name": "browser_wait_for_user",
            "description": "Pause and display a Passkey/Authentication assistance card asking the user to complete sign-in manually.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "reason": { "type": "string", "description": "Clear explanation of what authentication step the user needs to complete." }
                },
                "required": ["reason"]
            }
        }
    ])
}
