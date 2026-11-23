//! pi-acp takes a session's `mcpServers` and never hands them to pi (its README's "Limitations"), so a bot on pi
//! had none of Codync's tools. pi-acp starts the command in `PI_ACP_PI_COMMAND`: Codync's launcher, which starts pi
//! with an extension that registers the servers it finds in `CODYNC_PI_MCP_SERVERS` (`pi.registerMcpServer`, pi
//! 0.99 or later).
// ponytail: drop this once pi-acp passes `mcpServers` to pi itself (svkozak/pi-acp#164).

use super::codex_names::{self, Renamed};
use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Older pi has no `registerMcpServer` and starts without the tools. With pi-acp's own bridge (`PI_ACP_MCP_SERVERS`)
/// pi already has them, and registering them twice would stop pi. A server that can't register (another extension
/// already has its name) is the only one left out: pi doesn't start when an extension throws.
const EXTENSION: &str = r#"export default function (pi) {
  if (typeof pi.registerMcpServer !== "function" || process.env.PI_ACP_MCP_SERVERS) return;
  const servers = JSON.parse(process.env.CODYNC_PI_MCP_SERVERS || "{}");
  for (const [name, config] of Object.entries(servers)) {
    try {
      pi.registerMcpServer(name, config);
    } catch (e) {
      console.error(`codync: ${name}: ${e.message}`);
    }
  }
}
"#;

/// pi-acp's own command for pi, plus the extension beside the launcher.
#[cfg(unix)]
const LAUNCHER: (&str, &str) = ("pi-codync", "#!/bin/sh\nexec pi -e \"$(dirname \"$0\")/codync.js\" \"$@\"\n");
#[cfg(windows)]
const LAUNCHER: (&str, &str) = ("pi-codync.cmd", "@pi.cmd -e \"%~dp0codync.js\" %*\r\n");

/// Seconds pi waits for a tool call (60 by default): past the host's own limit for any of them (`mcp::call`, the
/// longest a team change waiting for its approval).
const CALL_TIMEOUT: u64 = crate::chat::manage::APPROVAL_TIMEOUT.as_secs() + 60;

/// The environment that gives pi `servers` (ACP `mcpServers`).
pub(super) async fn env(servers: &Value) -> Result<Vec<(String, String)>> {
    let dir = crate::service::data_dir().join("pi");
    let launcher = tokio::task::spawn_blocking(move || write_launcher(&dir)).await??;
    Ok(vec![
        ("PI_ACP_PI_COMMAND".into(), launcher.to_string_lossy().into_owned()),
        ("CODYNC_PI_MCP_SERVERS".into(), pi_servers(servers).to_string()),
    ])
}

/// Renames each of `servers` whose name pi's own MCP config has (`mcp.json` in pi's agent folder or the project's
/// `.pi`, disabled entries too): pi leaves out a registered server its config names. `agent_dir` is the agent's
/// own `PI_CODING_AGENT_DIR` when it has one.
pub(super) async fn avoid_clashes(servers: &mut Value, cwd: &str, agent_dir: Option<PathBuf>) -> Renamed {
    let agent_dir = agent_dir
        .or_else(|| std::env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from))
        .or_else(|| dirs::home_dir().map(|h| h.join(".pi/agent")));
    let files = agent_dir.into_iter().chain([Path::new(cwd).join(".pi")]).map(|dir| dir.join("mcp.json")).collect();
    codex_names::rename_taken(servers, files, taken).await
}

/// Every server name in the `mcpServers` of `files`.
fn taken(files: &[PathBuf]) -> HashSet<String> {
    files
        .iter()
        .filter_map(|file| std::fs::read_to_string(file).ok())
        .filter_map(|text| serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')).ok())
        .filter_map(|config| config["mcpServers"].as_object().map(|s| s.keys().cloned().collect::<Vec<_>>()))
        .flatten()
        .collect()
}

/// Blocking: the launcher and the extension in `dir`, each written beside the old one and renamed over it, and
/// only when it changed, so a pi starting meanwhile never reads half a file.
fn write_launcher(dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let launcher = dir.join(LAUNCHER.0);
    for (path, text) in [(dir.join("codync.js"), EXTENSION), (launcher.clone(), LAUNCHER.1)] {
        if std::fs::read_to_string(&path).is_ok_and(|t| t == text) {
            continue;
        }
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        std::fs::write(&temporary, text).with_context(|| format!("writing {}", temporary.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&temporary, &path).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(launcher)
}

/// ACP's `[{name, command, args, env: [{name, value}]}]` as pi's `{name: {command, args, env: {name: value}}}`.
/// Tools are declared directly (`direct`) while they fit. OpenBot 0.77.0: when there are many of them or
/// Computer Use fills the schema budget, the CLI may drop its tool search, leaving file upload and other
/// own tools undiscoverable, so `tool_search_visible` keeps the declaration's search path instead.
fn pi_servers(servers: &Value) -> Value {
    let list: Vec<&Value> = servers.as_array().into_iter().flatten().collect();
    let computer_use = list.iter().any(|s| s["name"].as_str() == Some("computer"));
    let keep_search = crate::agent::approval_rules::tool_search_visible("pi", list.len(), computer_use);
    let mut out = Map::new();
    for s in list {
        let Some(name) = s["name"].as_str() else { continue };
        let env: Map<String, Value> = s["env"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| Some((e["name"].as_str()?.to_owned(), e["value"].clone())))
            .collect();
        let mut server = json!({"command": s["command"], "args": s["args"], "env": env, "timeout": CALL_TIMEOUT});
        if !keep_search {
            server["exposure"] = json!("direct");
        }
        out.insert(name.to_owned(), server);
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acp_servers_become_direct_pi_servers() {
        let acp = json!([{"name": "manage", "command": "/bin/codync-host", "args": ["mcp", "manage"],
            "env": [{"name": "CODYNC_HOME", "value": "/h"}]}]);
        assert_eq!(
            pi_servers(&acp),
            json!({"manage": {"command": "/bin/codync-host", "args": ["mcp", "manage"],
                "env": {"CODYNC_HOME": "/h"}, "exposure": "direct", "timeout": 330}})
        );
    }

    #[test]
    fn many_or_computer_use_tools_keep_pis_tool_search() {
        let computer = json!([{"name": "computer", "command": "x"}]);
        assert!(pi_servers(&computer)["computer"].get("exposure").is_none());
        let many: Value = (0..crate::agent::approval_rules::MANY_MCP_TOOLS)
            .map(|i| json!({"name": format!("mcp-{i}"), "command": "x"}))
            .collect();
        let declared = pi_servers(&many);
        assert_eq!(declared.as_object().map_or(0, Map::len), crate::agent::approval_rules::MANY_MCP_TOOLS);
        assert!(declared["mcp-0"].get("exposure").is_none(), "tool search stays available");
    }

    #[test]
    fn the_launcher_is_written_once() {
        let dir = std::env::temp_dir().join(format!("codync-pi-tools-{}", uuid::Uuid::new_v4()));
        let launcher = write_launcher(&dir).unwrap();
        let written = std::fs::metadata(&launcher).unwrap().modified().unwrap();
        assert_eq!(write_launcher(&dir).unwrap(), launcher);
        assert_eq!(std::fs::metadata(&launcher).unwrap().modified().unwrap(), written);
        assert_eq!(std::fs::read_to_string(dir.join("codync.js")).unwrap(), EXTENSION);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2, "no temporary file is left");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn servers_pis_config_names_get_a_free_name() {
        let dir = std::env::temp_dir().join(format!("codync-pi-names-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("agent")).unwrap();
        std::fs::create_dir_all(dir.join("app/.pi")).unwrap();
        std::fs::write(dir.join("agent/mcp.json"), "\u{feff}{\"mcpServers\": {\"memory\": {\"enabled\": false}}}")
            .unwrap();
        std::fs::write(dir.join("app/.pi/mcp.json"), r#"{"mcpServers": {"google_drive": {"command": "x"}}}"#).unwrap();
        let mut servers = json!([{"name": "memory"}, {"name": "google-drive"}, {"name": "chat"}]);
        let cwd = dir.join("app").display().to_string();
        let renamed = avoid_clashes(&mut servers, &cwd, Some(dir.join("agent"))).await;
        assert_eq!(servers, json!([{"name": "codync-memory"}, {"name": "codync-google-drive"}, {"name": "chat"}]));
        assert_eq!(renamed.len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
