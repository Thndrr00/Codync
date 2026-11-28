#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from mercury-agent's `cli/service.ts`: launch-agent/systemd/Task Scheduler
//! parsing and building, Windows quoting and service paths. Composes helpers already ported into
//! this host. Only `botInsights` calls this.

use serde_json::Value;

/// Windows scheduled task name (WIN_TASK_NAME).
pub const WIN_TASK_NAME: &str = "MercuryAgent";

/// Per-user autostart registry key (WIN_RUN_KEY).
pub const WIN_RUN_KEY: &str = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Value under the autostart key (WIN_RUN_VALUE).
pub const WIN_RUN_VALUE: &str = "MercuryAgent";

/// Default Termux prefix (TERMUX_DEFAULT_PREFIX).
pub const TERMUX_DEFAULT_PREFIX: &str = "/data/data/com.termux/files/usr";

/// LaunchAgent label installed on macOS.
pub const LAUNCH_AGENT_LABEL: &str = "com.cosmicstack.mercury";

/// Systemd user unit installed on Linux.
pub const SERVICE_UNIT: &str = "mercury.service";

/// Decode the five XML entities used in service files.
pub fn xml_unescape(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Encode the five XML entities for a service file (xmlEscape).
pub fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Text between an XML tag pair, first occurrence.
fn tag_content(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].to_owned())
}

/// Program arguments of a launchd plist (parseLaunchAgentArgs).
pub fn parse_launch_agent_args(plist: &str) -> Vec<String> {
    let Some(block) = tag_content(plist, "array").filter(|_| plist.contains("<key>ProgramArguments</key>")) else {
        return Vec::new();
    };
    let mut args = Vec::new();
    let mut rest = block.as_str();
    while let Some(start) = rest.find("<string>") {
        rest = &rest[start + 8..];
        let Some(end) = rest.find("</string>") else {
            break;
        };
        args.push(xml_unescape(&rest[..end]));
        rest = &rest[end + 9..];
    }
    args
}

/// Words of a systemd `ExecStart=` line (parseSystemdExecStart).
pub fn parse_systemd_exec_start(unit: &str) -> Vec<String> {
    let Some(line) = unit.lines().find(|line| line.starts_with("ExecStart=")) else {
        return Vec::new();
    };
    let chars: Vec<char> = line["ExecStart=".len()..].trim().chars().collect();
    let mut words = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == ' ' {
            index += 1;
            continue;
        }
        let mut word = String::new();
        if chars[index] == '"' {
            index += 1;
            while index < chars.len() && chars[index] != '"' {
                if chars[index] == '\\' && chars.get(index + 1) == Some(&'"') {
                    word.push('"');
                    index += 2;
                } else {
                    word.push(chars[index]);
                    index += 1;
                }
            }
            index += 1;
        } else {
            while index < chars.len() && chars[index] != ' ' {
                word.push(chars[index]);
                index += 1;
            }
        }
        words.push(word);
    }
    words
}

/// Split a Windows command line into argv (CreateProcess quoting).
pub fn split_windows_command_line(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.trim().chars().collect();
    let mut words = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == ' ' {
            index += 1;
            continue;
        }
        let mut word = String::new();
        let mut quoted = false;
        while index < chars.len() && (quoted || chars[index] != ' ') {
            if chars[index] == '\\' && chars.get(index + 1) == Some(&'"') {
                word.push('"');
                index += 2;
            } else if chars[index] == '"' {
                quoted = !quoted;
                index += 1;
            } else {
                word.push(chars[index]);
                index += 1;
            }
        }
        words.push(word);
    }
    words
}

/// Command and arguments of a Task Scheduler task XML (parseWindowsTaskXml).
pub fn parse_windows_task_xml(xml: &str) -> Vec<String> {
    let Some(command) = tag_content(xml, "Command") else {
        return Vec::new();
    };
    let args = tag_content(xml, "Arguments");
    let mut out = vec![xml_unescape(command.trim())];
    if let Some(args) = args {
        out.extend(split_windows_command_line(&xml_unescape(&args)));
    }
    out
}

/// REG_SZ data of a `reg query` line (parseRegQueryValue).
pub fn parse_reg_query_value(output: &str, value_name: &str) -> Option<String> {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&format!("{value_name} ")) || line.starts_with(&format!("{value_name}\t")))?;
    for marker in ["REG_EXPAND_SZ", "REG_SZ"] {
        if let Some(index) = line.find(marker) {
            return Some(line[index + marker.len()..].trim().to_owned());
        }
    }
    None
}

/// Quote one Windows command-line argument (quoteWindowsArg).
pub fn quote_windows_arg(arg: &str) -> String {
    if arg.is_empty() {
        "\"\"".to_owned()
    } else if !arg.chars().any(|c| c.is_whitespace() || c == '"') {
        arg.to_owned()
    } else {
        format!("\"{}\"", arg.replace('"', "\\\""))
    }
}

/// Full Windows command line for a launch argv (buildWindowsCommandLine).
pub fn build_windows_command_line(args: &[String]) -> String {
    args.iter().map(|arg| quote_windows_arg(arg)).collect::<Vec<_>>().join(" ")
}

/// `schtasks /create` command for cmd.exe (buildSchtasksCreateCommand).
pub fn build_schtasks_create_command(args: &[String], task_name: &str) -> String {
    let command_line = build_windows_command_line(args).replace('"', "\\\"");
    format!("schtasks /create /tn \"{task_name}\" /tr \"{command_line}\" /sc onlogon /rl limited /f")
}

/// Single-quote one word for POSIX sh (shQuote).
pub fn sh_quote(arg: &str) -> String {
    if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_alphanumeric() || "@%+=:,./-".contains(c)) {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

/// Termux:Boot script path (termuxBootScriptPath).
pub fn termux_boot_script_path(home: &str) -> String {
    format!("{home}/.termux/boot/mercury.sh")
}

/// PATH pinned into a service file (pinnedServicePath).
pub fn pinned_service_path(platform: &str, node_bin_dir: &str, system_root: &str, program_files: &str) -> String {
    let mut dirs: Vec<String> = if platform == "win32" {
        vec![
            format!("{system_root}\\System32"),
            system_root.to_owned(),
            format!("{system_root}\\System32\\Wbem"),
            format!("{system_root}\\System32\\WindowsPowerShell\\v1.0"),
            format!("{program_files}\\Git\\cmd"),
            node_bin_dir.to_owned(),
        ]
    } else {
        let mut dirs = vec!["/usr/bin", "/bin", "/usr/sbin", "/sbin", "/usr/local/bin"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if platform == "darwin" {
            dirs.push("/opt/homebrew/bin".to_owned());
        }
        if node_bin_dir.starts_with('/') {
            dirs.push(node_bin_dir.to_owned());
        }
        dirs
    };
    dirs.dedup();
    dirs.join(if platform == "win32" { ";" } else { ":" })
}

/// Default service file path for a platform (serviceFilePath).
pub fn service_file_path(platform: &str, home: &str) -> String {
    match platform {
        "darwin" => format!("{home}/Library/LaunchAgents/{LAUNCH_AGENT_LABEL}.plist"),
        "linux" => format!("{home}/.config/systemd/user/{SERVICE_UNIT}"),
        "win32" => WIN_RUN_KEY.to_owned(),
        "android" => termux_boot_script_path(home),
        _ => String::new(),
    }
}

/// Whether a Windows service mode is one of the two known forms.
pub fn is_service_mode_valid(mode: &str) -> bool {
    matches!(mode, "task" | "run-key")
}

/// One line describing an installed service (serviceLine).
pub fn service_line(platform: &str, mode: Option<&str>, running: bool) -> String {
    format!("{platform} · {} · {}", mode.unwrap_or("none"), if running { "running" } else { "stopped" })
}

/// Whether two argv lists launch the same program (programMatches).
pub fn program_matches(installed: &[String], expected: &[String]) -> bool {
    installed.first().zip(expected.first()).is_some_and(|(a, b)| a == b)
}

/// Drift between the installed and expected launch argv (serviceDrift).
pub fn service_drift(installed: &[String], expected: &[String]) -> Value {
    if installed.is_empty() || program_matches(installed, expected) {
        return Value::Null;
    }
    serde_json::json!({"installed": installed.join(" "), "expected": expected.join(" ")})
}

/// Minimal launchd plist for a launch argv (buildLaunchAgentPlist).
pub fn launch_agent_plist(args: &[String]) -> String {
    let entries =
        args.iter().map(|arg| format!("    <string>{}</string>", xml_escape(arg))).collect::<Vec<_>>().join("\n");
    format!(
        "<plist>\n  <dict>\n    <key>Label</key>\n    <string>{LAUNCH_AGENT_LABEL}</string>\n    <key>ProgramArguments</key>\n    <array>\n{entries}\n    </array>\n  </dict>\n</plist>"
    )
}

/// Minimal systemd unit for a launch argv (buildSystemdUnit).
pub fn systemd_unit(args: &[String], working_directory: &str) -> String {
    let command = build_windows_command_line(args);
    format!(
        "[Unit]\nDescription=Mercury Agent\n\n[Service]\nType=simple\nWorkingDirectory={working_directory}\nExecStart={command}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target"
    )
}

/// Termux:Boot script for `mercury up` argv (buildTermuxBootScript).
pub fn termux_boot_script(args: &[String], mercury_home: &str) -> String {
    let command = args.iter().map(|arg| sh_quote(arg)).collect::<Vec<_>>().join(" ");
    format!(
        "#!/data/data/com.termux/files/usr/bin/sh\ncd \"$HOME\"\n{command} >> {} 2>&1\n",
        format!("{mercury_home}/boot.log")
    )
}

/// Run-key launches `mercury start`, not the daemon directly (runKeyLaunchArgs).
pub fn run_key_launch_args(args: &[String]) -> Vec<String> {
    if args.is_empty() { Vec::new() } else { vec!["mercury".to_owned(), "start".to_owned()] }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(platforms: usize, modes: usize, drift: bool, running: bool) -> Vec<(&'static str, String)> {
    vec![
        ("platforms", count_label(platforms)),
        ("modes", count_label(modes)),
        ("drift", if drift { "yes" } else { "no" }.to_owned()),
        ("running", if running { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_and_plist() {
        assert_eq!(xml_unescape("&lt;a&gt;&quot;b&quot;&apos;c&apos;&amp;"), "<a>\"b\"'c&");
        assert_eq!(xml_escape("&<>'\""), "&amp;&lt;&gt;&apos;&quot;");
        let plist = "<plist><key>ProgramArguments</key><array><string>/usr/bin/node</string><string>index.js</string></array></plist>";
        assert_eq!(parse_launch_agent_args(plist), vec!["/usr/bin/node", "index.js"]);
        assert!(parse_launch_agent_args("<plist/>").is_empty());
        let xml =
            "<Task><Command>C:\\node.exe</Command><Arguments>\"C:\\Program Files\\app.js\" --up</Arguments></Task>";
        assert_eq!(parse_windows_task_xml(xml)[0], "C:\\node.exe");
        assert_eq!(parse_windows_task_xml(xml).len(), 3);
    }

    #[test]
    fn systemd_and_registry() {
        let unit = "[Service]\nExecStart=\"/usr/bin/node\" \"/opt/app/index.js\" --up\n";
        assert_eq!(parse_systemd_exec_start(unit), vec!["/usr/bin/node", "/opt/app/index.js", "--up"]);
        assert!(parse_systemd_exec_start("[Service]").is_empty());
        let reg = "    MercuryAgent    REG_SZ    C:\\node.exe --up\n";
        assert_eq!(parse_reg_query_value(reg, "MercuryAgent").as_deref(), Some("C:\\node.exe --up"));
        assert_eq!(parse_reg_query_value("other REG_SZ x", "MercuryAgent"), None);
    }

    #[test]
    fn windows_quoting() {
        assert_eq!(quote_windows_arg(""), "\"\"");
        assert_eq!(quote_windows_arg("plain"), "plain");
        assert_eq!(quote_windows_arg("C:\\Program Files\\node.exe"), "\"C:\\Program Files\\node.exe\"");
        let args = vec!["C:\\Program Files\\node.exe".to_owned(), "index.js".to_owned()];
        assert_eq!(build_windows_command_line(&args), "\"C:\\Program Files\\node.exe\" index.js");
        assert!(build_schtasks_create_command(&args, WIN_TASK_NAME).contains("/sc onlogon"));
        assert_eq!(
            split_windows_command_line("\"C:\\Program Files\\node.exe\" index.js"),
            vec!["C:\\Program Files\\node.exe", "index.js"]
        );
        assert_eq!(run_key_launch_args(&args), vec!["mercury", "start"]);
    }

    #[test]
    fn paths_and_scripts() {
        assert_eq!(sh_quote("simple"), "simple");
        assert_eq!(sh_quote("a b"), "'a b'");
        assert_eq!(termux_boot_script_path("/home/u"), "/home/u/.termux/boot/mercury.sh");
        assert!(
            pinned_service_path("win32", "/n", "C:\\Windows", "C:\\Program Files").contains("C:\\Windows\\System32")
        );
        assert!(pinned_service_path("darwin", "/opt/node/bin", "", "").contains("/opt/homebrew/bin"));
        assert!(service_file_path("linux", "/home/u").ends_with("mercury.service"));
        assert!(is_service_mode_valid("run-key"));
        assert!(!is_service_mode_valid("other"));
        assert!(service_line("linux", Some("task"), true).contains("running"));
        assert!(termux_boot_script(&["mercury".to_owned(), "up".to_owned()], "/home/u/.mercury").contains("boot.log"));
        assert!(launch_agent_plist(&["node".to_owned()]).contains(LAUNCH_AGENT_LABEL));
        assert!(systemd_unit(&["node".to_owned()], "/home/u").contains("ExecStart=node"));
    }

    #[test]
    fn drift_and_summary() {
        let installed = vec!["node".to_owned(), "old.js".to_owned()];
        let expected = vec!["node".to_owned(), "new.js".to_owned()];
        assert!(program_matches(&installed, &expected));
        assert_eq!(service_drift(&installed, &expected), Value::Null);
        let drift = service_drift(&vec!["python".to_owned()], &expected);
        assert_eq!(drift["installed"], "python");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, false, true).len(), 4);
    }
}
