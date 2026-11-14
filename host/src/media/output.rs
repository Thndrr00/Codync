//! What a media tool call leaves behind, ported from `OpenBot` 0.70.0 `grok-media.ts` and
//! `image-generation.ts`.
//!
//! Only the first updates of a Grok call name the tool, in `_meta["x.ai/tool"].name`; a later one
//! renames the call to "imagine: …". The completed update names the file it wrote as
//! `rawOutput.path`, with `rawOutput.type` "`ImageGen`", "`ImageEdit`", "`ImageToVideo`" or
//! "`ReferenceToVideo`". Codex's `image_gen` names the item `image_generation` and gives the picture
//! inline in `result` (base64 or a data URL), sometimes with `saved_path`. The kind is read from
//! the tool the CLI named, never from what the model wrote as arguments.

use super::Kind;
use anyhow::{Result, bail, ensure};
use base64::Engine as _;
use serde_json::Value;

/// The tools that make a picture, by the name the CLI gives them.
const TOOL_KINDS: [(&str, Kind); 5] = [
    ("image_gen", Kind::Image),
    ("image_edit", Kind::Image),
    ("image_to_video", Kind::Video),
    ("reference_to_video", Kind::Video),
    ("image_generation", Kind::Image),
];

/// The output types that name a kind, when the update does not name the tool.
const OUTPUT_KINDS: [(&str, Kind); 4] = [
    ("ImageGen", Kind::Image),
    ("ImageEdit", Kind::Image),
    ("ImageToVideo", Kind::Video),
    ("ReferenceToVideo", Kind::Video),
];

/// The media kind the update names, or `None` when it names no media tool. The tool name is read
/// from `_meta`, the item type, `rawInput.tool` or `title`, then the output or input type.
pub fn kind(update: &Value) -> Option<Kind> {
    let named = update["_meta"]["x.ai/tool"]["name"]
        .as_str()
        .or_else(|| update["type"].as_str())
        .or_else(|| update["rawInput"]["tool"].as_str())
        .or_else(|| update["title"].as_str().filter(|title| !title.contains(' ')));
    if let Some(kind) = named.and_then(|name| TOOL_KINDS.iter().find(|(tool, _)| *tool == name).map(|(_, kind)| *kind))
    {
        return Some(kind);
    }
    for value in [&update["rawOutput"], &update["rawInput"]] {
        let named = value["type"].as_str().or_else(|| value["variant"].as_str());
        if let Some(kind) = named.and_then(|name| OUTPUT_KINDS.iter().find(|(t, _)| *t == name).map(|(_, k)| *k)) {
            return Some(kind);
        }
    }
    None
}

/// What one completed media call left: a path it wrote, or the picture inline.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// The file the tool wrote, if it wrote one (Grok, and Codex's `saved_path`).
    pub path: Option<String>,
    /// A base64 picture or data URL (Codex's `result`).
    pub data: Option<String>,
}

/// What the update leaves, when it names a file or inline picture.
pub fn output(update: &Value) -> Option<Output> {
    let object = &update["rawOutput"];
    let path = object["path"]
        .as_str()
        .or_else(|| update["saved_path"].as_str())
        .or_else(|| update["savedPath"].as_str())
        .filter(|path| !path.is_empty())
        .map(str::to_owned);
    let data = object["result"]
        .as_str()
        .or_else(|| update["result"].as_str())
        .filter(|data| !data.is_empty())
        .map(str::to_owned);
    (path.is_some() || data.is_some()).then_some(Output { path, data })
}

/// The display name of a made file (see `super::name`).
pub fn display_name(kind: Kind, path: Option<&str>, index: usize) -> String {
    super::name(kind, path.unwrap_or_default(), index)
}

/// The bytes of a base64 picture or data URL. The most a file may be is `limit`.
pub fn decode_base64(value: &str, limit: usize) -> Result<Vec<u8>> {
    let payload = value.split_once(',').filter(|(head, _)| head.starts_with("data:")).map_or(value, |(_, body)| body);
    let payload: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    ensure!(payload.len() % 4 != 1, "the generated data is not base64");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.as_bytes())
        .map_err(|_| anyhow::anyhow!("the generated data is not base64"))?;
    ensure!(!bytes.is_empty(), "the generated file is empty");
    if bytes.len() > limit {
        bail!("the generated file is larger than {} MB", limit / 1024 / 1024);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_tool_of_both_a_generation_and_an_edit_is_known() {
        for (tool, made) in TOOL_KINDS {
            assert_eq!(kind(&json!({"_meta": {"x.ai/tool": {"name": tool}}})), Some(made), "{tool}");
            assert_eq!(kind(&json!({"title": tool})), Some(made));
            assert_eq!(kind(&json!({"rawInput": {"tool": tool}})), Some(made));
        }
        assert_eq!(kind(&json!({"title": "imagine: a red fox"})), None);
        assert_eq!(kind(&json!({"title": "Read a file"})), None);
    }

    #[test]
    fn an_output_type_is_read_when_no_name_is() {
        assert_eq!(kind(&json!({"rawOutput": {"type": "ReferenceToVideo"}})), Some(Kind::Video));
        assert_eq!(kind(&json!({"rawOutput": {"type": "ImageGen"}})), Some(Kind::Image));
        assert_eq!(kind(&json!({"rawInput": {"variant": "ImageEdit"}})), Some(Kind::Image));
        assert_eq!(kind(&json!({"rawOutput": {"type": "Other"}})), None);
        // The model's own arguments never decide: a "ratio" is not a type.
        assert_eq!(kind(&json!({"rawInput": {"ratio": "16:9"}})), None);
    }

    #[test]
    fn a_path_and_inline_data_are_both_found() {
        assert_eq!(output(&json!({"rawOutput": {"path": "/tmp/a.png"}})).unwrap().path.as_deref(), Some("/tmp/a.png"));
        assert_eq!(output(&json!({"saved_path": "/tmp/b.mp4"})).unwrap().path.as_deref(), Some("/tmp/b.mp4"));
        assert!(output(&json!({"rawOutput": {"result": "aGk="}})).unwrap().data.is_some());
        assert_eq!(output(&json!({"rawOutput": {}})), None);
        assert_eq!(output(&json!({"rawOutput": {"path": ""}})), None);
    }

    #[test]
    fn a_picture_decodes_from_a_plain_value_or_a_data_url() {
        assert_eq!(decode_base64("aGk=", 100).unwrap(), b"hi");
        assert_eq!(decode_base64("data:image/png;base64,aGk=", 100).unwrap(), b"hi");
        assert_eq!(decode_base64("aG k=", 100).unwrap(), b"hi");
        assert!(decode_base64("", 100).is_err());
        assert!(decode_base64("a", 100).is_err(), "a broken length");
        assert!(decode_base64("not base64!", 100).is_err());
        assert!(decode_base64("aGk=", 1).is_err(), "over the limit");
    }

    #[test]
    fn a_display_name_uses_what_the_tool_wrote() {
        assert_eq!(display_name(Kind::Image, Some("/tmp/x.webp"), 0), "generated-image.webp");
        assert_eq!(display_name(Kind::Video, None, 2), "generated-video-3.mp4");
    }
}
