//! Email over the account's own IMAP and SMTP servers. The protocol work lives in a small Python
//! bridge (`email_bridge.py`, standard library only) so no mail crate or OAuth app is required;
//! the host keeps the account list and the password in its encrypted vault.

use crate::hub::Hub;
use crate::market::vault;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

pub const METHODS: &[&str] = &["emailAccounts", "saveEmailAccount", "deleteEmailAccount", "emailInbox", "emailSend"];
const KEY: &str = "email-accounts.v1";
const TIMEOUT: Duration = Duration::from_secs(60);
const BRIDGE: &str = include_str!("email_bridge.py");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Account {
    id: String,
    name: String,
    address: String,
    imap_host: String,
    imap_port: u16,
    smtp_host: String,
    smtp_port: u16,
    username: String,
    #[serde(default)]
    imap_start_tls: bool,
    #[serde(default = "default_true")]
    smtp_start_tls: bool,
}

fn default_true() -> bool {
    true
}

fn password_slot(id: &str) -> String {
    format!("email-password.{id}")
}

fn accounts(store: &crate::store::Store) -> Vec<Account> {
    store.kv_read(KEY).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn save_accounts(store: &crate::store::Store, accounts: &[Account]) -> Result<()> {
    store.kv_set(KEY, &serde_json::to_string(accounts)?)
}

fn account_json(account: &Account) -> Value {
    json!({
        "id": account.id,
        "name": account.name,
        "address": account.address,
        "imapHost": account.imap_host,
        "imapPort": account.imap_port,
        "smtpHost": account.smtp_host,
        "smtpPort": account.smtp_port,
        "username": account.username,
        "imapStartTls": account.imap_start_tls,
        "smtpStartTls": account.smtp_start_tls,
    })
}

pub async fn call(hub: &Hub, method: &str, b: &Value) -> Result<Value> {
    match method {
        "emailAccounts" => {
            let accounts: Vec<Value> = accounts(&hub.store)
                .into_iter()
                .map(|account| {
                    let mut value = account_json(&account);
                    let configured = vault::read(&hub.store, &password_slot(&account.id))
                        .ok()
                        .flatten()
                        .is_some_and(|password| !password.is_empty());
                    value["configured"] = configured.into();
                    value
                })
                .collect();
            Ok(json!({"accounts": accounts}))
        }
        "saveEmailAccount" => {
            let id = b["id"].as_str().filter(|id| !id.is_empty()).map(str::to_owned);
            let mut accounts = accounts(&hub.store);
            let existing = id.as_ref().and_then(|id| accounts.iter().position(|account| &account.id == id));
            let password = b["password"].as_str().unwrap_or_default().trim();
            if existing.is_none() {
                ensure!(!password.is_empty(), "the email password is required");
                ensure!(accounts.len() < 10, "at most 10 email accounts");
            }
            let account = Account {
                id: id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                name: b["name"]
                    .as_str()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .unwrap_or("Email")
                    .chars()
                    .take(120)
                    .collect(),
                address: b["address"].as_str().unwrap_or_default().trim().to_owned(),
                imap_host: host_arg(b, "imapHost")?,
                imap_port: port_arg(b, "imapPort", 993)?,
                smtp_host: host_arg(b, "smtpHost")?,
                smtp_port: port_arg(b, "smtpPort", 587)?,
                username: b["username"]
                    .as_str()
                    .map(str::trim)
                    .filter(|u| !u.is_empty())
                    .unwrap_or_else(|| b["address"].as_str().unwrap_or_default().trim())
                    .to_owned(),
                imap_start_tls: b["imapStartTls"].as_bool().unwrap_or(false),
                smtp_start_tls: b["smtpStartTls"].as_bool().unwrap_or(true),
            };
            ensure!(account.address.contains('@'), "the email address looks incomplete");
            if !password.is_empty() {
                vault::write(&hub.store, &password_slot(&account.id), password)?;
            }
            match existing {
                Some(index) => accounts[index] = account.clone(),
                None => accounts.push(account.clone()),
            }
            save_accounts(&hub.store, &accounts)?;
            Ok(json!({"account": account_json(&account)}))
        }
        "deleteEmailAccount" => {
            let id = str_arg(b, "id")?;
            let mut accounts = accounts(&hub.store);
            let before = accounts.len();
            accounts.retain(|account| account.id != id);
            save_accounts(&hub.store, &accounts)?;
            let _ = vault::write(&hub.store, &password_slot(id), "");
            Ok(json!({"deleted": accounts.len() != before}))
        }
        "emailInbox" => {
            let (account, password) = chosen(hub, b)?;
            let limit = b["limit"].as_u64().unwrap_or(20).clamp(1, 100);
            let answer = bridge("inbox", &account, &password, json!({"limit": limit})).await?;
            Ok(answer)
        }
        "emailSend" => {
            let (account, password) = chosen(hub, b)?;
            let to = b["to"].as_str().unwrap_or_default().trim();
            ensure!(to.contains('@'), "`to` must be an email address");
            let body = b["body"].as_str().unwrap_or_default();
            let answer = bridge(
                "send",
                &account,
                &password,
                json!({"to": to, "subject": b["subject"].as_str().unwrap_or(""), "body": body}),
            )
            .await?;
            Ok(answer)
        }
        other => bail!("unknown email method {other}"),
    }
}

fn chosen(hub: &Hub, b: &Value) -> Result<(Account, String)> {
    let accounts = accounts(&hub.store);
    let account = match b["account"].as_str().filter(|id| !id.is_empty()) {
        Some(id) => accounts
            .into_iter()
            .find(|account| account.id == id || account.name == id)
            .context("unknown email account")?,
        None => accounts.into_iter().next().context("add an email account first")?,
    };
    let password = vault::read(&hub.store, &password_slot(&account.id))?.filter(|p| !p.is_empty());
    let password = password.context("that email account has no password saved")?;
    Ok((account, password))
}

fn host_arg(b: &Value, key: &str) -> Result<String> {
    let host = b[key].as_str().unwrap_or_default().trim().to_owned();
    ensure!(!host.is_empty(), "`{key}` is required");
    ensure!(!host.contains(['/', ' ']) && host.len() <= 255, "`{key}` isn't a host name");
    Ok(host)
}

fn port_arg(b: &Value, key: &str, default: u16) -> Result<u16> {
    match b[key].as_u64() {
        Some(port) => {
            u16::try_from(port).ok().filter(|port| *port > 0).with_context(|| format!("`{key}` isn't a port"))
        }
        None => Ok(default),
    }
}

fn str_arg<'a>(b: &'a Value, key: &str) -> Result<&'a str> {
    b[key].as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

fn python() -> Result<&'static str> {
    for program in ["python3", "python", "py"] {
        if std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(program);
                candidate.is_file() || candidate.with_extension("exe").is_file()
            })
        }) {
            return Ok(program);
        }
    }
    bail!("email needs Python 3 on this computer (python3, python or py on PATH)")
}

async fn bridge(action: &str, account: &Account, password: &str, extra: Value) -> Result<Value> {
    let script = std::env::temp_dir().join("codync-email-bridge.py");
    std::fs::write(&script, BRIDGE).context("writing the email bridge")?;
    let mut account = account_json(account);
    account["password"] = password.into();
    let mut request = json!({"action": action, "account": account});
    if let (Some(request), Some(extra)) = (request.as_object_mut(), extra.as_object()) {
        request.extend(extra.clone());
    }
    let mut child = Command::new(python()?)
        .arg(&script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("starting Python")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(request.to_string().as_bytes()).await?;
    }
    let output = tokio::time::timeout(TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| anyhow::anyhow!("the mail server took too long"))??;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("the email bridge answered unexpectedly: {}", stdout.trim()))?;
    if let Some(error) = value["error"].as_str() {
        bail!("{error}");
    }
    ensure!(output.status.success(), "the email bridge failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    Ok(value)
}

pub fn tools() -> Value {
    json!([
        {
            "name": "email_inbox",
            "description": "Read the newest messages in an email account the user added on this computer. Headers only: sender, subject and date. Read-only.",
            "inputSchema": {"type": "object", "properties": {
                "account": {"type": "string", "description": "Account id or name; the first account when omitted."},
                "limit": {"type": "integer", "description": "How many messages, 1 to 100. Default 10."},
            }},
            "annotations": {"readOnlyHint": true, "idempotentHint": true},
        },
        {
            "name": "email_send",
            "description": "Send an email from one of the user's accounts on this computer. This is a real send: use it only when the user asked for it.",
            "inputSchema": {"type": "object", "properties": {
                "account": {"type": "string", "description": "Account id or name; the first account when omitted."},
                "to": {"type": "string", "description": "Recipient address."},
                "subject": {"type": "string"},
                "body": {"type": "string", "description": "Plain text body."},
            }, "required": ["to", "body"]},
            "annotations": {"readOnlyHint": false, "idempotentHint": false},
        },
    ])
}

pub async fn tool_call(hub: &Hub, name: &str, args: &Value) -> Result<Value> {
    match name {
        "email_inbox" => {
            let request = json!({"account": args["account"], "limit": args["limit"].as_u64().unwrap_or(10)});
            let answer = call(hub, "emailInbox", &request).await?;
            let messages = answer["messages"].as_array().cloned().unwrap_or_default();
            if messages.is_empty() {
                return Ok(json!({"text": "The inbox has no messages."}));
            }
            let lines: Vec<String> = messages
                .iter()
                .map(|message| {
                    format!(
                        "- {} | {} | {}",
                        message["date"].as_str().unwrap_or(""),
                        message["from"].as_str().unwrap_or(""),
                        message["subject"].as_str().unwrap_or("(no subject)")
                    )
                })
                .collect();
            Ok(json!({"text": lines.join("\n")}))
        }
        "email_send" => {
            let answer = call(hub, "emailSend", args).await?;
            Ok(
                json!({"text": if answer["sent"].as_bool() == Some(true) { "Email sent." } else { "The mail server accepted it." }}),
            )
        }
        other => bail!("unknown email tool: {other}"),
    }
}
