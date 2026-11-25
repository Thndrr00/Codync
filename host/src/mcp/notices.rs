//! Fixed, agent-facing MCP wording, chosen by a small kind and never by server text.
//!
//! OpenBot 0.7x taught two lessons the proxy used to miss: a saved sign-in is not the same as a
//! working sign-in (a server can keep its client registration and still need a person), and a
//! missing item in a list is a stale id, not a failed tool. Both are decided here, without
//! carrying an error's text into a notice, so the same cause always gets the same plain advice.

/// Which fixed notice applies. The kind is the only input: no URL, error or tool output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum McpNotice {
    /// A server whose sign-in app was saved still has no usable sign-in.
    SignInNeeded,
    /// What a Test connection says when it reaches such a server.
    VerifyAsksSignIn,
    /// The fields a server may need after a failed Test connection (its sign-in app).
    SignInFields,
}

/// The exact label/message for `kind`, shared by the host's tool errors and the panels.
pub(super) fn mcp_notice(kind: McpNotice) -> &'static str {
    match kind {
        McpNotice::SignInNeeded => "Sign-in needed",
        McpNotice::VerifyAsksSignIn => "Sign in to this connection before checking it.",
        McpNotice::SignInFields => "If this server needs a sign-in app, add its client id and secret here.",
    }
}

/// Which Computer Use permission is missing (OpenBot triage-15424). A plain "not ready" left the
/// person guessing; name the one grant that is absent and where to turn it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UnreadyPermission {
    Capture,
    Input,
    /// Linux' desktop portal: the helper is connected but its grant is missing.
    Portal,
}

/// The fixed words for an unready Computer Use permission. The path is the app's own Computer
/// access page, where the permission can be granted; no deep link or system prompt is opened here.
pub(super) fn unready_permission(kind: UnreadyPermission) -> &'static str {
    match kind {
        UnreadyPermission::Capture => {
            "Screen recording is not allowed. Open Settings → Computer access → Computer Use and allow screen recording."
        }
        UnreadyPermission::Input => {
            "Input control is not allowed. Open Settings → Computer access → Computer Use and allow input control."
        }
        UnreadyPermission::Portal => {
            "The desktop portal is not connected. Open Settings → Computer access → Computer Use and connect it."
        }
    }
}

/// The first missing grant, in the order Computer Use needs them. A helper that has never
/// reported its status is not called unready: that is a connection problem, not a permission one.
pub(super) fn first_missing(status: &serde_json::Value) -> Option<UnreadyPermission> {
    if status["connected"] != true {
        return None;
    }
    if status["capture"] == false {
        Some(UnreadyPermission::Capture)
    } else if status["input"] == false {
        Some(UnreadyPermission::Input)
    } else {
        None
    }
}

/// An error that says a list item is gone is a stale id. Tell the agent how to get a current one
/// (OpenBot triage-16245): retrying the same call can never succeed until it lists again.
///
/// Only the fixed sentence is added; the upstream error stays in the tool result. A different
/// failure keeps its own text, so a permission error is never answered with "list again".
pub(super) fn missing_item_error(error: &str) -> Option<String> {
    let lower = error.to_ascii_lowercase();
    let missing = lower.contains("no such item")
        || lower.contains("item not found")
        || lower.contains("not in the list")
        || lower.contains("unknown item")
        || lower.contains("invalid item id")
        || (lower.contains("not found") && lower.contains("id"));
    if !missing {
        return None;
    }
    let mut text = error.trim().chars().take(400).collect::<String>();
    text.push_str(" List the items again and resend the request with an id from that list.");
    Some(text)
}

/// Adds resend advice to a failed tool reply without changing a successful one (OpenBot
/// triage-16245). The proxy sees the upstream reply before the agent does, so this reaches tools
/// that neither the host's own bridge nor the chat layer knows about.
pub(super) fn annotate_tool_reply(reply: &mut serde_json::Value) {
    if reply["result"]["isError"] != true {
        return;
    }
    let Some(content) = reply["result"]["content"].as_array_mut() else {
        return;
    };
    for item in content {
        if item["type"] != "text" {
            continue;
        }
        let Some(text) = item["text"].as_str() else {
            continue;
        };
        if let Some(advice) = missing_item_error(text) {
            item["text"] = serde_json::Value::String(advice);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unready_computer_use_names_the_permission_it_needs() {
        use serde_json::json;
        assert!(unready_permission(UnreadyPermission::Capture).contains("Screen recording"));
        assert!(unready_permission(UnreadyPermission::Capture).contains("Computer access"));
        assert!(unready_permission(UnreadyPermission::Input).contains("Input control"));
        assert!(unready_permission(UnreadyPermission::Portal).contains("portal"));
        assert_eq!(
            first_missing(&json!({"connected":true,"capture":false,"input":false})),
            Some(UnreadyPermission::Capture)
        );
        assert_eq!(
            first_missing(&json!({"connected":true,"capture":true,"input":false})),
            Some(UnreadyPermission::Input)
        );
        assert_eq!(first_missing(&json!({"connected":true,"capture":true,"input":true})), None);
        assert_eq!(first_missing(&json!({"connected":false,"capture":false})), None, "offline is not a missing grant");
    }

    #[test]
    fn a_saved_sign_in_is_not_reported_as_connected() {
        assert_eq!(mcp_notice(McpNotice::SignInNeeded), "Sign-in needed");
        assert!(mcp_notice(McpNotice::VerifyAsksSignIn).contains("Sign in"));
        assert!(mcp_notice(McpNotice::SignInFields).contains("sign-in app"));
    }

    #[test]
    fn a_missing_list_item_says_how_to_resend() {
        let hint = missing_item_error("Error: item not found: 42").expect("a stale item gets advice");
        assert!(hint.contains("item not found"), "the original error stays");
        assert!(hint.contains("List the items again and resend"));
        assert!(missing_item_error("No such item").is_some());
        assert!(missing_item_error("unknown item id 7").is_some());
    }

    #[test]
    fn an_unrelated_failure_keeps_its_own_error() {
        assert_eq!(missing_item_error("permission denied"), None);
        assert_eq!(missing_item_error(""), None);
    }

    #[test]
    fn only_failed_tool_text_is_annotated() {
        let mut error = serde_json::json!({"result":{"isError":true,"content":[
            {"type":"text","text":"item not found: 9"},{"type":"image","data":"..."}]}});
        annotate_tool_reply(&mut error);
        assert!(error["result"]["content"][0]["text"].as_str().unwrap().contains("List the items again"));
        assert_eq!(error["result"]["content"][1], serde_json::json!({"type":"image","data":"..."}));

        let mut ok = serde_json::json!({"result":{"content":[{"type":"text","text":"item not found: 9"}]}});
        annotate_tool_reply(&mut ok);
        assert_eq!(ok["result"]["content"][0]["text"], "item not found: 9");
    }

    #[test]
    fn a_long_error_cannot_push_the_advice_out_of_a_tool_result() {
        let hint = missing_item_error(&format!("item not found: {}", "x".repeat(2_000))).expect("the hint");
        assert!(hint.chars().count() < 500);
        assert!(hint.ends_with("List the items again and resend the request with an id from that list."));
    }
}
