//! Teaching a bot a workflow: the user walks it through a task once, then asks it to remember
//! how. Supports both conversation step extraction (`recorded_steps`) and browser teaching
//! sessions (`start_teaching`, `finish_teaching`) with privacy-preserving DOM event capture
//! (`RECORDER`: Ideas 38, 39, 40, 49 from `bdots`).

use crate::hub::Hub;
use crate::store::{Entry, EntryKind, Lane};
use anyhow::{Result, bail};
use serde_json::{Value, json};

pub const TOOLS: [&str; 3] = ["recorded_steps", "start_teaching", "finish_teaching"];
/// Transcript entries read back, newest first.
const READ: i64 = 500;
/// Steps returned, the latest ones.
const MAX_STEPS: usize = 200;
/// Longest piece of a user message in a step.
const MAX_SAID: usize = 400;

/// Browser DOM recorder injected via `Page.addScriptToEvaluateOnNewDocument` (Ideas 38 & 39).
/// Captures semantic clicks, non-secret inputs, submits and SPA navigations while persisting
/// across full page navigations via `window.name` and `localStorage`.
pub const RECORDER: &str = r#"(() => {
  if (window.__bdotsTeachInstalled) return;
  window.__bdotsTeachInstalled = true;
  const KEY = "__bdots_teach";
  const SECRET = /pass|otp|code|cvv|card|ssn|pin|secret|token/i;
  function load() {
    let a = [];
    try { a = JSON.parse(localStorage.getItem(KEY) || "[]"); } catch {}
    try {
      if (window.name && window.name.startsWith(KEY + ":")) {
        const b = JSON.parse(window.name.slice(KEY.length + 1));
        if (Array.isArray(b) && b.length > a.length) a = b;
      }
    } catch {}
    return Array.isArray(a) ? a : [];
  }
  function push(ev) {
    const log = load();
    if (log.length >= 200) return;
    log.push({ ...ev, url: location.href, at: Date.now() });
    const s = JSON.stringify(log);
    try { localStorage.setItem(KEY, s); } catch {}
    try { window.name = KEY + ":" + s; } catch {}
  }
  function labelOf(el) {
    if (!el || !el.getAttribute) return "";
    const aria = el.getAttribute("aria-label") || el.getAttribute("placeholder") || el.getAttribute("name") || el.getAttribute("title") || "";
    if (aria.trim()) return aria.trim().slice(0, 80);
    if (el.id) {
      const lbl = document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
      if (lbl && lbl.innerText) return lbl.innerText.trim().slice(0, 80);
    }
    const p = el.closest && el.closest("label,button,a,[role=button],[role=menuitem],[role=tab]");
    const txt = ((p || el).innerText || el.value || "").trim().replace(/\s+/g, " ");
    return txt.slice(0, 80);
  }
  function isSecret(el) {
    if (!el) return false;
    const t = (el.type || "").toLowerCase();
    if (t === "password") return true;
    const ac = (el.autocomplete || "").toLowerCase();
    if (ac.includes("cc-") || ac.includes("password") || ac.includes("one-time")) return true;
    const sig = [el.name, el.id, el.placeholder, el.getAttribute && el.getAttribute("aria-label")].filter(Boolean).join(" ");
    return SECRET.test(sig);
  }
  push({ kind: "nav", title: document.title.slice(0, 80) });
  document.addEventListener("click", (e) => {
    const t = e.target && e.target.closest ? e.target.closest("a,button,input,select,textarea,[role=button],[role=link],[role=menuitem],[role=tab],[onclick],label") || e.target : e.target;
    if (!t) return;
    const tag = (t.tagName || "").toLowerCase();
    if (tag === "input" && !["button", "submit", "checkbox", "radio"].includes((t.type || "").toLowerCase())) return;
    const href = t.href && typeof t.href === "string" ? t.href : undefined;
    push({ kind: "click", tag, label: labelOf(t), href });
  }, true);
  document.addEventListener("change", (e) => {
    const t = e.target;
    if (!t || !t.tagName) return;
    const tag = t.tagName.toLowerCase();
    const type = (t.type || "").toLowerCase();
    const label = labelOf(t);
    if (isSecret(t)) {
      push({ kind: "secret", label: label || "password/code field" });
      return;
    }
    if (type === "checkbox" || type === "radio") {
      push({ kind: "toggle", label, value: t.checked ? "checked" : "unchecked" });
    } else if (tag === "select") {
      const opt = t.selectedOptions && t.selectedOptions[0];
      push({ kind: "select", label, value: ((opt && opt.text) || t.value || "").slice(0, 60) });
    } else if (tag === "input" || tag === "textarea") {
      push({ kind: "fill", label, fieldType: type || "text", sample: String(t.value || "").slice(0, 60) });
    }
  }, true);
  document.addEventListener("submit", (e) => {
    const f = e.target;
    push({ kind: "submit", label: labelOf(f) || (f && f.action) || "form" });
  }, true);
  window.__bdotsTeachRead = () => load();
})();"#;

pub fn tools() -> [Value; 3] {
    [
        json!({
            "name": "recorded_steps",
            "description": "When the user has walked you through a task and asks you to remember how: what they said and the tool steps you took in this conversation, oldest first. Write it up with save_skill.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true},
        }),
        json!({
            "name": "start_teaching",
            "description": "Begin a guided teaching session where the user demonstrates a workflow (in the browser or desktop) so you can turn it into a reusable skill with finish_teaching and save_skill.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "Optional starting URL for browser demonstration."},
                    "goal": {"type": "string", "description": "What the user is about to demonstrate."}
                }
            }
        }),
        json!({
            "name": "finish_teaching",
            "description": "End the active teaching session, collect the recorded steps and navigation history, and return a structured trace to save with save_skill.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
    ]
}

fn teach_kv_key(bot: &str) -> String {
    format!("teach.session.{bot}")
}

pub fn call(hub: &Hub, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "recorded_steps" => steps(hub, bot),
        "start_teaching" => start_teaching(hub, bot, args),
        "finish_teaching" => finish_teaching(hub, bot, args),
        _ => bail!("unknown teach tool: {name}"),
    }
}

fn start_teaching(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let url = args["url"].as_str().map(str::trim).filter(|s| !s.is_empty());
    let goal = args["goal"].as_str().map(str::trim).filter(|s| !s.is_empty());
    let state = json!({
        "startedAt": crate::store::now_ms(),
        "startSeq": hub.store.last_seq(bot),
        "url": url,
        "goal": goal,
    });
    hub.store.kv_set(&teach_kv_key(bot), &state.to_string())?;
    let notice = json!({
        "text": format!("Teaching mode started{}", goal.map_or(String::new(), |g| format!(": {g}"))),
        "style": "info",
        "teaching": {"status": "recording", "url": url, "goal": goal}
    });
    hub.add_entry(&Lane::main(bot), EntryKind::Notice, hub.store.max_turn(bot), &notice);
    Ok(json!({
        "status": "recording",
        "url": url,
        "goal": goal,
        "recorderScriptBytes": RECORDER.len(),
        "message": "Teaching session started. Ask the user to perform the steps and tell you when they are done, then call finish_teaching."
    }))
}

fn finish_teaching(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let session = hub
        .store
        .kv_get(&teach_kv_key(bot))
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));
    let _ = hub.store.kv_delete(&teach_kv_key(bot));
    let recorded = steps(hub, bot)?;
    let dom_events = args["events"].as_array().map_or(&[][..], |v| v.as_slice());
    let nav_history = args["navHistory"].as_array().map_or(&[][..], |v| v.as_slice());
    let dom_steps = format_dom_events(dom_events, nav_history);
    let notice = json!({
        "text": "Teaching session finished",
        "style": "info",
        "teaching": {"status": "finished"}
    });
    hub.add_entry(&Lane::main(bot), EntryKind::Notice, hub.store.max_turn(bot), &notice);
    Ok(json!({
        "status": "finished",
        "goal": session["goal"],
        "startUrl": session["url"],
        "steps": recorded["steps"],
        "domSteps": dom_steps,
        "instructions": "Generalize these steps into a parameterized reusable skill (never hardcode personal values) and save it with save_skill."
    }))
}

/// Formats raw DOM events captured by `RECORDER` into a human- and LLM-readable step trace,
/// collapsing consecutive duplicate navigations (Ideas 38–40).
pub fn format_dom_events(events: &[Value], nav_history: &[Value]) -> Vec<String> {
    let mut lines = Vec::new();
    for nav in nav_history.iter().take(20) {
        if let Some(url) = nav["url"].as_str().filter(|u| !u.is_empty() && *u != "about:blank") {
            let title = nav["title"].as_str().unwrap_or_default();
            let entry = format!("Visited {url}{}", if title.is_empty() { String::new() } else { format!(" ({title})") });
            if lines.last() != Some(&entry) {
                lines.push(entry);
            }
        }
    }
    for ev in events {
        let kind = ev["kind"].as_str().unwrap_or_default();
        let label = ev["label"].as_str().unwrap_or_default();
        let line = match kind {
            "nav" => {
                let url = ev["url"].as_str().unwrap_or_default();
                format!("Opened {url}")
            }
            "click" => {
                let tag = ev["tag"].as_str().unwrap_or("element");
                format!("Clicked <{tag}> \"{label}\"")
            }
            "fill" => {
                let sample = ev["sample"].as_str().unwrap_or_default();
                format!("Filled \"{label}\" (example: \"{sample}\")")
            }
            "select" => {
                let val = ev["value"].as_str().unwrap_or_default();
                format!("Selected \"{val}\" in \"{label}\"")
            }
            "toggle" => {
                let val = ev["value"].as_str().unwrap_or_default();
                format!("Toggled \"{label}\" -> {val}")
            }
            "secret" => format!("Entered secret in \"{label}\" [value never recorded]"),
            "submit" => format!("Submitted \"{label}\""),
            _ => continue,
        };
        if lines.last() != Some(&line) {
            lines.push(line);
        }
    }
    lines
}

/// One entry as a step, when it's one: the user's words, or a tool step of this bot's.
fn step(e: &Entry, bot: &str) -> Option<Value> {
    if e.kind == EntryKind::User.as_str() && e.data["from"] != "host" {
        let said: String = e.data["text"].as_str()?.chars().take(MAX_SAID).collect();
        return Some(json!({"user": said}));
    }
    // In a group, only this bot's own steps.
    let mine = e.data["author"].as_str().is_none_or(|author| author == bot);
    (e.kind == EntryKind::Tool.as_str() && mine).then(|| {
        json!({"tool": e.data["title"], "kind": e.data["toolKind"], "status": e.data["status"],
            "files": e.data["locations"]})
    })
}

fn steps(hub: &Hub, bot: &str) -> Result<Value> {
    let lane = hub.runtime(bot).lane.unwrap_or_else(|| Lane::main(bot));
    let mut entries = match &lane.thread {
        Some(root) => hub.store.thread(&lane.chat, root, READ)?,
        None => hub.store.history(&lane.chat, i64::MAX, READ)?,
    };
    entries.sort_by_key(|e| e.seq);
    let all: Vec<Value> = entries.iter().filter_map(|e| step(e, bot)).collect();
    let skip = all.len().saturating_sub(MAX_STEPS);
    Ok(json!({"steps": all[skip..].to_vec(), "earlierLeftOut": skip}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind, seq: i64, data: Value) -> Entry {
        Entry {
            id: seq.to_string(),
            seq,
            bot_id: "a".into(),
            thread_id: None,
            rev: 0,
            kind: kind.as_str().into(),
            turn: 0,
            data,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn steps_are_the_users_words_and_this_bots_tools() {
        let said = entry(EntryKind::User, 1, json!({"text": "Deploy it like this"}));
        let note = entry(EntryKind::User, 2, json!({"text": "[Connection setup completed]", "from": "host"}));
        let mine =
            entry(EntryKind::Tool, 3, json!({"title": "npm run build", "toolKind": "execute", "status": "completed"}));
        let theirs = entry(EntryKind::Tool, 4, json!({"title": "ls", "author": "b"}));
        let reply = entry(EntryKind::Agent, 5, json!({"text": "Done"}));
        assert_eq!(step(&said, "a").unwrap()["user"], "Deploy it like this");
        assert!(step(&note, "a").is_none(), "the host's own notes aren't the user's words");
        assert_eq!(step(&mine, "a").unwrap()["tool"], "npm run build");
        assert!(step(&theirs, "a").is_none(), "another bot's step in a group");
        assert!(step(&reply, "a").is_none());
    }

    #[test]
    fn dom_events_format_excludes_secrets_and_deduplicates_navigation() {
        let nav = vec![
            json!({"url": "https://example.com/login", "title": "Login"}),
            json!({"url": "https://example.com/login", "title": "Login"}),
        ];
        let events = vec![
            json!({"kind": "fill", "label": "Email", "sample": "user@example.com"}),
            json!({"kind": "secret", "label": "Password"}),
            json!({"kind": "click", "tag": "button", "label": "Sign in"}),
        ];
        let lines = format_dom_events(&events, &nav);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "Visited https://example.com/login (Login)");
        assert!(lines[2].contains("[value never recorded]"));
    }
}
