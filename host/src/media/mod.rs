//! Images and videos made with the user's own agent subscription, ported from `OpenBot` 0.70.0
//! (`src/backend/agent/image-generation.ts`, `src/backend/grok-media.ts`,
//! `src/backend/workflows/workflow-walk.ts` `mediaStepPrompt`, and
//! `packages/contracts/src/ipc-workflows.ts` `WORKFLOW_MEDIA_MODELS`).
//!
//! No API key is added and no second runner starts: the Codex CLI's `image_gen` and the Grok CLI's
//! `image_gen`, `image_edit`, `image_to_video` and `reference_to_video` tools make the file inside
//! the bot's own turn, and its subscription pays for it. The host writes the request (`command`),
//! reads what the tool left (`output`) and keeps the picture as a file message (`share`).
//!
//! The command is a chat message that starts with `/image` or `/video`; its flags say what to
//! make. Clients offer the flags as a picker, so a person never types one. Ported behaviour that
//! matters: an unknown or unsupported setting falls back to the model's first choice rather than
//! failing the turn, a transparent background is asked for in words the provider's tool obeys
//! (Codex's tool makes clear pixels, Grok's cannot, so it gets one flat colour), and a video names
//! its frames, its pictures and its voice by the tool's own inputs.

pub mod batch;
pub mod catalog;
pub mod command;
pub mod flow;
pub mod output;
pub mod size;
pub mod sunra;

pub use batch::{
    BATCH_SIZES, BatchResult, BatchStatus, DEFAULT_BATCH, MAX_BATCH, MAX_STEP_FILES, MODEL_UNAVAILABLE,
    REFERENCE_RETENTION, REFERENCE_RETENTION_DAYS, batch_credits, batch_credits_view, batch_results, default_batch,
    kept_files, media_batch, model_available, reference_expired,
};
pub use output::{Output, decode_base64, display_name, display_name_from_prompt, kind, output};

use anyhow::{Result, bail, ensure};
use command::{Frames, Request};
use serde_json::Value;

/// The aspect ratios a model makes when it names none; the first is the default. Ported from
/// `OpenBot` 0.73.0, which added 4:5 for a social post and 5:4 for a print.
pub const ASPECTS: [&str; 6] = ["1:1", "4:3", "16:9", "9:16", "4:5", "5:4"];
/// The lengths of a video in seconds, for a model that names none.
pub const DURATIONS: [u32; 2] = [6, 10];
/// The resolutions of a video, for a model that names none; the first is the default.
pub const RESOLUTIONS: [&str; 2] = ["480p", "720p"];
/// The preset voices the Grok Imagine video tools speak in (Grok CLI 1.0.77).
pub const VOICES: [(&str, &str); 4] = [("ara", "Ara"), ("eve", "Eve"), ("leo", "Leo"), ("rex", "Rex")];
/// The most reference pictures a Grok video tool takes.
pub const MAX_REFERENCES: usize = 14;
/// The longest description a request may carry.
pub const MAX_PROMPT_BYTES: usize = 8_000;

/// What a model makes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Image,
    Video,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

/// What a model makes, as the clients list it: a model never offers a flag it ignores.
pub fn capabilities(model: &catalog::Model) -> Value {
    catalog::capabilities_view(
        catalog::aspects(model),
        catalog::durations(model),
        catalog::resolutions(model),
        model.transparent,
        model.voices,
    )
}

/// The nearest value of `choices` to `asked`, or the first choice: a setting kept from another
/// model becomes one this one makes (port of `workflowMediaSpecs`).
pub fn nearest(choices: &[u32], asked: u32) -> u32 {
    choices
        .iter()
        .copied()
        .reduce(|best, value| if value.abs_diff(asked) <= best.abs_diff(asked) { value } else { best })
        .unwrap_or(asked)
}

/// The first words of a prompt as a safe, short file-name stem. Pure: only letters and digits
/// (of any script), `-` and `_` survive, so a path, quote or control character never reaches a name.
fn prompt_stem(prompt: &str) -> Option<String> {
    const WORDS: usize = 5;
    const CHARS: usize = 40;
    let stem: String = prompt
        .split_whitespace()
        .filter_map(|word| {
            let clean: String = word.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
            (!clean.is_empty()).then_some(clean)
        })
        .take(WORDS)
        .collect::<Vec<_>>()
        .join("-");
    let stem: String = stem.chars().take(CHARS).collect::<String>().to_lowercase();
    let stem = stem.trim_matches(['-', '_']);
    (!stem.is_empty()).then(|| stem.to_owned())
}

/// OpenBot 0.72.0: a made file is named from the first words of its prompt, so the chat shows what
/// was asked for instead of `generated-video.mp4`. Pure; an empty or punctuation-only prompt falls
/// back to the generic name.
pub fn name_from_prompt(prompt: &str, ext: &str) -> String {
    format!("{}.{ext}", prompt_stem(prompt).unwrap_or_else(|| "generated".to_owned()))
}

/// The safe display name of a made file: `generated-image.png`, or `generated-video.mp4`. The
/// extension comes from what the tool wrote, and is one the clients can play or show.
pub fn name(kind: Kind, path: &str, index: usize) -> String {
    name_with_prompt(kind, path, index, None)
}

/// As [`name`], but named after the first words of the prompt when it says something.
pub fn name_with_prompt(kind: Kind, path: &str, index: usize, prompt: Option<&str>) -> String {
    let extension = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    let allowed: &[&str] = match kind {
        Kind::Image => &["png", "jpg", "jpeg", "webp", "gif", "avif"],
        Kind::Video => &["mp4", "webm", "mov"],
    };
    let extension = if allowed.contains(&extension.as_str()) {
        extension
    } else {
        match kind {
            Kind::Image => "png".to_owned(),
            Kind::Video => "mp4".to_owned(),
        }
    };
    let stem = prompt.and_then(prompt_stem).unwrap_or_else(|| format!("generated-{}", kind.as_str()));
    let suffix = if index == 0 { String::new() } else { format!("-{}", index + 1) };
    format!("{stem}{suffix}.{extension}")
}

/// A setting of a request that the model does not make, named for the person who asked.
pub fn unsupported(what: &str, value: &str, model: &catalog::Model) -> Result<()> {
    bail!("{} does not make {what} {value}; try another model", model.name)
}

/// The request a flow's media node asks for, checked against its model.
pub fn request(node: &crate::store::FlowMedia) -> Result<Request> {
    let request = check_settings(node)?;
    ensure!(!request.prompt.is_empty(), "the step has no description yet");
    command::check(&request)?;
    Ok(request)
}

/// A node's settings as a request, without needing its description yet.
pub fn check_settings(node: &crate::store::FlowMedia) -> Result<Request> {
    let kind = match node.kind.as_str() {
        "image" => Kind::Image,
        "video" => Kind::Video,
        other => bail!("unknown media kind `{other}`"),
    };
    let model = catalog::model(&node.model).ok_or_else(|| anyhow::anyhow!("unknown media model `{}`", node.model))?;
    ensure!(model.kinds.contains(&kind), "{} does not make a {}; pick another model", model.name, kind.as_str());
    let aspect = catalog::aspects(model)
        .iter()
        .find(|value| **value == node.aspect)
        .ok_or_else(|| anyhow::anyhow!("{} does not make the frame {}", model.name, node.aspect))?;
    let request = Request {
        model,
        kind,
        prompt: node.prompt.trim().to_owned(),
        aspect: (*aspect).to_owned(),
        batch: media_batch(node.batch),
        duration: node.duration,
        resolution: (kind == Kind::Video).then(|| {
            catalog::resolutions(model)
                .iter()
                .find(|value| **value == node.resolution)
                .map(|value| (*value).to_owned())
                .unwrap_or_default()
        }),
        transparent: node.transparent && kind == Kind::Image && model.transparent,
        voice: node
            .voice
            .clone()
            .filter(|voice| kind == Kind::Video && model.voices && VOICES.iter().any(|(id, _)| id == voice)),
        frames: Frames::default(),
        size: None,
    };
    if let Some(resolution) = &request.resolution {
        ensure!(!resolution.is_empty(), "{} does not make the size {}", model.name, node.resolution);
    }
    if kind == Kind::Video {
        // OpenBot 0.74.0: a flow step may pick any length in the model's range, not just its presets.
        ensure!(catalog::valid_duration(request.duration), "{} does not make {} seconds", model.name, node.duration);
    }
    if node.voice.is_some() && kind == Kind::Video && model.voices && request.voice.is_none() {
        bail!("unknown voice `{}`", node.voice.as_deref().unwrap_or_default());
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_keeps_a_shown_extension_and_falls_back_for_others() {
        assert_eq!(name(Kind::Image, "/tmp/a.webp", 0), "generated-image.webp");
        assert_eq!(name(Kind::Image, "/tmp/a.bmp", 0), "generated-image.png");
        assert_eq!(name(Kind::Video, "/tmp/a.mov", 1), "generated-video-2.mov");
        assert_eq!(name(Kind::Video, "/tmp/a.m4v", 0), "generated-video.mp4");
    }

    #[test]
    fn a_prompt_names_a_file_with_its_first_safe_words() {
        assert_eq!(name_from_prompt("A red fox in the snow, cinematic lighting", "mp4"), "a-red-fox-in-the.mp4");
        assert_eq!(name_from_prompt("  multiple   spaces\n and dashes  ", "png"), "multiple-spaces-and-dashes.png");
        assert_eq!(name_from_prompt("../etc/passwd \\ dangerous: name", "mp4"), "etcpasswd-dangerous-name.mp4");
        // Punctuation-only, empty and whitespace prompts keep the generic name.
        for prompt in ["", "   ", "!!! ???", "— —"] {
            assert_eq!(name_from_prompt(prompt, "mp4"), "generated.mp4", "{prompt}");
        }
        // Unicode letters survive, but a name is capped by characters, never cut mid-codepoint.
        let long = "狐狸 在 雪地 上 奔跑 ".repeat(20);
        let named = name_from_prompt(&long, "png");
        assert!(named.starts_with("狐狸-在-雪地-上-奔跑.png"), "{named}");
        assert!(named.chars().count() <= 40 + 4, "{named}");
        let capped = name_from_prompt(&"a".repeat(200), "mp4");
        assert_eq!(capped.chars().count(), 40 + 4, "five long words still cap at 40 characters");
        assert!(!capped.contains("--"), "the cap cannot leave a dangling separator");
    }

    #[test]
    fn a_named_file_keeps_its_extension_and_batch_suffix() {
        assert_eq!(
            name_with_prompt(Kind::Video, "/tmp/a.mov", 2, Some("a paper boat in a gutter")),
            "a-paper-boat-in-a-3.mov"
        );
        assert_eq!(name_with_prompt(Kind::Video, "/tmp/a.m4v", 0, None), "generated-video.mp4");
        assert_eq!(name_with_prompt(Kind::Image, "/tmp/a.bmp", 0, Some("   ")), "generated-image.png");
        assert_eq!(name(Kind::Image, "/tmp/a.webp", 0), "generated-image.webp", "the old path is unchanged");
    }

    #[test]
    fn a_length_moves_to_the_nearest_one_a_model_makes() {
        assert_eq!(nearest(&[4, 6, 8], 7), 8);
        assert_eq!(nearest(&[4, 6, 8], 1), 4);
        assert_eq!(nearest(&[], 9), 9);
    }

    #[test]
    fn an_unsupported_setting_names_the_model() {
        let model = catalog::model("gpt-image").unwrap();
        let error = unsupported("a video", "", model).unwrap_err().to_string();
        assert!(error.contains("GPT Image"), "{error}");
    }

    #[test]
    fn a_step_request_bounds_its_batch_and_needs_a_description() {
        let mut node = crate::store::flow_media::new_media("m".into(), "r".into(), "b".into());
        node.model = "gpt-image".into();
        node.batch = 9;
        assert!(check_settings(&node).is_err(), "a step without a description is not ready");
        node.prompt = "a red fox".into();
        assert_eq!(check_settings(&node).unwrap().batch, 4);
        node.batch = 0;
        assert_eq!(request(&node).unwrap().batch, 1);
    }
}
