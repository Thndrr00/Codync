use super::*;
use crate::hub::Hub;
use crate::store::Store;
use serde_json::json;
use std::path::Path;

#[test]
fn titles_are_trimmed_and_bounded() {
    assert_eq!(title("  Weekly report ").unwrap(), "Weekly report");
    assert!(title("   ").is_err());
    assert!(title(&"x".repeat(MAX_TITLE + 1)).is_err());
    assert!(title("a\nb").is_err());
}

#[tokio::test]
async fn a_diagram_scene_saves_and_reopens_through_the_owner_api() {
    use base64::Engine as _;
    let dir = std::env::temp_dir().join(format!("codync-diagram-api-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = Store::open(Path::new(":memory:")).unwrap();
    let cfg: crate::store::BotConfig = serde_json::from_value(json!({"id":"bot", "name":"Bot"})).unwrap();
    store.save_bot(&cfg).unwrap();
    let identity = crate::remote::identity::Identity::load_or_create(&dir).unwrap();
    let hub = Hub::new(store, "diagram-test".into(), identity, "t".into(), 0);
    let first = json!({
        "type": "excalidraw", "version": 2,
        "elements": [{"id": "a", "type": "rectangle", "x": 1, "y": 2, "width": 10, "height": 10}],
    });
    let id = uuid::Uuid::new_v4().to_string();
    let path = content_path("bot", &id, ArtifactKind::Diagram).unwrap();
    let (size, sha256) = crate::chat::diagrams::write(&path, &first).unwrap();
    let mut artifact = Artifact {
        id: id.clone(),
        bot_id: "bot".into(),
        title: "Plan".into(),
        kind: ArtifactKind::Diagram,
        size,
        sha256: sha256.clone(),
        created_at: 1,
        updated_at: 1,
        entry_id: None,
        rev: 0,
        publication: Publication::Public,
    };
    put(&hub, &mut artifact).unwrap();
    // The owner moves the box and saves; the saved version keeps the element.
    let moved = json!({
        "type": "excalidraw", "version": 2,
        "elements": [{"id": "a", "type": "rectangle", "x": 40, "y": 2, "width": 10, "height": 10}],
    });
    let saved =
        api(&hub, "saveArtifactScene", &json!({"botId": "bot", "artifactId": id, "scene": moved, "sha256": sha256}))
            .await
            .unwrap();
    assert_eq!(saved["artifact"]["kind"].as_str(), Some("diagram"));
    let read = api(&hub, "readArtifact", &json!({"botId": "bot", "artifactId": id, "offset": 0})).await.unwrap();
    let bytes = base64::engine::general_purpose::STANDARD.decode(read["data"].as_str().unwrap()).unwrap();
    let reopened = crate::chat::diagrams::parse(&bytes).unwrap();
    assert_eq!(reopened, moved, "the scene reopens exactly as it was saved");
    // A stale editor cannot overwrite a newer scene.
    assert!(
        api(&hub, "saveArtifactScene", &json!({"botId": "bot", "artifactId": id, "scene": first, "sha256": sha256}),)
            .await
            .is_err()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn artifacts_are_written_whole_and_read_in_chunks() {
    let dir = std::env::temp_dir().join(format!("codync-artifacts-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let (source, dest) = (dir.join("artifact.html"), dir.join("artifacts/p.html"));
    let html = "<h1>Hi</h1>".repeat(50_000);
    std::fs::write(&source, &html).unwrap();
    let (size, sha) = write(&dest, &source).unwrap();
    assert_eq!(size, html.len() as u64);
    assert_eq!(sha, files::digest_hex(&Sha256::digest(html.as_bytes())));
    let first = read(&dest, size, 0).unwrap();
    assert_eq!(first.len(), files::CHUNK_SIZE);
    let rest = read(&dest, size, first.len() as u64).unwrap();
    assert_eq!([first, rest].concat(), html.as_bytes());
    assert!(read(&dest, size, size + 1).is_err(), "past the end");
    std::fs::write(&source, "").unwrap();
    assert!(write(&dest, &source).is_err(), "an empty artifact");
    assert!(html_path("b", "../x").is_err(), "ids are UUIDs");
    let _ = std::fs::remove_dir_all(dir);
}
