//! Diagram scenes for artifacts: the Excalidraw scene JSON shape, without bundling Excalidraw's
//! React app. The desktop app draws and edits these elements natively as SVG; the host owns the
//! stored scene and refuses anything it cannot safely hand to a client.
//!
//! A scene is `{type: "excalidraw", version, source, elements, appState}`. Elements are validated
//! (bounded count and geometry, plain text, no HTML) and the whole scene is capped at 2 MiB.

use crate::chat::files;
use anyhow::{Context as _, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::io::{Read as _, Write as _};
use std::path::Path;

/// Largest scene, in bytes.
pub const MAX_SCENE: u64 = 2 * 1024 * 1024;
const MAX_ELEMENTS: usize = 2_000;
const MAX_TEXT: usize = 8_000;
const MAX_ID: usize = 128;
const MAX_NUMBER: f64 = 10_000_000.0;

/// The `chat` server's diagram tool (when to use it: `outbox::INSTRUCTIONS`).
pub const TOOL: &str = "publish_diagram";

pub fn tool() -> Value {
    json!({
        "name": TOOL,
        "description": "Publish a Diagram: an editable Excalidraw scene (up to 2 MiB) the user opens from a card in your chat, edits and keeps. Write the scene JSON to a file and pass its path, or pass scene directly. Without artifact it creates one (title required); with artifact it replaces that diagram. Only diagrams can be published this way; use publish_artifact for an HTML artifact.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "A .json file with the Excalidraw scene."},
                "scene": {"type": "object", "description": "The scene itself: {type, version, elements, appState}."},
                "title": {"type": "string", "description": "The diagram's title, as the user should see it."},
                "artifact": {"type": "string", "description": "The id of the diagram to update."},
            },
            "required": [],
        },
        "annotations": {"readOnlyHint": false, "idempotentHint": false},
    })
}

fn number(value: Option<&Value>, name: &str) -> Result<f64> {
    let number = value.and_then(Value::as_f64).with_context(|| format!("element {name} must be a number"))?;
    ensure!(number.is_finite() && number.abs() <= MAX_NUMBER, "element {name} is out of range");
    Ok(number)
}

fn string<'a>(value: Option<&'a Value>, name: &str, max: usize) -> Result<&'a str> {
    let text = value.and_then(Value::as_str).with_context(|| format!("element {name} must be text"))?;
    ensure!(text.chars().count() <= max, "element {name} is too long");
    ensure!(!text.chars().any(|c| c == '\u{0}'), "element {name} holds an invalid character");
    Ok(text)
}

fn color(value: Option<&Value>, name: &str) -> Result<()> {
    let Some(value) = value else { return Ok(()) };
    let text = value.as_str().with_context(|| format!("element {name} must be text"))?;
    ensure!(text.len() <= 64 && !text.chars().any(char::is_control), "element {name} is not a color");
    ensure!(
        text == "transparent"
            || text.starts_with('#')
            || text.starts_with("rgb(")
            || text.starts_with("rgba(")
            || text.starts_with("hsl("),
        "element {name} is not a color"
    );
    Ok(())
}

fn points(value: Option<&Value>) -> Result<()> {
    let Some(value) = value else { return Ok(()) };
    let points = value.as_array().context("element points must be a list")?;
    ensure!(points.len() <= 10_000, "an element has too many points");
    for point in points {
        let pair = point.as_array().context("a point must be a pair of numbers")?;
        ensure!(pair.len() == 2, "a point must be a pair of numbers");
        number(pair.first(), "point x")?;
        number(pair.get(1), "point y")?;
    }
    Ok(())
}

fn validate_element(value: &Value) -> Result<()> {
    let element = value.as_object().context("every element must be an object")?;
    let id = string(element.get("id"), "id", MAX_ID)?;
    ensure!(!id.is_empty(), "an element needs an id");
    let kind = string(element.get("type"), "type", 32)?;
    ensure!(!kind.is_empty(), "an element needs a type");
    for name in ["x", "y"] {
        number(element.get(name), name)?;
    }
    for name in ["width", "height"] {
        if element.get(name).is_some() {
            ensure!(number(element.get(name), name)? >= 0.0, "element {name} can't be negative");
        }
    }
    if element.get("angle").is_some() {
        number(element.get("angle"), "angle")?;
    }
    for name in ["strokeColor", "backgroundColor"] {
        color(element.get(name), name)?;
    }
    if let Some(text) = element.get("text") {
        string(Some(text), "text", MAX_TEXT)?;
    }
    points(element.get("points"))?;
    Ok(())
}

/// Rejects anything that is not a bounded Excalidraw scene.
pub fn validate(scene: &Value) -> Result<()> {
    let value = scene.as_object().context("the diagram must be a JSON object")?;
    ensure!(value.get("type").and_then(Value::as_str) == Some("excalidraw"), "the scene must have type \"excalidraw\"");
    ensure!(value.get("elements").is_some_and(Value::is_array), "the scene needs an elements list");
    let elements = value["elements"].as_array().expect("checked above");
    ensure!(elements.len() <= MAX_ELEMENTS, "a diagram can hold at most {MAX_ELEMENTS} elements");
    for element in elements {
        validate_element(element)?;
    }
    if let Some(files) = value.get("files") {
        let files = files.as_object().context("scene files must be an object")?;
        ensure!(files.len() <= 64, "a diagram can hold at most 64 images");
    }
    Ok(())
}

/// The bytes an artifact keeps: the scene, validated and written compactly.
pub fn encode(scene: &Value) -> Result<Vec<u8>> {
    validate(scene)?;
    let bytes = serde_json::to_vec(scene)?;
    ensure!(u64::try_from(bytes.len())? <= MAX_SCENE, "a diagram can be at most 2 MiB");
    Ok(bytes)
}

/// Writes a scene whole, or not at all. Returns its size and SHA-256.
pub fn write(path: &Path, scene: &Value) -> Result<(u64, String)> {
    let bytes = encode(scene)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating the diagrams folder")?;
    }
    let partial = path.with_extension("partial");
    {
        let mut file = std::fs::File::create(&partial)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&partial, path)?;
    Ok((u64::try_from(bytes.len())?, files::digest_hex(&Sha256::digest(&bytes))))
}

/// One chunk of a stored scene.
pub fn read(path: &Path, size: u64, offset: u64) -> Result<Vec<u8>> {
    ensure!(offset <= size, "invalid diagram offset");
    files::read_chunk(path, size, offset, "the diagram is missing or damaged")
}

/// Parses a stored scene. Used by tests and by an owner reading a diagram back.
pub fn parse(bytes: &[u8]) -> Result<Value> {
    let scene: Value = serde_json::from_slice(bytes).context("the diagram is not valid JSON")?;
    validate(&scene)?;
    Ok(scene)
}

/// The scene an agent hands over, from a JSON file.
pub fn from_path(path: &Path) -> Result<Value> {
    let input = files::open_regular(path)?;
    ensure!(input.metadata()?.len() <= MAX_SCENE, "a diagram can be at most 2 MiB");
    let mut bytes = Vec::new();
    input.take(MAX_SCENE + 1).read_to_end(&mut bytes)?;
    ensure!(u64::try_from(bytes.len())? <= MAX_SCENE, "a diagram can be at most 2 MiB");
    parse(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> Value {
        json!({
            "type": "excalidraw",
            "version": 2,
            "source": "codync",
            "elements": [{
                "id": "box", "type": "rectangle", "x": 10, "y": 20, "width": 120, "height": 60,
                "angle": 0, "strokeColor": "#1e1e1e", "backgroundColor": "transparent",
                "text": "Start"
            }],
            "appState": {"viewBackgroundColor": "#ffffff"},
        })
    }

    #[test]
    fn scenes_are_validated_for_shape_size_and_geometry() {
        validate(&scene()).unwrap();
        let mut wrong: Value = scene();
        wrong["type"] = "html".into();
        assert!(validate(&wrong).is_err(), "not an Excalidraw scene");
        let mut lost: Value = scene();
        lost.as_object_mut().unwrap().remove("elements");
        assert!(validate(&lost).is_err(), "no elements");
        let mut huge: Value = scene();
        huge["elements"] = Value::Array((0..MAX_ELEMENTS + 1).map(|_| scene()["elements"][0].clone()).collect());
        assert!(validate(&huge).is_err(), "too many elements");
        let mut far: Value = scene();
        far["elements"][0]["x"] = json!(MAX_NUMBER * 2.0);
        assert!(validate(&far).is_err(), "out of range");
        let mut bad_color: Value = scene();
        bad_color["elements"][0]["strokeColor"] = "<script>".into();
        assert!(validate(&bad_color).is_err(), "a color, not markup");
        let mut long: Value = scene();
        long["elements"][0]["text"] = json!("x".repeat(MAX_TEXT + 1));
        assert!(validate(&long).is_err(), "text is bounded");
    }

    #[test]
    fn a_scene_saves_and_reopens_whole() {
        let dir = std::env::temp_dir().join(format!("codync-diagram-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("diagram.scene.json");
        let (size, sha) = write(&path, &scene()).unwrap();
        let first = read(&path, size, 0).unwrap();
        let reopened = parse(&first).unwrap();
        assert_eq!(reopened, scene());
        assert_eq!(sha, files::digest_hex(&Sha256::digest(&first)));
        assert!(read(&path, size, size + 1).is_err(), "past the end");
        let _ = std::fs::remove_dir_all(dir);
    }
}
