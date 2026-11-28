#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure configuration checks for a bot, shaping a small "health" card. Composes the validators
//! already ported from bdots (`is_model_id`, `provider_of`, `valid_secret_name`, `is_app_slug`)
//! instead of adding a second copy. The host `botInsights` endpoint is the only caller, so every
//! helper here is used; clients only render the result.

/// One finding from a health check.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub id: &'static str,
    pub title: String,
    pub detail: String,
    pub severity: &'static str,
    pub icon: &'static str,
}

fn check(id: &'static str, title: &str, detail: String, severity: &'static str, icon: &'static str) -> Check {
    Check { id, title: title.to_owned(), detail, severity, icon }
}

/// Ordering of severities, worst first.
pub fn severity_rank(severity: &str) -> u8 {
    match severity {
        "error" => 0,
        "warn" => 1,
        "ok" => 2,
        _ => 3,
    }
}

/// SF Symbol for a severity, shared by all clients.
pub fn severity_icon(severity: &str) -> &'static str {
    match severity {
        "error" => "exclamationmark.octagon.fill",
        "warn" => "exclamationmark.triangle.fill",
        _ => "checkmark.circle.fill",
    }
}

/// Semantic tone for a severity; views map it to their own palette.
pub fn severity_tone(severity: &str) -> &'static str {
    match severity {
        "error" => "danger",
        "warn" => "attention",
        _ => "good",
    }
}

/// Worst findings first, stable within a severity.
pub fn sort_checks(checks: &[Check]) -> Vec<Check> {
    let mut out = checks.to_vec();
    out.sort_by_key(|c| severity_rank(c.severity));
    out
}

/// Count findings per severity, worst first.
pub fn count_by_severity(checks: &[Check]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    for check in checks {
        if let Some(entry) = out.iter_mut().find(|(severity, _)| *severity == check.severity) {
            entry.1 += 1;
        } else {
            out.push((check.severity, 1));
        }
    }
    out.sort_by_key(|(severity, _)| severity_rank(severity));
    out
}

/// Number of blocking findings.
pub fn error_count(checks: &[Check]) -> usize {
    checks.iter().filter(|c| c.severity == "error").count()
}

/// Number of non-blocking warnings.
pub fn warn_count(checks: &[Check]) -> usize {
    checks.iter().filter(|c| c.severity == "warn").count()
}

/// Whether any finding blocks the bot from working.
pub fn has_blocking(checks: &[Check]) -> bool {
    error_count(checks) > 0
}

/// Cap the list shown in a card without hiding the worst findings.
pub fn limit_checks(checks: &[Check], max: usize) -> Vec<Check> {
    sort_checks(checks).into_iter().take(max).collect()
}

/// A 0–100 score: errors cost 25, warnings 8.
pub fn health_score(checks: &[Check]) -> u32 {
    let penalty = error_count(checks) * 25 + warn_count(checks) * 8;
    100_u32.saturating_sub(penalty as u32)
}

/// Short label for a health score.
pub fn health_label(score: u32) -> &'static str {
    if score >= 90 {
        "Healthy"
    } else if score >= 60 {
        "Needs attention"
    } else {
        "Needs setup"
    }
}

/// Semantic tone for a health score.
pub fn health_tone(score: u32) -> &'static str {
    if score >= 90 {
        "good"
    } else if score >= 60 {
        "attention"
    } else {
        "danger"
    }
}

/// One line describing the card's overall state.
pub fn health_headline(checks: &[Check]) -> String {
    let score = health_score(checks);
    let errors = error_count(checks);
    let warnings = warn_count(checks);
    if errors == 0 && warnings == 0 {
        return format!("{} · every check passed", health_label(score));
    }
    format!("{} · {errors} blocking, {warnings} to review", health_label(score))
}

/// A bot with no name can't be told apart in the roster.
pub fn check_name(name: &str) -> Check {
    let empty = name.trim().is_empty();
    check(
        "name",
        "Name",
        if empty { "Give this bot a name.".to_owned() } else { name.to_owned() },
        if empty { "warn" } else { "ok" },
        if empty { "pencil" } else { "checkmark.circle.fill" },
    )
}

/// A description helps other bots and people know what this one does.
pub fn check_description(description: &str) -> Check {
    let empty = description.trim().is_empty();
    check(
        "description",
        "Description",
        if empty { "Add a short description.".to_owned() } else { crate::bdots_text::summarize(description) },
        if empty { "warn" } else { "ok" },
        if empty { "text.badge.xmark" } else { "checkmark.circle.fill" },
    )
}

/// Reuse the model-id validator instead of writing a second one.
pub fn check_model(model: &str) -> Check {
    let valid = crate::bdots_desk::is_model_id(model);
    let provider = crate::bdots_desk::provider_of(model).map_or("unknown provider", str::to_owned);
    check(
        "model",
        "Model",
        if valid { provider } else { "Choose a model for this bot.".to_owned() },
        if valid { "ok" } else { "warn" },
        if valid { "cpu" } else { "questionmark.circle.fill" },
    )
}

/// A managed workspace is where the bot's files live.
pub fn check_cwd(cwd: &str) -> Check {
    let empty = cwd.trim().is_empty();
    check(
        "cwd",
        "Workspace",
        if empty { "Pick a folder for this bot's files.".to_owned() } else { crate::bdots_more::store_host(cwd, cwd) },
        if empty { "error" } else { "ok" },
        if empty { "folder.badge.questionmark" } else { "folder.fill" },
    )
}

/// Skills point at scripts, references and assets; report what's missing.
pub fn check_skills(skills: &[String]) -> Check {
    let missing = skills.iter().filter(|s| crate::bdots_desk::resource_kind(s) == "file").count();
    check(
        "skills",
        "Skills",
        if missing == 0 { format!("{} installed", skills.len()) } else { format!("{missing} skill files to review") },
        if missing == 0 { "ok" } else { "warn" },
        if missing == 0 { "puzzlepiece.extension" } else { "exclamationmark.triangle.fill" },
    )
}

/// A bot with no routines never runs on its own.
pub fn check_routines(routines: usize) -> Check {
    check(
        "routines",
        "Routines",
        if routines == 0 { "No scheduled work yet.".to_owned() } else { format!("{routines} scheduled") },
        if routines == 0 { "warn" } else { "ok" },
        if routines == 0 { "calendar.badge.clock" } else { "checkmark.circle.fill" },
    )
}

/// Warn when the daily token budget is nearly spent.
pub fn check_usage(used: u64, limit: Option<u64>) -> Check {
    let percent = crate::bdots_activity::percent(used as usize, limit.unwrap_or(0) as usize);
    let tight = limit.is_some_and(|l| l > 0 && used * 100 >= l * 85);
    check(
        "usage",
        "Daily budget",
        match limit {
            Some(limit) => format!("{} / {limit} · {percent}%", crate::bdots_desk::compact_count(used)),
            None => format!("{} used", crate::bdots_desk::compact_count(used)),
        },
        if tight { "warn" } else { "ok" },
        if tight { "gauge.badge.exclamationmark" } else { "gauge" },
    )
}

/// Connectors give a bot access to other accounts.
pub fn check_connectors(connectors: &[String]) -> Check {
    let empty = connectors.is_empty();
    check(
        "connectors",
        "Connectors",
        if empty { "No connectors attached.".to_owned() } else { format!("{} attached", connectors.len()) },
        if empty { "warn" } else { "ok" },
        if empty { "plugs" } else { "checkmark.circle.fill" },
    )
}

/// A secret name the model can refer to must be valid.
pub fn check_secret(name: &str) -> Check {
    let valid = crate::bdots_more::valid_secret_name(name);
    check(
        "secret",
        "Secret name",
        if valid { name.to_owned() } else { "Use letters, digits, _ or -.".to_owned() },
        if valid { "ok" } else { "error" },
        if valid { "key.fill" } else { "exclamationmark.octagon.fill" },
    )
}

/// A public mini-app slug must be safe to put in a URL.
pub fn check_app_slug(slug: &str) -> Check {
    let valid = crate::bdots_more::is_app_slug(slug);
    check(
        "appSlug",
        "App link",
        if valid { slug.to_owned() } else { "Use lowercase letters, digits or -.".to_owned() },
        if valid { "ok" } else { "error" },
        if valid { "link" } else { "exclamationmark.octagon.fill" },
    )
}

/// Build the full list of checks for one bot.
pub fn build_checks(
    name: &str,
    description: &str,
    model: &str,
    cwd: &str,
    skills: &[String],
    routines: usize,
    used: u64,
    limit: Option<u64>,
    connectors: &[String],
    secret: &str,
    slug: &str,
) -> Vec<Check> {
    vec![
        check_name(name),
        check_description(description),
        check_model(model),
        check_cwd(cwd),
        check_skills(skills),
        check_routines(routines),
        check_usage(used, limit),
        check_connectors(connectors),
        check_secret(secret),
        check_app_slug(slug),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severities_sort_worst_first() {
        assert_eq!(severity_rank("error"), 0);
        assert_eq!(severity_rank("ok"), 2);
        assert_eq!(severity_tone("warn"), "attention");
        assert_eq!(severity_icon("error"), "exclamationmark.octagon.fill");
    }

    #[test]
    fn checks_count_and_limit_keep_the_worst() {
        let checks = vec![
            check("a", "A", String::new(), "ok", "x"),
            check("b", "B", String::new(), "error", "x"),
            check("c", "C", String::new(), "warn", "x"),
        ];
        assert_eq!(error_count(&checks), 1);
        assert_eq!(warn_count(&checks), 1);
        assert!(has_blocking(&checks));
        assert_eq!(limit_checks(&checks, 2)[0].id, "b");
        assert_eq!(count_by_severity(&checks)[0].0, "error");
    }

    #[test]
    fn score_and_labels_follow_the_findings() {
        assert_eq!(health_score(&[]), 100);
        assert_eq!(health_label(95), "Healthy");
        assert_eq!(health_tone(50), "danger");
        assert!(health_headline(&[]).contains("every check passed"));
        let bad = vec![check("a", "A", String::new(), "error", "x")];
        assert_eq!(health_score(&bad), 75);
        assert!(health_headline(&bad).contains("1 blocking"));
    }

    #[test]
    fn validators_reject_bad_values() {
        assert_eq!(check_name("").severity, "warn");
        assert_eq!(check_description("").severity, "warn");
        assert_eq!(check_cwd("").severity, "error");
        assert_eq!(check_secret("1bad").severity, "error");
        assert_eq!(check_app_slug("Bad Slug").severity, "error");
        assert_eq!(check_routines(0).severity, "warn");
        assert_eq!(check_connectors(&[]).severity, "warn");
    }

    #[test]
    fn build_reports_every_area() {
        let checks =
            build_checks("Ada", "helps", "claude", "/tmp", &[], 1, 10, Some(100), &[], "api_key", "split-bill");
        assert_eq!(checks.len(), 10);
        assert_eq!(check_usage(90, Some(100)).severity, "warn");
        assert_eq!(check_usage(10, Some(100)).severity, "ok");
    }
}
