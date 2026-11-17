//! Tests for modules: the fixtures the other tests share, and the checks a module can't talk its way out of.

use super::install::Source;
use super::manifest::{self, Manifest};
use super::{Approved, Modules};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::path::PathBuf;

/// A fresh folder under the system's temporary directory.
pub(crate) fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("codync-module-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A module folder signed by `key`: the manifest, its signature and one file the module runs.
pub(crate) fn signed_folder(name: &str, id: &str, key: &SigningKey, command: &str) -> PathBuf {
    let dir = temp_dir(name);
    std::fs::write(dir.join("main.js"), b"// a module\n").unwrap();
    let mut manifest = manifest::tests::manifest();
    manifest.id = id.into();
    manifest.name = name.into();
    manifest.entry.command = command.into();
    manifest.publisher.key = B64.encode(key.verifying_key().to_bytes());
    manifest.hash_files(&dir).unwrap();
    std::fs::write(dir.join(manifest::MANIFEST), serde_json::to_vec(&manifest).unwrap()).unwrap();
    std::fs::write(dir.join(manifest::SIGNATURE), manifest.sign(key).unwrap()).unwrap();
    dir
}

fn store(name: &str) -> crate::store::Store {
    // A different folder from the module's: `temp_dir` starts clean on every call, so reusing the
    // module's name here would delete the manifest and signature a test just wrote there.
    crate::store::Store::open(&temp_dir(&format!("{name}-db")).join("test.db")).unwrap()
}

fn modules(name: &str) -> (crate::store::Store, Modules) {
    let store = store(name);
    let modules = Modules::load(&store, tokio::sync::broadcast::channel(8).0);
    (store, modules)
}

fn approved(modules: &Modules, id: &str) -> Approved {
    modules.get(id).expect("the module is installed").approved().clone()
}

#[test]
fn a_module_starts_off_with_nothing_approved() {
    let (store, modules) = modules("off");
    let key = SigningKey::from_bytes(&[11; 32]);
    let folder = signed_folder("off", "com.example.off", &key, "module-demo");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    assert_eq!(approved(&modules, "com.example.off"), Approved::default());
    assert!(!modules.get("com.example.off").unwrap().enabled(), "off until approved and turned on");
    assert!(modules.for_bots().is_empty(), "a bot gets no tools from a module nobody approved");
}

#[test]
fn approval_never_exceeds_what_the_manifest_asks() {
    let (store, modules) = modules("approve");
    let key = SigningKey::from_bytes(&[12; 32]);
    let folder = signed_folder("approve", "com.example.approve", &key, "module-demo");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.approve", &json!({"tools": ["forecast", "rm_rf"], "files": "write"})).unwrap();
    let approved = approved(&modules, "com.example.approve");
    assert_eq!(approved.tools, vec!["forecast".to_owned()], "a tool it never declared is dropped");
    assert_eq!(approved.files, super::Files::Read, "it asked for read, so write is cut down to read");
    assert!(approved.events.is_empty(), "approving tools alone doesn't approve events");
}

#[tokio::test]
async fn a_bundle_carries_the_module_to_another_computer() {
    let (store, modules) = modules("bundle");
    let key = SigningKey::from_bytes(&[13; 32]);
    let folder = signed_folder("bundle", "com.example.bundle", &key, "module-demo");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    let bundle = super::install::export(&modules, "com.example.bundle").unwrap()["bundle"].clone();
    assert_eq!(bundle["format"], manifest::BUNDLE);
    super::install::uninstall(&modules, &store, "com.example.bundle").unwrap();
    assert!(modules.get("com.example.bundle").is_none());

    let elsewhere = Modules::load(&store, tokio::sync::broadcast::channel(8).0);
    super::install::install(&elsewhere, &store, Source::Bundle(bundle)).unwrap();
    assert_eq!(elsewhere.get("com.example.bundle").unwrap().name(), "bundle");
}

#[test]
fn a_tampered_bundle_is_refused() {
    let (store, modules) = modules("tamper");
    let key = SigningKey::from_bytes(&[14; 32]);
    let folder = signed_folder("tamper", "com.example.tamper", &key, "module-demo");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    let mut bundle = super::install::export(&modules, "com.example.tamper").unwrap()["bundle"].clone();
    bundle["files"][0]["content"] = json!(B64.encode(b"// something else\n"));
    let error = super::install::install(&modules, &store, Source::Bundle(bundle)).unwrap_err().to_string();
    assert!(error.contains("changed after it was signed"), "{error}");
}

#[tokio::test]
async fn an_update_from_another_key_starts_over_off() {
    let (store, modules) = modules("key");
    let first = SigningKey::from_bytes(&[15; 32]);
    let folder = signed_folder("key-one", "com.example.key", &first, "module-demo");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.key", &json!(null)).unwrap();
    modules.set_enabled(&store, "com.example.key", true).unwrap();
    assert!(modules.get("com.example.key").unwrap().enabled());

    let second = SigningKey::from_bytes(&[16; 32]);
    let folder = signed_folder("key-two", "com.example.key", &second, "module-demo");
    super::install::update(&modules, &store, "com.example.key", Source::Folder(folder)).unwrap();
    let module = modules.get("com.example.key").unwrap();
    assert!(!module.enabled(), "a new publisher is turned off until the owner looks");
    assert_eq!(module.approved(), &Approved::default(), "and nothing is approved");
}

#[test]
fn the_manifest_of_a_folder_is_read_before_anything_runs() {
    let key = SigningKey::from_bytes(&[17; 32]);
    let folder = signed_folder("read", "com.example.read", &key, "module-demo");
    let manifest = Manifest::read(&folder).unwrap();
    assert_eq!(manifest.id, "com.example.read");
    assert!(manifest.tool("forecast").is_some());
    assert_eq!(manifest.limits.clamped(), manifest.limits, "a modest ask is kept");
}

#[test]
fn a_folder_without_its_signature_is_not_a_module() {
    let key = SigningKey::from_bytes(&[18; 32]);
    let folder = signed_folder("unsigned", "com.example.unsigned", &key, "module-demo");
    std::fs::remove_file(folder.join(manifest::SIGNATURE)).unwrap();
    let error = Manifest::read(&folder).unwrap_err().to_string();
    assert!(error.contains("module.sig"), "{error}");
}

/// The sample module `cargo` builds beside the host, or `None` when this build didn't make one.
pub(crate) fn demo() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.parent()?;
    ["module-demo", "module-demo.exe"].iter().map(|name| dir.join(name)).find(|path| path.is_file())
}

/// A signed module that runs the sample: its three tools, its view, and the quotas asked for.
fn demo_folder(name: &str, id: &str, time_ms: u64) -> Option<PathBuf> {
    let demo = demo()?;
    let key = SigningKey::from_bytes(&[21; 32]);
    let dir = temp_dir(name);
    std::fs::write(dir.join("main.js"), b"// the sample\n").unwrap();
    let mut manifest = manifest::tests::manifest();
    manifest.id = id.into();
    manifest.name = name.into();
    manifest.entry.command = demo.to_string_lossy().into_owned();
    manifest.limits = manifest::Limits { calls_per_minute: 10, time_ms, memory_mb: 256 };
    manifest.tools = ["forecast", "hang", "announce"]
        .iter()
        .map(|name| manifest::Tool {
            name: (*name).into(),
            description: format!("{name} from the sample"),
            input_schema: json!({"type": "object", "properties": {}}),
        })
        .collect();
    manifest.permissions.tools = vec!["forecast".into(), "hang".into(), "announce".into()];
    manifest.views = vec![manifest::View { id: "today".into(), title: "Today".into() }];
    manifest.publisher.key = B64.encode(key.verifying_key().to_bytes());
    manifest.hash_files(&dir).unwrap();
    std::fs::write(dir.join(manifest::MANIFEST), serde_json::to_vec(&manifest).unwrap()).unwrap();
    std::fs::write(dir.join(manifest::SIGNATURE), manifest.sign(&key).unwrap()).unwrap();
    Some(dir)
}

#[tokio::test]
async fn a_module_answers_its_tools_and_views() {
    let Some(folder) = demo_folder("sample", "com.example.sample", 20_000) else { return };
    let (store, modules) = modules("sample");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.sample", &json!(null)).unwrap();
    modules.set_enabled(&store, "com.example.sample", true).unwrap();

    let answer = modules.call("com.example.sample", "forecast", &json!({}), "bot").await.unwrap();
    assert_eq!(answer["content"][0]["text"], "Sunny, 24 °C.");
    let view = modules.view("com.example.sample", "today", "bot").await.unwrap();
    assert_eq!(view["title"], "Today");
    assert_eq!(view["sections"][0]["rows"][1]["value"], "24 °C");
}

#[tokio::test]
async fn a_module_that_never_answers_is_stopped_and_the_next_call_starts_fresh() {
    let Some(folder) = demo_folder("hang", "com.example.hang", 1_000) else { return };
    let (store, modules) = modules("hang");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.hang", &json!(null)).unwrap();
    modules.set_enabled(&store, "com.example.hang", true).unwrap();

    let error = modules.call("com.example.hang", "hang", &json!({}), "bot").await.unwrap_err().to_string();
    assert!(error.contains("took longer than 1s"), "{error}");
    assert_eq!(modules.rows()[0]["stopped"], "took longer than 1s and was stopped");
    // The hung process is gone, so the next call gets a new one and answers.
    let answer = modules.call("com.example.hang", "forecast", &json!({}), "bot").await.unwrap();
    assert_eq!(answer["content"][0]["text"], "Sunny, 24 °C.");
}

#[tokio::test]
async fn a_tool_nobody_approved_never_reaches_the_module() {
    let Some(folder) = demo_folder("approval", "com.example.approval", 20_000) else { return };
    let (store, modules) = modules("approval");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.approval", &json!({"tools": ["forecast"]})).unwrap();
    modules.set_enabled(&store, "com.example.approval", true).unwrap();
    let error = modules.call("com.example.approval", "hang", &json!({}), "bot").await.unwrap_err().to_string();
    assert!(error.contains("isn't approved"), "{error}");
    assert_eq!(modules.rows()[0]["state"], "stopped", "nothing was started for it");
}

#[tokio::test]
async fn a_module_the_owner_turned_off_answers_nothing() {
    let Some(folder) = demo_folder("off-call", "com.example.offcall", 20_000) else { return };
    let (store, modules) = modules("off-call");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.offcall", &json!(null)).unwrap();
    let error = modules.call("com.example.offcall", "forecast", &json!({}), "bot").await.unwrap_err().to_string();
    assert!(error.contains("is off"), "{error}");
    modules.set_enabled(&store, "com.example.offcall", true).unwrap();
    modules.set_enabled(&store, "com.example.offcall", false).unwrap();
    assert_eq!(modules.rows()[0]["state"], "disabled");
}

#[tokio::test]
async fn a_module_that_never_closes_is_killed_when_it_is_stopped() {
    let Some(folder) = demo_folder("stop", "com.example.stop", 20_000) else { return };
    let (store, modules) = modules("stop");
    super::install::install(&modules, &store, Source::Folder(folder)).unwrap();
    modules.approve(&store, "com.example.stop", &json!(null)).unwrap();
    modules.set_enabled(&store, "com.example.stop", true).unwrap();
    modules.call("com.example.stop", "forecast", &json!({}), "bot").await.unwrap();
    assert_eq!(modules.rows()[0]["state"], "running");
    modules.stop("com.example.stop");
    assert_eq!(modules.rows()[0]["state"], "stopped");
}
