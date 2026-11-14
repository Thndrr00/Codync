//! Indexing filters, ranking and the MCP tool round trip.

use super::{index, search};
use crate::hub::Hub;
use crate::remote::identity::Identity;
use crate::store::{BotConfig, Store};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("codync-docs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, path: &str, text: &str) -> PathBuf {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn repository() -> Temp {
    let temp = Temp::new();
    temp.write(".gitignore", "ignored.md\nbuild/\nnode_modules/\n*.log\n");
    temp.write("docs/guide.md", "# Setup\nRun cargo test before shipping.\nThe token stays here.\n");
    temp.write("docs/ignored.md", "secret");
    temp.write("src/main.rs", "fn main() { cargo_test(); }\n");
    temp.write("build/generated.rs", "generated");
    temp.write("node_modules/x.js", "module.exports = 1");
    temp.write("debug.log", "noise");
    temp.write(".env", "TOKEN=secret");
    temp.write(".hidden.md", "hidden");
    temp.write("notes.txt", "release notes");
    temp.write("nested/.gitignore", "draft.md\n");
    temp.write("nested/draft.md", "draft");
    temp.write("nested/keep.md", "keep this");
    temp.write("binary.bin", "\u{0}\u{1}\u{2}");
    temp.write("large.md", &"x".repeat(500 * 1024));
    temp
}

#[test]
fn the_index_respects_gitignore_size_and_hidden_files() {
    let temp = repository();
    let index = index::build(&temp.0);
    let paths: Vec<&str> = index.docs.iter().map(|doc| doc.path.as_str()).collect();
    assert_eq!(paths, ["docs/guide.md", "nested/keep.md", "notes.txt", "src/main.rs"]);
    assert!(!index.truncated);
}

#[test]
fn ranking_names_the_file_and_line_and_docs_only_keeps_documentation() {
    let temp = repository();
    let index = index::build(&temp.0);
    let hits = search::search(&index, &search::terms("cargo test"), true, 8);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "docs/guide.md");
    assert_eq!(hits[0].line, 2);
    assert!(hits[0].snippet.contains("cargo test"));
    assert!(hits[0].score > 0);
    let code = search::search(&index, &search::terms("fn main"), false, 8);
    assert_eq!(code[0].path, "src/main.rs");
    let read = search::read(&index, "docs/guide.md", Some(1), Some(2)).expect("a read");
    assert_eq!(read.text, "1: # Setup\n2: Run cargo test before shipping.\n");
}

fn hub(root: &Path) -> std::sync::Arc<Hub> {
    let dir = std::env::temp_dir().join(format!("codync-docs-hub-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let hub = Hub::new(
        Store::open(Path::new(":memory:")).unwrap(),
        "t".into(),
        Identity::load_or_create(&dir).unwrap(),
        "t".into(),
        0,
    );
    let bot: BotConfig =
        serde_json::from_value(json!({"id": "b", "name": "B", "backend": "fixture", "cwd": root})).unwrap();
    hub.store.save_bot(&bot).unwrap();
    std::sync::Arc::new(hub)
}

fn answer(hub: &std::sync::Arc<Hub>, name: &str, arguments: Value) -> Value {
    super::call(hub, "docsCall", &json!({"botId": "b", "name": name, "arguments": arguments})).expect("a tool answer")
}

#[test]
fn the_mcp_catalog_names_the_three_tools() {
    let names: Vec<&str> =
        super::tools().as_array().expect("tools").iter().filter_map(|tool| tool["name"].as_str()).collect();
    assert_eq!(names, ["docs_search", "code_search", "docs_read"]);
}

#[test]
fn the_tools_search_read_and_cite_inside_the_bot_folder() {
    let temp = repository();
    let hub = hub(&temp.0);
    let found = answer(&hub, "docs_search", json!({"query": "token"}));
    assert_eq!(found["results"][0]["path"], "docs/guide.md");
    assert_eq!(found["results"][0]["citation"], "docs/guide.md:3");
    let read = answer(&hub, "docs_read", json!({"path": "docs/guide.md", "start": 3, "end": 3}));
    assert_eq!(read["text"], "3: The token stays here.\n");
    assert_eq!(read["citation"], "docs/guide.md:3");
    let code = answer(&hub, "code_search", json!({"query": "fn main"}));
    assert_eq!(code["results"][0]["path"], "src/main.rs");
    let status = super::call(&hub, "docsStatus", &json!({"botId": "b"})).expect("a status");
    assert_eq!(status["files"], 4);
    assert!(
        super::call(&hub, "docsCall", &json!({"botId": "b", "name": "docs_read", "arguments": {"path": "../.env"}}))
            .is_err()
    );
}

#[test]
fn a_named_folder_replaces_the_bot_folder_until_it_is_cleared() {
    let first = repository();
    let second = Temp::new();
    second.write("only.md", "# Chosen\nchosen text");
    let hub = hub(&first.0);
    let chosen = super::call(&hub, "setDocsFolder", &json!({"botId": "b", "path": second.0})).expect("a folder");
    assert_eq!(chosen["files"], 1);
    let found = answer(&hub, "docs_search", json!({"query": "chosen"}));
    assert_eq!(found["results"][0]["path"], "only.md");
    let back = super::call(&hub, "setDocsFolder", &json!({"botId": "b", "path": ""})).expect("cleared");
    let expected = first.0.canonicalize().unwrap();
    assert_eq!(back["folder"].as_str(), Some(expected.to_string_lossy().as_ref()));
}
