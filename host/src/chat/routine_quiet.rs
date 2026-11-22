//! Quiet scheduled runs (OpenBot nightly `triage-13537`): a routine with nothing to report ends
//! without posting a message.
//!
//! The host already settles a scheduled turn whose whole answer is a silence marker
//! (`agent::bot::proactive::is_silent`). OpenCode Code Mode only keeps the built-in tools inside
//! `execute`, so a local model can try the quiet tool as a separate call, get "No tool named", and
//! then post a filler answer. The pure decision here tells that model the `execute` form and the
//! text fallback, and only for OpenCode: a refusal from a real channel or teammate still gives a
//! normal answer.

/// The text fallback that ends a run quietly when the quiet tool can't be called.
pub const NO_UPDATE: &str = "[[no-update]]";

/// The quiet tool's name in the built-in `routines` MCP server.
pub const QUIET_TOOL: &str = "routine_no_update";

/// The extra scheduled-run instruction for `backend`. Other harnesses get none: their instructions
/// already say to call the quiet tool or answer `(pass)`.
pub fn guidance(backend: &str) -> Option<String> {
    (backend == "opencode").then(|| {
        format!(
            "On a scheduled run with nothing to report, call the no-update tool in execute as \
             tools.openbot.{QUIET_TOOL}({{}}) — never as a separate tool. If OpenCode refuses it with \
             \"No tool named\", answer only {NO_UPDATE} instead of posting a message."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_opencode_gets_the_execute_form_and_text_fallback() {
        let text = guidance("opencode").expect("opencode guidance");
        let execute_call = format!("tools.openbot.{QUIET_TOOL}({{}})");
        assert!(text.contains(execute_call.as_str()));
        assert!(text.contains(NO_UPDATE));
        assert!(text.contains("No tool named"));
        assert_eq!(guidance("claude"), None);
        assert_eq!(guidance("codex"), None);
        assert_eq!(guidance("grok"), None);
    }

    #[test]
    fn the_fallback_marker_is_the_hosts_silence_marker() {
        // `agent::bot::proactive::is_silent` settles a scheduled turn whose whole text is this
        // marker; the existing proactive tests cover that recognition. Keep the two in step.
        assert_eq!(NO_UPDATE, "[[no-update]]");
        assert!(guidance("opencode").expect("guidance").ends_with("instead of posting a message."));
    }
}
