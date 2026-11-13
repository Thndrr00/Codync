//! What a workflow produced: its type, its MIME and a poster for the card. ComfyUI writes whatever
//! a node saves; a video must not reach the chat as a PNG with a wrong extension, so every output is
//! classified by its extension and copied with the type it really has.

use crate::tools::comfy;
use std::path::{Path, PathBuf};

/// One finished file and how the apps should show it.
#[derive(Clone, Debug)]
pub struct Output {
    pub path: PathBuf,
    /// `image`, `video`, `audio`, `mesh` or `file`.
    pub kind: &'static str,
    pub mime: &'static str,
    /// A JPEG frame for a video, made once with ffmpeg; `None` when ffmpeg is not installed.
    pub thumbnail: Option<PathBuf>,
}

impl Output {
    pub fn describe(path: &Path) -> Self {
        let kind = kind_of(path);
        Self { path: path.to_owned(), kind, mime: mime_of(path), thumbnail: poster(path, kind) }
    }

    pub fn public(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path.to_string_lossy(),
            "kind": self.kind,
            "mime": self.mime,
            "thumbnail": self.thumbnail.as_ref().map(|t| t.to_string_lossy()),
        })
    }
}

/// The type a file has, from its extension; ComfyUI nodes keep the extension they saved with.
pub fn kind_of(path: &Path) -> &'static str {
    match extension(path).as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tiff" | "avif" => "image",
        "mp4" | "webm" | "mov" | "mkv" | "avi" | "m4v" => "video",
        "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => "audio",
        "glb" | "gltf" | "obj" | "ply" | "stl" | "splat" | "ksplat" | "spz" => "mesh",
        _ => "file",
    }
}

pub fn mime_of(path: &Path) -> &'static str {
    match extension(path).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" => "image/tiff",
        "avif" => "image/avif",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" | "opus" => "audio/ogg",
        "m4a" | "aac" => "audio/mp4",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "obj" => "model/obj",
        "ply" => "application/octet-stream",
        "stl" => "model/stl",
        "splat" | "ksplat" | "spz" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

fn extension(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase()
}

/// A video's first half second, as a small JPEG in [`comfy::thumbs_dir`]. Images, audio and meshes
/// need no poster: the chat shows the picture, the player draws the waveform and the viewer the mesh.
fn poster(path: &Path, kind: &str) -> Option<PathBuf> {
    if kind != "video" {
        return None;
    }
    let target = comfy::thumbs_dir().join(format!("{}.jpg", cache_name(path)));
    if target.is_file() {
        return Some(target);
    }
    if std::fs::create_dir_all(comfy::thumbs_dir()).is_err() {
        return None;
    }
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-ss", "0.5", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-vf", "scale='min(640,iw)':-2"])
        .arg(&target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    (status.success() && target.is_file()).then_some(target)
}

/// A name that changes with the file: the same output re-rendered gets a new poster.
fn cache_name(path: &Path) -> String {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(&mut hasher);
        if let Ok(modified) = meta.modified() {
            modified.hash(&mut hasher);
        }
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let stem: String = stem.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(40).collect();
    format!("{stem}-{:016x}", hasher.finish())
}

/// What `upload_image`-style tools accept: pictures a `LoadImage` reads, the audio and video custom
/// nodes read from the same input folder, and the scene files a splat, camera pack or mesh node
/// reads by path (`SpEdE` scenes, `Reshoot` captures, SMPL-X bodies).
pub fn ingestible(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "bmp"
            | "gif"
            | "wav"
            | "mp3"
            | "flac"
            | "ogg"
            | "mp4"
            | "webm"
            | "mov"
            | "ply"
            | "npz"
            | "spz"
            | "splat"
            | "obj"
            | "glb"
            | "gltf"
            | "zip"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_keeps_its_type_and_mime() {
        assert_eq!(kind_of(Path::new("a.PNG")), "image");
        assert_eq!(mime_of(Path::new("a.PNG")), "image/png");
        assert_eq!(kind_of(Path::new("clip.mp4")), "video");
        assert_eq!(mime_of(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(kind_of(Path::new("voice.wav")), "audio");
        assert_eq!(mime_of(Path::new("voice.wav")), "audio/wav");
        assert_eq!(kind_of(Path::new("scene.glb")), "mesh");
        assert_eq!(mime_of(Path::new("scene.glb")), "model/gltf-binary");
        assert_eq!(kind_of(Path::new("notes.txt")), "file");
        assert_eq!(mime_of(Path::new("notes.txt")), "application/octet-stream");
    }

    #[test]
    fn only_inputs_a_loader_reads_are_accepted() {
        assert!(ingestible(Path::new("cat.jpg")) && ingestible(Path::new("take.wav")));
        assert!(ingestible(Path::new("scene.ply")) && ingestible(Path::new("cameras.npz")));
        assert!(!ingestible(Path::new("run.exe")) && !ingestible(Path::new("no-extension")));
    }

    #[test]
    fn a_poster_name_changes_with_the_file() {
        let path = Path::new("C:/out/clip.mp4");
        assert!(cache_name(path).starts_with("clip-"));
        assert_eq!(poster(Path::new("C:/out/clip.mp4"), "audio"), None);
    }
}
