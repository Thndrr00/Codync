//! The media models of each agent backend, from `OpenBot` 0.70.0 `WORKFLOW_MEDIA_MODELS`
//! (`packages/contracts/src/ipc-workflows.ts`). The agent provider that owns the model makes the
//! file with its own tool, so the model needs no key and no separate service.

use super::Kind;
use serde_json::{Value, json};

/// One model a bot may ask for.
#[derive(Debug)]
pub struct Model {
    /// The id the clients name; it never changes between releases.
    pub id: &'static str,
    /// The agent backend that runs the tool.
    pub backend: &'static str,
    pub name: &'static str,
    /// The company that makes the model.
    pub maker: &'static str,
    /// The subscription that pays for the file.
    pub subscription: &'static str,
    /// What the model makes.
    pub kinds: &'static [Kind],
    /// The aspect ratios it makes; all of `ASPECTS` when empty.
    pub aspects: &'static [&'static str],
    /// The lengths of a video in seconds; `DURATIONS` when empty.
    pub durations: &'static [u32],
    /// The resolutions of a video; `RESOLUTIONS` when empty.
    pub resolutions: &'static [&'static str],
    /// Whether an image can have a clear background.
    pub transparent: bool,
    /// Whether a video can speak in a preset voice.
    pub voices: bool,
}

const MODELS: [Model; 2] = [
    Model {
        id: "gpt-image",
        backend: "codex",
        name: "GPT Image",
        maker: "OpenAI",
        subscription: "ChatGPT",
        kinds: &[Kind::Image],
        aspects: &[],
        durations: &[],
        resolutions: &[],
        transparent: true,
        voices: false,
    },
    Model {
        id: "grok-imagine",
        backend: "grok",
        name: "Grok Imagine",
        maker: "xAI",
        subscription: "Grok",
        kinds: &[Kind::Image, Kind::Video],
        aspects: &[],
        durations: &[],
        resolutions: &[],
        transparent: true,
        voices: true,
    },
];

pub fn all() -> &'static [Model] {
    &MODELS
}

pub fn model(id: &str) -> Option<&'static Model> {
    MODELS.iter().find(|model| model.id == id)
}

/// The models `backend` can run, the ones of a kind only when `kind` is given.
pub fn models(backend: &str, kind: Option<Kind>) -> impl Iterator<Item = &'static Model> {
    MODELS
        .iter()
        .filter(move |model| model.backend == backend)
        .filter(move |model| kind.is_none_or(|kind| model.kinds.contains(&kind)))
}

/// The model a backend uses when the request names none: its first one that makes the kind.
pub fn default(backend: &str, kind: Kind) -> Option<&'static Model> {
    models(backend, Some(kind)).next()
}

/// The model a new flow step starts on: one of `preferred`'s own backend, else the first of the
/// kind. The person changes it in the step's settings.
pub fn first(preferred: &str, kind: Kind) -> Option<&'static Model> {
    default(preferred, kind).or_else(|| MODELS.iter().find(|model| model.kinds.contains(&kind)))
}

/// The aspect ratios a model makes, `ASPECTS` when it makes every one.
pub fn aspects(model: &Model) -> &'static [&'static str] {
    if model.aspects.is_empty() { &super::ASPECTS } else { model.aspects }
}

/// The lengths a model makes, `DURATIONS` when it names none.
pub fn durations(model: &Model) -> &'static [u32] {
    if model.durations.is_empty() { &super::DURATIONS } else { model.durations }
}

/// The resolutions a model makes, `RESOLUTIONS` when it names none.
pub fn resolutions(model: &Model) -> &'static [&'static str] {
    if model.resolutions.is_empty() { &super::RESOLUTIONS } else { model.resolutions }
}

/// A model as a client reads it.
pub fn json(model: &Model) -> Value {
    json!({
        "id": model.id,
        "name": model.name,
        "maker": model.maker,
        "subscription": model.subscription,
        "kinds": model.kinds.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
        "aspects": aspects(model),
        "transparent": model.transparent,
        "voices": model.voices,
        "capabilities": super::capabilities(model),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backend_offers_only_its_own_models_and_kind() {
        let codex: Vec<&str> = models("codex", None).map(|model| model.id).collect();
        assert_eq!(codex, ["gpt-image"]);
        assert_eq!(models("gpt-image", None).count(), 0, "the id is not the backend");
        assert_eq!(models("grok", Some(Kind::Video)).count(), 1);
        assert_eq!(models("gpt-image", Some(Kind::Video)).count(), 0);
        assert_eq!(default("grok", Kind::Video).unwrap().id, "grok-imagine");
        assert!(default("claude", Kind::Image).is_none());
    }

    #[test]
    fn a_model_fills_in_the_choices_it_leaves_out() {
        let node = model("grok-imagine").unwrap();
        assert_eq!(aspects(node), super::super::ASPECTS);
        assert_eq!(durations(node), super::super::DURATIONS);
        let listed = json(node);
        assert_eq!(listed["aspects"].as_array().unwrap().len(), 4);
        assert_eq!(listed["kinds"].as_array().unwrap().len(), 2);
        assert_eq!(listed["capabilities"]["voices"].as_array().unwrap().len(), 4);
        assert!(json(model("gpt-image").unwrap())["capabilities"]["voices"].is_null());
    }
}
