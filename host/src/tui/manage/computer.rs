//! Computer panel intents; data and capability enforcement remain in the host.
use super::super::app::{App, Editor, Overlay};
use super::{Reply, ctrl};
use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub enum ComputerResult {
    State,
    Files,
    Preview,
    Command,
    Changed,
}
#[derive(Clone)]
pub struct ComputerReply(pub String, pub u64, pub ComputerResult);
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ComputerMode {
    Browse,
    Path,
    Site,
    ConfirmCommand,
}
pub struct ComputerSheet {
    pub bot: String,
    pub scope: String,
    pub state: Value,
    pub tab: usize,
    pub rows: Vec<Value>,
    pub cursor: usize,
    pub path: Editor,
    pub folder: String,
    requested_folder: String,
    pub site: Editor,
    pub command: Editor,
    pub preview: Option<String>,
    pub output: Option<Value>,
    pub scroll: u16,
    pub mode: ComputerMode,
    pub busy: bool,
    pub running: bool,
    pub error: Option<String>,
    request: u64,
    command_request: u64,
}
fn s<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}
impl App {
    pub fn open_computer_panel(&mut self) {
        let Some(bot) = self.bot().filter(|bot| !bot.group).map(|bot| bot.id.clone()) else {
            self.flash("Select an individual bot to open its computer");
            return;
        };
        let mut sheet = Box::new(ComputerSheet {
            bot,
            scope: uuid::Uuid::new_v4().to_string(),
            state: Value::Null,
            tab: 0,
            rows: vec![],
            cursor: 0,
            path: Editor::default(),
            folder: String::new(),
            requested_folder: String::new(),
            site: Editor::default(),
            command: Editor::default(),
            preview: None,
            output: None,
            scroll: 0,
            mode: ComputerMode::Browse,
            busy: false,
            running: false,
            error: None,
            request: 0,
            command_request: 0,
        });
        self.computer_request(&mut sheet, "computerPanel", json!({}), ComputerResult::State);
        self.overlays.push(Overlay::Computer(sheet));
    }
    fn computer_request(
        &mut self,
        sheet: &mut ComputerSheet,
        method: &'static str,
        mut body: Value,
        kind: ComputerResult,
    ) {
        if matches!(kind, ComputerResult::Files) {
            sheet.requested_folder = s(&body, "path").into();
        }
        if !matches!(method, "updateBot" | "finishProtectedEntry" | "openProtectedBrowser") {
            body["botId"] = sheet.bot.clone().into();
        }
        let request = if matches!(kind, ComputerResult::Command) {
            sheet.command_request += 1;
            sheet.running = true;
            sheet.command_request
        } else {
            sheet.request += 1;
            sheet.busy = true;
            sheet.request
        };
        sheet.error = None;
        self.sheet(method, body, Reply::Computer(ComputerReply(sheet.scope.clone(), request, kind)));
    }
    pub fn computer_reply(&mut self, reply: &ComputerReply, result: Result<Value, String>) {
        let Some(index) = self.overlays.iter().position(|overlay| matches!(overlay, Overlay::Computer(sheet)
            if sheet.scope == reply.0 && (if matches!(reply.2, ComputerResult::Command) {sheet.command_request} else {sheet.request}) == reply.1)) else { return; };
        let Overlay::Computer(mut sheet) = self.overlays.remove(index) else {
            return;
        };
        if matches!(reply.2, ComputerResult::Command) {
            sheet.running = false;
        } else {
            sheet.busy = false;
        }
        match result {
            Err(error) => sheet.error = Some(error),
            Ok(value) => match reply.2 {
                ComputerResult::State => {
                    if value["enabled"] != true || value["permissions"]["files"] != true {
                        sheet.rows.clear();
                        sheet.preview = None;
                    }
                    sheet.state = value;
                }
                ComputerResult::Files => {
                    sheet.folder.clone_from(&sheet.requested_folder);
                    sheet.path = Editor::with(&sheet.folder);
                    sheet.rows = value["items"].as_array().cloned().unwrap_or_default();
                    sheet.cursor = 0;
                    sheet.preview = None;
                }
                ComputerResult::Preview => {
                    sheet.preview = Some(s(&value, "text").into());
                    sheet.scroll = 0;
                }
                ComputerResult::Command => {
                    sheet.output = Some(value);
                }
                ComputerResult::Changed => {
                    self.computer_request(&mut sheet, "computerPanel", json!({}), ComputerResult::State);
                }
            },
        }
        self.overlays.insert(index, Overlay::Computer(sheet));
    }
    pub fn computer_key(&mut self, mut sheet: Box<ComputerSheet>, key: KeyEvent) -> Option<Overlay> {
        if key.code == KeyCode::Esc {
            if sheet.mode != ComputerMode::Browse {
                sheet.mode = ComputerMode::Browse;
            } else if sheet.preview.take().is_none() {
                return None;
            }
        } else if key.code == KeyCode::Tab {
            sheet.preview = None;
            sheet.tab = (sheet.tab + 1) % 4;
            sheet.mode = ComputerMode::Browse;
            sheet.scroll = 0;
        } else if sheet.busy {
            return Some(Overlay::Computer(sheet));
        } else if matches!(sheet.mode, ComputerMode::Path | ComputerMode::Site) {
            if key.code == KeyCode::Enter {
                let (method, body, result) = if sheet.mode == ComputerMode::Path {
                    ("listComputerFiles", json!({"path":sheet.path.text}), ComputerResult::Files)
                } else {
                    ("beginProtectedEntry", json!({"site":sheet.site.text}), ComputerResult::Changed)
                };
                self.computer_request(&mut sheet, method, body, result);
                sheet.mode = ComputerMode::Browse;
            } else if sheet.mode == ComputerMode::Path {
                sheet.path.key(key);
            } else {
                sheet.site.key(key);
            }
        } else if sheet.mode == ComputerMode::ConfirmCommand {
            if key.code == KeyCode::Enter && !sheet.running {
                let body = json!({"command":sheet.command.text});
                self.computer_request(&mut sheet, "runComputerCommand", body, ComputerResult::Command);
                sheet.mode = ComputerMode::Browse;
            }
        } else if key.code == KeyCode::Char('r') && ctrl(key) {
            self.computer_request(&mut sheet, "computerPanel", json!({}), ComputerResult::State);
        } else if sheet.tab == 0 {
            match key.code {
                KeyCode::Char('0' | '1' | '2' | '3') => {
                    let mut permissions = sheet.state["permissions"].clone();
                    if !permissions.is_object() {
                        return Some(Overlay::Computer(sheet));
                    }
                    let mut enabled = sheet.state["enabled"] == true;
                    let key_name = match key.code {
                        KeyCode::Char('1') => Some("browser"),
                        KeyCode::Char('2') => Some("files"),
                        KeyCode::Char('3') => Some("shell"),
                        _ => None,
                    };
                    if let Some(name) = key_name {
                        permissions[name] = (permissions[name] != true).into();
                    } else {
                        enabled = !enabled;
                    }
                    let body = json!({"id":sheet.bot,"computer":enabled,"computerPermissions":permissions});
                    self.computer_request(&mut sheet, "updateBot", body, ComputerResult::Changed);
                }
                KeyCode::Char('p') => sheet.mode = ComputerMode::Site,
                KeyCode::Char('b')
                    if sheet.state["protectedBrowserAvailable"] == true
                        && sheet.state["screen"]["protectedEntry"].is_object() =>
                {
                    let body = json!({"id":sheet.state["screen"]["protectedEntry"]["id"]});
                    self.computer_request(&mut sheet, "openProtectedBrowser", body, ComputerResult::Changed);
                }
                KeyCode::Char(key @ ('l' | 'c'))
                    if sheet.state["protectedBrowserAvailable"] == true
                        && sheet.state["screen"]["protectedEntry"].is_object() =>
                {
                    let id = s(&sheet.state["screen"]["protectedEntry"], "id").to_owned();
                    self.overlays.push(Overlay::Computer(sheet));
                    return Some(Overlay::Fields(Box::new(super::protected_fill_fields(&id, key == 'c'))));
                }
                KeyCode::Char('f') if sheet.state["screen"]["protectedEntry"].is_object() => {
                    let body = json!({"id":sheet.state["screen"]["protectedEntry"]["id"]});
                    self.computer_request(&mut sheet, "finishProtectedEntry", body, ComputerResult::Changed);
                }
                KeyCode::Char('h')
                    if sheet.state["protectedBrowserAvailable"] == true
                        && sheet.state["screen"]["protectedEntry"].is_object() =>
                {
                    let body = json!({"id":sheet.state["screen"]["protectedEntry"]["id"],"keepSession":true});
                    self.computer_request(&mut sheet, "finishProtectedEntry", body, ComputerResult::Changed);
                }
                _ => {}
            }
        } else if sheet.preview.is_some() {
            if key.code == KeyCode::Down {
                sheet.scroll = sheet.scroll.saturating_add(1);
            }
            if key.code == KeyCode::Up {
                sheet.scroll = sheet.scroll.saturating_sub(1);
            }
        } else if sheet.tab == 1 {
            match key.code {
                KeyCode::Char('/') => sheet.mode = ComputerMode::Path,
                KeyCode::Up => sheet.cursor = sheet.cursor.saturating_sub(1),
                KeyCode::Down => sheet.cursor = (sheet.cursor + 1).min(sheet.rows.len().saturating_sub(1)),
                KeyCode::Enter => {
                    if let Some(row) = sheet.rows.get(sheet.cursor).filter(|row| row["link"] != true) {
                        let path = if sheet.folder.is_empty() {
                            s(row, "name").to_owned()
                        } else {
                            format!("{}/{}", sheet.folder, s(row, "name"))
                        };
                        let directory = row["directory"] == true;
                        self.computer_request(
                            &mut sheet,
                            if directory { "listComputerFiles" } else { "readComputerFile" },
                            json!({"path":path}),
                            if directory { ComputerResult::Files } else { ComputerResult::Preview },
                        );
                    }
                }
                _ => {}
            }
        } else if sheet.tab == 2 {
            if key.code == KeyCode::Enter && !sheet.command.text.trim().is_empty() && !sheet.running {
                sheet.mode = ComputerMode::ConfirmCommand;
            } else {
                sheet.command.key(key);
            }
        } else {
            if key.code == KeyCode::Down {
                sheet.scroll = sheet.scroll.saturating_add(1);
            }
            if key.code == KeyCode::Up {
                sheet.scroll = sheet.scroll.saturating_sub(1);
            }
        }
        Some(Overlay::Computer(sheet))
    }
}
