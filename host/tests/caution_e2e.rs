//! A real host driving a scripted ACP agent: a call that can leave the computer is checked for a
//! credential, and its row says so without copying the value. A local read stays quiet.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Child;
use std::time::{Duration, Instant};

struct Host {
    child: Child,
    base: String,
    token: String,
    home: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        common::stop(&mut self.child);
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

impl Host {
    async fn start() -> Self {
        let home = std::env::temp_dir().join(format!("codync-caution-e2e-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        let (child, base) = common::serve(&home).await;
        let token = std::fs::read_to_string(home.join("token")).unwrap().trim().to_owned();
        Self { child, base, token, home }
    }

    async fn call(&self, method: &str, body: Value) -> Value {
        let res = reqwest::Client::new()
            .post(format!("{}/api/{method}", self.base))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = res.status();
        let v: Value = res.json().await.unwrap();
        assert!(status.is_success(), "{method} failed: {v}");
        v
    }

    /// A bot running `fixture` in its own folder (`home/<name>`).
    async fn bot(&self, name: &str, fixture: &str) -> String {
        let cwd = self.home.join(name);
        std::fs::create_dir_all(&cwd).unwrap();
        let agent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(fixture);
        let bot = json!({
            "name": name, "backend": "custom", "command": common::python_agent(&agent),
            "cwd": cwd, "notify": false,
        });
        self.call("createBot", bot).await["bot"]["id"].as_str().unwrap().to_owned()
    }

    async fn entries(&self, chat: &str) -> Vec<Value> {
        self.call("history", json!({"botId": chat})).await["entries"].as_array().unwrap().clone()
    }

    /// Polls `chat`'s history until `pred` matches an entry.
    async fn wait_for(&self, chat: &str, within: Duration, pred: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + within;
        loop {
            if let Some(e) = self.entries(chat).await.into_iter().rev().find(|e| pred(e)) {
                return e;
            }
            assert!(Instant::now() < deadline, "timed out; history: {:?}", self.entries(chat).await);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[tokio::test]
async fn a_call_that_carries_a_password_says_so_without_copying_it() {
    if common::python_missing() {
        return;
    }
    let host = Host::start().await;
    let bot = host.bot("Mailer", "tests/fixtures/caution_agent.py").await;
    host.call("send", json!({"botId": bot, "text": "Send the report"})).await;
    let call = host
        .wait_for(&bot, Duration::from_mins(1), |e| {
            e["kind"] == "tool" && e["data"]["title"] == "mcp__gmail__send_message"
        })
        .await;
    let caution = &call["data"]["caution"];
    assert_eq!(caution["reason"], "sensitive_content");
    assert_eq!(caution["findings"][0]["category"], "credential_field");
    assert_eq!(caution["findings"][0]["path"], "$.password");
    assert!(!caution.to_string().contains("hunter2"), "the value stays out: {call}");
    // The edit carries a token in its arguments but stays on this computer, so it reads clean.
    let edit = host.entries(&bot).await.into_iter().find(|e| e["data"]["title"] == "Edit notes.md").unwrap();
    assert!(edit["data"].get("caution").is_none(), "a local edit stays quiet: {edit}");
}
