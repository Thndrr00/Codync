//! The user's rules for what a bot's agent may do without asking: each permission request the
//! agent sends is matched, first rule first, before the bot's general setting (`Permission`)
//! applies. `allow` answers it with the agent's allow option, `deny` with its reject option (or a
//! cancel), and `ask` always shows the card, even for a bot that approves everything.
//!
//! A rule matches the request's command (shell tools) or else its title, as a pattern where `*`
//! is any text, ignoring case; `kind` narrows it to one tool kind (`execute`, `edit`, `read`,
//! `fetch`…). An `allow` rule never approves a command that chains others (`;`, `&&`, `|`,
//! `` ` ``, `$(`, redirections, a newline): `git status*` must not let `git status && rm -rf ~`
//! through, so such a command falls to the card instead. `deny` and `ask` rules still apply to it.
//!
//! Idea from rakazo's `packages/core/src/action-approval.ts` (Apache-2.0); no code copied.
//! Its automated review of low-risk requests isn't here: no model decides for the user.

use super::security_profile::SecurityProfile;
use crate::store::{ActionClass, Permission, RulePolicy};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Rules one bot may have.
const MAX_RULES: usize = 100;
/// Longest pattern, in characters.
pub const MAX_PATTERN: usize = 500;
/// What makes one command line run more than one command.
const CHAINING: [&str; 9] = [";", "&&", "||", "|", "`", "$(", ">", "<", "\n"];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRule {
    /// The command or title it matches; `*` is any text.
    pub pattern: String,
    /// Only requests of this tool kind; any kind when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub decision: Decision,
}

pub fn validate(rules: &[ApprovalRule]) -> Result<()> {
    if rules.len() > MAX_RULES {
        bail!("A bot keeps at most {MAX_RULES} approval rules");
    }
    for rule in rules {
        let chars = rule.pattern.trim().chars().count();
        if chars == 0 || chars > MAX_PATTERN {
            bail!("An approval rule's pattern is 1 to {MAX_PATTERN} characters");
        }
    }
    Ok(())
}

/// Whether `command` runs more than one command (see the module docs): no `allow` rule approves it.
pub fn is_chained(command: &str) -> bool {
    CHAINING.iter().any(|c| command.contains(c))
}

/// The Codex app-server approval policy for a bot, or `None` for another backend (OpenBot nightly
/// `triage-14587`). At Ask first with no per-backend auto-approval, Codex must ask before each command and
/// file change; its harness default (`on-request`) lets commands outside the workspace, `sudo` included,
/// through without a card. An auto-approved backend keeps `on-request` (the "Full access" reach).
pub fn codex_approval_policy(backend: &str, permission: Permission, auto_approved: bool) -> Option<&'static str> {
    if backend != "codex" {
        return None;
    }
    Some(if permission == Permission::Ask && !auto_approved { "untrusted" } else { "on-request" })
}

/// OpenBot nightly triage-15626: OpenCode at Ask first asks before commands and file changes. The
/// caller has already established that this is OpenCode; an opted-in backend keeps its automation.
/// Pure and shared by the real permission path and its tests.
pub fn opencode_ask_first(permission: Permission, auto_approved: bool, kind: &str) -> bool {
    permission == Permission::Ask
        && !auto_approved
        && matches!(kind.to_ascii_lowercase().as_str(), "execute" | "edit" | "delete" | "move")
}

/// OpenBot 0.77.0: with this many MCP tools Antigravity's schema budget is tight enough that its CLI may
/// leave the tool search out, so own tools must stay findable through the declarations it does keep.
pub const MANY_MCP_TOOLS: usize = 20;

/// OpenBot 0.77.0: Codync's own MCP servers. `computer` (Computer Use) and `manage` (bot changes) are
/// deliberately absent: they keep their always-manual card (`approval_automation::always_manual`).
const OWN_SERVERS: [&str; 5] = ["chat", "team", "memory", "routines", "connectors"];

/// OpenBot 0.77.0: Gemini and Antigravity are the ACP backends that name an own-tool call in their own
/// way, so they are the ones that need the card-free shortcut.
pub fn is_gemini_backend(backend: &str) -> bool {
    matches!(backend, "gemini" | "antigravity")
}

/// An absolute path in a tool's input that lies outside `workspace` is a file write the confined path
/// must never approve. Relative paths stay inside by definition (OpenBot 0.77.0).
fn writes_outside_workspace(raw: &Value, workspace: &Path) -> bool {
    ["path", "filePath", "file_path", "TargetFile"]
        .into_iter()
        .filter_map(|key| raw[key].as_str())
        .chain(raw["locations"].as_array().into_iter().flatten().filter_map(|v| v["path"].as_str()))
        .chain(raw["diffs"].as_array().into_iter().flatten().filter_map(|v| v["path"].as_str()))
        .any(|path| {
            let path = Path::new(path);
            path.is_absolute() && !path.starts_with(workspace)
        })
}

/// OpenBot 0.77.0: whether a permission request is a call of one of Codync's own tools. ACP titles come
/// as `mcp__<server>__<tool>`, `<server>.<tool>` or, for Antigravity, `Run mcp_<server>_<tool>`;
/// Antigravity also sends `ServerName`/`ToolName` in `rawInput` with tool kind `execute`. A command line,
/// a file change, a path outside the workspace and every server that isn't ours keep their card, so a
/// shell command named like a tool can never take the shortcut.
fn is_own_tool(backend: &str, kind: &str, title: &str, raw: &Value, workspace: &Path) -> bool {
    if matches!(kind, "edit" | "delete" | "move" | "computer") || raw.get("CommandLine").is_some() {
        return false;
    }
    if writes_outside_workspace(raw, workspace) {
        return false;
    }
    let named = is_gemini_backend(backend)
        && raw["ServerName"].as_str().is_some_and(|name| OWN_SERVERS.iter().any(|s| name.eq_ignore_ascii_case(s)))
        && raw["ToolName"].as_str().is_some_and(|tool| !tool.is_empty());
    if named {
        return true;
    }
    if kind != "other" {
        return false;
    }
    let title = title.trim().trim_end_matches('?');
    OWN_SERVERS.iter().any(|server| {
        title.starts_with(&format!("mcp__{server}__"))
            || title.starts_with(&format!("{server}."))
            || (is_gemini_backend(backend) && title.starts_with(&format!("Run mcp_{server}_")))
    })
}

/// OpenBot 0.77.0: Gemini agents with Full access use Codync's own tools (memory, browser, channels,
/// messages to other agents) without an approval card, the same as Claude agents. Shell commands and
/// other tools still ask when automatic approval is off; an already auto-approved backend is handled by
/// `approval_automation`, which this decision does not replace. A confined agent uses
/// [`workspace_only_tool_allowed`] instead. Pure and shared by the real `on_permission` path and its tests.
pub fn own_tool_allowed(
    backend: &str,
    profile: SecurityProfile,
    auto_approved: bool,
    confined: bool,
    kind: &str,
    title: &str,
    raw: &Value,
    workspace: &Path,
) -> bool {
    if auto_approved || !is_gemini_backend(backend) || !profile.permits_tool(kind) {
        return false;
    }
    if !is_own_tool(backend, kind, title, raw, workspace) {
        return false;
    }
    profile == SecurityProfile::Full || (confined && workspace_only_tool_allowed(backend, kind, title, raw, workspace))
}

/// OpenBot 0.77.0: a Gemini agent in Workspace only still sends messages to other agents and uses
/// Codync's own tools without a card. The safe request shape is the whole guard: a command line, a file
/// change (including one that names an own tool) and a path outside the workspace are refused here, so
/// the existing sandbox stays the authority on file writes. `kind` and `raw` come from the permission
/// request; the title alone is never enough for an `execute` request.
pub fn workspace_only_tool_allowed(backend: &str, kind: &str, title: &str, raw: &Value, workspace: &Path) -> bool {
    is_gemini_backend(backend) && is_own_tool(backend, kind, title, raw, workspace)
}

/// OpenBot 0.77.0: Antigravity's CLI drops `tool_search` when its schema budget is full (Computer Use or
/// many MCP tools), leaving the browser file-upload tool undiscoverable. Keep tool search in the
/// declarations when that happens. Gemini keeps it even with few tools; other backends keep their own
/// direct-declaration behaviour unless they too have many tools or Computer Use.
pub fn tool_search_visible(backend: &str, mcp_tools: usize, computer_use: bool) -> bool {
    is_gemini_backend(backend) || computer_use || mcp_tools >= MANY_MCP_TOOLS
}

/// The rule that decides a request of `kind` doing `subject` (its command, or its title).
#[cfg(test)]
fn decide<'a>(rules: &'a [ApprovalRule], kind: &str, subject: &str) -> Option<&'a ApprovalRule> {
    find(rules, &[], kind, subject).0.map(|i| &rules[i])
}

/// The tool kinds each quick rule ("by kind of action") covers. A class with no ACP tool kind is not enforced here:
/// `browser`, `apps`, `bots` and `routines` reach the agent as MCP tools (kind `other`, named in the title), which the
/// connectors' tool policies govern (`docs/features/mcp-tool-policies.md`), so a rule on `other` would hit all of them.
pub const QUICK_KINDS: [(ActionClass, &[&str]); 3] = [
    (ActionClass::Files, &["read", "search", "edit", "delete", "move"]),
    (ActionClass::Shell, &["execute"]),
    (ActionClass::Browse, &["fetch"]),
];

/// The classes quick rules cannot enforce, with why.
pub const QUICK_UNENFORCED: [(ActionClass, &str); 4] = [
    (ActionClass::Browser, "browser tools are MCP tools: see the connectors' tool policies"),
    (ActionClass::Apps, "computer-use tools are MCP tools: see the connectors' tool policies"),
    (ActionClass::Bots, "team tools are MCP tools: see the connectors' tool policies"),
    (ActionClass::Routines, "routine tools are MCP tools: see the connectors' tool policies"),
];

/// The quick rules ("allow files, ask for the shell") as pattern rules: one `*` rule per tool kind of each class they
/// cover. They go after the person's own rules, so an own rule always wins.
pub fn quick_rules(policies: &BTreeMap<ActionClass, RulePolicy>) -> Vec<ApprovalRule> {
    let mut rules = Vec::new();
    for (class, kinds) in QUICK_KINDS {
        let Some(policy) = policies.get(&class) else { continue };
        let decision = match policy {
            RulePolicy::Allow => Decision::Allow,
            RulePolicy::Ask => Decision::Ask,
            RulePolicy::Block => Decision::Deny,
        };
        rules.extend(kinds.iter().map(|kind| ApprovalRule {
            pattern: "*".into(),
            kind: Some((*kind).into()),
            decision,
        }));
    }
    rules
}

/// The index of the deciding rule among `rules` then `quick`, and the first `allow` rule that matched but was passed
/// over because the command is chained.
fn find(rules: &[ApprovalRule], quick: &[ApprovalRule], kind: &str, subject: &str) -> (Option<usize>, Option<usize>) {
    let chained = is_chained(subject);
    let mut passed_over = None;
    for (i, rule) in rules.iter().chain(quick).enumerate() {
        if !rule.kind.as_deref().is_none_or(|k| k.eq_ignore_ascii_case(kind)) || !matches(&rule.pattern, subject) {
            continue;
        }
        if rule.decision == Decision::Allow && chained {
            passed_over.get_or_insert(i);
            continue;
        }
        return (Some(i), passed_over);
    }
    (None, passed_over)
}

/// What happens to a permission request, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum Outcome {
    /// A command no bot may run (`agent::blocklist`): refused before any rule.
    Forbidden { pattern: String },
    /// The rule at `index` decides.
    Rule { index: usize, decision: Decision, pattern: String, kind: Option<String>, quick: bool },
    /// No rule matched: the bot's setting decides (`auto` approves, `ask` shows a card).
    Setting { permission: Permission },
}

/// An [`Outcome`] with what a person testing rules wants to know besides it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluation {
    #[serde(flatten)]
    pub outcome: Outcome,
    /// The command runs several commands, so no `allow` rule approves it.
    pub chained: bool,
    /// An `allow` rule that matched but was passed over for that reason.
    pub allow_passed_over: Option<usize>,
}

/// The one place that decides a request: the real approval (`on_permission`) and the editor's tester both call it,
/// so what the tester says is what happens. `command` is a shell tool's command; a request without one is judged by
/// its `title`.
pub fn evaluate(
    rules: &[ApprovalRule],
    quick: &[ApprovalRule],
    permission: Permission,
    kind: &str,
    command: Option<&str>,
    title: &str,
) -> Evaluation {
    let subject = command.unwrap_or(title);
    let chained = is_chained(subject);
    if let Some(pattern) = command.and_then(super::blocklist::blocked) {
        return Evaluation { outcome: Outcome::Forbidden { pattern }, chained, allow_passed_over: None };
    }
    let (decided, allow_passed_over) = find(rules, quick, kind, subject);
    let outcome = match decided {
        Some(index) => {
            let rule = rules.iter().chain(quick).nth(index).expect("`find` returned the index of a rule it walked");
            Outcome::Rule {
                index,
                decision: rule.decision,
                pattern: rule.pattern.clone(),
                kind: rule.kind.clone(),
                quick: index >= rules.len(),
            }
        }
        None => Outcome::Setting { permission },
    };
    Evaluation { outcome, chained, allow_passed_over }
}

/// Whether `pattern` (with `*` for any text) covers all of `text`, ignoring case.
fn matches(pattern: &str, text: &str) -> bool {
    glob(pattern, text, false)
}

/// [`matches`], and with `single` a `?` in the pattern stands for exactly one character. The one matcher: the
/// user's rules and the forbidden commands (`agent::blocklist`) both use it.
pub(super) fn glob(pattern: &str, text: &str, single: bool) -> bool {
    let pattern: Vec<char> = pattern.trim().to_lowercase().chars().collect();
    let text: Vec<char> = text.trim().to_lowercase().chars().collect();
    // Two cursors; on a mismatch, go back to just after the last `*` and let it take one more char.
    let (mut p, mut t) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if p < pattern.len() && (pattern[p] == text[t] || (single && pattern[p] == '?')) {
            p += 1;
            t += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rule(pattern: &str, decision: Decision) -> ApprovalRule {
        ApprovalRule { pattern: pattern.into(), kind: None, decision }
    }

    #[test]
    fn patterns_cover_the_whole_text() {
        assert!(matches("git status*", "git status --short"));
        assert!(matches("GIT *", "git push origin main"));
        assert!(matches("*.env", "cat .env"));
        assert!(!matches("git status", "git status --short"));
        assert!(!matches("git push*", "git pull"));
        assert!(matches("*", "anything"));
    }

    #[test]
    fn evaluation_names_the_rule_the_forbidden_command_or_the_setting() {
        let rules = [rule("git push*", Decision::Ask), rule("git *", Decision::Allow), rule("rm *", Decision::Deny)];
        let eval = |command: &str| evaluate(&rules, &[], Permission::Auto, "execute", Some(command), "Run");
        assert_eq!(
            eval("git push x").outcome,
            Outcome::Rule { index: 0, decision: Decision::Ask, pattern: "git push*".into(), kind: None, quick: false }
        );
        assert!(matches!(eval("git status").outcome, Outcome::Rule { index: 1, decision: Decision::Allow, .. }));
        assert!(matches!(eval("rm -rf build").outcome, Outcome::Rule { index: 2, decision: Decision::Deny, .. }));
        assert_eq!(eval("ls").outcome, Outcome::Setting { permission: Permission::Auto });
        assert!(matches!(eval("rm -rf /").outcome, Outcome::Forbidden { .. }), "forbidden before any rule");
        let chained = eval("git status && ls");
        assert_eq!(chained.outcome, Outcome::Setting { permission: Permission::Auto }, "the allow rule is passed over");
        assert_eq!((chained.chained, chained.allow_passed_over), (true, Some(1)));
        // A request with no command is judged by its title.
        let by_title = evaluate(&rules, &[], Permission::Ask, "edit", None, "git config");
        assert!(matches!(by_title.outcome, Outcome::Rule { index: 1, .. }));
    }

    fn policies(pairs: &[(ActionClass, RulePolicy)]) -> BTreeMap<ActionClass, RulePolicy> {
        pairs.iter().copied().collect()
    }

    /// What a quick rule says about one request of `kind`, with no rule of the person's own.
    fn quick_says(pairs: &[(ActionClass, RulePolicy)], kind: &str) -> Outcome {
        let quick = quick_rules(&policies(pairs));
        evaluate(&[], &quick, Permission::Auto, kind, Some("anything"), "Run").outcome
    }

    #[test]
    fn each_class_a_quick_rule_can_enforce_becomes_rules_for_its_tool_kinds() {
        for (class, kinds) in QUICK_KINDS {
            for (policy, decision) in [
                (RulePolicy::Allow, Decision::Allow),
                (RulePolicy::Ask, Decision::Ask),
                (RulePolicy::Block, Decision::Deny),
            ] {
                for kind in kinds {
                    let outcome = quick_says(&[(class, policy)], kind);
                    assert!(
                        matches!(&outcome, Outcome::Rule { decision: d, quick: true, .. } if *d == decision),
                        "{class:?} {policy:?} on a {kind} request gave {outcome:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_quick_rule_covers_only_its_own_kinds_and_loses_to_the_persons_own_rules() {
        let shell_blocked = [(ActionClass::Shell, RulePolicy::Block)];
        assert_eq!(quick_says(&shell_blocked, "read"), Outcome::Setting { permission: Permission::Auto });
        let quick = quick_rules(&policies(&shell_blocked));
        let own = [rule("echo *", Decision::Allow)];
        let allowed = evaluate(&own, &quick, Permission::Ask, "execute", Some("echo hi"), "Run");
        assert!(matches!(allowed.outcome, Outcome::Rule { index: 0, decision: Decision::Allow, quick: false, .. }));
        let blocked = evaluate(&own, &quick, Permission::Ask, "execute", Some("make"), "Run");
        assert!(matches!(blocked.outcome, Outcome::Rule { index: 1, decision: Decision::Deny, quick: true, .. }));
        // A quick "allow" is a rule like any: it never approves a chained command.
        let allow_shell = quick_rules(&policies(&[(ActionClass::Shell, RulePolicy::Allow)]));
        let chained = evaluate(&[], &allow_shell, Permission::Ask, "execute", Some("ls && rm x"), "Run");
        assert_eq!(chained.outcome, Outcome::Setting { permission: Permission::Ask });
    }

    #[test]
    fn the_classes_without_a_tool_kind_are_named_not_enforced() {
        let mut every: Vec<ActionClass> = QUICK_KINDS.iter().map(|(class, _)| *class).collect();
        every.extend(QUICK_UNENFORCED.iter().map(|(class, _)| *class));
        every.sort();
        assert_eq!(every.len(), 7, "all seven classes are either enforced or explained");
        every.dedup();
        assert_eq!(every.len(), 7, "none is both");
        let all = policies(&[(ActionClass::Bots, RulePolicy::Block), (ActionClass::Apps, RulePolicy::Block)]);
        assert!(quick_rules(&all).is_empty(), "a class with no tool kind adds no rule");
    }

    #[test]
    fn a_question_mark_is_one_character_only_when_asked() {
        assert!(glob("?:\\", "c:\\", true));
        assert!(!glob("?:\\", "cc:\\", true));
        assert!(!matches("?:\\", "c:\\"), "the user's rules keep a plain question mark");
        assert!(matches("why?", "why?"));
    }

    #[test]
    fn the_first_rule_decides_and_chained_commands_are_never_allowed() {
        let rules = [rule("git push*", Decision::Ask), rule("git *", Decision::Allow), rule("rm *", Decision::Deny)];
        assert_eq!(decide(&rules, "execute", "git push origin").unwrap().decision, Decision::Ask);
        assert_eq!(decide(&rules, "execute", "git status").unwrap().decision, Decision::Allow);
        assert!(decide(&rules, "execute", "git status && rm -rf ~").is_none(), "falls to the card");
        assert_eq!(decide(&rules, "execute", "rm -rf build; ls").unwrap().decision, Decision::Deny);
        assert!(decide(&rules, "execute", "ls").is_none());
        let only_edits = [ApprovalRule { pattern: "*".into(), kind: Some("edit".into()), decision: Decision::Deny }];
        assert!(decide(&only_edits, "execute", "ls").is_none());
        assert!(decide(&only_edits, "edit", "Edit main.rs").is_some());
    }

    #[test]
    fn rules_are_bounded() {
        assert!(validate(&[rule(" ", Decision::Allow)]).is_err());
        let many: Vec<ApprovalRule> = std::iter::repeat_n(rule("x", Decision::Ask), MAX_RULES + 1).collect();
        assert!(validate(&many).is_err());
        assert!(validate(&[rule("npm test", Decision::Allow)]).is_ok());
    }

    #[test]
    fn own_tools_are_recognised_by_title_or_by_antigravity_input() {
        let workspace = Path::new("/w");
        for title in ["mcp__chat__send_message", "chat.send_message", "Run mcp_team_ask?"] {
            assert!(is_own_tool("antigravity", "other", title, &json!({}), workspace), "{title}");
        }
        let named = json!({"ServerName": "memory", "ToolName": "search_memory"});
        assert!(is_own_tool("antigravity", "execute", "MCP Tool search_memory", &named, workspace));
        assert!(!is_own_tool("claude", "execute", "MCP Tool search_memory", &named, workspace));
        assert!(is_gemini_backend("gemini") && is_gemini_backend("antigravity") && !is_gemini_backend("claude"));
    }

    #[test]
    fn a_command_a_file_change_or_another_server_is_not_an_own_tool() {
        let workspace = Path::new("/w");
        for (kind, title, raw) in [
            ("execute", "Run mcp_chat_send_message", json!({})),
            ("edit", "Edit notes.md", json!({"ServerName": "chat", "ToolName": "send_message"})),
            ("delete", "mcp__chat__send_message", json!({})),
            ("move", "mcp__chat__send_message", json!({})),
            ("computer", "mcp__chat__send_message", json!({})),
            ("other", "Run mcp_github_create_issue", json!({})),
            ("other", "mcp__computer__click", json!({})),
            ("other", "mcp__chat__send_message", json!({"CommandLine": "rm -rf /"})),
            ("other", "mcp__chat__send_message", json!({"path": "/elsewhere/notes.md"})),
            ("other", "mcp__chat__send_message", json!({"diffs": [{"path": "/elsewhere/notes.md"}]})),
        ] {
            assert!(!is_own_tool("antigravity", kind, title, &raw, workspace), "{kind} {title} {raw}");
        }
        // The same name on a server of our own, with a path inside the workspace, is still ours.
        let inside = json!({"path": "/w/notes.md"});
        assert!(is_own_tool("gemini", "other", "mcp__memory__edit_memory", &inside, workspace));
    }

    #[test]
    fn full_access_gemini_uses_own_tools_without_a_card() {
        let workspace = Path::new("/w");
        let own = json!({});
        for backend in ["gemini", "antigravity"] {
            assert!(own_tool_allowed(
                backend,
                SecurityProfile::Full,
                false,
                false,
                "other",
                "mcp__chat__send_message",
                &own,
                workspace,
            ));
            assert!(own_tool_allowed(
                backend,
                SecurityProfile::Full,
                false,
                false,
                "other",
                "mcp__memory__search_memory",
                &own,
                workspace,
            ));
        }
        // Shell commands, file changes, other servers and a profile's denial never take this path.
        for (kind, title, raw) in [
            ("execute", "git push", json!({})),
            ("execute", "Run mcp_chat_send_message", json!({})),
            ("edit", "mcp__chat__send_message", json!({})),
            ("other", "mcp__github__create_issue", json!({})),
            ("other", "mcp__chat__send_message", json!({"CommandLine": "rm -rf /"})),
        ] {
            assert!(
                !own_tool_allowed("antigravity", SecurityProfile::Full, false, false, kind, title, &raw, workspace),
                "{kind} {title} {raw}"
            );
        }
        // An own-tool shape whose kind the profile denies is refused too.
        let named = json!({"ServerName": "chat", "ToolName": "send_message"});
        assert!(
            !own_tool_allowed(
                "antigravity",
                SecurityProfile::Builder,
                false,
                true,
                "execute",
                "MCP Tool send_message",
                &named,
                workspace,
            ),
            "Builder denies execute"
        );
    }

    #[test]
    fn no_other_backend_or_auto_approved_backend_changes() {
        let workspace = Path::new("/w");
        let own = json!({});
        for backend in ["claude", "codex", "grok", "pi", "custom"] {
            assert!(
                !own_tool_allowed(
                    backend,
                    SecurityProfile::Full,
                    false,
                    true,
                    "other",
                    "mcp__chat__send_message",
                    &own,
                    workspace
                ),
                "{backend} keeps its own behaviour"
            );
            assert!(!workspace_only_tool_allowed(backend, "other", "mcp__chat__send_message", &own, workspace));
        }
        assert!(
            !own_tool_allowed(
                "antigravity",
                SecurityProfile::Full,
                true,
                false,
                "other",
                "mcp__chat__send_message",
                &own,
                workspace
            ),
            "automation already answers"
        );
        assert!(
            !own_tool_allowed(
                "antigravity",
                SecurityProfile::Operator,
                false,
                false,
                "other",
                "mcp__chat__send_message",
                &own,
                workspace
            ),
            "Ask first still asks for an own tool"
        );
    }

    #[test]
    fn workspace_only_gemini_keeps_messages_and_own_tools_but_not_writes() {
        let workspace = Path::new("/w");
        let message = json!({"ServerName": "chat", "ToolName": "send_message"});
        assert!(workspace_only_tool_allowed("gemini", "execute", "MCP Tool send_message", &message, workspace));
        assert!(workspace_only_tool_allowed("antigravity", "other", "Run mcp_team_ask?", &json!({}), workspace));
        // A file change that names an own tool, a command line and a path outside the workspace stay denied.
        assert!(
            !workspace_only_tool_allowed("antigravity", "edit", "Edit notes.md", &message, workspace),
            "an edit with an own-tool input is still an edit"
        );
        for raw in [json!({"CommandLine": "ls"}), json!({"path": "/elsewhere/x.md"})] {
            assert!(
                !workspace_only_tool_allowed("antigravity", "other", "mcp__memory__edit_memory", &raw, workspace),
                "{raw}"
            );
        }
        // The confined branch of the one decision agrees with its helper.
        assert!(own_tool_allowed(
            "antigravity",
            SecurityProfile::Operator,
            false,
            true,
            "other",
            "mcp__chat__send_message",
            &json!({}),
            workspace,
        ));
    }

    #[test]
    fn tool_search_stays_visible_when_the_schema_budget_is_tight() {
        assert!(tool_search_visible("gemini", 0, false));
        assert!(tool_search_visible("antigravity", 0, false));
        assert!(tool_search_visible("antigravity", MANY_MCP_TOOLS, false));
        assert!(tool_search_visible("pi", MANY_MCP_TOOLS, false), "many tools need search on pi too");
        assert!(tool_search_visible("pi", 0, true), "Computer Use fills the budget");
        assert!(!tool_search_visible("claude", MANY_MCP_TOOLS - 1, false));
        assert!(!tool_search_visible("codex", 0, false), "no behaviour change for Codex");
    }

    #[test]
    fn codex_asks_before_each_command_only_at_ask_first() {
        assert_eq!(codex_approval_policy("codex", Permission::Ask, false), Some("untrusted"));
        assert_eq!(codex_approval_policy("codex", Permission::Ask, true), Some("on-request"));
        assert_eq!(codex_approval_policy("codex", Permission::Auto, false), Some("on-request"));
        assert_eq!(codex_approval_policy("claude", Permission::Ask, false), None, "only Codex reads this");
    }

    #[test]
    fn opencode_ask_first_asks_before_commands_and_file_changes() {
        for kind in ["execute", "edit", "delete", "move", "EXECUTE"] {
            assert!(opencode_ask_first(Permission::Ask, false, kind), "{kind} gets a card");
        }
        assert!(!opencode_ask_first(Permission::Ask, false, "read"), "reads keep their reach");
        assert!(!opencode_ask_first(Permission::Ask, false, "fetch"));
        assert!(!opencode_ask_first(Permission::Ask, true, "execute"), "an opted-in backend stays automatic");
        assert!(!opencode_ask_first(Permission::Auto, false, "edit"), "Always allow keeps its setting");
    }
}
