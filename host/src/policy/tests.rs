//! The policy engine and its store: the tests a compiling agent keeps green.

use super::{Decision, Request, evaluate, explain, trace, validate};
use crate::store::{BotConfig, Policy, PolicyEffect, Store};
use serde_json::json;

/// A rule as an editor's draft: no id yet, which is what saving one adds.
fn rule(name: &str, expression: &str, effect: PolicyEffect) -> Policy {
    Policy {
        id: String::new(),
        name: name.into(),
        expression: expression.into(),
        effect,
        message: format!("{name} says no"),
        enabled: true,
        version: 0,
        updated_at: 0,
    }
}

fn shell(command: &str) -> Request {
    Request { command: command.into(), ..Request::tool("shell") }
}

#[test]
fn deny_rules_are_tried_before_allow_rules() {
    let policies = [
        rule("Anything", "true", PolicyEffect::Allow),
        rule("No shell", r#"tool.kind == "execute""#, PolicyEffect::Deny),
    ];
    let decision = evaluate(&policies, &shell("ls"));
    assert!(decision.denied(), "{decision:?}");
    assert_eq!(decision.rule().map(|rule| rule.name.as_str()), Some("No shell"));
    // The allow rule would have matched too; it is never reached, whatever the list order.
    assert!(!decision.allowed());
}

#[test]
fn an_allow_rule_permits_and_names_itself() {
    let policies = [rule("Reads", r#"tool.kind == "read""#, PolicyEffect::Allow)];
    let decision = evaluate(&policies, &Request { file_path: "notes.md".into(), ..Request::tool("read_file") });
    assert!(decision.allowed(), "{decision:?}");
    assert!(!decision.denied());
    assert_eq!(decision.rule().map(|rule| rule.effect), Some(PolicyEffect::Allow));
    assert!(explain(&decision, &Request::tool("read_file")).contains("Reads"));
}

#[test]
fn a_broken_rule_refuses_and_names_the_error() {
    // It does not compile.
    let broken = [rule("Typo", r#"tool.name == "#, PolicyEffect::Deny)];
    match evaluate(&broken, &shell("ls")) {
        Decision::Error { name, error } => {
            assert_eq!(name, "Typo");
            assert!(!error.is_empty());
        }
        other => panic!("a rule that doesn't compile must refuse, got {other:?}"),
    }
    // It compiles but fails while evaluating: `int()` of a word.
    let failing = [rule("Numbers", r#"int(tool.name) > 3"#, PolicyEffect::Allow)];
    let decision = evaluate(&failing, &shell("ls"));
    assert!(decision.denied(), "{decision:?}");
    assert!(matches!(decision, Decision::Error { .. }));
    // And it isn't a boolean.
    let not_a_bool = [rule("Sentence", r#""yes""#, PolicyEffect::Allow)];
    assert!(evaluate(&not_a_bool, &shell("ls")).denied());
}

#[test]
fn a_missing_policy_permits_nothing_once_an_allow_rule_is_saved() {
    // An allow rule makes the list an allowlist: what no rule names is refused.
    let allowlist = [rule("Reads", r#"tool.kind == "read""#, PolicyEffect::Allow)];
    let decision = evaluate(&allowlist, &shell("ls"));
    assert!(decision.denied(), "{decision:?}");
    assert!(matches!(decision, Decision::NoMatch { allowlist: true }));
    let refusal = decision.refusal().unwrap_or_default();
    assert!(refusal.contains("no policy"), "{refusal}");
    // A deny-only list narrows instead: the approval rules decide what it doesn't refuse.
    let narrowing = [rule("No shell", r#"tool.kind == "execute""#, PolicyEffect::Deny)];
    let decision = evaluate(&narrowing, &Request::tool("read_file"));
    assert!(!decision.denied(), "{decision:?}");
    assert!(matches!(decision, Decision::NoMatch { allowlist: false }));
    // No rule at all leaves the approval rules and the Approvals setting exactly as they were.
    assert_eq!(evaluate(&[], &shell("ls")), Decision::Unset);
    assert!(!evaluate(&[], &shell("ls")).denied());
    // A disabled rule governs nothing.
    let disabled = [Policy { enabled: false, ..rule("No shell", "true", PolicyEffect::Deny) }];
    assert_eq!(evaluate(&disabled, &shell("ls")), Decision::Unset);
}

#[test]
fn variables_resolve_for_each_initiator() {
    let routine = rule("Routines", r#"initiator.kind == "routine""#, PolicyEffect::Deny);
    for kind in ["person", "routine", "channel", "network"] {
        let request = Request { initiator: kind.into(), ..shell("make") };
        assert_eq!(evaluate(&[routine.clone()], &request).denied(), kind == "routine", "{kind}");
    }
    let computer = rule(
        "This computer",
        r#"bot.name == "Build bot" && actor.role == "member" && command.startsWith("cargo")
            && file.op == "edit" && mcp.server == "github" && page.host == "example.com" && key == "Enter""#,
        PolicyEffect::Deny,
    );
    let request = Request {
        bot_name: "Build bot".into(),
        actor_role: "member".into(),
        file_path: "src/main.rs".into(),
        mcp_server: "github".into(),
        page_url: "https://example.com/issues?token=abc".into(),
        key: "Enter".into(),
        ..shell("cargo test")
    };
    assert!(evaluate(&[computer], &request).denied(), "every variable resolves");
}

#[test]
fn a_connector_tool_says_which_connector_and_tool_it_is() {
    let config: BotConfig = serde_json::from_value(
        json!({"id": "bot", "name": "Bot", "backend": "fixture", "command": "true", "cwd": "/tmp"}),
    )
    .expect("a bot");
    let call = json!({"name": "mcp__github__create_issue", "kind": "other", "rawInput": {}});
    let request = Request::from_tool_call(&call, "Create an issue", &config, None, "person");
    assert_eq!((request.mcp_server.as_str(), request.mcp_tool.as_str()), ("github", "create_issue"));
    assert_eq!((request.bot.as_str(), request.bot_name.as_str()), ("bot", "Bot"));
    // A plain tool has no connector, and its title's first word stands in for a name it didn't give.
    let plain = json!({"kind": "execute", "rawInput": {"command": "ls"}});
    let request = Request::from_tool_call(&plain, "Run the tests", &config, Some("ls"), "routine");
    assert_eq!((request.tool.as_str(), request.mcp_server.as_str()), ("run", ""));
    assert_eq!((request.kind.as_str(), request.initiator.as_str()), ("execute", "routine"));
}

#[test]
fn a_rule_is_tried_before_it_is_saved() {
    for preset in super::presets::PRESETS {
        assert!(
            validate(&rule(preset.name, preset.expression, preset.effect)).is_ok(),
            "the {} preset must compile and run",
            preset.name
        );
    }
    assert!(validate(&rule("Typo", r#"tool.nmae == "x""#, PolicyEffect::Allow)).is_err(), "a field no rule has");
    assert!(validate(&rule("No bool", "1 + 1", PolicyEffect::Allow)).is_err(), "not true or false");
    assert!(validate(&rule("Fine", r#"tool.kind == "execute""#, PolicyEffect::Deny)).is_ok());
}

#[test]
fn the_simulator_lists_every_candidate_and_its_outcome() {
    let policies = [
        rule("Reads", r#"tool.kind == "read""#, PolicyEffect::Allow),
        rule("No shell", r#"tool.kind == "execute""#, PolicyEffect::Deny),
        Policy { enabled: false, ..rule("Off", "true", PolicyEffect::Deny) },
    ];
    let traced = trace(&policies, &shell("ls"));
    assert_eq!(traced.len(), 2, "a rule that is off is not tried");
    assert_eq!(traced[0].name, "No shell", "deny rules are listed first");
    assert!(traced[0].matched);
    assert_eq!((traced[1].name.as_str(), traced[1].matched), ("Reads", false));
    // The verdict names the rule that decided, and the trace says what each rule would say.
    let decision = evaluate(&policies, &shell("ls"));
    assert_eq!(decision.rule().map(|rule| rule.name.as_str()), Some("No shell"));
    assert!(traced.iter().all(|rule| rule.error.is_none()));
    // A broken rule is a candidate too, with the error a real request would refuse with.
    let broken = trace(&[rule("Typo", r#"tool.nmae == "x""#, PolicyEffect::Deny)], &shell("ls"));
    assert!(broken[0].error.is_some());
    assert!(!broken[0].matched);
}

#[test]
fn the_explanation_names_the_rule_and_says_nothing_ran() {
    let policies = [rule("No shell", r#"tool.kind == "execute""#, PolicyEffect::Deny)];
    let request = shell("rm -rf build");
    let decision = evaluate(&policies, &request);
    let sentence = explain(&decision, &request);
    assert!(sentence.contains("No shell"), "{sentence}");
    assert!(sentence.ends_with("Nothing ran."), "{sentence}");
    assert_eq!(decision.refusal().unwrap_or_default(), "Refused by the policy \"No shell\": No shell says no");
}

fn store() -> Store {
    crate::store::tests::temp_store()
}

#[test]
fn policies_are_saved_versioned_and_removed() {
    let store = store();
    assert!(store.policies().unwrap().is_empty());
    let first = store
        .save_policy(rule("No shell", r#"tool.kind == "execute""#, PolicyEffect::Deny))
        .expect("a new rule is saved");
    assert_eq!(first.version, 1);
    assert!(!first.id.is_empty());
    assert!(first.enabled, "a new rule is on");
    // A change is one version up, and the versions keep what each save said.
    let changed = store
        .save_policy(Policy {
            expression: r#"tool.kind == "edit""#.into(),
            message: "not edits".into(),
            ..first.clone()
        })
        .expect("an existing rule is updated");
    assert_eq!(changed.version, 2);
    let versions = store.policy_versions(&first.id).unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["version"], 2, "newest first");
    assert_eq!(versions[0]["policy"]["expression"], r#"tool.kind == "edit""#);
    assert_eq!(versions[1]["policy"]["expression"], r#"tool.kind == "execute""#);
    // Switching it off is a change too, and keeps its place in the list.
    let off = store.set_policy_enabled(&first.id, false).unwrap();
    assert!(!off.enabled);
    assert_eq!(off.version, 3);
    assert_eq!(store.policies().unwrap().len(), 1);
    assert!(!evaluate(&store.policies().unwrap(), &shell("ls")).denied(), "off, so nothing governs");
    // Removing it takes its versions with it.
    assert!(store.remove_policy(&first.id).unwrap());
    assert!(store.policy_versions(&first.id).unwrap().is_empty());
    assert!(store.policies().unwrap().is_empty());
}

#[test]
fn a_rule_keeps_its_shape_and_the_lists_limits() {
    let store = store();
    assert!(store.save_policy(rule(" ", "true", PolicyEffect::Allow)).is_err(), "a blank name");
    assert!(store.save_policy(rule("Empty", "  ", PolicyEffect::Allow)).is_err(), "a blank expression");
    let long = rule(&"n".repeat(200), "true", PolicyEffect::Allow);
    assert!(store.save_policy(long).is_err(), "a name past the limit");
    let saved = store.save_policy(rule("Reads", r#"tool.kind == "read""#, PolicyEffect::Allow)).unwrap();
    let list = store.policies().unwrap();
    assert_eq!(list.len(), 1, "the saves that failed left nothing");
    assert_eq!(list[0].id, saved.id);
    assert_eq!(list[0].effect, PolicyEffect::Allow);
}
