//! The one reload rule shared by the bot's browser tools and the Browser panel. Kept here, off the
//! Windows-only modules, so the decision and its test run on every host.

#![allow(dead_code, reason = "the Windows browser tools and panel are the only callers")]

use serde_json::Value;
use serde_json::json;

/// The control that asked for a reload: a tool call or the panel's Reload button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReloadKind {
    Agent,
    Panel,
}

/// OpenBot 0.70: "Reload page bypasses the cache." Both callers use the same strategy — a reload
/// that answers with a stale page is no use to an agent checking a change — so both send
/// `ignoreCache: true` to `Page.reload`. The kind is kept so the rule is named at each call site.
pub(crate) fn reload_strategy(kind: ReloadKind) -> Value {
    let _ = kind;
    json!({"ignoreCache": true})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reload_bypasses_the_cache() {
        for kind in [ReloadKind::Agent, ReloadKind::Panel] {
            assert_eq!(reload_strategy(kind), json!({"ignoreCache": true}));
        }
        assert_ne!(reload_strategy(ReloadKind::Agent), json!({"ignoreCache": false}));
    }
}
