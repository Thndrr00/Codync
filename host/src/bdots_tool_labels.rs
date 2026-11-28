#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `tool-labels.ts` and `tool-review.ts`: tool names, app
//! prefixes, catalog labels and search matching. Composes helpers already ported into this host.
//! Only `botInsights` calls this.

use serde_json::{Value, json};

/// Words that only say where a tool came from, not its app (SKIPPED).
pub const SKIPPED: [&str; 6] = ["mcp", "ai", "com", "io", "app", "dev"];

/// Articles dropped from a tool key (ARTICLES).
pub const ARTICLES: [&str; 3] = ["a", "an", "the"];

/// App prefixes and how each app writes its own name (APPS, the most used ones).
pub const APPS: [(&str, &str); 12] = [
    ("github", "GitHub"),
    ("gitlab", "GitLab"),
    ("linear", "Linear"),
    ("jira", "Jira"),
    ("slack", "Slack"),
    ("notion", "Notion"),
    ("googledrive", "Google Drive"),
    ("googledocs", "Google Docs"),
    ("gmail", "Gmail"),
    ("figma", "Figma"),
    ("stripe", "Stripe"),
    ("whatsapp", "WhatsApp"),
];

/// Catalog words for the most common tools (English and pt-BR).
pub const LABELS: [(&str, &str, &str); 12] = [
    ("git_clone", "Clone repository", "Clonar repositório"),
    ("get_file_contents", "Read file", "Ler arquivo"),
    ("create_or_update_file", "Create or change a file", "Criar ou alterar um arquivo"),
    ("delete_file", "Delete file", "Apagar arquivo"),
    ("list_branches", "List branches", "Listar branches"),
    ("create_branch", "Create branch", "Criar branch"),
    ("delete_branches", "Delete branches", "Apagar branches"),
    ("open_pull_request", "Open pull request", "Abrir PR"),
    ("list_pull_requests", "List pull requests", "Listar PRs"),
    ("search_code", "Search the code", "Buscar no código"),
    ("list_commits", "List commits", "Listar commits"),
    ("get_me", "See the signed-in account", "Ver a conta conectada"),
];

/// Languages with a catalog column.
pub const LANGUAGES: [&str; 2] = ["en", "pt-BR"];

/// Verbs whose noun is plural (MANY).
pub const MANY: [&str; 3] = ["list", "search", "get"];

/// Words of a tool name: camelCase, dots, dashes and underscores (wordsOf).
pub fn words_of(name: &str) -> Vec<String> {
    let mut spaced = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() && index > 0 && chars[index - 1].is_ascii_lowercase() {
            spaced.push('_');
        }
        spaced.push(*ch);
    }
    spaced
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// How an app writes its own name, from a leading word (appNameOf).
pub fn app_name_of(word: &str) -> Option<&'static str> {
    let key: String = word.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    APPS.iter().find(|(candidate, _)| *candidate == key).map(|(_, label)| *label)
}

/// Whether a word only says where the tool came from.
pub fn is_skipped(word: &str) -> bool {
    SKIPPED.contains(&word)
}

/// Whether a word is an article dropped from keys.
pub fn is_article(word: &str) -> bool {
    ARTICLES.contains(&word)
}

/// Normalize a tool name into its app and catalog key (toolKey).
pub fn tool_key(name: &str) -> Value {
    let mut words = words_of(name);
    let mut app: Option<String> = None;
    loop {
        if words.is_empty() || words.len() == 1 {
            break;
        }
        let first = words[0].clone();
        if is_skipped(&first) || (app.is_some() && first.chars().all(|c| c.is_ascii_digit())) {
            words.remove(0);
            continue;
        }
        let mut matched = false;
        for count in [3_usize, 2, 1] {
            if words.len() <= count {
                continue;
            }
            let brand: String = words[..count].concat();
            if let Some(label) = app_name_of(&brand) {
                if app.is_none() {
                    app = Some(label.to_owned());
                }
                words.drain(..count);
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }
    let key = words.into_iter().filter(|word| !is_article(word)).collect::<Vec<_>>().join("_");
    json!({"app": app, "key": key})
}

/// Whether a saved label is just the technical name (labelIsName).
pub fn label_is_name(label: &str, name: &str) -> bool {
    if label.trim().is_empty() {
        return true;
    }
    tool_key(label)["key"] == tool_key(name)["key"]
}

/// Catalog words for a tool key in a language (catalogLabel).
pub fn catalog_label(key: &str, language: &str) -> Option<&'static str> {
    if !LANGUAGES.contains(&language) {
        return None;
    }
    LABELS
        .iter()
        .find(|(candidate, _, _)| *candidate == key)
        .map(|(_, en, pt)| if language == "pt-BR" { *pt } else { *en })
}

/// Words of a tool in its language, else a humanized key (toolName).
pub fn tool_name(name: &str, language: &str) -> String {
    let key = tool_key(name)["key"].as_str().unwrap_or("").to_owned();
    catalog_label(&key, language).map_or_else(|| humanize(&key), str::to_owned)
}

/// Readable words of a key: `get_file_contents` → "Get file contents".
pub fn humanize(key: &str) -> String {
    key.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Search normalization: lowercase, no accents, apostrophes or dashes (norm).
pub fn norm(text: &str) -> String {
    text.to_ascii_lowercase()
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u")
        .replace('ã', "a")
        .replace('õ', "o")
        .replace('ç', "c")
        .replace('\'', "")
        .replace('-', "")
}

/// Search a tool by label or technical name (matchTool).
pub fn match_tool(label: &str, name: &str, query: &str) -> Value {
    let needle = norm(query.trim());
    if needle.is_empty() {
        return json!({"ok": true, "tech": false});
    }
    if norm(label).contains(&needle) {
        return json!({"ok": true, "tech": false});
    }
    let technical = name.to_ascii_lowercase();
    if technical.contains(&needle.replace(' ', "_")) {
        return json!({"ok": true, "tech": true});
    }
    json!({"ok": false, "tech": false})
}

/// Name of a review subject (subjectLabel).
pub fn subject_label(subject: &str) -> String {
    if subject == "other" {
        return "Other".to_owned();
    }
    let text = humanize(subject);
    let mut chars = text.chars();
    chars.next().map_or(text.clone(), |first| format!("{}{}", first.to_ascii_uppercase(), chars.as_str()))
}

/// Whether a verb takes a plural noun (MANY).
pub fn is_many(verb: &str) -> bool {
    MANY.contains(&verb)
}

/// Body of a tool-review decision list (reviewDecisions).
pub fn review_decisions(tools: &[Value], levels: &Value, decided: &[String]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let name = tool["name"].as_str().unwrap_or("");
            let level = levels[name].as_str().map_or_else(|| tool["level"].as_str().unwrap_or(""), str::to_owned);
            let confirmed = tool["pending"] != true || decided.iter().any(|value| value == name);
            json!({"name": name, "access": level, "label": tool["savedLabel"], "confirmed": confirmed})
        })
        .collect()
}

/// Grey hint beside a tool line (toolLineHint).
pub fn tool_line_hint(line: &Value) -> Value {
    if line["code"] == true {
        return Value::Null;
    }
    if line["autoApproved"] == true {
        return Value::String("Auto-approved".to_owned());
    }
    line["hint"].clone()
}

/// Number of catalog entries for a language (catalogSize).
pub fn catalog_size(language: &str) -> usize {
    if !LANGUAGES.contains(&language) {
        return 0;
    }
    LABELS.len()
}

/// All app keys in the catalog (appKeys).
pub fn app_keys() -> Vec<&'static str> {
    APPS.iter().map(|(key, _)| *key).collect()
}

/// A phrase for one tool call (toolPhrase).
pub fn tool_phrase(name: &str, language: &str, state: &str) -> String {
    let verb = match state {
        "fail" => "Failed",
        "stop" => "Stopped",
        "run" => "Running",
        _ => "Used",
    };
    format!("{verb} {}", tool_name(name, language).to_ascii_lowercase())
}

/// One line describing a tool catalog (toolLine).
pub fn tool_line(name: &str, language: &str) -> String {
    let key = tool_key(name);
    let app = key["app"].as_str().unwrap_or("tool");
    format!("{app} · {} · {}", key["key"], tool_name(name, language))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(tools: usize, apps: usize, matched: usize, technical: bool) -> Vec<(&'static str, String)> {
    vec![
        ("tools", count_label(tools)),
        ("apps", count_label(apps)),
        ("matched", count_label(matched)),
        ("technical", if technical { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_apps() {
        assert_eq!(words_of("mergePullRequest"), vec!["merge", "pull", "request"]);
        assert_eq!(words_of("github.merge_pull_request"), vec!["github", "merge", "pull", "request"]);
        assert_eq!(app_name_of("GitHub"), Some("GitHub"));
        assert_eq!(app_name_of("constructor"), None);
        assert!(is_skipped("mcp"));
        assert!(is_article("the"));
        let key = tool_key("github.merge_pull_request");
        assert_eq!(key["app"], "GitHub");
        assert_eq!(key["key"], "merge_pull_request");
        assert_eq!(tool_key("get_file_contents")["key"], "get_file_contents");
        assert!(label_is_name("", "anything"));
        assert!(label_is_name("Get file contents", "get_file_contents"));
    }

    #[test]
    fn labels_and_search() {
        assert_eq!(catalog_label("get_file_contents", "en"), Some("Read file"));
        assert_eq!(catalog_label("get_file_contents", "pt-BR"), Some("Ler arquivo"));
        assert_eq!(catalog_label("get_file_contents", "fr"), None);
        assert_eq!(tool_name("github.merge_pull_request", "en"), "Merge pull request");
        assert_eq!(humanize("get_file_contents"), "Get file contents");
        assert_eq!(norm("Coração"), "coracao");
        assert_eq!(match_tool("Read file", "get_file_contents", "read")["ok"], true);
        assert_eq!(match_tool("Read file", "get_file_contents", "get file")["tech"], true);
        assert_eq!(match_tool("Read file", "get_file_contents", "zzz")["ok"], false);
        assert_eq!(subject_label("other"), "Other");
        assert!(is_many("list"));
    }

    #[test]
    fn reviews_and_hints() {
        let tools = vec![
            json!({"name":"a","level":"ask","pending":true,"savedLabel":"A"}),
            json!({"name":"b","level":"allow","pending":false,"savedLabel":"B"}),
        ];
        let decisions = review_decisions(&tools, &json!({"a":"deny"}), &[]);
        assert_eq!(decisions[0]["access"], "deny");
        assert_eq!(decisions[0]["confirmed"], false);
        assert_eq!(decisions[1]["confirmed"], true);
        assert_eq!(tool_line_hint(&json!({"code":true,"hint":"x"})), Value::Null);
        assert_eq!(tool_line_hint(&json!({"autoApproved":true})), "Auto-approved");
        assert_eq!(catalog_size("en"), 12);
        assert!(app_keys().contains(&"github"));
        assert_eq!(tool_phrase("delete_file", "en", "fail"), "Failed delete file");
        assert!(tool_line("github.get_me", "en").contains("GitHub"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(12, 12, 2, false).len(), 4);
    }
}
