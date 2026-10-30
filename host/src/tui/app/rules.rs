//! A bot's approval rules in the terminal: a sheet to add, edit, reorder and delete them, and to try a command against
//! them. The host decides what a rule does (`testApprovalRule`); this only words its answer.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::{After, App, Editor, Overlay};

/// The tool kinds a rule can be limited to.
pub const KINDS: [&str; 9] = ["execute", "edit", "delete", "move", "read", "search", "fetch", "think", "other"];
const DECISIONS: [&str; 3] = ["allow", "ask", "deny"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleRow {
    pub pattern: String,
    pub kind: Option<String>,
    pub decision: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    List,
    /// Typing the selected rule's pattern.
    Pattern,
    /// Typing a command to try.
    Test,
}

pub struct RulesSheet {
    pub bot_id: String,
    pub bot_name: String,
    pub rules: Vec<RuleRow>,
    pub sel: usize,
    pub mode: Mode,
    pub editor: Editor,
    /// What the host said about the last command tried.
    pub answer: Option<String>,
    pub saving: bool,
    pub error: Option<String>,
}

impl RulesSheet {
    pub fn new(bot_id: &str, bot_name: &str, saved: &[Value]) -> Self {
        let rules = saved
            .iter()
            .filter_map(|r| {
                Some(RuleRow {
                    pattern: r["pattern"].as_str()?.to_owned(),
                    kind: r["kind"].as_str().filter(|k| !k.is_empty()).map(str::to_owned),
                    decision: r["decision"].as_str().unwrap_or("ask").to_owned(),
                })
            })
            .collect();
        Self {
            bot_id: bot_id.to_owned(),
            bot_name: bot_name.to_owned(),
            rules,
            sel: 0,
            mode: Mode::List,
            editor: Editor::with(""),
            answer: None,
            saving: false,
            error: None,
        }
    }

    /// The rules as the host reads them: blank patterns dropped, the rest trimmed.
    pub fn body(&self) -> Value {
        let rules: Vec<Value> = self
            .rules
            .iter()
            .filter(|r| !r.pattern.trim().is_empty())
            .map(|r| json!({"pattern": r.pattern.trim(), "kind": r.kind, "decision": r.decision}))
            .collect();
        Value::Array(rules)
    }

    pub fn cycle_decision(&mut self) {
        if let Some(rule) = self.rules.get_mut(self.sel) {
            let at = DECISIONS.iter().position(|d| *d == rule.decision).unwrap_or(1);
            DECISIONS[(at + 1) % DECISIONS.len()].clone_into(&mut rule.decision);
        }
    }

    /// Any tool, then each kind in turn, then any tool again.
    pub fn cycle_kind(&mut self) {
        if let Some(rule) = self.rules.get_mut(self.sel) {
            rule.kind = match rule.kind.as_deref().and_then(|k| KINDS.iter().position(|x| *x == k)) {
                None if rule.kind.is_none() => Some(KINDS[0].to_owned()),
                Some(i) if i + 1 < KINDS.len() => Some(KINDS[i + 1].to_owned()),
                _ => None,
            };
        }
    }

    /// Moves the selected rule by `delta` places (the first match decides, so order matters).
    pub fn move_selected(&mut self, delta: isize) {
        let to = self.sel.checked_add_signed(delta).filter(|to| *to < self.rules.len());
        if let Some(to) = to {
            self.rules.swap(self.sel, to);
            self.sel = to;
        }
    }
}

/// One sentence for the host's answer (`evaluation`); rule numbers count from 1, as the list shows them.
pub fn describe(e: &Value) -> String {
    let passed = e["allowPassedOver"]
        .as_u64()
        .map(|i| format!(" Rule {} would allow it, but it runs several commands.", i + 1))
        .unwrap_or_default();
    match e["outcome"].as_str() {
        Some("forbidden") => format!(
            "Never allowed: it matches \"{}\" in the list of forbidden commands, whatever the rules say.",
            e["pattern"].as_str().unwrap_or_default()
        ),
        Some("rule") => {
            let verb = match e["decision"].as_str() {
                Some("allow") => "Allowed",
                Some("deny") => "Denied",
                _ => "Asks you",
            };
            format!(
                "{verb} by rule {} (\"{}\").",
                e["index"].as_u64().unwrap_or(0) + 1,
                e["pattern"].as_str().unwrap_or_default()
            )
        }
        _ => {
            let how = if e["permission"] == "auto" { "it is approved automatically" } else { "it asks you" };
            format!("No rule matches: {how}, as the bot's Approvals setting says.{passed}")
        }
    }
}

impl App {
    /// Opens the selected bot's rules.
    pub(super) fn open_rules(&mut self) {
        let Some(bot) = self.bot().filter(|b| !b.group) else {
            self.flash("Pick a bot first");
            return;
        };
        let sheet = RulesSheet::new(&bot.id, &bot.name, &bot.approval_rules);
        self.overlays.push(Overlay::Rules(Box::new(sheet)));
    }

    pub(super) fn approval_rules_key(&mut self, mut s: Box<RulesSheet>, k: KeyEvent) -> Option<Overlay> {
        match s.mode {
            Mode::Pattern => match k.code {
                KeyCode::Enter => {
                    let text = s.editor.text.trim().to_owned();
                    if text.is_empty() {
                        if s.sel < s.rules.len() {
                            s.rules.remove(s.sel);
                            s.sel = s.sel.min(s.rules.len().saturating_sub(1));
                        }
                    } else if let Some(rule) = s.rules.get_mut(s.sel) {
                        rule.pattern = text;
                    }
                    s.mode = Mode::List;
                }
                KeyCode::Esc => {
                    // A rule added and never given a pattern is no rule.
                    if s.rules.get(s.sel).is_some_and(|r| r.pattern.trim().is_empty()) {
                        s.rules.remove(s.sel);
                        s.sel = s.sel.min(s.rules.len().saturating_sub(1));
                    }
                    s.mode = Mode::List;
                }
                _ => {
                    s.editor.key(k);
                }
            },
            Mode::Test => match k.code {
                KeyCode::Enter if !s.editor.text.trim().is_empty() => {
                    let body = json!({"botId": s.bot_id, "command": s.editor.text.trim(), "rules": s.body()});
                    self.call("testApprovalRule", body, After::RuleTest);
                }
                KeyCode::Esc => s.mode = Mode::List,
                _ => {
                    s.editor.key(k);
                    s.answer = None;
                }
            },
            Mode::List => match k.code {
                KeyCode::Esc | KeyCode::Char('q') => return None,
                KeyCode::Char('s') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    s.saving = true;
                    s.error = None;
                    let body = json!({"id": s.bot_id, "approvalRules": s.body()});
                    self.call("updateBot", body, After::RulesSaved);
                }
                KeyCode::Down | KeyCode::Char('j') => s.sel = (s.sel + 1).min(s.rules.len().saturating_sub(1)),
                KeyCode::Up | KeyCode::Char('k') => s.sel = s.sel.saturating_sub(1),
                KeyCode::Char('a') => {
                    s.rules.push(RuleRow { pattern: String::new(), kind: None, decision: "ask".into() });
                    s.sel = s.rules.len() - 1;
                    s.editor = Editor::with("");
                    s.mode = Mode::Pattern;
                }
                KeyCode::Enter | KeyCode::Char('e') if !s.rules.is_empty() => {
                    s.editor = Editor::with(&s.rules[s.sel].pattern);
                    s.mode = Mode::Pattern;
                }
                KeyCode::Char('d' | 'x') if !s.rules.is_empty() => {
                    s.rules.remove(s.sel);
                    s.sel = s.sel.min(s.rules.len().saturating_sub(1));
                }
                KeyCode::Char('K') => s.move_selected(-1),
                KeyCode::Char('J') => s.move_selected(1),
                KeyCode::Char('m') => s.cycle_decision(),
                KeyCode::Char('t') => s.cycle_kind(),
                KeyCode::Char('T' | '/') => {
                    s.editor = Editor::with("");
                    s.mode = Mode::Test;
                }
                _ => {}
            },
        }
        Some(Overlay::Rules(s))
    }

    /// The host's answer to a command tried in the sheet.
    pub(super) fn rules_tested(&mut self, v: &Value) {
        if let Some(Overlay::Rules(s)) = self.overlays.last_mut() {
            s.answer = Some(describe(&v["evaluation"]));
        }
    }

    /// The rules were saved (or refused): close the sheet, or say why not.
    pub(super) fn rules_saved(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                if matches!(self.overlays.last(), Some(Overlay::Rules(_))) {
                    self.overlays.pop();
                }
                self.flash("Approval rules saved");
            }
            Err(error) => {
                if let Some(Overlay::Rules(s)) = self.overlays.last_mut() {
                    s.saving = false;
                    s.error = Some(error);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> RulesSheet {
        RulesSheet::new(
            "b",
            "Bot",
            &[
                json!({"pattern": "git status*", "decision": "allow"}),
                json!({"pattern": "rm *", "kind": "execute", "decision": "deny"}),
            ],
        )
    }

    #[test]
    fn rules_load_move_and_cycle() {
        let mut s = sheet();
        assert_eq!(s.rules.len(), 2);
        s.move_selected(1);
        assert_eq!((s.rules[0].pattern.as_str(), s.sel), ("rm *", 1));
        s.move_selected(1);
        assert_eq!(s.sel, 1, "can't go past the end");
        s.cycle_decision();
        assert_eq!(s.rules[1].decision, "ask", "allow, then ask");
        s.sel = 0;
        s.cycle_kind();
        assert_eq!(s.rules[0].kind.as_deref(), Some("edit"), "execute, then edit");
        for _ in 1..KINDS.len() {
            s.cycle_kind();
        }
        assert_eq!(s.rules[0].kind, None, "after the last kind, any tool again");
    }

    #[test]
    fn the_body_drops_blank_patterns_and_trims() {
        let mut s = sheet();
        s.rules.push(RuleRow { pattern: "  ".into(), kind: None, decision: "ask".into() });
        s.rules[0].pattern = "  ls  ".into();
        let body = s.body();
        assert_eq!(body.as_array().map(Vec::len), Some(2));
        assert_eq!(body[0]["pattern"], "ls");
        assert_eq!(body[1]["kind"], "execute");
    }

    #[test]
    fn the_hosts_answer_is_worded() {
        let forbidden = json!({"outcome": "forbidden", "pattern": "rm -rf /"});
        assert!(describe(&forbidden).starts_with("Never allowed"));
        let rule = json!({"outcome": "rule", "index": 1, "decision": "allow", "pattern": "git *"});
        assert_eq!(describe(&rule), "Allowed by rule 2 (\"git *\").");
        let setting = json!({"outcome": "setting", "permission": "ask", "allowPassedOver": 0});
        let text = describe(&setting);
        assert!(text.contains("it asks you") && text.contains("Rule 1 would allow it"), "{text}");
    }
}
