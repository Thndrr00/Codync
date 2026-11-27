#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `command-policy.ts`: shell parsing, read-only calls,
//! secret paths, deny rules, merges, pushes and approval wording. Composes helpers already ported
//! into this host. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Runner home of system-mode agents (AGENT_WORKER_HOMES).
pub const AGENT_WORKER_HOMES: &str = "/var/lib/agentes-runner-agents";

/// Runner home of the system install (SYSTEM_RUNNER_HOME).
pub const SYSTEM_RUNNER_HOME: &str = "/var/lib/agentes-runner";

/// Browser profile holding agent logins (AGENT_BROWSER_PROFILE).
pub const AGENT_BROWSER_PROFILE: &str = ".agentes/browser";

/// Kinds on the hard deny list.
pub const DENY_KINDS: [&str; 5] = ["merge", "deploy", "rollback", "publish", "gh api"];

/// Ways to ask a person (ASK_IN_CHAT, ASK_AGAIN, ASK_AFTER_TASK).
pub const ASK_MESSAGES: [&str; 3] = ["Ask in chat", "Ask again", "Ask after task"];

/// The rule that a refusal never gives a terminal command (NO_TERMINAL_COMMANDS).
pub const NO_TERMINAL_COMMANDS: &str = "Never give the person a terminal command to run.";

/// Paths holding keys or tokens, anywhere.
const SECRET_DIRS: [&str; 13] = [
    ".agentes/browser",
    ".ssh",
    ".aws",
    ".gnupg",
    ".kube",
    ".docker",
    ".azure",
    ".gcloud",
    ".password-store",
    "Keychains",
    ".agentes-runner",
    "agentes-runner-agents",
    ".config/gh",
];

/// Files holding tokens, anywhere.
const SECRET_FILES: [&str; 10] = [
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".git-credentials",
    ".pgpass",
    "credentials",
    "credentials.json",
    ".credentials.json",
    "mcp-token",
    "host-computer-token",
];

/// Refuse a word, keeping the operation marker separate.
fn push_word(out: &mut Vec<Value>, cur: &mut String, has: &mut bool) {
    if *has {
        out.push(json!({"text": cur.as_str()}));
        cur.clear();
        *has = false;
    }
}

/// Shell words with quotes removed and operators split out (shellWords).
pub fn shell_words(line: &str) -> Option<Vec<Value>> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut has = false;
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        match ch {
            '\'' => {
                let end = chars.iter().enumerate().skip(index + 1).find(|(_, c)| **c == '\'').map(|(i, _)| i)?;
                cur.extend(chars[index + 1..end].iter());
                has = true;
                index = end + 1;
            }
            '"' => {
                let mut end = index + 1;
                while end < chars.len() && chars[end] != '"' {
                    match chars[end] {
                        '$' | '`' => return None,
                        '\\' => {
                            end += 1;
                            if end >= chars.len() {
                                return None;
                            }
                            cur.push(chars[end]);
                        }
                        other => cur.push(other),
                    }
                    end += 1;
                }
                if end >= chars.len() {
                    return None;
                }
                has = true;
                index = end + 1;
            }
            '\\' => {
                if index + 1 >= chars.len() {
                    return None;
                }
                cur.push(chars[index + 1]);
                has = true;
                index += 2;
            }
            '$' | '`' | '(' | ')' | '{' | '}' => return None,
            ' ' | '\t' => {
                push_word(&mut out, &mut cur, &mut has);
                index += 1;
            }
            '\n' | ';' => {
                push_word(&mut out, &mut cur, &mut has);
                out.push(json!({"text": ";", "op": "list"}));
                index += 1;
            }
            '|' => {
                push_word(&mut out, &mut cur, &mut has);
                if chars.get(index + 1) == Some(&'|') {
                    out.push(json!({"text": "||", "op": "list"}));
                    index += 2;
                } else if chars.get(index + 1) == Some(&'&') {
                    return None;
                } else {
                    out.push(json!({"text": "|", "op": "pipe"}));
                    index += 1;
                }
            }
            '&' => {
                if chars.get(index + 1) == Some(&'&') {
                    push_word(&mut out, &mut cur, &mut has);
                    out.push(json!({"text": "&&", "op": "list"}));
                    index += 2;
                } else if chars.get(index + 1) == Some(&'>') {
                    push_word(&mut out, &mut cur, &mut has);
                    let append = chars.get(index + 2) == Some(&'>');
                    out.push(json!({"text": if append { "&>>" } else { "&>" }, "op": "redirect"}));
                    index += if append { 3 } else { 2 };
                } else {
                    return None;
                }
            }
            '>' | '<' => {
                if ch == '<' && chars.get(index + 1) == Some(&'<') {
                    return None;
                }
                let fd = if cur.len() == 1 && cur.chars().all(|c| c.is_ascii_digit()) {
                    let value = cur.clone();
                    cur.clear();
                    has = false;
                    value
                } else {
                    push_word(&mut out, &mut cur, &mut has);
                    String::new()
                };
                if ch == '>' {
                    let append = chars.get(index + 1) == Some(&'>');
                    out.push(json!({"text": format!("{fd}{}", if append { ">>" } else { ">" }), "op": "redirect"}));
                    index += if append { 2 } else { 1 };
                } else {
                    out.push(json!({"text": "<", "op": "input"}));
                    index += 1;
                }
            }
            other => {
                cur.push(other);
                has = true;
                index += 1;
            }
        }
    }
    push_word(&mut out, &mut cur, &mut has);
    Some(out)
}

/// Simple commands of a line: pipelines and lists split (simpleCommands).
pub fn simple_commands(line: &str) -> Option<Vec<Value>> {
    let words = shell_words(line)?;
    let mut out = Vec::new();
    let mut argv = Vec::new();
    let mut writes = Vec::new();
    let mut reads = Vec::new();
    let mut piped = false;
    let mut index = 0;
    while index < words.len() {
        let word = &words[index];
        match word["op"].as_str().unwrap_or("") {
            "pipe" | "list" => {
                if argv.is_empty() {
                    return None;
                }
                out.push(json!({"argv": argv, "writes": writes, "reads": reads, "piped": piped}));
                argv = Vec::new();
                writes = Vec::new();
                reads = Vec::new();
                piped = word["op"].as_str() == Some("pipe");
            }
            "redirect" | "input" => {
                let target = words.get(index + 1)?;
                if !target["op"].is_null() {
                    return None;
                }
                index += 1;
                let text = target["text"].as_str().unwrap_or("");
                if word["op"].as_str() == Some("input") {
                    reads.push(text.to_owned());
                } else {
                    let is_fd = text.len() == 2 && text.starts_with('&') && text.as_bytes()[1].is_ascii_digit();
                    if !is_fd && text != "/dev/null" {
                        writes.push(text.to_owned());
                    }
                }
            }
            _ => argv.push(word["text"].as_str().unwrap_or("").to_owned()),
        }
        index += 1;
    }
    if argv.is_empty() {
        return if out.is_empty() { Some(Vec::new()) } else { None };
    }
    out.push(json!({"argv": argv, "writes": writes, "reads": reads, "piped": piped}));
    let assignment = |name: &str| {
        let mut chars = name.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.contains('=')
            && name.split('=').next().is_some_and(|part| part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    };
    if out.iter().any(|command| command["argv"][0].as_str().is_some_and(assignment)) {
        return None;
    }
    Some(out)
}

/// Normalize a path relative to cwd, accepting `~` from the scope.
fn resolve_path(scope: &Value, path: &str) -> Option<String> {
    let home = scope["home"].as_str().unwrap_or("");
    let expanded = if path == "~" {
        home.to_owned()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else {
        path.to_owned()
    };
    if expanded.starts_with('~') || expanded.is_empty() {
        return None;
    }
    let cwd = scope["cwd"].as_str().unwrap_or("/");
    let joined = if expanded.starts_with('/') { expanded } else { format!("{cwd}/{expanded}") };
    let mut parts: Vec<&str> = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(format!("/{}", parts.join("/")))
}

/// Whether a path is the root itself or under it.
fn under(path: &str, root: &str) -> bool {
    path == root || path.strip_prefix(root).is_some_and(|rest| rest.starts_with('/'))
}

/// Inside a mission folder or the task's own (insideFolders).
pub fn inside_folders(scope: &Value, path: &str) -> bool {
    let Some(resolved) = resolve_path(scope, path) else {
        return false;
    };
    scope["folders"].as_array().is_some_and(|folders| {
        folders.iter().filter_map(Value::as_str).any(|folder| {
            let scope_without_home =
                json!({"cwd": scope["cwd"].as_str().unwrap_or("/"), "home": scope["home"].as_str().unwrap_or("")});
            resolve_path(&scope_without_home, folder).is_some_and(|root| under(&resolved, &root))
        })
    })
}

/// Whether a path is an agent browser profile (inBrowserProfile).
pub fn in_browser_profile(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split(|c| c == '/' || c == '\\').collect();
    parts.windows(2).any(|pair| pair[0] == ".agentes" && pair[1] == "browser")
}

/// Whether a read touches keys or tokens (isSecretRead).
pub fn is_secret_read(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split(|c| c == '/' || c == '\\').filter(|part| !part.is_empty()).collect();
    if parts.iter().any(|part| {
        SECRET_DIRS.iter().any(|dir| {
            dir.split('/').all(|piece| parts.contains(&piece)) && part == dir.split('/').last().unwrap_or("")
        })
    }) {
        return true;
    }
    let name = parts.last().copied().unwrap_or("");
    if SECRET_FILES.contains(&name) {
        return true;
    }
    if name.starts_with(".env")
        && !matches!(name, ".env.example" | ".env.sample" | ".env.template" | ".env.dist" | ".env.defaults")
    {
        return true;
    }
    if name.starts_with("id_") || name.ends_with(".pem") || name.ends_with(".key") {
        return true;
    }
    if lower.contains(".codex/auth.json") || lower.contains("accounts/") && name == "auth.json" {
        return true;
    }
    parts.first() == Some(&"~") && parts.get(1) == Some(&".agentes")
}

/// Whether a tool call reads a secret (secretRead).
pub fn secret_read(tool_name: &str, path: &str) -> bool {
    if !matches!(tool_name, "Bash" | "Read" | "Glob" | "Grep") {
        return false;
    }
    is_secret_read(path) || path.to_ascii_lowercase().contains("token")
}

/// Command words with git/gh global options normalized (normalizedCommand).
pub fn normalized_command(words: &[String]) -> String {
    let mut parts = words.to_vec();
    if let Some(first) = parts.first_mut() {
        *first = first.rsplit('/').next().unwrap_or(first).to_owned();
    }
    let program = parts.first().cloned().unwrap_or_default();
    let mut rest = parts.split_off(1);
    if program == "git" {
        let mut index = 0;
        while index < rest.len() {
            let word = rest[index].as_str();
            if matches!(word, "-C" | "-c" | "--git-dir" | "--work-tree" | "--exec-path") {
                index += 2;
            } else if word.starts_with("-") {
                index += 1;
            } else {
                break;
            }
        }
        rest = rest.split_off(index);
    }
    let cleaned: Vec<String> = rest
        .into_iter()
        .map(|word| {
            word.strip_prefix("refs/heads/").map_or(word.clone(), |value| value.to_owned()).replace(":refs/heads/", ":")
        })
        .collect();
    let mut out = vec![program];
    out.extend(cleaned);
    out.join(" ")
}

/// Words of each simple command in a command string (commandWords).
pub fn command_words(command: &str) -> Vec<Vec<String>> {
    if let Some(commands) = simple_commands(command) {
        return commands
            .iter()
            .map(|item| {
                item["argv"]
                    .as_array()
                    .map_or(Vec::new(), |values| values.iter().filter_map(Value::as_str).map(str::to_owned).collect())
            })
            .collect();
    }
    command.lines().map(|line| line.split_whitespace().map(str::to_owned).collect()).collect()
}

/// Message asking for approval (approvalMessage).
pub fn approval_message(what: &str, how: &str, instead: Option<&str>) -> String {
    let mut message = format!("{what}. {how}");
    if let Some(instead) = instead {
        message.push_str(&format!(" Instead: {instead}"));
    }
    message
}

/// Refusal from the deny list (deniedMessage).
pub fn denied_message(tool_name: &str, command: &str, how: &str) -> Option<String> {
    let normalized =
        command_words(command).first().map(|words| normalized_command(words)).unwrap_or_else(|| command.to_owned());
    let lower = normalized.to_ascii_lowercase();
    if lower.contains("gh api")
        || lower.contains("printenv")
        || lower.contains("echo $")
        || (lower.contains("curl") && lower.contains("token"))
    {
        return Some("Secrets must never be printed or sent. Use a request that keeps them hidden.".to_owned());
    }
    let what = if lower.contains("git merge") || lower.contains("gh pr merge") {
        Some("A merge needs a person")
    } else if lower.contains("terraform apply") || lower.contains("kubectl apply") || lower.contains("deploy") {
        Some("A deploy needs a person")
    } else if lower.contains("rollback") || lower.contains("git revert") {
        Some("A rollback needs a person")
    } else if lower.contains("npm publish") || lower.contains("cargo publish") || lower.contains("publish") {
        Some("Publishing needs a person")
    } else {
        None
    };
    let _ = tool_name;
    what.map(|what| approval_message(what, how, None))
}

/// Whether a call is on the hard deny list (deniedInMissions).
pub fn denied_in_missions(tool_name: &str, command: &str) -> bool {
    denied_message(tool_name, command, ASK_MESSAGES[0]).is_some()
}

/// `owner/name` of a GitHub repository (githubRepo).
pub fn github_repo(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let proxied =
        trimmed.split("/api/git/").nth(1).and_then(|rest| rest.split('/').nth(2..).map(|parts| parts.join("/")));
    let source = proxied.as_deref().unwrap_or(trimmed);
    let stripped = source
        .strip_prefix("https://github.com/")
        .or_else(|| source.strip_prefix("ssh://git@github.com/"))
        .or_else(|| source.strip_prefix("git@github.com:"))
        .unwrap_or(source)
        .trim_end_matches('/');
    let repo = stripped.strip_suffix(".git").unwrap_or(stripped);
    let valid = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    };
    let mut parts = repo.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(name), None) if valid(owner) && valid(name) => Some(format!("{owner}/{name}")),
        _ => None,
    }
}

/// A merge the computer may run without a person (loneMerge).
pub fn lone_merge(text: &str) -> Option<Value> {
    let lines: String = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let commands = simple_commands(&lines)?;
    let last = commands.last()?;
    if commands.iter().any(|command| {
        command["piped"] == true
            || command["writes"].as_array().is_some_and(|v| !v.is_empty())
            || command["reads"].as_array().is_some_and(|v| !v.is_empty())
    }) {
        return None;
    }
    if lines.contains("||") {
        return None;
    }
    let mut dir = ".".to_owned();
    for command in &commands[..commands.len() - 1] {
        let argv = command["argv"].as_array()?;
        let words: Vec<&str> = argv.iter().filter_map(Value::as_str).collect();
        if words.first() == Some(&"cd") && words.len() == 2 {
            dir = words[1].to_owned();
        } else if !words.join(" ").starts_with("set ") {
            return None;
        }
    }
    let argv = last["argv"].as_array()?;
    let words: Vec<&str> = argv.iter().filter_map(Value::as_str).collect();
    if words.first() == Some(&"gh") && words.windows(2).any(|pair| pair == ["pr", "merge"]) {
        let pr = words.iter().find_map(|word| word.parse::<u64>().ok()).map(|value| value as usize);
        return Some(json!({"dir": dir, "repo": Value::Null, "pr": pr, "ref": Value::Null}));
    }
    if words.first() == Some(&"git") && words.get(1) == Some(&"merge") {
        return Some(json!({"dir": dir, "repo": Value::Null, "pr": Value::Null, "ref": words.get(2).copied()}));
    }
    None
}

/// Pushes a script makes, in order (gitPushes).
pub fn git_pushes(script: &str) -> Option<Vec<Value>> {
    let mut pushes = Vec::new();
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let commands = simple_commands(trimmed)?;
        for command in commands {
            let argv = command["argv"].as_array()?;
            let words: Vec<&str> = argv.iter().filter_map(Value::as_str).collect();
            if words.first() != Some(&"git") || words.get(1) != Some(&"push") {
                continue;
            }
            let remote = words.get(2).copied().unwrap_or("origin");
            let refspec = words.get(3).copied();
            pushes.push(json!({
                "remote": remote,
                "refspec": refspec,
                "force": words.iter().any(|word| word.starts_with("--force") || *word == "-f"),
                "delete": refspec.is_some_and(|value| value.starts_with(':')),
            }));
        }
    }
    Some(pushes)
}

/// Whether a URL is the git proxy (isProxyUrl).
pub fn is_proxy_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.contains("/api/git/") && (lower.starts_with("http://") || lower.starts_with("https://"))
}

/// Whether argv runs inline code (inlineCode).
pub fn inline_code(argv: &[String]) -> bool {
    let Some(first) = argv.first() else {
        return false;
    };
    let base = first.rsplit('/').next().unwrap_or(first);
    let flag = match base {
        "python" | "python3" | "node" | "ruby" | "perl" | "bash" | "sh" | "php" => true,
        "deno" => argv.iter().any(|word| word == "eval"),
        _ => false,
    };
    flag && argv[1..].iter().any(|word| matches!(word.as_str(), "-c" | "-e" | "-r" | "-eval"))
}

/// Whether a tool call runs inline code (inlineCodeCall).
pub fn inline_code_call(tool_name: &str, command: &str) -> bool {
    tool_name == "Bash" && command_words(command).first().is_some_and(|words| inline_code(words))
}

/// Whether a refusal ends with the no-terminal rule (policyRefusal).
pub fn policy_refusal(message: &str) -> bool {
    message.trim_end().ends_with(NO_TERMINAL_COMMANDS)
}

/// Hint for a forbidden reason (forbiddenHint).
pub fn forbidden_hint(reason: &str) -> Option<&'static str> {
    match reason {
        "sudo" => Some("No sudo: ask for the change instead."),
        "delete" => Some("Ask with delete_branches (repo, branches, why)."),
        "upload" => Some("No uploads to external services."),
        "remote_shell" => Some("No remote shells."),
        "outside" => Some("Stay inside the mission folder."),
        "inline_code" => Some("Inline code is not allowed."),
        _ => None,
    }
}

/// Pointer for branches a command would delete (otherBranchPointer).
pub fn other_branch_pointer(branches: &[String], handle: &str) -> String {
    format!("Ask with delete_branches (repo, {}, why) for {handle}. {NO_TERMINAL_COMMANDS}", branches.join(", "))
}

/// Whether a call only reads (readOnlyCall).
pub fn read_only_call(tool_name: &str, path: &str) -> bool {
    if secret_read(tool_name, path) {
        return false;
    }
    matches!(tool_name, "Read" | "Glob" | "Grep" | "Bash" | "View")
}

/// Verdict the computer decides by itself (localVerdict).
pub fn local_verdict(command: &str) -> &'static str {
    if denied_in_missions("Bash", command) {
        "deny"
    } else {
        let lower = command.to_ascii_lowercase();
        if lower.contains("push") || lower.contains("merge") || lower.contains("deploy") { "ask" } else { "allow" }
    }
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(words: usize, commands: usize, secrets: usize, denied: bool) -> Vec<(&'static str, String)> {
    vec![
        ("words", count_label(words)),
        ("commands", count_label(commands)),
        ("secrets", count_label(secrets)),
        ("denied", if denied { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_parsing() {
        let words = shell_words("git status && echo hi | cat").unwrap();
        assert_eq!(words.len(), 7);
        assert_eq!(words[2]["op"], "list");
        assert_eq!(words[5]["op"], "pipe");
        assert_eq!(shell_words("echo \"$HOME\"").is_none(), true);
        assert_eq!(shell_words("echo 'a b'")[1]["text"], "a b");
        let commands = simple_commands("echo hi > out.txt && git status").unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0]["writes"][0], "out.txt");
        assert!(simple_commands("FOO=1 echo hi").is_none());
    }

    #[test]
    fn folders_and_secrets() {
        let scope = json!({"cwd": "/work", "home": "/home/ana", "folders": ["/work/mission"]});
        assert!(inside_folders(&scope, "mission/file.txt"));
        assert!(!inside_folders(&scope, "/etc/passwd"));
        assert!(in_browser_profile("/home/ana/.agentes/browser/Cookies"));
        assert!(is_secret_read("/home/ana/.ssh/id_rsa"));
        assert!(is_secret_read("/work/.env.production"));
        assert!(!is_secret_read("/work/.env.example"));
        assert!(secret_read("Bash", "cat ~/.aws/credentials"));
        assert!(read_only_call("Read", "/work/mission/a.txt"));
        assert!(!read_only_call("Read", "/home/.ssh/id_rsa"));
    }

    #[test]
    fn deny_and_approval() {
        assert!(denied_in_missions("Bash", "git merge feature"));
        assert!(denied_in_missions("Bash", "gh pr merge 12"));
        assert!(!denied_in_missions("Bash", "git status"));
        assert!(denied_message("Bash", "gh api user", ASK_MESSAGES[0]).unwrap().contains("Secrets"));
        let message = approval_message("A merge needs a person", "Ask in chat", None);
        assert!(message.contains("Ask in chat"));
        assert!(policy_refusal(&format!("Ask first. {NO_TERMINAL_COMMANDS}")));
        assert_eq!(forbidden_hint("sudo"), Some("No sudo: ask for the change instead."));
        assert_eq!(local_verdict("git push"), "ask");
        assert_eq!(local_verdict("git status"), "allow");
    }

    #[test]
    fn repositories_and_merges() {
        assert_eq!(github_repo("https://github.com/o/r.git").as_deref(), Some("o/r"));
        assert_eq!(github_repo("git@github.com:o/r.git").as_deref(), Some("o/r"));
        assert_eq!(github_repo("https://gitlab.com/o/r"), None);
        let merge = lone_merge("cd app\ngit merge feature").unwrap();
        assert_eq!(merge["dir"], "app");
        assert_eq!(merge["ref"], "feature");
        assert!(lone_merge("git merge feature | cat").is_none());
        let pushes = git_pushes("git push origin main\ngit push --force other").unwrap();
        assert_eq!(pushes.len(), 2);
        assert_eq!(pushes[1]["force"], true);
        assert!(is_proxy_url("https://example.com/api/git/one/two/o/r"));
    }

    #[test]
    fn inline_and_presentation() {
        assert!(inline_code(&["python3".to_owned(), "-c".to_owned(), "print(1)".to_owned()]));
        assert!(inline_code_call("Bash", "node -e 'x'"));
        assert!(!inline_code_call("Read", "node -e 'x'"));
        assert_eq!(
            normalized_command(&["/usr/bin/git".to_owned(), "-C".to_owned(), "x".to_owned(), "push".to_owned()]),
            "git push"
        );
        assert!(other_branch_pointer(&["a".to_owned()], "alice").contains("delete_branches"));
        assert_eq!(command_words("echo hi")[0][0], "echo");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 2, 1, true).len(), 4);
    }
}
