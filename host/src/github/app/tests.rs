use super::*;
use crate::hub::Hub;
use crate::remote::identity::Identity;
use crate::store::Store;
use hmac::KeyInit as _;
use hmac::Mac as _;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

const SECRET: &str = "codync-webhook-secret";
const REPO: &str = "octo/app";

fn hub() -> (Arc<Hub>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("codync-github-app-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::open(Path::new(":memory:")).unwrap();
    let hub = Hub::new(store, "test".into(), Identity::load_or_create(&dir).unwrap(), "token".into(), 0);
    (hub, dir)
}

fn bot(hub: &Hub, id: &str) {
    let cwd = std::env::temp_dir().to_string_lossy().into_owned();
    let config =
        serde_json::from_value(json!({"id": id, "name": id, "cwd": cwd, "backend": "custom", "command": "true"}))
            .unwrap();
    hub.store.save_bot(&config).unwrap();
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn head(pairs: &[(&str, &str)]) -> Headers {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

fn signature(secret: &str, body: &[u8]) -> String {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    format!("sha256={}", hex(mac.finalize().into_bytes().as_slice()))
}

/// A signed delivery of `event` from GitHub.
fn delivery(event: &str, id: &str, payload: &Value) -> (Headers, Vec<u8>) {
    let body = serde_json::to_vec(payload).unwrap();
    let headers = head(&[
        ("x-github-event", event),
        ("x-github-delivery", id),
        ("x-hub-signature-256", &signature(SECRET, &body)),
    ]);
    (headers, body)
}

fn board(hub: &Hub, repos: BTreeMap<String, RepoMap>) {
    save(
        &hub.store,
        &Config {
            app_id: "1234".into(),
            slug: "codync-test".into(),
            private_key: "-----BEGIN RSA PRIVATE KEY-----".into(),
            webhook_secret: SECRET.into(),
            installations: vec![Installation { id: "7".into(), account: "octo".into(), repos: vec![REPO.into()] }],
            repos,
        },
    )
    .unwrap();
}

/// Maps `REPO` to `bot` on `branches` (empty for all of them).
fn mapped(hub: &Hub, bot: &str, branches: &[&str]) {
    let mut repos = BTreeMap::new();
    repos.insert(
        REPO.to_owned(),
        RepoMap { bots: vec![bot.to_owned()], branches: branches.iter().map(|b| (*b).to_owned()).collect() },
    );
    board(hub, repos);
}

fn opened(branch: &str) -> Value {
    json!({"action": "opened", "repository": {"full_name": REPO},
        "pull_request": {"title": "Fix login", "base": {"ref": branch}}})
}

fn routine(hub: &Hub, bot: &str, trigger: &Value) -> String {
    let saved = hub
        .routines
        .save(hub, bot, &json!({"name": "On it", "instruction": "Review it", "triggers": [trigger]}))
        .unwrap();
    saved["routine"]["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn the_app_fields_have_the_shape_github_uses() {
    assert!(app_id_ok("123456"));
    assert!(secret_ok(SECRET));
    assert!(key_ok("-----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----"));
    assert!(repo_ok("octo/app"));
    for bad in ["", "12a", "x", &"9".repeat(21)] {
        assert!(!app_id_ok(bad), "{bad}");
    }
    for bad in ["", "short", "with space here", &"x".repeat(201)] {
        assert!(!secret_ok(bad), "{bad}");
    }
    assert!(!key_ok("not a key"));
    for bad in ["app", "/app", "octo/", "octo/app x", &format!("{}/{}", "o".repeat(70), "a".repeat(70))] {
        assert!(!repo_ok(bad), "{bad}");
    }
}

#[tokio::test]
async fn the_status_shows_the_app_without_its_secrets() {
    let (hub, dir) = hub();
    assert_eq!(status(&hub.store)["configured"], false);
    board(&hub, BTreeMap::new());
    let value = status(&hub.store);
    assert_eq!(value["configured"], true);
    assert_eq!(value["appId"], "1234");
    assert_eq!(value["events"], json!(EVENTS));
    assert!(value.get("webhookSecret").is_none());
    assert!(value.get("privateKey").is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_configured_app_registers_one_hook_with_the_webhook_secret() {
    let (hub, dir) = hub();
    assert!(hook(&hub.store).is_none());
    board(&hub, BTreeMap::new());
    assert_eq!(hook(&hub.store).unwrap(), json!({"id": "github", "key": SECRET, "enabled": true}));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_delivery_is_remembered_once_inside_the_window() {
    let (hub, dir) = hub();
    assert!(remember(&hub.store, "d1").unwrap());
    assert!(!remember(&hub.store, "d1").unwrap());
    let old = now_ms() - REPLAY_WINDOW_MS - 1;
    hub.store.db.locked().execute("UPDATE github_deliveries SET at = ? WHERE delivery = 'd1'", [old]).unwrap();
    assert!(remember(&hub.store, "d1").unwrap(), "outside the window it is new again");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn an_unsigned_delivery_is_refused() {
    let (hub, dir) = hub();
    let body = serde_json::to_vec(&opened("main")).unwrap();
    let headers = head(&[("x-github-event", "pull_request"), ("x-github-delivery", "d1")]);
    assert_eq!(receive(&hub, &headers, &body), Err(Refused::Unauthorized), "no App is connected");
    board(&hub, BTreeMap::new());
    assert_eq!(receive(&hub, &headers, &body), Err(Refused::Unauthorized));
    let wrong = signature("another-secret", &body);
    let headers =
        head(&[("x-github-event", "pull_request"), ("x-github-delivery", "d1"), ("x-hub-signature-256", &wrong)]);
    assert_eq!(receive(&hub, &headers, &body), Err(Refused::Unauthorized));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_ping_answers_ping() {
    let (hub, dir) = hub();
    board(&hub, BTreeMap::new());
    let body = b"{}".to_vec();
    let headers = head(&[("x-github-event", "ping"), ("x-hub-signature-256", &signature(SECRET, &body))]);
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"ping": true}));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn events_outside_the_list_are_ignored() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    mapped(&hub, "b1", &[]);
    let (headers, body) = delivery("push", "d1", &json!({"repository": {"full_name": REPO}}));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"ignored": "event does not reach routines"}));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn an_unmapped_repository_is_ignored() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    board(&hub, BTreeMap::new());
    let (headers, body) = delivery("pull_request", "d1", &opened("main"));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"ignored": "repository is not mapped"}));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_mapped_repository_queues_only_its_bots_matching_routines() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    bot(&hub, "b2");
    mapped(&hub, "b1", &[]);
    let hit = routine(&hub, "b1", &json!({"type": "event", "source": "github", "event": "pull_request.opened"}));
    routine(&hub, "b1", &json!({"type": "event", "source": "github", "event": "issues.closed"}));
    let other = routine(&hub, "b2", &json!({"type": "event", "source": "github", "event": "pull_request.opened"}));

    let (headers, body) = delivery("pull_request", "d1", &opened("main"));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"queued": 1}));
    let queued: Vec<String> = hub.routines.list("b1", |_| false)["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["routineId"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(queued, vec![hit]);
    assert_eq!(hub.routines.list(&other, |_| false)["runs"], json!([]));
    let run = &hub.routines.list("b1", |_| false)["runs"][0];
    assert_eq!(run["event"]["repo"], REPO);
    assert_eq!(run["deliveryId"], "d1");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_replayed_delivery_queues_nothing() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    mapped(&hub, "b1", &[]);
    routine(&hub, "b1", &json!({"type": "event", "source": "github", "event": "pull_request.opened"}));
    let (headers, body) = delivery("pull_request", "d1", &opened("main"));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"queued": 1}));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"duplicate": true}));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn a_branch_filter_keeps_other_branches_out() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    mapped(&hub, "b1", &["main"]);
    routine(&hub, "b1", &json!({"type": "event", "source": "github", "event": "pull_request.opened"}));
    let (headers, body) = delivery("pull_request", "d1", &opened("dev"));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"ignored": "branch is not mapped"}));
    let (headers, body) = delivery("pull_request", "d2", &opened("main"));
    assert_eq!(receive(&hub, &headers, &body).unwrap(), json!({"queued": 1}));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn an_oversize_or_bad_delivery_is_invalid() {
    let (hub, dir) = hub();
    board(&hub, BTreeMap::new());
    let body = vec![b'{'; MAX_BODY + 1];
    let headers = head(&[("x-hub-signature-256", &signature(SECRET, &body))]);
    assert!(matches!(receive(&hub, &headers, &body), Err(Refused::Invalid(_))));

    let body = b"not json".to_vec();
    let headers = head(&[
        ("x-github-event", "pull_request"),
        ("x-github-delivery", "has space"),
        ("x-hub-signature-256", &signature(SECRET, &body)),
    ]);
    assert_eq!(receive(&hub, &headers, &body), Err(Refused::Invalid("invalid delivery id".into())));
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn mapping_needs_a_known_repository_and_a_real_bot() {
    let (hub, dir) = hub();
    bot(&hub, "b1");
    assert!(map(&hub.store, &json!({"repo": REPO, "bots": ["b1"]})).is_err(), "no App yet");
    board(&hub, BTreeMap::new());
    let err = map(&hub.store, &json!({"repo": "octo/other", "bots": ["b1"]})).unwrap_err().to_string();
    assert!(err.contains("not on any installation"), "{err}");
    let mut config = load(&hub.store).unwrap().unwrap();
    config.installations[0].repos.push("octo/other".into());
    save(&hub.store, &config).unwrap();
    let err = map(&hub.store, &json!({"repo": "octo/other", "bots": ["ghost"]})).unwrap_err().to_string();
    assert!(err.contains("Unknown bot"), "{err}");
    assert!(map(&hub.store, &json!({"repo": "octo/other", "bots": []})).is_err(), "a repository needs a bot");
    map(&hub.store, &json!({"repo": "octo/other", "bots": ["b1"]})).unwrap();
    assert_eq!(load(&hub.store).unwrap().unwrap().repos["octo/other"].bots, vec!["b1".to_owned()]);
    unmap(&hub.store, &json!({"repo": "octo/other"})).unwrap();
    assert!(unmap(&hub.store, &json!({"repo": "octo/other"})).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
