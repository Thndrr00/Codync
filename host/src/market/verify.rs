//! Check the MCP handshake and tool discovery before reporting a connector ready,
//! and classify/persist connection failure diagnostics (`lastFailure`: `auth` | `unreachable` | `tool`, Idea 50).
//! The stored message also carries what to do next: wait for a server that says it is unavailable, check the
//! address of one that cannot be reached, or reconnect the built-in GitHub connection (OpenBot 0.75.0).
use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

fn failure_key(id: &str) -> String {
    format!("connector.failure.{id}")
}

/// The cause of an MCP connection or tool error, before it is mapped onto the client's three kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureCause {
    /// The connector needs signing in again.
    Auth,
    /// The server answered but said it isn't available (5xx / service unavailable): wait and retry.
    Unavailable,
    /// The host couldn't reach the server (refused connection, DNS failure): check the address.
    Unreachable,
    /// Everything else: a problem with the tool itself.
    Tool,
}

/// A 5xx status in a diagnostic: the server is up but not available right now.
fn has_5xx(lower: &str) -> bool {
    lower.split(|c: char| !c.is_ascii_digit()).any(|part| part.len() == 3 && part.starts_with('5'))
}

fn cause(message: &str) -> FailureCause {
    let lower = message.to_ascii_lowercase();
    if lower.contains("sign in")
        || lower.contains("credential")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("401")
        || lower.contains("403")
        || lower.contains("token")
        || lower.contains("rejected setup")
    {
        return FailureCause::Auth;
    }
    // OpenBot 0.75.0 (#12894): a server that answers "not available" and one the host cannot reach used to
    // share the general failure text. Keep them apart so the advice says what actually helps.
    if has_5xx(&lower)
        || lower.contains("service unavailable")
        || lower.contains("bad gateway")
        || lower.contains("gateway timeout")
        || lower.contains("gateway timed out")
    {
        return FailureCause::Unavailable;
    }
    if lower.contains("could not start")
        || lower.contains("timed out")
        || lower.contains("exited")
        || lower.contains("unreachable")
        || lower.contains("can't reach")
        || lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("econnrefused")
        || lower.contains("econnreset")
        || lower.contains("enotfound")
        || lower.contains("eai_again")
        || lower.contains("ehostunreach")
        || lower.contains("enetunreach")
        || lower.contains("dns error")
        || lower.contains("failed to lookup address")
        || lower.contains("no route to host")
        || lower.contains("network is unreachable")
        || lower.contains("error sending request")
    {
        return FailureCause::Unreachable;
    }
    FailureCause::Tool
}

/// Classifies an MCP connection or tool error into `"auth"`, `"unreachable"`, or `"tool"` (Idea 50).
pub fn classify_failure(message: &str) -> &'static str {
    match cause(message) {
        FailureCause::Auth => "auth",
        // `unavailable` and `unreachable` share the client's kind; the stored message carries the specific advice.
        FailureCause::Unavailable | FailureCause::Unreachable => "unreachable",
        FailureCause::Tool => "tool",
    }
}

/// What the person should do, in the stored failure's message. OpenBot 0.75.0 (#12894): wait for a server that
/// says it is unavailable; check the address and that the server is running when it cannot be reached.
pub fn failure_advice(message: &str) -> Option<&'static str> {
    match cause(message) {
        FailureCause::Auth => Some("Sign in again in Extensions, then start a new chat."),
        FailureCause::Unavailable => {
            Some("The server is not available now. Wait a few minutes, then start a new chat to try again.")
        }
        FailureCause::Unreachable => Some(
            "Check the server's address in Marketplace, Apps, and that the server is running, then start a new chat to try again.",
        ),
        FailureCause::Tool => None,
    }
}

/// OpenBot 0.75.0 (#13057): the built-in GitHub connection can fail to start for an agent while Marketplace still
/// shows GitHub as connected. Only the key Codync gave the connection counts — a saved server someone named
/// `github` is theirs, not this connection.
pub fn is_github_connection(key: &str) -> bool {
    matches!(key, "github" | "openbot_github")
}

/// The actionable text for a GitHub connection that failed to start (OpenBot 0.75.0, #13057).
pub fn github_start_failure_advice() -> &'static str {
    "GitHub did not answer, so its tools are not available. Open GitHub in Marketplace, Apps, connect it again if it asks you to, then start a new chat."
}

pub fn record_failure(store: &Store, id: &str, message: &str) {
    let mut text = message.to_owned();
    if is_github_connection(id) && cause(message) != FailureCause::Auth {
        text.push(' ');
        text.push_str(github_start_failure_advice());
    } else if let Some(advice) = failure_advice(message) {
        text.push(' ');
        text.push_str(advice);
    }
    let entry = json!({
        "kind": classify_failure(message),
        "message": text,
        "at": crate::store::now_ms(),
    });
    let _ = store.kv_set(&failure_key(id), &entry.to_string());
}

pub fn clear_failure(store: &Store, id: &str) {
    let _ = store.kv_delete(&failure_key(id));
}

pub fn last_failure(store: &Store, id: &str) -> Option<Value> {
    store.kv_get(&failure_key(id)).and_then(|s| serde_json::from_str(&s).ok())
}

pub async fn verify(store: &Store, id: &str, port: u16) -> Result<Value> {
    let res = verify_inner(store, id, port).await;
    match &res {
        Ok(_) => clear_failure(store, id),
        Err(e) => record_failure(store, id, &format!("{e:#}")),
    }
    res
}

async fn verify_inner(store: &Store, id: &str, port: u16) -> Result<Value> {
    if let Some(toolkit) = id.strip_prefix(super::composio::PREFIX) {
        let connections = super::composio::refresh(store).await?;
        if connections.iter().any(|c| c.toolkit == toolkit && c.active()) {
            return Ok(json!({"status":"ready"}));
        }
        bail!("Finish signing in before checking this connection.");
    }
    let c = super::connectors(store)?.into_iter().find(|c| c.id == id).ok_or_else(|| anyhow!("Unknown connector"))?;
    if crate::native_channels::is_native(&c) {
        return crate::native_channels::apps::verify(store, &c);
    }
    check_connector(store, &c, port).await
}

/// The same check for a definition that doesn't have to be saved yet.
pub(super) async fn check_connector(store: &Store, c: &super::Connector, port: u16) -> Result<Value> {
    // A header name saved by an older client (or an agent) is refused on the next
    // test/edit too, with the same message the form shows (OpenBot triage-13174).
    super::mcp_headers::check_headers(&c.headers)?;
    if c.oauth.as_ref().is_some_and(|o| !o.signed_in()) {
        bail!("Sign in before checking this connection.");
    }
    let mut command = if let Some(exe) = &c.command {
        // Found and started as `mcp::serve_local` does for a bot, so what passes here is what runs:
        // the agent starts it in its bot's folder, which a relative path in the command line means.
        let mut command = Command::new(crate::shell::program(exe));
        command.args(&c.args);
        for (name, value) in &c.env {
            command.env(name, super::passwords::resolve(store, value).await?);
        }
        let mut folders = super::bots_using(store, &c.id)?.into_iter().map(|row| row.config.cwd);
        if let Some(folder) = folders.find(|cwd| Path::new(cwd).is_dir()) {
            command.current_dir(folder);
        }
        command
    } else {
        let mut command = Command::new(std::env::current_exe()?);
        command.args(["mcp", "remote", "--connector", &c.id, "--port", &port.to_string()]);
        command
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| anyhow!("Could not start the connector. Install its runtime on this computer and retry."))?;
    let mut input = child.stdin.take().ok_or_else(|| anyhow!("Connector input unavailable"))?;
    let output = child.stdout.take().ok_or_else(|| anyhow!("Connector output unavailable"))?;
    let mut lines = BufReader::new(output).lines();
    let check = async {
        for (id, method, params) in [
            (
                1,
                "initialize",
                json!({"protocolVersion":crate::mcp::PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"Codync","version":env!("CARGO_PKG_VERSION")}}),
            ),
            (2, "tools/list", json!({})),
        ] {
            let wire = format!("{}\n", json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
            input.write_all(wire.as_bytes()).await?;
            input.flush().await?;
            loop {
                let line = lines.next_line().await?.ok_or_else(|| anyhow!("Connector exited during setup"))?;
                if line.len() > 4 * 1024 * 1024 {
                    bail!("Connector response too large");
                }
                let Ok(reply) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if reply["id"] != id {
                    continue;
                }
                if reply.get("error").is_some() {
                    bail!("Connector rejected setup. Check its credentials and try again.");
                }
                if id == 1 {
                    if !reply["result"]["protocolVersion"].is_string() {
                        bail!("Invalid MCP handshake");
                    }
                    input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").await?;
                } else {
                    let tools = reply["result"]["tools"]
                        .as_array()
                        .ok_or_else(|| anyhow!("Connector did not return its tools"))?;
                    return Ok(json!({"status":"ready","toolCount":tools.len()}));
                }
                break;
            }
        }
        bail!("Connector verification did not complete")
    };
    let result = tokio::time::timeout(Duration::from_secs(90), check)
        .await
        .map_err(|_| anyhow!("Connector setup timed out. Check the runtime and connection, then retry."));
    let _ = child.kill().await;
    let _ = child.wait().await;
    result?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_classification_distinguishes_auth_unreachable_and_tool() {
        assert_eq!(classify_failure("Sign in before checking this connection."), "auth");
        assert_eq!(classify_failure("Connector rejected setup. Check its credentials and try again."), "auth");
        assert_eq!(classify_failure("Could not start the connector."), "unreachable");
        assert_eq!(classify_failure("Connector setup timed out."), "unreachable");
        assert_eq!(classify_failure("Connector did not return its tools"), "tool");
    }

    #[test]
    fn an_unavailable_server_and_an_unreachable_one_get_different_advice() {
        // OpenBot 0.75.0 (#12894): a 5xx answer means wait; a refused connection or DNS failure means check
        // the address and that the server is running.
        let unavailable = "MCP server returned HTTP 503 Service Unavailable";
        assert_eq!(cause(unavailable), FailureCause::Unavailable);
        assert_eq!(classify_failure(unavailable), "unreachable");
        assert!(failure_advice(unavailable).unwrap().contains("Wait a few minutes"));

        for message in [
            "error sending request: connection refused",
            "dial tcp: lookup mcp.example.com: ENOTFOUND",
            "no route to host",
        ] {
            assert_eq!(cause(message), FailureCause::Unreachable, "{message}");
            assert!(failure_advice(message).unwrap().contains("address"), "{message}");
        }
        assert_eq!(cause("Connector did not return its tools"), FailureCause::Tool);
        assert!(failure_advice("Connector did not return its tools").is_none());
        assert!(failure_advice("Sign in before checking this connection.").unwrap().contains("Sign in again"));
    }

    #[test]
    fn a_failed_github_connection_says_what_to_do() {
        // OpenBot 0.75.0 (#13057): the built-in GitHub tools can be missing while the row says connected.
        assert!(is_github_connection("github"));
        assert!(is_github_connection("openbot_github"));
        assert!(!is_github_connection("my-github-server"), "a user's own server is not this connection");
        assert!(github_start_failure_advice().contains("Marketplace"));

        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        record_failure(&store, "github", "Could not start the connector.");
        let saved = last_failure(&store, "github").expect("the failure");
        assert!(saved["message"].as_str().unwrap().contains("GitHub did not answer"));
        assert_eq!(saved["kind"], "unreachable");
    }
}
