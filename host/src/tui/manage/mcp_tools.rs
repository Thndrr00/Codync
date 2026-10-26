//! Owner controls for remote tool policies, exact approvals and connector-wide revocation.
use super::super::app::{App, Editor, Overlay};
use super::{Reply, ctrl};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct McpReply(pub String, pub u64, pub McpResult);
#[derive(Clone, Copy)]
pub enum McpResult {
    Policies,
    Requests(bool),
    Detail,
    Receipt,
    Changed,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum McpMode {
    Browse,
    Edit(bool),
    Reset,
    Revoke,
    Receipt,
}

pub struct McpToolsSheet {
    pub connector: String,
    pub name: String,
    pub scope: String,
    pub tab: usize,
    pub rows: Vec<Value>,
    pub next: Option<String>,
    pub cursor: usize,
    pub detail: Option<Value>,
    pub scroll: u16,
    pub mode: McpMode,
    pub tool: Editor,
    pub policy: usize,
    pub busy: bool,
    pub error: Option<String>,
    request: u64,
}
const MODES: [&str; 3] = ["ask", "allow", "deny"];
fn s<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

impl App {
    pub fn open_mcp_tools(&mut self, connector: String, name: String) {
        let mut sheet = Box::new(McpToolsSheet {
            connector,
            name,
            scope: uuid::Uuid::new_v4().to_string(),
            tab: 0,
            rows: vec![],
            next: None,
            cursor: 0,
            detail: None,
            scroll: 0,
            mode: McpMode::Browse,
            tool: Editor::default(),
            policy: 0,
            busy: false,
            error: None,
            request: 0,
        });
        self.reload_mcp_tools(&mut sheet, false);
        self.overlays.push(Overlay::McpTools(sheet));
    }
    fn mcp_command(&mut self, sheet: &mut McpToolsSheet, method: &'static str, mut body: Value, result: McpResult) {
        if method != "resolveToolCall" {
            body["connectorId"] = sheet.connector.clone().into();
        }
        sheet.request += 1;
        sheet.busy = true;
        sheet.error = None;
        self.sheet(method, body, Reply::McpTools(McpReply(sheet.scope.clone(), sheet.request, result)));
    }
    fn reload_mcp_tools(&mut self, sheet: &mut McpToolsSheet, more: bool) {
        if sheet.tab == 0 {
            self.mcp_command(sheet, "getMcpToolSettings", json!({}), McpResult::Policies);
        } else {
            let body = if more { json!({"afterId":sheet.next}) } else { json!({}) };
            self.mcp_command(sheet, "listMcpToolCalls", body, McpResult::Requests(more));
        }
    }
    pub fn mcp_tools_reply(&mut self, reply: &McpReply, response: Result<Value, String>) {
        let Some(index) = self.overlays.iter().position(|overlay| {
            matches!(overlay,
            Overlay::McpTools(sheet) if sheet.scope == reply.0 && sheet.request == reply.1)
        }) else {
            return;
        };
        let Overlay::McpTools(mut sheet) = self.overlays.remove(index) else {
            return;
        };
        sheet.busy = false;
        match response {
            Err(error) => sheet.error = Some(error),
            Ok(value) => match reply.2 {
                McpResult::Policies | McpResult::Requests(_) => {
                    let rows = value["items"].as_array().cloned().unwrap_or_default();
                    if matches!(reply.2, McpResult::Requests(true)) {
                        sheet.rows.extend(rows);
                    } else {
                        sheet.rows = rows;
                    }
                    sheet.next = value["next"].as_str().map(str::to_owned);
                    sheet.cursor = sheet.cursor.min(sheet.rows.len().saturating_sub(1));
                }
                McpResult::Detail => {
                    sheet.detail = Some(value);
                    sheet.scroll = 0;
                }
                McpResult::Receipt => {
                    sheet.detail = Some(
                        json!({"id":sheet.tool.text,"tool":"Saved execution receipt","status":"receipt only","arguments":value}),
                    );
                    sheet.mode = McpMode::Browse;
                    sheet.scroll = 0;
                }
                McpResult::Changed => {
                    sheet.detail = None;
                    sheet.mode = McpMode::Browse;
                    self.reload_mcp_tools(&mut sheet, false);
                }
            },
        }
        self.overlays.insert(index, Overlay::McpTools(sheet));
    }
    pub fn mcp_tools_key(&mut self, mut sheet: Box<McpToolsSheet>, key: KeyEvent) -> Option<Overlay> {
        if key.code == KeyCode::Esc {
            if sheet.busy {
                return Some(Overlay::McpTools(sheet));
            }
            if sheet.detail.take().is_some() || sheet.mode != McpMode::Browse {
                sheet.mode = McpMode::Browse;
            } else {
                return None;
            }
        } else if sheet.busy {
            return Some(Overlay::McpTools(sheet));
        } else if sheet.mode == McpMode::Receipt {
            if key.code == KeyCode::Enter && !sheet.tool.text.trim().is_empty() {
                let body = json!({"id":sheet.tool.text.trim()});
                self.mcp_command(&mut sheet, "getMcpToolResult", body, McpResult::Receipt);
            } else {
                sheet.tool.key(key);
            }
        } else if matches!(sheet.mode, McpMode::Reset | McpMode::Revoke) {
            if key.code == KeyCode::Enter {
                let (method, body) = if sheet.mode == McpMode::Reset {
                    ("resetMcpToolPolicy", json!({"tool":sheet.tool.text}))
                } else {
                    ("revokeMcpToolCalls", json!({}))
                };
                self.mcp_command(&mut sheet, method, body, McpResult::Changed);
            }
        } else if let McpMode::Edit(policy_focus) = sheet.mode {
            match key.code {
                KeyCode::Tab => sheet.mode = McpMode::Edit(!policy_focus),
                KeyCode::Left if policy_focus => sheet.policy = (sheet.policy + 2) % 3,
                KeyCode::Right if policy_focus => sheet.policy = (sheet.policy + 1) % 3,
                KeyCode::Enter if !sheet.tool.text.trim().is_empty() => {
                    let body = json!({"tool":sheet.tool.text.trim(),"mode":MODES[sheet.policy]});
                    self.mcp_command(&mut sheet, "setMcpToolPolicy", body, McpResult::Changed);
                }
                _ if !policy_focus => {
                    sheet.tool.key(key);
                }
                _ => {}
            }
        } else if let Some(detail) = &sheet.detail {
            match key.code {
                KeyCode::Down => sheet.scroll = sheet.scroll.saturating_add(1),
                KeyCode::Up => sheet.scroll = sheet.scroll.saturating_sub(1),
                KeyCode::Char('a' | 'd') if s(detail, "status") == "pending" => {
                    let body = json!({"id":detail["id"],"approve":key.code == KeyCode::Char('a')});
                    self.mcp_command(&mut sheet, "resolveToolCall", body, McpResult::Changed);
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Tab => {
                    sheet.tab = 1 - sheet.tab;
                    sheet.rows.clear();
                    sheet.cursor = 0;
                    self.reload_mcp_tools(&mut sheet, false);
                }
                KeyCode::Down => sheet.cursor = (sheet.cursor + 1).min(sheet.rows.len().saturating_sub(1)),
                KeyCode::Up => sheet.cursor = sheet.cursor.saturating_sub(1),
                KeyCode::Char('r') => self.reload_mcp_tools(&mut sheet, false),
                KeyCode::Char('l') if sheet.tab == 1 && sheet.next.is_some() => self.reload_mcp_tools(&mut sheet, true),
                KeyCode::Char('x') if ctrl(key) => sheet.mode = McpMode::Revoke,
                KeyCode::Char('g') => {
                    sheet.tool = Editor::default();
                    sheet.mode = McpMode::Receipt;
                }
                KeyCode::Char('n') if sheet.tab == 0 => {
                    sheet.tool = Editor::default();
                    sheet.policy = 0;
                    sheet.mode = McpMode::Edit(false);
                }
                KeyCode::Enter | KeyCode::Delete => {
                    if let Some(row) = sheet.rows.get(sheet.cursor) {
                        if sheet.tab == 0 {
                            sheet.tool = Editor::with(s(row, "tool"));
                            sheet.policy = MODES.iter().position(|mode| *mode == s(row, "mode")).unwrap_or(0);
                            sheet.mode =
                                if key.code == KeyCode::Delete { McpMode::Reset } else { McpMode::Edit(false) };
                        } else if key.code == KeyCode::Enter {
                            let body = json!({"id":row["id"]});
                            self.mcp_command(&mut sheet, "getMcpToolCall", body, McpResult::Detail);
                        }
                    }
                }
                _ => {}
            }
        }
        Some(Overlay::McpTools(sheet))
    }
}
