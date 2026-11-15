//! A module's process: one child per module, JSON-RPC over stdio, held to its quotas.
//!
//! It never runs inside a bot's actor: a module that hangs, spins or dies takes its own call down and nothing else.
//! The host writes one JSON object per line on stdin and reads one per line from stdout.

use super::manifest::{Limits, Manifest};
use anyhow::{Context, Result, anyhow, bail, ensure};
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex as AsyncMutex, broadcast, oneshot};

/// A module's line, both ways: past this it is broken, not merely chatty.
const MAX_LINE: usize = 1024 * 1024;
/// How often a running module's memory is read, and how long it gets to go down.
const WATCH: Duration = Duration::from_secs(2);

/// One module's running process.
pub struct Running {
    pub id: String,
    stdin: AsyncMutex<tokio::process::ChildStdin>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>,
    next: AtomicU64,
    /// When each call in the last minute started, for the rate.
    calls: Mutex<VecDeque<Instant>>,
    limits: Limits,
    alive: AtomicBool,
    /// The handshake ran on this process.
    initialized: AtomicBool,
    /// Why it stopped, for the row the apps show.
    stopped: Mutex<Option<String>>,
}

impl Running {
    /// Starts a module and its two watchers: the reader that answers calls and the one that holds its memory.
    /// `heard` are the events it may hear, already checked against the manifest.
    pub fn spawn(
        dir: &Path,
        manifest: &Manifest,
        heard: Vec<String>,
        events: broadcast::Sender<Value>,
    ) -> Result<Arc<Self>> {
        let limits = manifest.limits.clamped();
        let mut command = tokio::process::Command::new(&manifest.entry.command);
        command
            .args(&manifest.entry.args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .env_clear()
            // Enough to find its own programs; no host token, no data directory, nothing of the person's.
            .envs(std::env::vars().filter(|(k, _)| matches!(k.as_str(), "PATH" | "HOME" | "USER" | "LANG")))
            .env("CODYNC_MODULE_ID", &manifest.id)
            .env("CODYNC_MODULE_DATA", dir.join(super::manifest::DATA))
            .env("CODYNC_MODULE_CONTRACT", super::manifest::CONTRACT.to_string());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            // Its own process group, so stopping it stops whatever it started.
            command.as_std_mut().process_group(0);
        }
        std::fs::create_dir_all(dir.join(super::manifest::DATA))?;
        let mut child =
            command.spawn().with_context(|| format!("starting {} ({})", manifest.name, manifest.entry.command))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("the module has no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("the module has no stdout"))?;
        let pid = child.id();
        let id = manifest.id.clone();
        let this = Arc::new(Self {
            id: id.clone(),
            stdin: AsyncMutex::new(stdin),
            pending: Mutex::default(),
            next: AtomicU64::new(1),
            calls: Mutex::default(),
            limits,
            alive: AtomicBool::new(true),
            initialized: AtomicBool::new(false),
            stopped: Mutex::default(),
        });
        tokio::spawn(read(Arc::downgrade(&this), BufReader::new(stdout), events, id, heard));
        tokio::spawn(watch(Arc::downgrade(&this), child, pid, limits.memory_mb));
        Ok(this)
    }

    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Why it stopped, when it did.
    pub fn stopped(&self) -> Option<String> {
        self.stopped.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Asks the module one thing. Past the module's time, its rate or a broken line it is stopped and the caller
    /// gets a sentence it can show.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        ensure!(self.alive(), "{} is not running", self.id);
        {
            let mut calls = self.calls.lock().map_err(|_| anyhow!("module lock poisoned"))?;
            calls.retain(|at| at.elapsed() < Duration::from_mins(1));
            if calls.len() >= self.limits.calls_per_minute as usize {
                bail!("this module is called more than {} times a minute", self.limits.calls_per_minute);
            }
            calls.push_back(Instant::now());
        }
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().map_err(|_| anyhow!("module lock poisoned"))?.insert(id, tx);
        let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let mut bytes = serde_json::to_vec(&line)?;
        bytes.push(b'\n');
        let written = {
            let mut stdin = self.stdin.lock().await;
            match stdin.write_all(&bytes).await {
                Ok(()) => stdin.flush().await,
                Err(error) => Err(error),
            }
        };
        if let Err(error) = written {
            self.stop(format!("couldn't be reached: {error}"));
            return Err(anyhow!("{} couldn't be reached", self.id));
        }
        match tokio::time::timeout(self.limits.time(), rx).await {
            Ok(Ok(answer)) => answer,
            Ok(Err(_)) => Err(anyhow!("{} stopped before it answered", self.id)),
            Err(_) => {
                if let Ok(mut pending) = self.pending.lock() {
                    pending.remove(&id);
                }
                let seconds = Duration::from_millis(self.limits.time_ms).as_secs_f32();
                self.stop(format!("took longer than {seconds}s"));
                bail!("{} took longer than {seconds}s and was stopped", self.id)
            }
        }
    }

    /// Stops the module with a reason the apps show.
    pub fn stop(&self, reason: String) {
        if let Ok(mut stopped) = self.stopped.lock() {
            stopped.get_or_insert(reason);
        }
        if self.alive.swap(false, Ordering::SeqCst)
            && let Ok(mut pending) = self.pending.lock()
        {
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err(anyhow!("{} stopped", self.id)));
            }
        }
    }

    /// Runs the handshake once per process, so a module that can't start says why once.
    pub async fn ready(&self, manifest: &Manifest, bot_folder: Option<&Path>) -> Result<()> {
        if self.initialized.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let answer = self
            .request(
                "initialize",
                json!({
                    "contract": super::manifest::CONTRACT,
                    "hostVersion": env!("CARGO_PKG_VERSION"),
                    "moduleId": manifest.id,
                    "botFolder": bot_folder.map(|p| p.to_string_lossy().into_owned()),
                    "files": manifest.permissions.files.name(),
                }),
            )
            .await;
        if answer.is_err() {
            self.initialized.store(false, Ordering::SeqCst);
        }
        answer.map(|_| ())
    }

    /// Tells the module something happened, without waiting for an answer.
    pub async fn notify(&self, name: &str, body: Value) -> Result<()> {
        ensure!(self.alive(), "{} is not running", self.id);
        let mut bytes = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": {"name": name, "body": body},
        }))?;
        bytes.push(b'\n');
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(&bytes).await?;
        stdin.flush().await?;
        Ok(())
    }
}

/// Answers each reply and forwards what the module says the host should hear.
async fn read(
    this: Weak<Running>,
    stdout: BufReader<tokio::process::ChildStdout>,
    events: broadcast::Sender<Value>,
    id: String,
    heard: Vec<String>,
) {
    let mut lines = stdout;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match lines.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        if buf.len() > MAX_LINE {
            if let Some(running) = this.upgrade() {
                running.stop("wrote a line larger than 1 MB".into());
            }
            return;
        }
        let Ok(msg) = serde_json::from_slice::<Value>(&buf) else { continue };
        if let Some(note) = msg.get("method").and_then(Value::as_str) {
            if note == "event"
                && let Some(name) = msg["params"]["name"].as_str()
                && heard.iter().any(|e| e == name)
            {
                let _ = events.send(json!({
                    "type": "moduleEvent",
                    "module": id,
                    "name": name,
                    "body": msg["params"]["body"],
                }));
            }
            continue;
        }
        let Some(id) = msg.get("id").and_then(Value::as_u64) else { continue };
        let Some(running) = this.upgrade() else { break };
        let Ok(mut pending) = running.pending.lock() else { break };
        if let Some(tx) = pending.remove(&id) {
            let answer = match msg.get("error") {
                Some(error) => Err(anyhow!("{}", error["message"].as_str().unwrap_or("the module failed"))),
                None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
            };
            let _ = tx.send(answer);
        }
    }
    // Its stdout closed: the process is gone or going.
    if let Some(running) = this.upgrade() {
        running.stop("exited".into());
    }
}

/// Holds the module to its memory, and notices when it exits.
async fn watch(this: Weak<Running>, mut child: tokio::process::Child, pid: Option<u32>, memory_mb: u32) {
    loop {
        tokio::time::sleep(WATCH).await;
        let Some(running) = this.upgrade() else {
            let _ = child.start_kill();
            return;
        };
        if !running.alive() {
            let _ = child.start_kill();
            return;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                running.stop(format!("exited ({status})"));
                return;
            }
            Ok(None) => {}
            Err(_) => return,
        }
        if let Some(pid) = pid
            && let Some(kb) = resident_kb(pid).await
            && kb > u64::from(memory_mb) * 1024
        {
            running.stop(format!("used more than {memory_mb} MB"));
            let _ = child.start_kill();
            return;
        }
    }
}

/// A process's resident memory in KB, where the platform tells it cheaply.
async fn resident_kb(pid: u32) -> Option<u64> {
    tokio::task::spawn_blocking(move || {
        #[cfg(target_os = "linux")]
        {
            let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
            let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
            line.split_whitespace().nth(1)?.parse().ok()
        }
        #[cfg(target_os = "macos")]
        {
            let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid.to_string()]).output().ok()?;
            String::from_utf8_lossy(&out.stdout).trim().parse().ok()
        }
        #[cfg(windows)]
        {
            // `tasklist` prints `"name","pid","session","#","12,345 K"`.
            let out = std::process::Command::new("tasklist")
                .args(["/NH", "/FO", "CSV", "/FI", &format!("PID eq {pid}")])
                .output()
                .ok()?;
            parse_tasklist(&String::from_utf8_lossy(&out.stdout))
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            let _ = pid;
            None
        }
    })
    .await
    .ok()
    .flatten()
}

/// The memory column of `tasklist`'s CSV: thousands separators and a trailing ` K`.
#[cfg(any(windows, test))]
pub(super) fn parse_tasklist(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.contains("\",\""))?;
    let field = line.rsplit("\",\"").next()?.trim_matches('"').trim();
    let digits: String = field.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Where a module writes the answer of one view: rows of text the apps draw with their own components.
pub fn sections(result: &Value) -> Result<Value> {
    let sections = result["sections"].as_array().cloned().unwrap_or_default();
    ensure!(sections.len() <= 20, "a view holds at most 20 sections");
    let mut out = Vec::with_capacity(sections.len());
    for section in sections {
        let rows = section["rows"].as_array().cloned().unwrap_or_default();
        ensure!(rows.len() <= 200, "a section holds at most 200 rows");
        out.push(json!({
            "title": section["title"].as_str().unwrap_or_default(),
            "rows": rows.into_iter().map(|row| json!({
                "label": row["label"].as_str().unwrap_or_default(),
                "value": row["value"].as_str().unwrap_or_default(),
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({
        "title": result["title"].as_str().unwrap_or_default(),
        "sections": out,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tasklists_memory_column_is_read() {
        let text = "\"node.exe\",\"4321\",\"Console\",\"1\",\"12,345 K\"\r\n";
        assert_eq!(parse_tasklist(text), Some(12_345));
        assert_eq!(parse_tasklist("INFO: No tasks are running which match the specified criteria."), None);
    }
}
