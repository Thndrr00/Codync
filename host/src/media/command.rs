//! The `/image` and `/video` chat commands: a request for one picture or clip, made with the
//! tools of the bot's own agent. Ported from `OpenBot` 0.70.0 `mediaStepPrompt` and
//! `mediaInputText` (`src/backend/workflows/workflow-walk.ts`): the frame, the length and the
//! resolution go before the description, a transparent background is asked for in the words each
//! provider's tool obeys, and Grok's tools are given their inputs by name.
//!
//! The flags are what a client's picker sends; a person may also type them.

use super::{Kind, MAX_PROMPT_BYTES, MAX_REFERENCES, catalog};
use anyhow::{Result, bail, ensure};
use std::fmt::Write as _;

/// How Codex's `image_gen` tool is asked for clear pixels: it makes a cut-out PNG itself.
const TRANSPARENT_ASKS: [(&str, &str); 2] = [
    ("codex", "Give it a transparent background: a cut-out PNG with clear pixels around the subject."),
    (
        "grok",
        "Put the subject on one flat, solid background colour, with no shadow, floor, gradient or texture, and keep a margin around the subject. Use pure green #00FF00, or pure magenta #FF00FF when the subject is green. Do not use that colour in the subject.",
    ),
];

/// What follows a command: the first frame of a video, the last one, and reference pictures.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Frames {
    pub first: Option<String>,
    pub last: Option<String>,
    pub references: Vec<String>,
}

/// One request for a picture or a clip.
#[derive(Clone, Debug)]
pub struct Request {
    pub model: &'static catalog::Model,
    pub kind: Kind,
    /// What the model is asked to make.
    pub prompt: String,
    pub aspect: String,
    /// How many results to make; one for a chat command, the step's batch in a flow.
    pub batch: u32,
    /// Seconds; only a video uses it.
    pub duration: u32,
    /// `None` for an image of a model without resolutions.
    pub resolution: Option<String>,
    pub transparent: bool,
    /// A preset voice of a video, from `VOICES`.
    pub voice: Option<String>,
    pub frames: Frames,
    /// The exact width and height a picture is cut and scaled to, when asked for.
    pub size: Option<super::size::Size>,
}

fn short(value: &str) -> bool {
    !value.is_empty() && value.len() <= 200 && !value.chars().any(char::is_control)
}

fn path_arg(flag: &str, value: &str) -> Result<String> {
    ensure!(short(value), "`{flag}` needs a file path of at most 200 characters");
    Ok(value.to_owned())
}

fn number_arg(flag: &str, value: &str) -> Result<u32> {
    value.parse().map_err(|_| anyhow::anyhow!("`{flag}` needs a whole number of seconds"))
}

/// The value of the flag the loop just read.
fn take(words: &mut std::iter::Peekable<std::str::SplitWhitespace<'_>>, flag: &str) -> Result<String> {
    words.next().map(str::to_owned).ok_or_else(|| anyhow::anyhow!("`{flag}` needs a value"))
}

/// The request that `text` asks for, after the command `kind` was cut off.
///
/// A model the request does not name is the backend's first for the kind; a setting the model does
/// not make falls back to the model's first choice, so a value kept from another model never fails.
pub fn parse(kind: Kind, backend: &str, text: &str) -> Result<Request> {
    let mut words = text.split_whitespace().peekable();
    let mut model_id: Option<String> = None;
    let mut aspect: Option<String> = None;
    let mut duration: Option<u32> = None;
    let mut resolution: Option<String> = None;
    let mut transparent = false;
    let mut voice: Option<String> = None;
    let mut frames = Frames::default();
    let mut size: Option<super::size::Size> = None;
    let mut rest: Vec<&str> = Vec::new();
    while let Some(word) = words.next() {
        // A flag only counts before the description; "-" and words are the description.
        match word {
            "--model" => model_id = Some(take(&mut words, word)?),
            "--aspect" => aspect = Some(take(&mut words, word)?),
            "--duration" => duration = Some(number_arg(word, &take(&mut words, word)?)?),
            "--resolution" => resolution = Some(take(&mut words, word)?),
            "--voice" => voice = Some(take(&mut words, word)?),
            "--first" => frames.first = Some(path_arg(word, &take(&mut words, word)?)?),
            "--last" => frames.last = Some(path_arg(word, &take(&mut words, word)?)?),
            "--ref" => {
                ensure!(frames.references.len() < MAX_REFERENCES, "at most {MAX_REFERENCES} reference pictures");
                frames.references.push(path_arg(word, &take(&mut words, word)?)?);
            }
            "--size" => size = Some(super::size::parse(&take(&mut words, word)?)?),
            "--transparent" => transparent = true,
            _ => {
                rest.push(word);
                rest.extend(words);
                break;
            }
        }
    }
    let model = match model_id.as_deref() {
        Some(id) => {
            let model = catalog::model(id).ok_or_else(|| anyhow::anyhow!("unknown media model `{id}`"))?;
            ensure!(
                model.kinds.contains(&kind),
                "{} does not make a {}; use {}",
                model.name,
                kind.as_str(),
                if kind == Kind::Image { "/image" } else { "/video" }
            );
            model
        }
        None => catalog::default(backend, kind)
            .ok_or_else(|| anyhow::anyhow!("{} cannot make a {} yet", backend_name(backend), kind.as_str()))?,
    };
    if kind == Kind::Image {
        ensure!(frames.first.is_none() && frames.last.is_none(), "only a video takes a first or a last frame");
    }
    let aspects = catalog::aspects(model);
    let aspect =
        aspects.iter().find(|value| Some(**value) == aspect.as_deref()).copied().unwrap_or(aspects[0]).to_owned();
    let resolutions = catalog::resolutions(model);
    let resolution = (kind == Kind::Video).then(|| {
        resolution.filter(|asked| resolutions.contains(&asked.as_str())).unwrap_or_else(|| resolutions[0].to_owned())
    });
    let duration = super::nearest(catalog::durations(model), duration.unwrap_or(super::DURATIONS[0]));
    let voice = match voice {
        Some(id) if kind == Kind::Video && model.voices => {
            ensure!(
                super::VOICES.iter().any(|(known, _)| *known == id),
                "unknown voice `{id}`; use one of {}",
                super::VOICES.map(|(id, _)| id).join(", ")
            );
            Some(id)
        }
        _ => None,
    };
    let prompt = rest.join(" ");
    ensure!(!prompt.trim().is_empty(), "describe what to make after the command");
    ensure!(prompt.len() <= MAX_PROMPT_BYTES, "the description is longer than {MAX_PROMPT_BYTES} bytes");
    Ok(Request {
        model,
        kind,
        prompt: prompt.trim().to_owned(),
        aspect,
        batch: super::DEFAULT_BATCH,
        duration,
        resolution,
        transparent: transparent && kind == Kind::Image && model.transparent,
        voice,
        frames,
        size: size.filter(|_| kind == Kind::Image),
    })
}

fn backend_name(backend: &str) -> &str {
    catalog::models(backend, None).next().map_or(backend, |model| model.subscription)
}

impl Request {
    /// The tool the agent uses, in the words of its own CLI.
    fn tool(&self) -> &'static str {
        match (self.kind, self.model.backend) {
            (Kind::Video, _) => "reference_to_video",
            (Kind::Image, "grok") if !self.frames.references.is_empty() => "image_edit",
            (Kind::Image, _) => "image_gen",
        }
    }

    /// The note put before the description: the frame and, for a video, its length, resolution,
    /// pictures and voice, then the description, then how to hand the file over.
    pub fn note(&self) -> String {
        let mut parts = Vec::new();
        match self.kind {
            Kind::Video => {
                let ask = format!(
                    "Make one video of {} seconds at {}, aspect ratio {}.",
                    self.duration,
                    self.resolution.as_deref().unwrap_or(super::RESOLUTIONS[0]),
                    self.aspect
                );
                let tool = self.tool();
                let mut lines = vec![format!("Make it with the `{tool}` tool and these inputs:")];
                if let Some(first) = &self.frames.first {
                    lines.push(format!("- first_frame: {first}"));
                }
                if let Some(last) = &self.frames.last {
                    lines.push(format!("- last_frame: {last}"));
                }
                if !self.frames.references.is_empty() {
                    lines.push(
                        "- images, in this order; refer to each in the description with its <IMAGE_n> tag:".to_owned(),
                    );
                    lines.extend(self.frames.references.iter().take(MAX_REFERENCES).map(|path| format!("  - {path}")));
                }
                if let Some(voice) = &self.voice {
                    lines.push(format!(
                        "- voices: [\"{voice}\"]; the person who speaks is <AUDIO_1> in the description."
                    ));
                }
                parts.push(ask);
                parts.push(lines.join("\n"));
            }
            Kind::Image => {
                let mut ask = format!("Make one image with the `{}` tool, aspect ratio {}.", self.tool(), self.aspect);
                if let Some(size) = self.size {
                    // The size is applied here, on the made file: the model only has to get close.
                    let _ = write!(
                        ask,
                        " It will be cut to its middle and scaled to exactly {}x{} pixels (--size {}x{}), so any aspect close to that is fine; do not add borders.",
                        size.width, size.height, size.width, size.height
                    );
                }
                if self.transparent
                    && let Some((_, words)) =
                        TRANSPARENT_ASKS.iter().find(|(backend, _)| *backend == self.model.backend)
                {
                    ask.push(' ');
                    ask.push_str(words);
                }
                if !self.frames.references.is_empty() && self.model.backend == "grok" {
                    let lines =
                        self.frames.references.iter().map(|path| format!("- {path}")).collect::<Vec<_>>().join("\n");
                    let _ = write!(
                        ask,
                        " Edit the pictures of the host into the new image with the `image_edit` tool:\n{lines}"
                    );
                }
                parts.push(ask);
                if self.model.backend != "grok" && !self.frames.references.is_empty() {
                    parts.push(format!("Use these pictures as references: {}", self.frames.references.join(", ")));
                }
            }
        }
        let mut note =
            format!("[Codync: the user asks you to make a {}.\n{}\n", self.kind.as_str(), parts.join("\n\n"));
        note.push_str(&self.prompt);
        note.push('\n');
        note.push_str(
            "Make exactly one file. When the tool is done, share it with `send_file` and the path the tool wrote, \
             then say in one line what you made. If the tool fails, say why once; make no second attempt with another model.]",
        );
        note
    }

    /// The same request as a flow step's prompt: nobody is in the chat to see a `send_file`, so the
    /// agent is told the host reads the files itself, and what the steps before it handed on.
    pub fn note_for_flow(&self, handed: &str) -> String {
        let mut note = self.note();
        let tail = "\nMake exactly one file. When the tool is done, name the path it wrote in one line.";
        note = note.replace(
            "When the tool is done, share it with `send_file` and the path the tool wrote, then say in one line what you made.",
            tail.trim_start_matches('\n'),
        );
        if !handed.trim().is_empty() {
            note.insert_str(note.len() - 1, &format!("\n\nThe steps before it handed on:\n\n{handed}\n"));
        }
        note
    }
}

/// The note for a request that could not be read, so the bot tells the person why.
pub fn refusal(kind: Kind, error: &anyhow::Error) -> String {
    format!("[Codync: the user asks you to make a {}. {} Tell them in one line, then stop.]", kind.as_str(), error)
}

/// `text` as an `/image` or `/video` request for `backend`, or `None` when it is another message.
/// A request that cannot be read still answers a note, with the reason.
pub fn expand(text: &str, backend: &str) -> Option<String> {
    let (command, rest) = match text.trim_start().split_once(char::is_whitespace) {
        Some((command, rest)) => (command, rest),
        None => (text.trim_start(), ""),
    };
    let kind = match command {
        "/image" => Kind::Image,
        "/video" => Kind::Video,
        _ => return None,
    };
    Some(match parse(kind, backend, rest) {
        Ok(request) => request.note(),
        Err(error) => refusal(kind, &error),
    })
}

/// A flag list for a client that builds the command: the choices of a model, as the clients show them.
pub fn flags(kind: Kind, model: &catalog::Model, aspect: &str) -> String {
    let mut flags = format!("--model {} --aspect {aspect}", model.id);
    if kind == Kind::Video {
        let _ = write!(flags, " --duration {}", catalog::durations(model)[0]);
        let _ = write!(flags, " --resolution {}", catalog::resolutions(model)[0]);
    }
    flags
}

/// Every validation problem of a request, before it reaches a bot.
pub fn check(request: &Request) -> Result<()> {
    ensure!(!request.prompt.trim().is_empty(), "empty description");
    ensure!(super::media_batch(request.batch) == request.batch, "unsupported batch");
    ensure!(catalog::aspects(request.model).contains(&request.aspect.as_str()), "unsupported aspect");
    if request.kind == Kind::Video {
        ensure!(
            request.resolution.as_deref().is_some_and(|value| catalog::resolutions(request.model).contains(&value)),
            "unsupported resolution"
        );
        // OpenBot 0.74.0: any whole-second length in the model's range, not only its presets.
        ensure!(catalog::valid_duration(request.duration), "unsupported length");
    } else if request.resolution.is_some() {
        bail!("an image has no resolution");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(text: &str) -> Request {
        parse(Kind::Image, "grok", text).unwrap()
    }

    #[test]
    fn a_plain_request_is_one_grok_image_at_the_first_frame() {
        let request = image("a red fox in the snow");
        assert_eq!(request.model.id, "grok-imagine");
        assert_eq!(request.prompt, "a red fox in the snow");
        assert_eq!(request.aspect, "1:1");
        assert!(request.note().contains("the `image_gen` tool") && request.note().contains("a red fox"));
    }

    #[test]
    fn a_flag_moves_and_an_unknown_value_falls_back() {
        let request = parse(Kind::Video, "grok", "--aspect 9:16 --duration 99 --resolution 1080p a train").unwrap();
        assert_eq!((request.aspect.as_str(), request.duration), ("9:16", 10));
        assert_eq!(request.resolution.as_deref(), Some("480p"));
        assert!(request.note().contains("10 seconds") && request.note().contains("reference_to_video"));
    }

    #[test]
    fn the_other_backend_is_the_other_model_and_order_does_not_matter() {
        let request = parse(Kind::Image, "codex", "--aspect 16:9 a lighthouse").unwrap();
        assert_eq!(request.model.id, "gpt-image");
        assert!(request.note().contains("image_gen") && request.note().contains("16:9"));
    }

    #[test]
    fn a_transparent_background_is_asked_for_in_each_providers_own_words() {
        let grok = image("--transparent a cup");
        assert!(grok.note().contains("#00FF00") && grok.note().contains("flat"));
        let codex = parse(Kind::Image, "codex", "--transparent a cup").unwrap();
        assert!(codex.note().contains("clear pixels"));
        assert!(!parse(Kind::Image, "codex", "--transparent a cup").unwrap().note().contains("#00FF00"));
    }

    #[test]
    fn grok_reference_pictures_become_an_edit_and_video_frames_are_named() {
        let edit = image("--ref /tmp/one.png --ref /tmp/two.png two cups");
        assert!(edit.note().contains("`image_edit`") && edit.note().contains("/tmp/two.png"));
        let video = parse(Kind::Video, "grok", "--first /tmp/a.png --last /tmp/b.png a pan").unwrap();
        assert!(video.note().contains("first_frame: /tmp/a.png") && video.note().contains("last_frame: /tmp/b.png"));
    }

    #[test]
    fn a_known_voice_is_kept_and_a_video_without_one_names_none() {
        let request = parse(Kind::Video, "grok", "--voice ara a person says hello").unwrap();
        assert!(request.note().contains("AUDIO_1") && request.note().contains("\"ara\""));
        assert!(parse(Kind::Video, "grok", "a pan").unwrap().voice.is_none());
        assert!(parse(Kind::Video, "grok", "--voice nope a pan").is_err());
        assert!(parse(Kind::Image, "grok", "--voice ara a cup").unwrap().voice.is_none());
    }

    #[test]
    fn a_model_that_makes_no_video_and_a_missing_description_are_refused() {
        assert!(parse(Kind::Video, "codex", "a train").is_err());
        assert_eq!(parse(Kind::Image, "codex", "--model grok-imagine a cup").unwrap().model.id, "grok-imagine");
        assert!(parse(Kind::Image, "grok", "  ").is_err());
        assert!(parse(Kind::Image, "claude", "a cup").is_err());
    }

    #[test]
    fn only_the_exact_commands_expand_and_a_bad_request_answers_why() {
        assert!(expand("/image a fox", "grok").is_some());
        assert!(expand("/video a fox", "grok").is_some());
        assert_eq!(expand("hello /image a fox", "grok"), None);
        assert_eq!(expand("/imagex a fox", "grok"), None);
        let refused = expand("/video a fox", "codex").unwrap();
        assert!(refused.starts_with("[Codync:") && refused.contains("cannot make a video"), "{refused}");
    }

    #[test]
    fn the_flags_a_client_builds_parse_back() {
        let model = catalog::model("grok-imagine").unwrap();
        let command = format!("/image {} a fox", flags(Kind::Video, model, "16:9"));
        let note = expand(&command, "grok").unwrap();
        assert!(note.contains("16:9") && note.contains("a fox"));
    }

    #[test]
    fn a_checked_request_keeps_its_own_values() {
        assert!(check(&image("a fox")).is_ok());
        let mut request = parse(Kind::Video, "grok", "a fox").unwrap();
        request.duration = 7;
        assert!(check(&request).is_ok(), "a custom length in range is allowed (OpenBot 0.74.0)");
        request.duration = 0;
        assert!(check(&request).is_err(), "a length of zero is never allowed");
        request.duration = 6;
        request.resolution = None;
        assert!(check(&request).is_err());
        request.resolution = Some("720p".to_owned());
        assert!(check(&request).is_ok());
        request.batch = 4;
        assert!(check(&request).is_ok());
        request.batch = 5;
        assert!(check(&request).is_err(), "a batch past four is refused before a call");
    }

    #[test]
    fn a_chat_command_asks_for_one_result() {
        assert_eq!(image("a red fox").batch, 1);
        assert_eq!(parse(Kind::Video, "grok", "a pan").unwrap().batch, 1);
    }
}
