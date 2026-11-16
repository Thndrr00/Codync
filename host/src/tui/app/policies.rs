//! This computer's policies in the terminal: the list, the presets to start from, and the what-if. The host
//! decides every answer (`policies`, `savePolicy`, `setPolicyEnabled`, `removePolicy`, `previewPolicy`); this only
//! words it. See `docs/features/policy-engine.md`.

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::{After, App, Editor, Overlay};

#[derive(Clone, Debug)]
pub struct PolicyRow {
    pub id: String,
    pub name: String,
    pub expression: String,
    pub effect: String,
    pub message: String,
    pub enabled: bool,
}

impl PolicyRow {
    fn of(value: &Value) -> Option<Self> {
        Some(Self {
            id: value["id"].as_str()?.to_owned(),
            name: value["name"].as_str().unwrap_or_default().to_owned(),
            expression: value["expression"].as_str().unwrap_or_default().to_owned(),
            effect: value["effect"].as_str().unwrap_or("deny").to_owned(),
            message: value["message"].as_str().unwrap_or_default().to_owned(),
            enabled: value["enabled"].as_bool().unwrap_or(false),
        })
    }

    /// The rule as `savePolicy` takes it.
    fn body(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "expression": self.expression,
            "effect": self.effect,
            "message": self.message,
            "enabled": self.enabled,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    List,
    /// Typing the selected rule's name.
    Name,
    /// Typing the selected rule's expression.
    Expression,
    /// Typing the selected rule's message.
    Message,
    /// Typing a command for the what-if.
    Try,
}

pub struct PoliciesSheet {
    pub rows: Vec<PolicyRow>,
    /// The presets as the host lists them: `id`, `name`, `effect`, `expression`, `message`, `about`.
    pub presets: Vec<Value>,
    pub sel: usize,
    pub mode: Mode,
    pub editor: Editor,
    /// The host's sentence about the last what-if.
    pub answer: Option<String>,
    pub error: Option<String>,
    pub busy: bool,
    /// The next preset `a` adds.
    pub preset: usize,
}

impl PoliciesSheet {
    pub fn new(rows: Vec<PolicyRow>, presets: Vec<Value>) -> Self {
        Self {
            rows,
            presets,
            sel: 0,
            mode: Mode::List,
            editor: Editor::with(""),
            answer: None,
            error: None,
            busy: false,
            preset: 0,
        }
    }

    /// The preset `a` adds next, and moves past.
    fn next_preset(&mut self) -> Option<Value> {
        let preset = self.presets.get(self.preset % self.presets.len().max(1))?.clone();
        self.preset = self.preset.wrapping_add(1);
        Some(preset)
    }

    pub fn footer(&self) -> String {
        match self.mode {
            Mode::List if self.busy => "saving…".into(),
            Mode::List => {
                "a add a preset · n name · e expression · m message · space on/off · d remove · T try · r reload".into()
            }
            Mode::Name | Mode::Expression | Mode::Message => "↵ save · esc cancel".into(),
            Mode::Try => "↵ ask · esc back".into(),
        }
    }
}

/// The command the what-if asks about, read back out of the line the person typed.
fn what_if(command: &str) -> Value {
    json!({"tool": "shell", "command": command.trim(), "kind": "execute", "initiator": "person"})
}

impl App {
    /// Opens this computer's policies, asking the host for the list.
    pub(super) fn open_policies(&mut self) {
        self.call("policies", json!({}), After::Policies);
    }

    pub(super) fn policies_loaded(&mut self, v: &Value) {
        let rows: Vec<PolicyRow> = v["policies"].as_array().into_iter().flatten().filter_map(PolicyRow::of).collect();
        let presets = v["presets"].as_array().cloned().unwrap_or_default();
        match self.overlays.last_mut() {
            Some(Overlay::Policies(s)) => {
                s.rows = rows;
                s.presets = presets;
                s.busy = false;
                s.sel = s.sel.min(s.rows.len().saturating_sub(1));
            }
            _ => self.overlays.push(Overlay::Policies(Box::new(PoliciesSheet::new(rows, presets)))),
        }
    }

    /// The host's answer to a what-if.
    pub(super) fn policy_previewed(&mut self, v: &Value) {
        if let Some(Overlay::Policies(s)) = self.overlays.last_mut() {
            s.answer = v["preview"]["explanation"].as_str().map(str::to_owned);
        }
    }

    /// A policy call failed: its reason stays in the sheet, where it is being edited.
    pub(super) fn policy_failed(&mut self, error: &str) {
        match self.overlays.last_mut() {
            Some(Overlay::Policies(s)) => {
                s.busy = false;
                s.error = Some(error.to_owned());
            }
            _ => self.flash(error),
        }
    }

    /// A save, switch or removal went through: read the list again, so the sheet shows what was kept.
    pub(super) fn policy_saved(&mut self) {
        self.call("policies", json!({}), After::Policies);
    }

    pub(super) fn policies_key(&mut self, mut s: Box<PoliciesSheet>, k: KeyEvent) -> Option<Overlay> {
        match s.mode {
            Mode::Name | Mode::Expression | Mode::Message => match k.code {
                KeyCode::Enter => {
                    let text = s.editor.text.trim().to_owned();
                    if let Some(row) = s.rows.get_mut(s.sel) {
                        match s.mode {
                            Mode::Name => row.name = text,
                            Mode::Expression => row.expression = text,
                            _ => row.message = text,
                        }
                        s.busy = true;
                        s.error = None;
                        let body = row.body();
                        self.call("savePolicy", json!({"policy": body}), After::PolicySaved);
                    }
                    s.mode = Mode::List;
                }
                KeyCode::Esc => s.mode = Mode::List,
                _ => {
                    s.editor.key(k);
                }
            },
            Mode::Try => match k.code {
                KeyCode::Enter if !s.editor.text.trim().is_empty() => {
                    self.call("previewPolicy", what_if(&s.editor.text), After::PolicyPreview);
                }
                KeyCode::Esc => s.mode = Mode::List,
                _ => {
                    s.editor.key(k);
                    s.answer = None;
                }
            },
            Mode::List => match k.code {
                KeyCode::Esc | KeyCode::Char('q') => return None,
                KeyCode::Down | KeyCode::Char('j') => s.sel = (s.sel + 1).min(s.rows.len().saturating_sub(1)),
                KeyCode::Up | KeyCode::Char('k') => s.sel = s.sel.saturating_sub(1),
                KeyCode::Char('a') => {
                    if let Some(preset) = s.next_preset() {
                        s.busy = true;
                        s.error = None;
                        let body = json!({
                            "name": preset["name"].as_str().unwrap_or("Policy"),
                            "expression": preset["expression"].as_str().unwrap_or("false"),
                            "effect": preset["effect"].as_str().unwrap_or("deny"),
                            "message": preset["message"].as_str().unwrap_or_default(),
                            "enabled": true,
                        });
                        self.call("savePolicy", json!({"policy": body}), After::PolicySaved);
                    } else {
                        s.error = Some("This host has no policy presets".into());
                    }
                }
                KeyCode::Char('n' | 'e' | 'm') if !s.rows.is_empty() => {
                    let row = &s.rows[s.sel];
                    let (mode, text) = match k.code {
                        KeyCode::Char('n') => (Mode::Name, row.name.clone()),
                        KeyCode::Char('e') => (Mode::Expression, row.expression.clone()),
                        _ => (Mode::Message, row.message.clone()),
                    };
                    s.editor = Editor::with(&text);
                    s.mode = mode;
                }
                KeyCode::Char(' ') if !s.rows.is_empty() => {
                    let row = &mut s.rows[s.sel];
                    row.enabled = !row.enabled;
                    s.busy = true;
                    s.error = None;
                    let body = json!({"id": row.id, "enabled": row.enabled});
                    self.call("setPolicyEnabled", body, After::PolicySaved);
                }
                KeyCode::Char('d' | 'x') if !s.rows.is_empty() => {
                    s.busy = true;
                    s.error = None;
                    self.call("removePolicy", json!({"id": s.rows[s.sel].id}), After::PolicySaved);
                }
                KeyCode::Char('T' | '/') => {
                    s.editor = Editor::with("");
                    s.mode = Mode::Try;
                }
                KeyCode::Char('r') => self.call("policies", json!({}), After::Policies),
                _ => {}
            },
        }
        Some(Overlay::Policies(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> PoliciesSheet {
        PoliciesSheet::new(
            [PolicyRow {
                id: "pol_1".into(),
                name: "No shell".into(),
                expression: r#"tool.kind == "execute""#.into(),
                effect: "deny".into(),
                message: "Shell commands are off".into(),
                enabled: true,
            }]
            .into_iter()
            .collect(),
            vec![json!({"id": "readOnly", "name": "Read-only", "effect": "allow", "expression": "true"})],
        )
    }

    #[test]
    fn a_rule_is_saved_as_the_host_reads_it() {
        let s = sheet();
        let body = s.rows[0].body();
        assert_eq!(body["id"], "pol_1");
        assert_eq!(body["effect"], "deny");
        assert_eq!(body["enabled"], true);
    }

    #[test]
    fn presets_take_turns_and_wrap() {
        let mut s = sheet();
        assert_eq!(s.next_preset().and_then(|p| p["name"].as_str().map(str::to_owned)), Some("Read-only".into()));
        assert_eq!(s.next_preset().and_then(|p| p["name"].as_str().map(str::to_owned)), Some("Read-only".into()));
    }

    #[test]
    fn a_row_reads_the_hosts_list() {
        let row =
            PolicyRow::of(&json!({"id": "p", "name": "No sends", "expression": "true", "effect": "deny"})).unwrap();
        assert_eq!((row.name.as_str(), row.enabled), ("No sends", false));
    }

    #[test]
    fn the_what_if_asks_about_a_shell_command() {
        let body = what_if("  git push  ");
        assert_eq!((body["tool"].as_str(), body["command"].as_str()), (Some("shell"), Some("git push")));
    }
}
