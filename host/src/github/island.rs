//! Bounded, host-owned GitHub context for a bot's configured origin and current branch.

use crate::chat::calls::project::git_output;
use crate::{LockExt, hub::Hub};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

const MAX_BODY: usize = 1024 * 1024;
const KEEP: Duration = Duration::from_secs(60);
static CACHE: LazyLock<Mutex<HashMap<String, (Instant, Value)>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) async fn read(hub: &Arc<Hub>, bot_id: &str) -> Result<Value> {
    let bot = hub.store.bot(bot_id)?.filter(|row| !row.deleted).context("Unknown bot")?;
    ensure!(!bot.config.cwd.is_empty(), "This bot has no workspace folder.");
    let dir = Path::new(&bot.config.cwd);
    let (origin, branch) = tokio::join!(
        git_output(dir, &["config", "--get", "remote.origin.url"]),
        git_output(dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
    );
    let origin = origin.context("Could not read this workspace's origin")?;
    let repository = repository_of(String::from_utf8_lossy(&origin).trim())
        .context("The workspace origin is not a supported github.com repository.")?;
    let branch = String::from_utf8(branch.context("Could not read the workspace branch")?)?.trim().to_owned();
    ensure!(!branch.is_empty() && branch != "HEAD", "Select a named branch to read its GitHub pull requests.");
    crate::market::vault::unlock(hub.clone()).await?;
    let Some(config) = super::connection(&hub.store)? else {
        return Ok(
            json!({"configured":false,"repository":repository,"branch":branch,"pulls":[],"issues":[],"error":null,"fetchedAt":crate::store::now_ms()}),
        );
    };
    let key = format!("{}\0{}\0{}\0{}", config.generation, config.login, repository, branch);
    if let Some((_, value)) = CACHE.locked().get(&key).filter(|(at, _)| at.elapsed() < KEEP) {
        return Ok(value.clone());
    }
    let value =
        tokio::time::timeout(Duration::from_secs(15), snapshot(&config.token, &config.login, &repository, &branch))
            .await
            .context("GitHub workspace read timed out.")??;
    // A disconnect/account replacement while HTTP was in flight invalidates that account's metadata.
    let _configuration = hub.store.connector_lock.locked();
    let current = super::connection(&hub.store)?;
    ensure!(connection_matches(current.as_ref(), &config), "GitHub connection changed; refresh integrations.");
    let mut cache = CACHE.locked();
    cache.retain(|_, (at, _)| at.elapsed() < KEEP);
    if cache.len() >= 16 {
        let oldest = cache.iter().min_by_key(|(_, (at, _))| *at).map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            cache.remove(&oldest);
        }
    }
    cache.insert(key, (Instant::now(), value.clone()));
    Ok(value)
}

fn connection_matches(current: Option<&super::Config>, expected: &super::Config) -> bool {
    current.is_some_and(|current| {
        current.generation == expected.generation && current.login == expected.login && current.token == expected.token
    })
}

async fn snapshot(token: &str, login: &str, repository: &str, branch: &str) -> Result<Value> {
    let mut errors = Vec::new();
    let owner = repository.split('/').next().context("Invalid repository")?;
    let head = format!("{owner}:{branch}");
    let pulls_path = format!("/repos/{repository}/pulls");
    let issues_path = format!("/repos/{repository}/issues");
    let pulls_query =
        [("state", "all"), ("head", head.as_str()), ("sort", "updated"), ("direction", "desc"), ("per_page", "2")];
    let issues_query =
        [("state", "all"), ("assignee", login), ("sort", "updated"), ("direction", "desc"), ("per_page", "20")];
    let (pulls, issues) = tokio::join!(get(token, &pulls_path, &pulls_query), get(token, &issues_path, &issues_query),);
    let mut shown = Vec::new();
    match pulls {
        Ok(pulls) => {
            for pull in pulls.as_array().context("GitHub returned an invalid pull-request list")?.iter().take(2) {
                if let Some(number) = pull["number"].as_u64().filter(|number| *number > 0) {
                    shown.push(enrich_pull(token, repository, number, pull, &mut errors).await);
                }
            }
        }
        Err(error) => errors.push(format!("Pull requests: {error}")),
    }
    let issues = match issues {
        Ok(value) => issue_summaries(repository, &value)?,
        Err(error) => {
            errors.push(format!("Assigned issues: {error}"));
            Vec::new()
        }
    };
    Ok(
        json!({"configured":true,"repository":repository,"branch":branch,"pulls":shown,"issues":issues,"error":(!errors.is_empty()).then(||errors.join(" · ")),"fetchedAt":crate::store::now_ms()}),
    )
}

async fn enrich_pull(token: &str, repository: &str, number: u64, pull: &Value, errors: &mut Vec<String>) -> Value {
    let mut summary = json!({
        "number":number,"title":text(&pull["title"],200),"state":if pull["merged_at"].is_string() {"merged".to_owned()} else {text(&pull["state"],30)},
        "draft":pull["draft"].as_bool().unwrap_or(false),"updatedAt":text(&pull["updated_at"],40),
        "url":format!("https://github.com/{repository}/pull/{number}"),
        "reviewers":requested_reviewers(pull),"reviews":[],"checks":[],
    });
    let Some(sha) = pull["head"]["sha"]
        .as_str()
        .filter(|sha| (40..=64).contains(&sha.len()) && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
    else {
        errors.push(format!("PR #{number}: GitHub did not report a valid head commit."));
        return summary;
    };
    let reviews_path = format!("/repos/{repository}/pulls/{number}/reviews");
    let checks_path = format!("/repos/{repository}/commits/{sha}/check-runs");
    let statuses_path = format!("/repos/{repository}/commits/{sha}/status");
    let (reviews, checks, statuses) = tokio::join!(
        get(token, &reviews_path, &[("per_page", "100")]),
        get(token, &checks_path, &[("per_page", "20"), ("filter", "latest")]),
        get(token, &statuses_path, &[("per_page", "20")]),
    );
    match reviews {
        Ok(value) => match review_summaries(&value) {
            Ok(value) => summary["reviews"] = json!(value),
            Err(error) => errors.push(format!("PR #{number} reviews: {error}")),
        },
        Err(error) => errors.push(format!("PR #{number} reviews: {error}")),
    }
    let mut rows = Vec::new();
    for (result, field, legacy) in [(checks, "check_runs", false), (statuses, "statuses", true)] {
        match result {
            Ok(value) => {
                if let Some(values) = value[field].as_array() {
                    if value["total_count"].as_u64().is_some_and(|count| count > 20) {
                        errors.push(format!(
                            "PR #{number}: showing only the first 20 {}.",
                            if legacy { "commit statuses" } else { "checks" }
                        ));
                    }
                    rows.extend(values.iter().take(20).map(|check| check_summary(check, legacy)));
                } else {
                    errors.push(format!("PR #{number}: GitHub returned an invalid check list."));
                }
            }
            Err(error) => {
                errors.push(format!("PR #{number} {}: {error}", if legacy { "commit statuses" } else { "checks" }));
            }
        }
    }
    summary["checks"] = json!(rows);
    summary
}

async fn get(token: &str, path: &str, query: &[(&str, &str)]) -> Result<Value> {
    let url = reqwest::Url::parse_with_params(&format!("https://api.github.com{path}"), query)
        .context("Could not prepare GitHub request")?
        .to_string();
    let client = crate::egress::client(crate::egress::Policy::Only(&["api.github.com"]), &url).await?;
    let request = client
        .get(url)
        .bearer_auth(token)
        .header("accept", "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .header("user-agent", "codync")
        .timeout(Duration::from_secs(8))
        .build()
        .context("Could not prepare GitHub request")?;
    let requested_url = request.url().clone();
    let mut response = client.execute(request).await.context("GitHub request failed")?;
    if !response.status().is_success() {
        bail!("GitHub answered {}. Check repository read permissions or rate limits.", response.status());
    }
    // Egress never follows redirects to another origin; even a same-origin relocation is not accepted.
    ensure!(response.url() == &requested_url, "GitHub redirected this repository read; update the workspace origin.");
    ensure!(
        response.content_length().is_none_or(|size| size <= MAX_BODY as u64),
        "GitHub response exceeds the read limit"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.context("Could not read GitHub response")? {
        ensure!(bytes.len().saturating_add(chunk.len()) <= MAX_BODY, "GitHub response exceeds the read limit");
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).context("GitHub returned invalid JSON")
}

fn repository_of(origin: &str) -> Option<String> {
    if origin.contains(char::is_whitespace) {
        return None;
    }
    let Some(path) = origin.strip_prefix("git@github.com:") else {
        let url = reqwest::Url::parse(origin).ok()?;
        if url.host_str()? != "github.com"
            || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.password().is_some()
        {
            return None;
        }
        match url.scheme() {
            "https" if url.username().is_empty() => {}
            "ssh" if url.username() == "git" => {}
            _ => return None,
        }
        return valid_repository(url.path().strip_prefix('/')?);
    };
    valid_repository(path)
}

fn valid_repository(path: &str) -> Option<String> {
    let path = path.trim_end_matches('/').strip_suffix(".git").unwrap_or(path.trim_end_matches('/'));
    let (owner, name) = path.split_once('/')?;
    if [owner, name].iter().any(|part| {
        part.is_empty()
            || *part == "."
            || *part == ".."
            || part.len() > 100
            || !part.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    }) {
        return None;
    }
    Some(path.to_owned())
}

fn text(value: &Value, limit: usize) -> String {
    value.as_str().unwrap_or_default().chars().take(limit).collect()
}
fn logins(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .take(20)
        .filter_map(|user| user["login"].as_str())
        .map(|login| login.chars().take(100).collect())
        .collect()
}

fn requested_reviewers(pull: &Value) -> Vec<String> {
    let mut reviewers = logins(&pull["requested_reviewers"]);
    reviewers.extend(pull["requested_teams"].as_array().into_iter().flatten().take(20).filter_map(|team| {
        team["slug"].as_str().map(|slug| format!("team:{}", slug.chars().take(100).collect::<String>()))
    }));
    reviewers.truncate(20);
    reviewers
}

fn issue_summaries(repository: &str, value: &Value) -> Result<Vec<Value>> {
    Ok(value.as_array().context("GitHub returned an invalid issue list")?.iter().filter(|issue|issue.get("pull_request").is_none()).filter_map(|issue| {
        let number=issue["number"].as_u64().filter(|number|*number>0)?;
        Some(json!({"number":number,"title":text(&issue["title"],200),"state":text(&issue["state"],30),"assignees":logins(&issue["assignees"]),"updatedAt":text(&issue["updated_at"],40),"url":format!("https://github.com/{repository}/issues/{number}")}))
    }).take(10).collect())
}

fn review_summaries(value: &Value) -> Result<Vec<Value>> {
    let reviews = value.as_array().context("GitHub returned an invalid review list")?;
    ensure!(reviews.len() < 100, "Review history exceeds the bounded preview; open GitHub for current reviews.");
    let mut latest = BTreeMap::new();
    for review in reviews {
        let Some(login) = review["user"]["login"].as_str() else { continue };
        if review["submitted_at"].is_null() {
            continue;
        }
        latest.insert(
            login,
            json!({"login":login.chars().take(100).collect::<String>(),"state":text(&review["state"],40)}),
        );
    }
    Ok(latest.into_values().take(20).collect())
}

fn check_summary(value: &Value, legacy: bool) -> Value {
    let state = text(&value[if legacy { "state" } else { "status" }], 40);
    let raw = value[if legacy { "target_url" } else { "html_url" }].as_str();
    let url = raw.and_then(|url| {
        let parsed = reqwest::Url::parse(url).ok()?;
        (parsed.scheme() == "https"
            && parsed.host_str() == Some("github.com")
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.port().is_none())
        .then(|| parsed.to_string())
    });
    json!({"name":text(&value[if legacy {"context"} else {"name"}],120),"status":if legacy {if state=="pending" {"pending"} else {"completed"}} else {state.as_str()},"conclusion":if legacy {Some(state.as_str())} else {value["conclusion"].as_str()},"url":url})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_or_disconnected_accounts_cannot_publish_an_inflight_snapshot() {
        let expected = super::super::Config {
            token: "first".into(),
            login: "owner".into(),
            scopes: vec![],
            generation: "one".into(),
        };
        assert!(connection_matches(Some(&expected), &expected));
        assert!(!connection_matches(None, &expected));
        for (token, login, generation) in
            [("second", "owner", "one"), ("first", "other", "one"), ("first", "owner", "two")]
        {
            let current = super::super::Config {
                token: token.into(),
                login: login.into(),
                scopes: vec![],
                generation: generation.into(),
            };
            assert!(!connection_matches(Some(&current), &expected));
        }
    }

    #[test]
    fn origins_require_a_plain_github_repository_without_credentials_or_redirects() {
        for origin in
            ["git@github.com:org/repo.git", "https://github.com/org/repo.git", "ssh://git@github.com/org/repo.git"]
        {
            assert_eq!(repository_of(origin).as_deref(), Some("org/repo"));
        }
        for origin in [
            "https://token@github.com/o/r",
            "https://github.com.evil/o/r",
            "http://github.com/o/r",
            "https://github.com/o/r?x=token",
            "git@github.com:o/r/extra",
            "https://github.com/o/%2fr",
            "git@github.com:o/..",
            "git@github.com:o/r\nother",
        ] {
            assert!(repository_of(origin).is_none(), "{origin}");
        }
    }

    #[test]
    fn issue_metadata_excludes_pull_requests_and_message_bodies() {
        let issues=issue_summaries("o/r",&json!([{"number":1,"pull_request":{},"body":"secret"},{"number":2,"title":"Issue","state":"closed","body":"secret","assignees":[{"login":"person","token":"secret"}]}])).expect("valid issues");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0]["url"], "https://github.com/o/r/issues/2");
        assert!(!serde_json::to_string(&issues).expect("JSON").contains("secret"));
    }

    #[test]
    fn reviews_use_last_submitted_event_and_reject_a_truncated_history() {
        let reviews=review_summaries(&json!([{"user":{"login":"person"},"state":"CHANGES_REQUESTED","submitted_at":"one"},{"user":{"login":"person"},"state":"APPROVED","submitted_at":"two"},{"user":{"login":"other"},"state":"PENDING","submitted_at":null}])).expect("reviews");
        assert_eq!(reviews, vec![json!({"login":"person","state":"APPROVED"})]);
        assert!(review_summaries(&json!(vec![Value::Null; 100])).is_err());
    }

    #[test]
    fn review_requests_include_people_and_teams() {
        assert_eq!(
            requested_reviewers(
                &json!({"requested_reviewers":[{"login":"person"}],"requested_teams":[{"slug":"maintainers"}]})
            ),
            vec!["person", "team:maintainers"]
        );
    }

    #[test]
    fn ci_results_never_open_untrusted_links_or_expose_raw_payloads() {
        let check = check_summary(
            &json!({"name":"CI","status":"completed","conclusion":"failure","html_url":"https://evil.test/","output":{"text":"secret"}}),
            false,
        );
        assert_eq!(check["url"], Value::Null);
        assert_eq!(check["conclusion"], "failure");
        assert!(!check.to_string().contains("secret"));
        let legacy = check_summary(
            &json!({"context":"legacy","state":"pending","target_url":"https://github.com/o/r/actions/runs/1"}),
            true,
        );
        assert_eq!(legacy["status"], "pending");
        assert_eq!(legacy["url"], "https://github.com/o/r/actions/runs/1");
    }
}
