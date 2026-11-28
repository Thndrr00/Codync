#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from mercury-agent's `bots/store.ts`: bot ids, deliverable names, manifest
//! validation and fleet hierarchy. Composes helpers already ported into this host. Only
//! `botInsights` calls this.

use serde_json::Value;

/// Manifest filename of a bot.
pub const BOT_MANIFEST_FILENAME: &str = "bot.yaml";

/// Persona filename of a bot.
pub const BOT_PERSONA_FILENAME: &str = "persona.md";

/// Extension-less avatar filename of a bot.
pub const BOT_AVATAR_FILENAME: &str = "avatar";

/// Private sandbox folder of a bot.
pub const BOT_SANDBOX_DIRNAME: &str = "sandbox";

/// Tools denied by default for every bot unless allowed.
pub const BOT_DANGEROUS_TOOLS: [&str; 7] =
    ["run_command", "write_file", "edit_file", "create_file", "delete_file", "git_commit", "git_push"];

/// Fleet nesting cap (CEO → Lead → Crew).
pub const MAX_FLEET_DEPTH: usize = 3;

/// Whether an id is lowercase alphanumeric with dashes, at most 48 chars (BOT_ID_PATTERN).
pub fn is_valid_bot_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && id.len() <= 48
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Why an id is not usable, or None (assertValidBotId).
pub fn bot_id_problem(id: &str) -> Option<&'static str> {
    (!is_valid_bot_id(id)).then_some("invalid")
}

/// Drop filesystem-hostile characters and control codes (stripUnsafeChars).
pub fn strip_unsafe_chars(text: &str) -> String {
    text.chars()
        .filter(|c| (*c as u32) >= 32 && !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect()
}

/// Collapse whitespace runs into single spaces.
fn collapse_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A folder name a person recognises (deliverablesFolderName).
pub fn deliverables_folder_name(name: &str, id: &str) -> String {
    let clean = collapse_spaces(&strip_unsafe_chars(name));
    let trimmed = clean.trim();
    if trimmed.is_empty() { id.to_owned() } else { trimmed.chars().take(60).collect() }
}

/// A deliverable file named with a leading date (deliverableFileName).
pub fn deliverable_file_name(stem: &str, extension: &str, day: &str) -> String {
    let without_date = if stem.len() >= 11 && stem.as_bytes().get(4) == Some(&b'-') {
        stem.chars().skip(11).collect::<String>()
    } else {
        stem.to_owned()
    };
    let cleaned = strip_unsafe_chars(&without_date);
    let title = collapse_spaces(&cleaned.replace(['-', '_'], " "));
    let title = title.trim().chars().take(80).collect::<String>();
    format!("{} {}{}", day, if title.is_empty() { "untitled" } else { &title }, extension)
}

/// Filesystem-safe job id (safeJobId).
pub fn safe_job_id(job_id: &str) -> String {
    job_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).take(64).collect()
}

/// Checkpoint filename for a job.
pub fn checkpoint_file_name(job_id: &str) -> String {
    format!("{}.json", safe_job_id(job_id))
}

/// Whether an expression is a 5-field cron (isValidCronExpression).
pub fn is_five_field_cron(expression: &str) -> bool {
    let fields: Vec<&str> = expression.trim().split_whitespace().collect();
    fields.len() == 5 && fields.iter().all(|field| !field.is_empty())
}

/// Structural validation of a bot manifest (validateBotManifest).
pub fn validate_bot_manifest(manifest: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let id = manifest["id"].as_str().unwrap_or("");
    if bot_id_problem(id).is_some() {
        errors.push(format!("Invalid bot id \"{id}\": lowercase letters, digits, dashes; max 48 chars"));
    }
    if manifest["name"].as_str().is_none_or(|name| name.trim().is_empty()) {
        errors.push("name is required".to_owned());
    }
    if let Some(schedules) = manifest["schedules"].as_array() {
        for schedule in schedules {
            let name = schedule["name"].as_str().unwrap_or("(unnamed)");
            if schedule["name"].as_str().is_none_or(|value| value.is_empty())
                || schedule["prompt"].as_str().is_none_or(|value| value.is_empty())
            {
                errors.push(format!("schedule {name}: name and prompt are required"));
            }
            if !is_five_field_cron(schedule["cron"].as_str().unwrap_or("")) {
                errors.push(format!(
                    "schedule {name}: \"{}\" is not a 5-field cron expression",
                    schedule["cron"].as_str().unwrap_or("")
                ));
            }
        }
    }
    for (key, field) in [("allowCrossBotRecall", "memory"), ("canMessage", "comms")] {
        if let Some(ids) = manifest[field][key].as_array() {
            for other in ids.iter().filter_map(Value::as_str) {
                if other == id {
                    errors.push(format!("{key} cannot include the bot itself"));
                }
            }
        }
    }
    if let Some(parent) = manifest["parent"].as_str() {
        if parent == id {
            errors.push("parent cannot be the bot itself".to_owned());
        }
    }
    if manifest["fleetRole"].as_str() == Some("crew") && manifest["parent"].as_str().is_none_or(str::is_empty) {
        errors.push("fleetRole \"crew\" requires a parent (the lead bot id)".to_owned());
    }
    errors
}

/// Data directories that live inside a bot profile but are not bots.
pub fn is_data_dirname(name: &str) -> bool {
    matches!(name, "sandbox" | "outputs" | "skills" | "transcripts" | "checkpoints")
}

/// Whether a fleet depth is within the nesting cap.
pub fn fleet_depth_ok(depth: usize) -> bool {
    depth <= MAX_FLEET_DEPTH
}

/// Why a fleet role is invalid without a parent (fleetRole "crew").
pub fn fleet_role_problem(role: &str, parent: Option<&str>) -> Option<&'static str> {
    if role == "crew" && parent.is_none_or(str::is_empty) { Some("fleetRole \"crew\" requires a parent") } else { None }
}

/// Whether a list names the bot itself.
pub fn lists_self(ids: &[String], id: &str) -> bool {
    ids.iter().any(|value| value == id)
}

/// Whether a tool is denied by default (BOT_DANGEROUS_TOOLS).
pub fn is_dangerous_tool(name: &str) -> bool {
    BOT_DANGEROUS_TOOLS.contains(&name)
}

/// SF Symbol for a dangerous tool.
pub fn dangerous_tool_icon(name: &str) -> &'static str {
    match name {
        "run_command" => "terminal.fill",
        "write_file" | "edit_file" | "create_file" => "pencil",
        "delete_file" => "trash",
        _ => "arrow.triangle.branch",
    }
}

/// Whether a directory name looks like a bot profile rather than data.
pub fn is_bot_dirname(name: &str) -> bool {
    !name.is_empty() && !is_data_dirname(name) && !name.starts_with('_') && !name.starts_with('.')
}

/// Default persona markdown for a new bot (DEFAULT_PERSONA_TEMPLATE).
pub fn persona_template(name: &str, description: Option<&str>) -> String {
    format!(
        "# {name}\n\n{}\n\n## Character\n\nYou are {name}. Stay in character at all times.\n\n## Standing instructions\n\n- Be concise and concrete.\n",
        description.filter(|value| !value.is_empty()).unwrap_or("A focused specialist.")
    )
}

/// Summary of a manifest for an insights card.
pub fn manifest_summary(manifest: &Value) -> Value {
    let errors = validate_bot_manifest(manifest);
    serde_json::json!({
        "id": manifest["id"].as_str().unwrap_or(""),
        "name": manifest["name"].as_str().unwrap_or(""),
        "schedules": manifest["schedules"].as_array().map_or(0, Vec::len),
        "errors": errors,
        "ok": errors.is_empty(),
    })
}

/// A sanitized path under a base directory.
pub fn safe_relative_path(base: &str, name: &str) -> String {
    format!("{base}/{}", collapse_spaces(&strip_unsafe_chars(name)))
}

/// Transcript filename for a run.
pub fn transcript_file_name(run_id: &str) -> String {
    format!("{}.jsonl", safe_job_id(run_id))
}

/// State key of a routine (slugified).
pub fn routine_state_key(name: &str) -> String {
    crate::bdots_textfmt::slugify(name)
}

/// One line describing a bot for an insights card (botLine).
pub fn bot_line(id: &str, name: &str, schedules: usize, dangerous: bool) -> String {
    format!("{} · {} · {} schedules · {}", name, id, schedules, if dangerous { "restricted" } else { "open" })
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(bots: usize, schedules: usize, errors: usize, fleet: bool) -> Vec<(&'static str, String)> {
    vec![
        ("bots", count_label(bots)),
        ("schedules", count_label(schedules)),
        ("errors", count_label(errors)),
        ("fleet", if fleet { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ids_and_names() {
        assert!(is_valid_bot_id("research-bot"));
        assert!(!is_valid_bot_id("Bad"));
        assert!(!is_valid_bot_id("-bad"));
        assert_eq!(bot_id_problem("Bad"), Some("invalid"));
        assert_eq!(strip_unsafe_chars("a/b:c?*"), "abc");
        assert_eq!(deliverables_folder_name("  My / Bot  ", "id1"), "My Bot");
        assert_eq!(deliverables_folder_name("   ", "id1"), "id1");
        assert_eq!(safe_job_id("a/b:c"), "a_b_c");
        assert_eq!(checkpoint_file_name("job/1"), "job_1.json");
    }

    #[test]
    fn deliverables_and_cron() {
        assert_eq!(
            deliverable_file_name("2026-10-10 Oxide Series D", ".md", "2026-11-27"),
            "2026-11-27 Oxide Series D.md"
        );
        assert_eq!(deliverable_file_name("", ".txt", "2026-11-27"), "2026-11-27 untitled.txt");
        assert!(is_five_field_cron("*/5 * * * *"));
        assert!(!is_five_field_cron("* * * *"));
        assert_eq!(fleet_depth_ok(3), true);
        assert_eq!(fleet_depth_ok(4), false);
    }

    #[test]
    fn manifests_and_fleet() {
        let valid =
            json!({"id":"alpha","name":"Alpha","schedules":[{"name":"daily","prompt":"run","cron":"0 9 * * *"}]});
        assert!(validate_bot_manifest(&valid).is_empty());
        let invalid = json!({"id":"Bad","name":"","schedules":[{"name":"x","prompt":"","cron":"* * *"}],"memory":{"allowCrossBotRecall":["Bad"]},"fleetRole":"crew"});
        let errors = validate_bot_manifest(&invalid);
        assert!(errors.len() >= 5);
        let summary = manifest_summary(&valid);
        assert_eq!(summary["ok"], true);
        assert_eq!(summary["schedules"], 1);
        assert_eq!(fleet_role_problem("crew", None), Some("fleetRole \"crew\" requires a parent"));
        assert!(lists_self(&["alpha".to_owned()], "alpha"));
        assert!(is_dangerous_tool("git_push"));
        assert!(!is_dangerous_tool("read_file"));
        assert_eq!(dangerous_tool_icon("write_file"), "pencil");
    }

    #[test]
    fn dirs_and_presentation() {
        assert!(is_data_dirname("sandbox"));
        assert!(!is_data_dirname("alpha"));
        assert!(is_bot_dirname("alpha"));
        assert!(!is_bot_dirname("_shared"));
        assert!(persona_template("Ada", None).starts_with("# Ada"));
        assert_eq!(safe_relative_path("/bots", "a?b"), "/bots/ab");
        assert_eq!(transcript_file_name("run/1"), "run_1.jsonl");
        assert_eq!(routine_state_key("Daily Digest"), "daily-digest");
        assert!(bot_line("a", "A", 1, false).contains("A"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(2, 3, 1, true).len(), 4);
    }
}
