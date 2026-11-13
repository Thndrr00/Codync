//! Named ComfyUI templates: a manifest a person or a bot runs by id instead of writing a graph.
//!
//! One folder per template under `<data>/comfy/templates/<id>/template.json`. The manifests that
//! ship with Codync are embedded (`host/assets/comfy-templates`) and copied to the user's folder the
//! first time they are seen; a manifest the user edited is never overwritten (the new one is left
//! beside it as `template.json.new`). The graph is data, never code: it is validated against the
//! nodes the running server reports (`/object_info`) and only `{{placeholder}}` substitution is
//! applied, so a manifest can't run Python.

use crate::tools::comfy;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The manifests that ship with Codync, by id: their manifest is embedded in the binary.
const BASE: [(&str, &str); 5] = [
    ("text-to-image", include_str!("../../../assets/comfy-templates/text-to-image.json")),
    ("image-to-image", include_str!("../../../assets/comfy-templates/image-to-image.json")),
    ("canny-to-image", include_str!("../../../assets/comfy-templates/canny-to-image.json")),
    ("image-upscale", include_str!("../../../assets/comfy-templates/image-upscale.json")),
    ("smoke-test", include_str!("../../../assets/comfy-templates/smoke-test.json")),
];

/// The Open Source Higgsfield entries, with the author's repository and licence. They are not
/// templates: a graph and its models belong to its author, so the gallery lists them as imports.
const CURATED: &str = include_str!("../../../assets/comfy-templates/curated.json");

/// One challenge entry, as the gallery shows it before its graph is imported.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Curated {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub license: String,
    /// What it makes: `image`, `video`, `audio` or `mesh`, alone or combined.
    pub output: Vec<String>,
    pub summary: String,
    pub author: String,
}

/// The challenge catalogue; empty when the embedded list can't be read.
pub fn curated() -> Vec<Curated> {
    serde_json::from_str(CURATED).unwrap_or_default()
}

/// The folder every installed manifest lives in.
pub fn dir() -> PathBuf {
    comfy::data_dir().join("templates")
}

/// A manifest's folder, with the id checked so it can't leave the templates folder.
fn folder(root: &Path, id: &str) -> Result<PathBuf> {
    ensure!(valid_id(id), "invalid template id `{id}`");
    Ok(root.join(id))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// One named template, as `template.json` stores it.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// The graph in ComfyUI's API format, with `{{input}}` placeholders in its string values.
    pub workflow: Value,
    #[serde(default)]
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub models: Vec<Model>,
    pub output: Output,
    #[serde(default)]
    pub estimate: Estimate,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The workflow author's licence, as text ("Apache-2.0", "CC BY-NC 4.0", …).
    #[serde(default)]
    pub license: Option<String>,
}

/// One value the form (and the bot) fills in.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Input {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: InputKind,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    /// For `select`: the allowed values, in the order the form shows them.
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Text,
    Number,
    Integer,
    Select,
    /// A picture the host uploads to ComfyUI's input folder; the value is its name there.
    Image,
    Seed,
    Boolean,
}

/// A model the template needs, with where to get it. The URL is the only place a download comes
/// from: the model manager refuses anything a manifest doesn't declare.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Model {
    pub file: String,
    pub folder: String,
    pub size: u64,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
}

impl Model {
    pub fn path(&self) -> Result<PathBuf> {
        comfy::model_path(&self.folder, &self.file)
    }
}

/// What the template produces. The job copies each output with its own type, so a video is not a
/// strangely named PNG.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputKind {
    Image,
    Video,
    Audio,
    Mesh,
}

impl OutputKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Mesh => "mesh",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Output {
    #[serde(rename = "type")]
    pub kind: OutputKind,
    /// Overrides the extension's own MIME when the workflow names the format itself.
    #[serde(default)]
    pub mime: Option<String>,
}

/// How long a run takes and how much graphics memory it wants, overall and per GPU family.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Estimate {
    #[serde(default)]
    pub seconds: Option<u64>,
    #[serde(default)]
    pub vram_gb: Option<f64>,
    /// Free disk the output needs; 1 GB when absent.
    #[serde(default)]
    pub output_gb: Option<f64>,
    /// Keyed by a short GPU family ("rtx-4090", "rtx-3060"), or `default`.
    #[serde(default, rename = "byGpu")]
    pub by_gpu: BTreeMap<String, EstimateFor>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EstimateFor {
    #[serde(default)]
    pub seconds: Option<u64>,
    #[serde(default)]
    pub vram_gb: Option<f64>,
}

impl Estimate {
    /// The measurement for `gpu`, else the `default` row, else the top-level numbers.
    pub fn seconds_for(&self, gpu: Option<&str>) -> Option<u64> {
        gpu.and_then(|g| self.by_gpu.get(&family(g)))
            .and_then(|e| e.seconds)
            .or_else(|| self.by_gpu.get("default").and_then(|e| e.seconds).or(self.seconds))
    }

    pub fn vram_for(&self, gpu: Option<&str>) -> Option<f64> {
        gpu.and_then(|g| self.by_gpu.get(&family(g)))
            .and_then(|e| e.vram_gb)
            .or_else(|| self.by_gpu.get("default").and_then(|e| e.vram_gb).or(self.vram_gb))
    }
}

/// "NVIDIA GeForce RTX 4090" → "rtx-4090": a manifest names a family, not the whole model string.
fn family(gpu: &str) -> String {
    let lower = gpu.to_lowercase();
    if lower.contains("a100") {
        return "a100".to_owned();
    }
    if lower.contains("h100") {
        return "h100".to_owned();
    }
    let known = ["4090", "4080", "4070", "4060", "3090", "3080", "3070", "3060", "2080", "1080"];
    match known.iter().find(|model| lower.contains(*model)) {
        Some(model) => format!("rtx-{model}"),
        None => lower.split_whitespace().take(2).collect::<Vec<_>>().join("-"),
    }
}

impl Template {
    /// Checks the manifest itself: names, ranges, placeholders and model declarations.
    pub fn validate(&self) -> Result<()> {
        ensure!(valid_id(&self.id), "invalid template id `{}`", self.id);
        ensure!(!self.name.trim().is_empty(), "template `{}` has no name", self.id);
        ensure!(semver::Version::parse(&self.version).is_ok(), "template `{}` has an invalid version", self.id);
        ensure!(!self.description.trim().is_empty(), "template `{}` has no description", self.id);
        let nodes = self
            .workflow
            .as_object()
            .filter(|nodes| !nodes.is_empty())
            .with_context(|| format!("template `{}` has no workflow graph", self.id))?;
        for (node, body) in nodes {
            ensure!(
                body["class_type"].as_str().is_some_and(|c| !c.trim().is_empty()),
                "template `{}`: node {node} has no class_type",
                self.id
            );
        }

        let mut keys = BTreeSet::new();
        for input in &self.inputs {
            ensure!(
                input.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "template `{}`: invalid input key `{}`",
                self.id,
                input.key
            );
            ensure!(keys.insert(input.key.clone()), "template `{}`: duplicate input `{}`", self.id, input.key);
            if input.kind == InputKind::Select {
                ensure!(!input.options.is_empty(), "template `{}`: `{}` has no options", self.id, input.key);
                if let Some(default) = input.default.as_ref().and_then(Value::as_str) {
                    ensure!(
                        input.options.iter().any(|o| o == default),
                        "template `{}`: `{}` defaults to `{default}`, which is not an option",
                        self.id,
                        input.key
                    );
                }
            }
            if let (Some(min), Some(max)) = (input.min, input.max) {
                ensure!(min <= max, "template `{}`: `{}` has min above max", self.id, input.key);
            }
        }

        // Every placeholder names an input, and every input is used somewhere: a form field that
        // does nothing is worse than none.
        let mut used = BTreeSet::new();
        placeholders_in(&self.workflow, &mut used);
        let unknown: Vec<&String> = used.difference(&keys).collect();
        ensure!(
            unknown.is_empty(),
            "template `{}`: the workflow uses inputs the manifest does not declare: {}",
            self.id,
            unknown.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
        );
        let unused: Vec<&String> = keys.difference(&used).collect();
        ensure!(
            unused.is_empty(),
            "template `{}`: declared inputs the workflow never uses: {}",
            self.id,
            unused.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
        );

        for model in &self.models {
            ensure!(
                comfy::model_folder(&model.folder).is_some(),
                "template `{}`: unknown model folder `{}`",
                self.id,
                model.folder
            );
            model.path()?;
            ensure!(model.size > 0, "template `{}`: `{}` has no size", self.id, model.file);
            if let Some(hash) = &model.sha256 {
                ensure!(
                    hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()),
                    "template `{}`: `{}` has an invalid SHA-256",
                    self.id,
                    model.file
                );
            }
            if let Some(url) = &model.url {
                ensure!(
                    url.starts_with("https://") || url.starts_with("http://"),
                    "template `{}`: `{}` has a URL that is not HTTP",
                    self.id,
                    model.file
                );
            }
        }
        Ok(())
    }

    /// Every node class the template uses, for the `/object_info` check.
    pub fn classes(&self) -> Vec<String> {
        self.workflow
            .as_object()
            .into_iter()
            .flat_map(|nodes| nodes.values())
            .filter_map(|node| node["class_type"].as_str().map(str::to_owned))
            .collect()
    }

    /// The values for a run: what the caller gave, else the manifest's default, checked against the
    /// input's type and range. Unknown names are refused (a typo must not run with defaults).
    pub fn values(&self, given: &Map<String, Value>) -> Result<BTreeMap<String, Value>> {
        let unknown: Vec<&str> =
            given.keys().filter(|key| !self.inputs.iter().any(|i| &i.key == *key)).map(String::as_str).collect();
        ensure!(unknown.is_empty(), "unknown inputs for `{}`: {}", self.id, unknown.join(", "));
        let mut values = BTreeMap::new();
        for input in &self.inputs {
            let value = match given.get(&input.key).or(input.default.as_ref()) {
                Some(value) => input.coerce(value)?,
                None if input.kind == InputKind::Seed => json!(random_seed()),
                None if input.required => bail!("`{}` is required for `{}`", input.key, self.id),
                None => input.empty(),
            };
            values.insert(input.key.clone(), value);
        }
        Ok(values)
    }

    /// The graph with every placeholder replaced. `values` comes from [`Template::values`], with any
    /// `image` input already uploaded and replaced by the name ComfyUI knows.
    pub fn graph(&self, values: &BTreeMap<String, Value>) -> Result<Value> {
        render(&self.workflow, values)
    }

    /// What a gallery or a bot sees: the manifest without the graph.
    pub fn summary(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "version": self.version,
            "description": self.description,
            "output": self.output.kind.name(),
            "tags": self.tags,
            "estimateSeconds": self.estimate.seconds_for(None),
            "vramGb": self.estimate.vram_for(None),
            "inputs": self.inputs.len(),
            "models": self.models.len(),
        })
    }

    /// The full manifest a form is built from, with the current GPU's estimate filled in.
    pub fn describe(&self, gpu: Option<&str>) -> Value {
        let mut value = serde_json::to_value(self).unwrap_or(Value::Null);
        value["output"]["type"] = json!(self.output.kind.name());
        value["estimate"]["secondsForThisGpu"] = json!(self.estimate.seconds_for(gpu));
        value["estimate"]["vramForThisGpu"] = json!(self.estimate.vram_for(gpu));
        value["nodeCount"] = json!(self.workflow.as_object().map_or(0, Map::len));
        value["classes"] = json!(self.classes());
        value
    }
}

impl Input {
    fn coerce(&self, value: &Value) -> Result<Value> {
        match self.kind {
            InputKind::Text | InputKind::Image => {
                let text = value.as_str().with_context(|| format!("`{}` must be text", self.key))?.trim().to_owned();
                ensure!(!text.is_empty(), "`{}` can't be empty", self.key);
                if self.kind == InputKind::Image {
                    ensure!(
                        !text.contains(['/', '\\']) || Path::new(&text).is_file(),
                        "`{}` must be an image in ComfyUI's input folder",
                        self.key
                    );
                }
                Ok(json!(text))
            }
            InputKind::Number => {
                let number = as_f64(value).with_context(|| format!("`{}` must be a number", self.key))?;
                self.range(number)?;
                Ok(json!(number))
            }
            InputKind::Integer | InputKind::Seed => {
                let number = as_f64(value).with_context(|| format!("`{}` must be a whole number", self.key))?;
                ensure!(number.fract() == 0.0, "`{}` must be a whole number", self.key);
                self.range(number)?;
                Ok(json!(number as i64))
            }
            InputKind::Select => {
                let choice = value.as_str().with_context(|| format!("`{}` must be one of its options", self.key))?;
                ensure!(self.options.iter().any(|o| o == choice), "`{}` is not an option of `{}`", choice, self.key);
                Ok(json!(choice))
            }
            InputKind::Boolean => {
                let flag = value
                    .as_bool()
                    .or_else(|| {
                        value.as_str().and_then(|s| match s {
                            "true" | "1" | "yes" => Some(true),
                            "false" | "0" | "no" => Some(false),
                            _ => None,
                        })
                    })
                    .with_context(|| format!("`{}` must be true or false", self.key))?;
                Ok(json!(flag))
            }
        }
    }

    fn range(&self, number: f64) -> Result<()> {
        if let Some(min) = self.min {
            ensure!(number >= min, "`{}` is below {min}", self.key);
        }
        if let Some(max) = self.max {
            ensure!(number <= max, "`{}` is above {max}", self.key);
        }
        Ok(())
    }

    /// What an omitted, optional value becomes.
    fn empty(&self) -> Value {
        match self.kind {
            InputKind::Text | InputKind::Select | InputKind::Image => json!(""),
            InputKind::Number => json!(self.default.as_ref().and_then(Value::as_f64).unwrap_or(0.0)),
            InputKind::Integer | InputKind::Seed => json!(0),
            InputKind::Boolean => json!(false),
        }
    }
}

fn as_f64(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.trim().parse().ok())
}

/// The placeholders anywhere in `value`, added to `keys`.
fn placeholders_in(value: &Value, keys: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => keys.extend(placeholders_in_text(text)),
        Value::Array(items) => items.iter().for_each(|item| placeholders_in(item, keys)),
        Value::Object(map) => map.values().for_each(|item| placeholders_in(item, keys)),
        _ => {}
    }
}

fn placeholders_in_text(text: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        keys.push(after[..end].trim().to_owned());
        rest = &after[end + 2..];
    }
    keys.retain(|key| !key.is_empty());
    keys
}

/// One placeholder that is the whole string: its value keeps its JSON type (a number stays a number).
fn single_placeholder(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("{{")?.strip_suffix("}}")?;
    (!inner.contains("{{") && !inner.contains("}}")).then(|| inner.trim())
}

fn render(value: &Value, values: &BTreeMap<String, Value>) -> Result<Value> {
    match value {
        Value::String(text) => render_text(text, values),
        Value::Array(items) => Ok(Value::Array(items.iter().map(|item| render(item, values)).collect::<Result<_>>()?)),
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, item) in map {
                out.insert(key.clone(), render(item, values)?);
            }
            Ok(Value::Object(out))
        }
        other => Ok(other.clone()),
    }
}

fn render_text(text: &str, values: &BTreeMap<String, Value>) -> Result<Value> {
    if let Some(key) = single_placeholder(text) {
        return values
            .get(key)
            .cloned()
            .with_context(|| format!("the workflow uses {{{{{key}}}}} but the template has no such input"));
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return Ok(Value::String(out));
        };
        let key = after[..end].trim();
        let value = values
            .get(key)
            .with_context(|| format!("the workflow uses {{{{{key}}}}} but the template has no such input"))?;
        out.push_str(&scalar(value));
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(Value::String(out))
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

fn random_seed() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos() as u64);
    (((std::process::id() as u64) << 32) | nanos) as i64 & 0x7fff_ffff
}

/// Copies the embedded manifests into the user's folder. A manifest the user edited stays as it is,
/// and the new base version is written beside it as `template.json.new` for review.
pub fn install_base() -> Result<()> {
    install_into(&dir())
}

fn install_into(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    for (id, source) in BASE {
        let folder = folder(root, id)?;
        std::fs::create_dir_all(&folder)?;
        let manifest = folder.join("template.json");
        let marker = folder.join(".installed");
        let current = std::fs::read_to_string(&manifest).ok();
        match current {
            None => {
                std::fs::write(&manifest, source)?;
                std::fs::write(&marker, sha256(source.as_bytes()))?;
            }
            Some(text) if text == source => {
                let _ = std::fs::write(&marker, sha256(source.as_bytes()));
            }
            Some(text) => {
                let untouched = std::fs::read_to_string(&marker).is_ok_and(|old| old == sha256(text.as_bytes()));
                if untouched {
                    std::fs::write(&manifest, source)?;
                    std::fs::write(&marker, sha256(source.as_bytes()))?;
                } else if text != source {
                    std::fs::write(folder.join("template.json.new"), source)?;
                }
            }
        }
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hash = sha2::Sha256::new();
    hash.update(bytes);
    crate::chat::files::digest_hex(&hash.finalize())
}

/// Every installed template, by name; a manifest that doesn't validate is left out and its error is
/// in [`problems`], so one broken file doesn't hide the rest.
pub fn list() -> Vec<Template> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir()) else { return found };
    for entry in entries.filter_map(Result::ok) {
        let Ok(text) = std::fs::read_to_string(entry.path().join("template.json")) else { continue };
        let Ok(template) = serde_json::from_str::<Template>(&text) else { continue };
        if template.validate().is_ok() {
            found.push(template);
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

/// One installed template by id, validated.
pub fn load(id: &str) -> Result<Template> {
    let path = folder(&dir(), id)?.join("template.json");
    let text = std::fs::read_to_string(&path).with_context(|| format!("template `{id}` is not installed"))?;
    let template: Template = serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    ensure!(template.id == id, "template `{id}` declares the id `{}`", template.id);
    template.validate().with_context(|| format!("template `{id}` is invalid"))?;
    Ok(template)
}

/// The templates whose manifest declares `folder/file`, as (template id, model).
pub fn declaring(folder: &str, file: &str) -> Vec<(String, Model)> {
    list()
        .into_iter()
        .filter_map(|template| {
            let model = template.models.into_iter().find(|m| m.folder == folder && m.file == file)?;
            Some((template.id, model))
        })
        .collect()
}

/// Why the manifests that don't load are invalid, for the card's notice.
pub fn problems() -> Vec<Value> {
    let mut problems = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir()) else { return problems };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path().join("template.json");
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        match serde_json::from_str::<Template>(&text)
            .map_err(anyhow::Error::from)
            .and_then(|t| t.validate().map(|()| t))
        {
            Ok(_) => {}
            Err(error) => problems.push(json!({
                "folder": entry.file_name().to_string_lossy(),
                "error": format!("{error:#}"),
            })),
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template() -> Template {
        serde_json::from_value(json!({
            "id": "test",
            "name": "Test",
            "version": "1.0.0",
            "description": "A test",
            "workflow": {
                "1": {"class_type": "EmptyImage", "inputs": {"width": "{{width}}", "height": 64, "batch_size": 1, "color": 0}},
                "2": {"class_type": "SaveImage", "inputs": {"images": ["1", 0], "filename_prefix": "{{prefix}}"}}
            },
            "inputs": [
                {"key": "width", "label": "Width", "type": "integer", "default": 64, "min": 16, "max": 1024},
                {"key": "prefix", "label": "Prefix", "type": "text", "default": "x"}
            ],
            "output": {"type": "image"}
        }))
        .expect("a valid manifest")
    }

    #[test]
    fn a_good_manifest_validates() {
        template().validate().expect("valid");
    }

    #[test]
    fn a_placeholder_without_an_input_is_refused() {
        let mut t = template();
        t.workflow["2"]["inputs"]["filename_prefix"] = json!("{{missing}}");
        let error = t.validate().expect_err("unknown placeholder").to_string();
        assert!(error.contains("does not declare: missing"), "{error}");
    }

    #[test]
    fn an_input_the_graph_never_uses_is_refused() {
        let mut t = template();
        t.inputs.push(
            serde_json::from_value(json!({"key": "unused", "label": "Unused", "type": "text", "default": "x"}))
                .expect("an input"),
        );
        let error = t.validate().expect_err("unused input").to_string();
        assert!(error.contains("never uses: unused"), "{error}");
    }

    #[test]
    fn values_keep_their_type_and_range() {
        let t = template();
        let mut given = Map::new();
        given.insert("width".into(), json!("128"));
        given.insert("prefix".into(), json!("cat"));
        let values = t.values(&given).expect("values");
        assert_eq!(values["width"], json!(128));
        assert_eq!(t.graph(&values).expect("graph")["1"]["inputs"]["width"], json!(128));
        assert_eq!(t.graph(&values).expect("graph")["2"]["inputs"]["filename_prefix"], json!("cat"));

        given.insert("width".into(), json!(8));
        assert!(t.values(&given).expect_err("below the minimum").to_string().contains("below 16"));
        given.insert("width".into(), json!(128));
        given.insert("typo".into(), json!(1));
        assert!(t.values(&given).expect_err("unknown input").to_string().contains("unknown inputs"));
    }

    #[test]
    fn a_seed_without_a_value_is_random() {
        let mut t = template();
        t.inputs.push(serde_json::from_value(json!({"key": "seed", "label": "Seed", "type": "seed"})).expect("a seed"));
        t.workflow["2"]["inputs"]["filename_prefix"] = json!("{{seed}}");
        let values = t.values(&Map::new()).expect("values");
        assert!(values["seed"].as_i64().is_some_and(|seed| seed >= 0));
    }

    #[test]
    fn a_whole_placeholder_keeps_numbers_and_text_in_the_middle_joins() {
        let mut values = BTreeMap::new();
        values.insert("n".to_owned(), json!(5));
        values.insert("who".to_owned(), json!("cat"));
        assert_eq!(render(&json!("{{n}}"), &values).expect("number"), json!(5));
        assert_eq!(render(&json!("a {{who}} and {{n}}"), &values).expect("text"), json!("a cat and 5"));
        assert!(render(&json!("{{missing}}"), &values).is_err());
    }

    #[test]
    fn the_challenge_catalogue_names_its_repositories_and_licences() {
        let all = curated();
        assert_eq!(all.len(), 15);
        for entry in all {
            assert!(entry.repo.starts_with("https://github.com/"), "{}", entry.id);
            assert!(!entry.license.is_empty() && !entry.summary.is_empty(), "{}", entry.id);
            assert!(
                entry.output.iter().all(|kind| ["image", "video", "audio", "mesh"].contains(&kind.as_str())),
                "{}",
                entry.id
            );
        }
    }

    #[test]
    fn the_base_manifests_are_valid_and_copy_without_overwriting_a_user_edit() {
        let root = std::env::temp_dir().join(format!("codync-templates-{}", uuid::Uuid::new_v4()));
        install_into(&root).expect("installed");
        for (id, _) in BASE {
            let text = std::fs::read_to_string(root.join(id).join("template.json")).expect("installed");
            let template: Template = serde_json::from_str(&text).expect("valid JSON");
            template.validate().unwrap_or_else(|error| panic!("{id}: {error:#}"));
        }
        let path = root.join("text-to-image/template.json");
        let edited = std::fs::read_to_string(&path).expect("installed").replace("\"name\": \"", "\"name\": \"My ");
        std::fs::write(&path, &edited).expect("edited");
        install_into(&root).expect("installed again");
        assert!(std::fs::read_to_string(&path).expect("kept").contains("My "));
        assert!(root.join("text-to-image/template.json.new").is_file());
        let _ = std::fs::remove_dir_all(root);
    }
}
