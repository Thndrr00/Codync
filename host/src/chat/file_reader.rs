//! A read-only tool for the agent to inspect a file it shared with the user (`send_file`). It
//! returns only bounded metadata — kind, byte size, dimensions from the header when available, and
//! a short description from the file name for images. No bytes go into the model and nothing about
//! the file goes into analytics; the tool reuses `chat::files` for storage and safe opening.
//! Ported from OpenBot nightly's image/video reader tools.

use crate::chat::files;
use crate::hub::Hub;
use anyhow::{Context as _, Result, anyhow, ensure};
use serde_json::{Value, json};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

/// The chat tool name. Register it with the Chat MCP server; `api/bots.rs` dispatches it here.
pub const TOOL: &str = "read_shared_file";
/// Only this much of the file is read, and only to find header dimensions. It is never returned.
const MAX_HEADER: usize = 256 * 1024;

pub fn tools() -> [Value; 1] {
    [json!({
        "name": TOOL,
        "description": "Read bounded metadata of a file this bot shared with the user: its kind (image, video or file), byte size, and width/height when the file header holds them. Images also get a short description from the file name. Never returns file bytes.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path of the shared file, absolute or relative to this bot's shared-files folder."},
            },
            "required": ["path"],
        },
        "annotations": {"readOnlyHint": true},
    })]
}

/// Resolves a path the agent names to a file inside this bot's shared-files folder. Absolute paths
/// are accepted only when they are already under `root`; traversal and drive-relative paths are
/// refused. Pure so the boundary is testable without a host.
pub(crate) fn within(root: &Path, path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = Path::new(&trimmed.replace('\\', "/"));
    if candidate.components().any(|part| part == Component::ParentDir) {
        return None;
    }
    // An absolute path (including a Windows drive path) is only allowed when it already starts
    // with the bot's folder; the join keeps a relative path from ever escaping it.
    let resolved = if candidate.is_absolute() { candidate.to_path_buf() } else { root.join(candidate) };
    resolved.starts_with(root).then_some(resolved)
}

/// A coarse kind from the file name. The extension only classifies; it never restricts the file.
fn kind_of(name_or_path: &str) -> &'static str {
    let extension =
        Path::new(name_or_path).extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "heif" | "avif" | "svg" => "image",
        "mp4" | "mov" | "m4v" | "webm" | "mkv" | "avi" | "ogv" | "3gp" => "video",
        _ => "file",
    }
}

/// A short, bounded phrase from the file name for an image. Never the file's content, and no claim
/// about what the picture shows beyond the name the user or agent gave it.
fn description(name_or_path: &str) -> String {
    let clean = name_or_path.replace('\\', "/");
    let name = Path::new(&clean).file_name().and_then(|value| value.to_str()).unwrap_or(name_or_path);
    let stem = Path::new(name).file_stem().and_then(|value| value.to_str()).unwrap_or(name);
    let readable: String = stem.chars().map(|c| if c == '_' || c == '-' || c == '.' { ' ' } else { c }).collect();
    let readable = readable.split_whitespace().collect::<Vec<_>>().join(" ");
    let readable = if readable.is_empty() { "unnamed" } else { &readable };
    const MAX: usize = 96;
    if readable.chars().count() + 7 <= MAX {
        return format!("Image: {readable}");
    }
    let mut short = format!("Image: {}", readable.chars().take(MAX - 8).collect::<String>());
    short.push('…');
    short
}

/// Width and height from the header prefix, when the format keeps them there. Reads no bytes past
/// `MAX_HEADER` and returns nothing but two numbers. PNG, GIF and JPEG are the common cases; a
/// format without them in the prefix simply leaves the dimensions out.
fn dimensions(header: &[u8]) -> Option<(u32, u32)> {
    if header.len() >= 24 && header.starts_with(b"\x89PNG\r\n\x1a\n") && header.get(12..16) == Some(b"IHDR".as_slice())
    {
        let width = u32::from_be_bytes(header[16..20].try_into().ok()?);
        let height = u32::from_be_bytes(header[20..24].try_into().ok()?);
        return (width > 0 && height > 0).then_some((width, height));
    }
    if header.len() >= 10 && header.starts_with(b"GIF8") {
        let width = u16::from_le_bytes([header[6], header[7]]) as u32;
        let height = u16::from_le_bytes([header[8], header[9]]) as u32;
        return (width > 0 && height > 0).then_some((width, height));
    }
    if header.starts_with(&[0xff, 0xd8]) {
        let mut at = 2usize;
        while at + 9 <= header.len() {
            if header[at] != 0xff {
                at += 1;
                continue;
            }
            let marker = header[at + 1];
            if marker == 0xff {
                at += 1;
                continue;
            }
            if marker == 0x01 || marker == 0xd8 || (0xd0..=0xd7).contains(&marker) {
                at += 2;
                continue;
            }
            if marker == 0xd9 || marker == 0xda {
                break;
            }
            let length = u16::from_be_bytes([header[at + 2], header[at + 3]]) as usize;
            if length < 2 || at + 2 + length > header.len() {
                break;
            }
            if (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc) {
                let height = u16::from_be_bytes([header[at + 5], header[at + 6]]) as u32;
                let width = u16::from_be_bytes([header[at + 7], header[at + 8]]) as u32;
                if width > 0 && height > 0 {
                    return Some((width, height));
                }
            }
            at += 2 + length;
        }
    }
    None
}

/// The bounded answer the tool returns. Pure so its shape is testable without any file I/O.
pub(crate) fn describe(name_or_path: &str, size: u64, header: &[u8]) -> Value {
    let kind = kind_of(name_or_path);
    let mut answer = json!({"kind": kind, "size": size});
    if matches!(kind, "image" | "video")
        && let Some((width, height)) = dimensions(header)
    {
        answer["width"] = json!(width);
        answer["height"] = json!(height);
    }
    if kind == "image" {
        answer["description"] = description(name_or_path).into();
    }
    answer
}

/// `read_shared_file`: metadata only, with a size cap and the bot's own folder as the boundary.
pub fn call(hub: &Hub, bot: &str, args: &Value) -> Result<Value> {
    let path =
        args["path"].as_str().filter(|value| !value.trim().is_empty()).ok_or_else(|| anyhow!("path is required"))?;
    let root = files::root(bot);
    let resolved = within(&root, path).ok_or_else(|| anyhow!("path is outside this bot's shared files"))?;
    // Resolve symlinks too: a link inside the folder must not point at a file outside it.
    let real = resolved.canonicalize().context("shared file is missing")?;
    let real_root = root.canonicalize().unwrap_or_else(|_| root.clone());
    ensure!(real.starts_with(&real_root), "path is outside this bot's shared files");
    let meta = std::fs::metadata(&real).context("shared file is missing")?;
    ensure!(meta.is_file(), "only regular shared files can be read");
    let size = meta.len();
    ensure!(size <= crate::chat::uploads::MAX_FILE, "shared file is too large to read");
    let mut header = Vec::new();
    files::open_regular(&real)?.take(MAX_HEADER as u64).read_to_end(&mut header)?;
    Ok(describe(path, size, &header))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_paths_inside_the_bot_folder_are_accepted() {
        let root = Path::new("/data/bots/bot/files");
        assert_eq!(within(root, "a.png"), Some(root.join("a.png")));
        assert_eq!(within(root, "/data/bots/bot/files/a.png"), Some(root.join("a.png")));
        assert_eq!(within(root, "../secret.png"), None, "traversal is refused");
        assert_eq!(within(root, "a/../../secret.png"), None, "traversal after a folder is refused too");
        assert_eq!(within(root, "/etc/passwd"), None, "a path outside the folder is refused");
        assert_eq!(within(root, "  "), None, "an empty path names nothing");
    }

    #[test]
    fn metadata_is_bounded_and_never_carries_file_bytes() {
        // PNG signature + IHDR with a 1024x768 screen.
        let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x04\x00\x00\x00\x03\x00";
        let out = describe("holiday_photo.png", 1234, png);
        assert_eq!(out["kind"], "image");
        assert_eq!(out["size"], 1234);
        assert_eq!(out["width"], 1024);
        assert_eq!(out["height"], 768);
        assert_eq!(out["description"], "Image: holiday photo");
        assert!(out.get("data").is_none(), "no raw bytes go to the model");

        let video = describe("clip.mp4", 99, &[]);
        assert_eq!(video["kind"], "video");
        assert!(video.get("description").is_none(), "only images get a description");
        assert!(video.get("width").is_none(), "no dimensions without a header");

        let other = describe("notes.txt", 7, b"hello world");
        assert_eq!(other["kind"], "file");
        assert_eq!(other.as_object().unwrap().len(), 2, "other files carry only kind and size");

        let long = format!("{}.png", "x".repeat(200));
        let out = describe(&long, 1, png);
        assert!(out["description"].as_str().unwrap().chars().count() <= 96, "descriptions are capped");
    }
}
