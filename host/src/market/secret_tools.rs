//! The bot's tools for its own secrets (`bot_secrets`), on the built-in `connectors` MCP server.
//! Asking posts a secure card, the same `secret` card connectors use; using one runs on the host,
//! which attaches the value and blanks it out of whatever comes back. Not a sandbox: a bot
//! running a command it wrote can still encode the value it was given.

use super::bot_secrets::{self, Auth, BotSecret, Usage};
use super::requests::text;
use crate::egress::Policy;
use crate::hub::Hub;
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::time::Duration;

const API_TIMEOUT: Duration = Duration::from_secs(30);
/// Largest `api_request` answer read.
const MAX_ANSWER: usize = 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
/// Output kept from each stream of a `run_with_secrets` command.
const MAX_OUTPUT: usize = 64 * 1024;
/// Bytes read from each stream of a command; past it the stream isn't read (the timeout ends it).
const MAX_READ: u64 = 4 * 1024 * 1024;
const REDACTED: &str = "[secret]";
/// Headers `api_request` sets itself, whatever the bot asks.
const RESERVED_HEADERS: [&str; 4] = ["authorization", "proxy-authorization", "cookie", "host"];
const MAX_HEADERS: usize = 30;

pub fn tools() -> Vec<Value> {
    let variable = format!(
        "Ask the user for a secret a shell command needs as an environment variable (e.g. a setup key). They enter it on a secure card. Then run the command with run_with_secrets, where the name is upper case with - as _ (netbird-setup-key is {}).",
        crate::shell::var_ref("NETBIRD_SETUP_KEY")
    );
    vec![
        json!({"name":"list_secrets","description":"List the secrets you keep for API calls and commands (names and where they go; never values).","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"request_api_key","description":"Ask the user for an API key or token for one HTTPS service. They enter it on a secure card; you never see it. Then call that service with api_request.","inputSchema":{"type":"object","properties":{
            "name":{"type":"string","description":"A short name, letters, digits, - or _ (e.g. openweather)."},
            "origin":{"type":"string","description":"The service's https origin, e.g. https://api.openweathermap.org"},
            "auth":{"type":"string","enum":["bearer","header","basic"],"description":"bearer: Authorization: Bearer; header: a header you name; basic: username + the secret as password."},
            "header":{"type":"string","description":"For auth=header, e.g. X-Api-Key."},
            "username":{"type":"string","description":"For auth=basic."},
            "reason":{"type":"string"}},"required":["name","origin","auth","reason"]}}),
        json!({"name":"request_secret","description":"Alias for request_api_key: ask the user for an API key or token on a secure card without exposing the value to the model.","inputSchema":{"type":"object","properties":{
            "name":{"type":"string"},
            "origin":{"type":"string"},
            "auth":{"type":"string","enum":["bearer","header","basic"]},
            "header":{"type":"string"},
            "username":{"type":"string"},
            "reason":{"type":"string"}},"required":["name","origin","reason"]}}),
        json!({"name":"request_command_variable","description":variable,"inputSchema":{"type":"object","properties":{
            "name":{"type":"string","description":"Letters, digits, - or _ (e.g. netbird-setup-key)."},
            "reason":{"type":"string"}},"required":["name","reason"]}}),
        json!({"name":"api_request","description":"Call an HTTPS API with a saved key attached. The URL must be on the key's origin; the key never appears in the answer.","inputSchema":{"type":"object","properties":{
            "secret":{"type":"string","description":"The key's name from list_secrets."},
            "method":{"type":"string","enum":["GET","POST","PUT","PATCH","DELETE","HEAD"]},
            "url":{"type":"string"},
            "headers":{"type":"object","additionalProperties":{"type":"string"}},
            "body":{"description":"A string, or JSON sent as application/json."}},"required":["secret","url"]}}),
        json!({"name":"secret_request","description":"Alias for api_request: proxy an HTTPS request attaching a saved secret out-of-band and redacting any leaked forms from the response.","inputSchema":{"type":"object","properties":{
            "secret":{"type":"string"},
            "method":{"type":"string","enum":["GET","POST","PUT","PATCH","DELETE","HEAD"]},
            "url":{"type":"string"},
            "headers":{"type":"object","additionalProperties":{"type":"string"}},
            "body":{}},"required":["secret","url"]}}),
        json!({"name":"run_with_secrets","description":"Run a shell command in your working folder with your command variables in its environment (60 s limit). Secret values are blanked out of the output.","inputSchema":{"type":"object","properties":{
            "command":{"type":"string"}},"required":["command"]}}),
    ]
}

pub fn handles(name: &str) -> bool {
    matches!(
        name,
        "list_secrets"
            | "request_api_key"
            | "request_secret"
            | "request_command_variable"
            | "api_request"
            | "secret_request"
            | "run_with_secrets"
    )
}

pub async fn call(hub: &Hub, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "list_secrets" => {
            let items: Vec<Value> = bot_secrets::for_bot(&hub.store, bot)?.iter().map(BotSecret::public).collect();
            Ok(json!({"items": items}))
        }
        "request_api_key" | "request_secret" => {
            let secret = text(args, "name")?;
            let origin = bot_secrets::origin(text(args, "origin")?)?;
            crate::egress::check(Policy::Public, &origin).await?;
            let auth_kind = args["auth"].as_str().unwrap_or("bearer");
            let auth = match auth_kind {
                "bearer" => Auth::Bearer,
                "header" => Auth::Header { header: text(args, "header")?.trim().to_owned() },
                "basic" => Auth::Basic { username: text(args, "username")?.trim().to_owned() },
                other => bail!("Unknown auth `{other}`: use bearer, header or basic"),
            };
            bot_secrets::check_auth(&auth)?;
            ask(hub, bot, secret, &Usage::Http { origin, auth }, args)
        }
        "request_command_variable" => ask(hub, bot, text(args, "name")?, &Usage::Command, args),
        "api_request" | "secret_request" => api_request(hub, bot, args).await,
        "run_with_secrets" => run(hub, bot, args).await,
        _ => bail!("Unknown secret tool"),
    }
}

fn ask(hub: &Hub, bot: &str, name: &str, usage: &Usage, args: &Value) -> Result<Value> {
    bot_secrets::check_name(name)?;
    // The card's title: the service for a key, the variable for a command secret.
    let title = match usage {
        Usage::Http { origin, .. } => origin.trim_start_matches("https://").to_owned(),
        Usage::Command => bot_secrets::env_name(name),
    };
    let destination = usage.destination(name);
    let usage = serde_json::to_value(usage)?;
    super::requests::ask_bot_secret(hub, bot, &title, name, &destination, &usage, text(args, "reason")?)
}

/// Saves the secret a `secret` card asked for; the note that resumes the bot.
pub fn save_from_card(hub: &Hub, request: &Value, value: &str) -> Result<String> {
    let bot = request["botId"].as_str().ok_or_else(|| anyhow!("The request names no bot"))?;
    let name = request["field"].as_str().ok_or_else(|| anyhow!("The request names no secret"))?;
    let usage: Usage =
        serde_json::from_value(request["botSecret"].clone()).context("The request's destination is invalid")?;
    let saved = bot_secrets::save(&hub.store, bot, name, usage, value)?;
    let how = match saved.usage {
        Usage::Http { .. } => format!("Use it with api_request (secret `{}`)", saved.name),
        Usage::Command => {
            format!(
                "Run your command with run_with_secrets; it is {}",
                crate::shell::var_ref(&bot_secrets::env_name(&saved.name))
            )
        }
    };
    Ok(format!("[Secret `{}` saved securely; its value is never shown to you. {how}. Continue the task.]", saved.name))
}

fn url_encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// The forms of `value` to blank out of what its use returns (Idea 36):
/// plain, JSON-escaped, URL-encoded (`encodeURIComponent`), Base64-encoded, and for basic auth `base64(user:pass)`.
fn secret_forms(usage: &Usage, value: &str) -> Vec<String> {
    use base64::Engine as _;
    let mut forms = vec![value.to_owned()];
    if let Ok(quoted) = serde_json::to_string(value) {
        let escaped = &quoted[1..quoted.len() - 1];
        if escaped != value {
            forms.push(escaped.to_owned());
        }
    }
    let url_encoded = url_encode_component(value);
    if url_encoded != value {
        forms.push(url_encoded);
    }
    if let Usage::Http { auth: Auth::Basic { username }, .. } = usage {
        forms.push(base64::engine::general_purpose::STANDARD.encode(format!("{username}:{value}")));
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(value);
    if b64.len() >= 4 {
        forms.push(b64);
    }
    forms.sort_by_key(|s| std::cmp::Reverse(s.len()));
    forms
}

fn redact(text: &str, secrets: &[String]) -> String {
    // Too short to replace safely: one or two characters would blank ordinary text (saved
    // values are longer; an `op://` reference may resolve to less).
    secrets.iter().filter(|s| s.len() >= 3).fold(text.to_owned(), |out, s| out.replace(s.as_str(), REDACTED))
}

async fn api_request(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let secret = bot_secrets::find(&hub.store, bot, text(args, "secret")?)?;
    let Usage::Http { origin, auth } = &secret.usage else {
        bail!("`{}` is a command variable; use run_with_secrets", secret.name);
    };
    let url = reqwest::Url::parse(text(args, "url")?).context("`url` isn't an address")?;
    // A user:password in the URL would become an Authorization header of its own.
    if url.origin().ascii_serialization() != *origin || !url.username().is_empty() || url.password().is_some() {
        bail!("`{}` only works for {origin}", secret.name);
    }
    let method = args["method"].as_str().unwrap_or("GET").to_ascii_uppercase();
    if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD") {
        bail!("Unsupported method {method}");
    }
    let own = match auth {
        Auth::Header { header } => header.to_ascii_lowercase(),
        Auth::Bearer | Auth::Basic { .. } => "authorization".to_owned(),
    };
    let headers: Vec<(&String, &Value)> = args["headers"].as_object().into_iter().flatten().collect();
    if headers.len() > MAX_HEADERS {
        bail!("At most {MAX_HEADERS} headers");
    }
    let value = super::passwords::resolve(&hub.store, &secret.value).await?;
    let mut req = crate::egress::client(Policy::Public, url.as_str())
        .await?
        .request(reqwest::Method::from_bytes(method.as_bytes())?, url.clone())
        .timeout(API_TIMEOUT);
    for (k, v) in headers {
        let lower = k.to_ascii_lowercase();
        if lower == own || RESERVED_HEADERS.contains(&lower.as_str()) {
            bail!("The `{k}` header is set by Codync, not by you");
        }
        let v = v.as_str().ok_or_else(|| anyhow!("The `{k}` header needs a text value"))?;
        req = req.header(k.as_str(), v);
    }
    req = match auth {
        Auth::Bearer => req.bearer_auth(&value),
        Auth::Header { header } => req.header(header, &value),
        Auth::Basic { username } => req.basic_auth(username, Some(&value)),
    };
    req = match &args["body"] {
        Value::Null => req,
        Value::String(s) => req.body(s.clone()),
        other => req.json(other),
    };
    let host = url.host_str().unwrap_or_default().to_owned();
    let res = req.send().await.map_err(|e| anyhow!("can't reach {host}: {}", e.without_url()))?;
    let status = res.status().as_u16();
    let content_type = res.headers().get("content-type").and_then(|v| v.to_str().ok()).map(str::to_owned);
    tracing::info!(bot, secret = %secret.name, host = %host, status, "api_request");
    let body = crate::egress::body(res, MAX_ANSWER).await?;
    let body = redact(&String::from_utf8_lossy(&body), &secret_forms(&secret.usage, &value));
    Ok(json!({"status": status, "contentType": content_type, "body": body}))
}

async fn run(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let command = text(args, "command")?;
    if command.len() > 8 * 1024 {
        bail!("The command is too long");
    }
    let row = hub.store.bot(bot)?.ok_or_else(|| anyhow!("unknown bot"))?;
    let mut vars = vec![];
    let mut values = vec![];
    for s in bot_secrets::for_bot(&hub.store, bot)?.into_iter().filter(|s| s.usage == Usage::Command) {
        let value = super::passwords::resolve(&hub.store, &s.value).await?;
        values.extend(secret_forms(&s.usage, &value));
        vars.push((bot_secrets::env_name(&s.name), value));
    }
    if vars.is_empty() {
        bail!("You have no command variables yet; ask for one with request_command_variable");
    }
    let names: Vec<&str> = vars.iter().map(|(k, _)| k.as_str()).collect();
    let (shell, _script) = crate::shell::expanding(command, &names).await.context("couldn't prepare the command")?;
    let mut cmd = shell.tokio();
    cmd.current_dir(&row.config.cwd)
        .envs(vars)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    crate::agent::process::Process::prepare(&mut cmd);
    let mut child = cmd.spawn().context("couldn't start the command")?;
    let (stdout, stderr) = (read_all(child.stdout.take()), read_all(child.stderr.take()));
    let mut tree = crate::agent::process::Process::new(child)?;
    let status = tokio::time::timeout(COMMAND_TIMEOUT, tree.wait()).await;
    // What it left running (maybe still holding the pipes, and the secrets) goes too.
    tree.kill().await;
    let status = status
        .map_err(|_| anyhow!("The command ran longer than {} s and was stopped", COMMAND_TIMEOUT.as_secs()))?
        .context("couldn't run the command")?;
    tracing::info!(bot, code = ?status.code(), "run_with_secrets");
    // Blank secrets out before cutting, so a cut can't leave part of one.
    let stream = |bytes: Vec<u8>| capped(&redact(&String::from_utf8_lossy(&bytes), &values));
    Ok(json!({"exitCode": status.code(), "stdout": stream(stdout.await??), "stderr": stream(stderr.await??)}))
}

/// Reads up to `MAX_READ` bytes of a command's output stream, alongside the command.
fn read_all(
    pipe: Option<impl tokio::io::AsyncRead + Send + Unpin + 'static>,
) -> tokio::task::JoinHandle<std::io::Result<Vec<u8>>> {
    use tokio::io::AsyncReadExt as _;
    tokio::spawn(async move {
        let mut out = vec![];
        if let Some(pipe) = pipe {
            pipe.take(MAX_READ).read_to_end(&mut out).await?;
        }
        Ok(out)
    })
}

/// The last `MAX_OUTPUT` bytes of `text` (where errors usually are), at a character boundary.
fn capped(text: &str) -> String {
    if text.len() <= MAX_OUTPUT {
        return text.to_owned();
    }
    let mut start = text.len() - MAX_OUTPUT;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("[… earlier output cut]\n{}", &text[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_blanked_out_in_plain_urlencoded_base64_and_basic_forms() {
        let usage = Usage::Http { origin: "https://a.example".into(), auth: Auth::Basic { username: "me".into() } };
        let forms = secret_forms(&usage, "sec+ret/1");
        let out = redact("plain=sec+ret/1 url=sec%2Bret%2F1 b64=c2VjK3JldC8x basic=bWU6c2VjK3JldC8x", &forms);
        assert_eq!(out, "plain=[secret] url=[secret] b64=[secret] basic=[secret]");
        assert_eq!(redact("a b", &["a".into()]), "a b");
    }

    #[test]
    fn long_output_keeps_its_end() {
        let text = format!("{}END", "x".repeat(MAX_OUTPUT + 10));
        let out = capped(&text);
        assert!(out.ends_with("END"));
        assert!(out.starts_with("[… earlier output cut]"));
        assert!(out.len() <= MAX_OUTPUT + 32);
    }

    /// A started hub with one bot, whose agent never runs (the tools don't need it).
    fn hub() -> (std::sync::Arc<Hub>, std::path::PathBuf) {
        use crate::store::{BotConfig, Store};
        let dir = std::env::temp_dir().join(format!("codync-bot-secrets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let cfg: BotConfig = serde_json::from_value(
            json!({"id":"bot","name":"Bot","backend":"fixture","cwd":dir,"command":"codync-no-such-agent","notify":false}),
        )
        .unwrap();
        store.save_bot(&cfg).unwrap();
        let identity = crate::remote::identity::Identity::load_or_create(&dir).unwrap();
        let hub = Hub::new(store, "test".into(), identity, "test".into(), 19222);
        hub.start().unwrap();
        (hub, dir)
    }

    #[tokio::test]
    async fn a_command_variable_reaches_the_command_but_not_the_bot() {
        let (hub, dir) = hub();
        let asked =
            call(&hub, "bot", "request_command_variable", &json!({"name":"setup-key","reason":"Join the network"}))
                .await
                .unwrap();
        let id = asked["requestId"].as_str().unwrap();
        let card = &hub.store.entry(id).unwrap().data["connectionRequest"];
        assert_eq!(card["kind"], "secret");
        assert_eq!(card["location"], format!("{} in commands", crate::shell::var_ref("SETUP_KEY")));
        let done = super::super::requests::finish(&hub, &json!({"entryId":id,"value":"s3cret-value"})).await.unwrap();
        assert_eq!(done["entry"]["data"]["connectionRequest"]["status"], "ready");
        let listed = call(&hub, "bot", "list_secrets", &json!({})).await.unwrap();
        assert_eq!(listed["items"][0]["name"], "setup-key");
        assert!(!listed.to_string().contains("s3cret-value"));
        let echo = format!("echo {}", crate::shell::var_ref("SETUP_KEY"));
        let ran = call(&hub, "bot", "run_with_secrets", &json!({"command":echo})).await.unwrap();
        assert_eq!(ran["exitCode"], 0);
        assert_eq!(ran["stdout"].as_str().unwrap().trim(), REDACTED);
        let history = serde_json::to_string(&hub.store.history("bot", i64::MAX, 100).unwrap()).unwrap();
        assert!(!history.contains("s3cret-value"));
        // Removed in Credentials: the next command no longer gets it.
        let secret = bot_secrets::find(&hub.store, "bot", "setup-key").unwrap();
        bot_secrets::remove(&hub.store, &secret.id).unwrap();
        assert!(call(&hub, "bot", "run_with_secrets", &json!({"command":echo})).await.is_err());
        hub.shutdown().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn api_keys_only_work_for_their_origin() {
        let (hub, dir) = hub();
        let usage = Usage::Http { origin: "https://1.1.1.1".into(), auth: Auth::Bearer };
        bot_secrets::save(&hub.store, "bot", "dns", usage, "token-value").unwrap();
        let other = call(&hub, "bot", "api_request", &json!({"secret":"dns","url":"https://8.8.8.8/x"})).await;
        assert!(other.unwrap_err().to_string().contains("only works for https://1.1.1.1"));
        let userinfo = json!({"secret":"dns","url":"https://me:pw@1.1.1.1/"});
        assert!(call(&hub, "bot", "api_request", &userinfo).await.is_err());
        let header = json!({"secret":"dns","url":"https://1.1.1.1/","headers":{"Authorization":"x"}});
        assert!(call(&hub, "bot", "api_request", &header).await.is_err());
        let local = json!({"name":"local","origin":"https://127.0.0.1","auth":"bearer","reason":"x"});
        assert!(call(&hub, "bot", "request_api_key", &local).await.is_err());
        hub.shutdown().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn every_tool_is_handled() {
        for tool in tools() {
            assert!(handles(tool["name"].as_str().unwrap()), "{tool}");
        }
    }
}
