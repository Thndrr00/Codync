//! Anonymous inherited pipes avoid exposing a credential-bearing CDP server to local users.
use crate::agent::process::Process;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout},
};

/// Request ids stay within JavaScript's safe integers (the broker is Node).
const MAX_SAFE_ID: u64 = (1 << 53) - 1;
/// Largest request or reply line, in bytes.
const MAX_MESSAGE: usize = 256 * 1024;
/// Most page dialogs one pipe keeps until a tool result hands them to the agent (triage-15938).
const MAX_DIALOGS: usize = 8;

pub(crate) struct Pipe {
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next: u64,
    poisoned: bool,
    /// Dialogs seen while waiting for replies, oldest first, for the browser tool to hand to the agent.
    dialogs: VecDeque<Value>,
    /// Ids of dialog accepts sent from inside a read, so their replies are skipped, not mistaken for
    /// the answer to the tool's own request.
    accepted: HashSet<u64>,
    /// Only a bot's browser tools accept page dialogs. A person's protected browser keeps the sign-in
    /// and password approval cards exactly as they were (triage-15938).
    pub(crate) auto_dialog: bool,
}

impl Pipe {
    /// Closes a page dialog from inside a read so its script can go on. The command gets its own id;
    /// its reply is dropped in the read loop rather than returned to the tool that owns the pipe.
    async fn accept_dialog(&mut self, session: Option<&str>) -> Result<()> {
        let id = self
            .next
            .checked_add(1)
            .filter(|id| *id <= MAX_SAFE_ID)
            .context("Private browser request identity exhausted")?;
        self.next = id;
        let mut message = json!({"id": id, "method": "Page.handleJavaScriptDialog", "params": {"accept": true}});
        if let Some(session) = session {
            message["sessionId"] = json!(session);
        }
        let mut encoded = serde_json::to_vec(&message)?;
        encoded.push(b'\n');
        ensure!(encoded.len() <= MAX_MESSAGE, "Private browser request exceeded its limit");
        self.input.write_all(&encoded).await.context("Private browser connection failed")?;
        self.accepted.insert(id);
        Ok(())
    }

    /// Dialogs a browser tool still owes its agent. No page text is logged or sent to analytics here.
    pub(crate) fn take_dialogs(&mut self) -> Vec<Value> {
        self.dialogs.drain(..).collect()
    }
}
pub(crate) fn spawn(browser: &Path, folder: &Path, proxy: Option<&str>) -> Result<(Process, Pipe)> {
    let node = crate::agent::backends::which("node")
        .or_else(|| {
            crate::agent::node::search_dirs().iter().map(|dir| dir.join("node.exe")).find(|path| path.is_file())
        })
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        })
        .context("Private browser transport requires Node.js; add it through the agent setup first")?;
    let mut command = tokio::process::Command::new(node);
    command
        .args(["-e", include_str!("../protected-browser-broker.mjs"), "--"])
        .arg(browser)
        .arg(folder)
        .args(proxy.map(|proxy| format!("--proxy={proxy}")))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(
            windows_sys::Win32::System::Threading::CREATE_SUSPENDED
                | windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
        )
        .kill_on_drop(true);
    let mut child = command.spawn().context("Could not start private browser transport")?;
    let input = child.stdin.take().context("Private browser input pipe missing")?;
    let output = BufReader::new(child.stdout.take().context("Private browser output pipe missing")?);
    let process = Process::new(child)?;
    Ok((
        process,
        Pipe {
            input,
            output,
            next: 0,
            poisoned: false,
            dialogs: VecDeque::new(),
            accepted: HashSet::new(),
            auto_dialog: false,
        },
    ))
}
pub(crate) async fn rpc(pipe: &mut Pipe, method: &str, params: Value, session: Option<&str>) -> Result<Value> {
    ensure!(!pipe.poisoned, "Private browser transport was interrupted; start the browser again");
    pipe.next = pipe
        .next
        .checked_add(1)
        .filter(|id| *id <= MAX_SAFE_ID)
        .context("Private browser request identity exhausted")?;
    let id = pipe.next;
    let mut message = json!({"id":id,"method":method,"params":params});
    if let Some(session) = session {
        message["sessionId"] = json!(session);
    }
    let mut encoded = serde_json::to_vec(&message)?;
    encoded.push(b'\n');
    ensure!(encoded.len() <= MAX_MESSAGE, "Private browser request exceeded its limit");
    // A cancelled read_line can consume part of a reply. Never reuse that stream after cancellation.
    pipe.poisoned = true;
    pipe.input.write_all(&encoded).await.context("Private browser connection failed")?;
    loop {
        let mut line = String::new();
        ensure!(pipe.output.read_line(&mut line).await? > 0, "Private browser disconnected; outcome unknown");
        ensure!(line.len() <= MAX_MESSAGE, "Private browser reply exceeded its limit");
        let value: Value = serde_json::from_str(&line).context("Invalid private browser reply")?;
        if value["id"] != id {
            // The broker forwards only page dialog events besides replies. Keep the message as data
            // for the tool result and accept an ordinary dialog here, so a website's `alert()` closes
            // itself instead of covering the window until somebody clicks OK (triage-15938). A
            // `beforeunload` dialog is left for a person; so is every dialog in a protected browser.
            if value["id"].as_u64().is_some_and(|sent| pipe.accepted.remove(&sent)) {
                continue;
            }
            if value["method"] == "Page.javascriptDialogOpening" {
                let params = &value["params"];
                let kind = params["type"].as_str().unwrap_or_default();
                if pipe.dialogs.len() < MAX_DIALOGS {
                    pipe.dialogs.push_back(json!({
                        "kind": kind,
                        "text": params["message"].as_str().unwrap_or_default(),
                        "sessionId": value["sessionId"],
                    }));
                }
                if pipe.auto_dialog && matches!(kind, "alert" | "confirm" | "prompt") {
                    pipe.accept_dialog(value["sessionId"].as_str()).await?;
                }
            }
            continue;
        }
        // The reply arrived whole, so the stream is usable even when the browser refused the operation.
        pipe.poisoned = false;
        ensure!(
            value.get("error").is_none() && value["result"].get("exceptionDetails").is_none(),
            "Private browser refused the operation; no secret detail is returned"
        );
        return Ok(value["result"].clone());
    }
}
