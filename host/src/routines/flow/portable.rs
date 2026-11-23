//! The `openbot.workflow` file format and its export/import decisions, ported from OpenBot nightly.
//!
//! One file holds one workflow: its steps and notes, its asset files, the agents and channels it
//! names **by name**, and the schedule's time zone. Connections and media accounts are deliberately
//! absent — they belong to a computer, so the person reconnects them after an import. A file with a
//! secret in a step is refused, naming the steps to change. A file from a newer editor is refused
//! with "update required" and the step kinds this host doesn't know.
//!
//! The codec and every decision here are pure. The API in `api.rs` builds the data from the host's
//! flow store, calls `encode`/`decode`, and applies the results; the host owns the file handling,
//! the clients only send and receive its bytes.

use super::format;
use super::merge::Entries;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The one file name an exported workflow uses.
pub const FILE_NAME: &str = "openbot.workflow";
/// The step kinds this host can import. Anything else came from a newer editor.
pub const KNOWN_STEPS: &[&str] = &["trigger", "agent", "check", "media", "output", "note"];
/// Setting keys that name a computer's connection or media account: never put in a file.
const LOCAL_ONLY_KEYS: &[&str] = &["connection", "connections", "account", "accounts", "mediaAccount"];
/// Setting keys that hold a secret: a file with one is refused.
const SECRET_KEYS: &[&str] =
    &["secret", "token", "apiKey", "password", "credential", "credentials", "authorization", "accessKey"];

/// The format version stamped on a file when the field is absent.
fn current_version() -> u32 {
    format::FORMAT_VERSION
}

/// One workflow in a portable file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowFile {
    #[serde(default = "current_version")]
    pub format_version: u32,
    #[serde(default)]
    pub workflow: WorkflowData,
    /// Asset name to base64-encoded bytes.
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
}

/// The portable part of a workflow: no ids, connections or media accounts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowData {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub instruction: String,
    /// The schedule's IANA zone, kept so an import doesn't move the schedule to this computer's zone.
    #[serde(default)]
    pub time_zone: String,
    #[serde(default)]
    pub steps: Vec<WorkflowStep>,
    /// Names of the agents the steps hand work to.
    #[serde(default)]
    pub agents: Vec<String>,
    /// Names of the channels the workflow posts to.
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// One step of a portable workflow. `kind` is one of [`KNOWN_STEPS`] for this version.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub settings: BTreeMap<String, Value>,
}

/// Why a file cannot be imported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileProblem {
    /// The file is from a newer editor; the named step kinds are the ones this host doesn't know.
    NewerFormat { version: u32, steps: Vec<String> },
    /// Steps that hold a secret and must be changed before the workflow can be exported.
    Secrets(Vec<String>),
    /// The bytes are not a workflow file.
    Invalid(String),
}

impl FileProblem {
    /// The message shown to the person. A newer file always says an update is required.
    pub fn message(&self) -> String {
        match self {
            Self::NewerFormat { version, steps } => format!(
                "Update required: this workflow file uses format {version} and step kinds this version does not know ({}).",
                steps.join(", ")
            ),
            Self::Secrets(steps) => format!(
                "This workflow has a secret in a step and was not exported. Change the secret in: {}.",
                steps.join(", ")
            ),
            Self::Invalid(detail) => format!("That is not an openbot.workflow file: {detail}"),
        }
    }
}

impl std::fmt::Display for FileProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

/// The step kinds this host knows, as a set.
pub fn known_steps() -> BTreeSet<String> {
    KNOWN_STEPS.iter().map(|kind| (*kind).to_owned()).collect()
}

/// The step kinds in `data` that a newer editor introduced, in file order without duplicates.
pub fn unknown_steps(data: &WorkflowData) -> Vec<String> {
    let known = known_steps();
    let mut unknown: Vec<String> = Vec::new();
    for step in &data.steps {
        if !known.contains(&step.kind) && !unknown.contains(&step.kind) {
            unknown.push(step.kind.clone());
        }
    }
    unknown
}

/// Whether a setting value holds a secret, including one nested in an object or list.
fn has_secret(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            let name = key.to_ascii_lowercase();
            if SECRET_KEYS.iter().any(|secret| name.contains(secret)) {
                !value.is_null() && !value.as_str().is_some_and(str::is_empty)
            } else {
                has_secret(value)
            }
        }),
        Value::Array(items) => items.iter().any(has_secret),
        _ => false,
    }
}

/// The names of the steps that hold a secret, in file order.
pub fn secret_steps(data: &WorkflowData) -> Vec<String> {
    data.steps
        .iter()
        .filter(|step| {
            let settings: serde_json::Map<String, Value> = step.settings.clone().into_iter().collect();
            has_secret(&Value::Object(settings))
        })
        .map(|step| step.name.clone())
        .collect()
}

/// Strips the settings that belong to a computer, not to a workflow file: connections and media
/// accounts. Everything else, including the schedule zone, is kept.
pub fn portable_settings(settings: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    settings
        .iter()
        .filter(|(key, _)| !LOCAL_ONLY_KEYS.iter().any(|local| key.eq_ignore_ascii_case(local)))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Serializes one workflow to the bytes of an `openbot.workflow` file.
pub fn encode(file: &WorkflowFile) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(file)?)
}

/// Reads one workflow file, refusing newer formats and workflows with secrets before the caller
/// touches any of the host's state.
pub fn decode(bytes: &[u8]) -> std::result::Result<WorkflowFile, FileProblem> {
    let file: WorkflowFile = serde_json::from_slice(bytes).map_err(|error| FileProblem::Invalid(error.to_string()))?;
    if file.format_version > format::FORMAT_VERSION {
        return Err(FileProblem::NewerFormat { version: file.format_version, steps: unknown_steps(&file.workflow) });
    }
    let secrets = secret_steps(&file.workflow);
    if !secrets.is_empty() {
        return Err(FileProblem::Secrets(secrets));
    }
    Ok(file)
}

/// An imported workflow always starts paused: its connections and accounts still have to be made,
/// and none of its triggers may fire until the person resumes it.
pub fn import_starts_paused() -> bool {
    true
}

/// Maps each name in the file to the agent or channel the person chose on this computer. The
/// missing choices are returned so the caller can name every one at once.
pub fn resolve_names(
    names: &[String],
    chosen: &BTreeMap<String, String>,
) -> std::result::Result<BTreeMap<String, String>, Vec<String>> {
    let missing: Vec<String> =
        names.iter().filter(|name| chosen.get(*name).is_none_or(|id| id.trim().is_empty())).cloned().collect();
    if missing.is_empty() {
        Ok(names.iter().map(|name| (name.clone(), chosen[name].trim().to_owned())).collect())
    } else {
        Err(missing)
    }
}

/// Applies a file's step settings over the ones already saved on this computer: a setting the file
/// left out survives an import, the same way it survives a canvas save.
pub fn keep_saved_settings(imported: &mut WorkflowData, saved_steps: &BTreeMap<String, Entries>) {
    for step in &mut imported.steps {
        let Some(saved) = saved_steps.get(&step.name) else { continue };
        for (key, value) in format::surviving_settings(&Entries::from(step.settings.clone()), saved) {
            step.settings.insert(key, value);
        }
    }
}

/// Checks a file's own shape: one workflow, a name, and step kinds this version knows.
pub fn validate(file: &WorkflowFile) -> Result<()> {
    if file.workflow.name.trim().is_empty() {
        bail!("a workflow file needs a name");
    }
    if !unknown_steps(&file.workflow).is_empty() {
        bail!(
            "{}",
            FileProblem::NewerFormat { version: file.format_version, steps: unknown_steps(&file.workflow) }.message()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn step(name: &str, kind: &str, settings: Value) -> WorkflowStep {
        WorkflowStep {
            name: name.into(),
            kind: kind.into(),
            settings: serde_json::from_value(settings).expect("settings"),
        }
    }

    fn file() -> WorkflowFile {
        WorkflowFile {
            format_version: format::FORMAT_VERSION,
            workflow: WorkflowData {
                name: "Digest".into(),
                instruction: "Make the digest".into(),
                time_zone: "Asia/Taipei".into(),
                steps: vec![
                    step("When", "trigger", json!({"timeZone": "Asia/Taipei"})),
                    step("Ada", "agent", json!({"instruction": "Translate", "connection": "x"})),
                    step("Post", "output", json!({"channel": "general"})),
                ],
                agents: vec!["Ada".into()],
                channels: vec!["general".into()],
                notes: "a note".into(),
            },
            assets: BTreeMap::from([("cover.png".into(), "aGVsbG8=".into())]),
        }
    }

    #[test]
    fn a_file_round_trips_with_its_timezone_and_assets() {
        let original = file();
        let bytes = encode(&original).expect("a file");
        let read = decode(&bytes).expect("a readable file");
        assert_eq!(read, original);
        assert_eq!(read.workflow.time_zone, "Asia/Taipei");
        assert!(read.assets.contains_key("cover.png"));
        validate(&read).expect("a valid file");
    }

    #[test]
    fn a_newer_file_is_refused_with_the_unknown_step_names() {
        let mut newer = file();
        newer.format_version = format::FORMAT_VERSION + 1;
        newer.workflow.steps.push(step("Next", "futureStep", json!({})));
        let bytes = encode(&newer).expect("a file");
        let problem = decode(&bytes).expect_err("a newer file is refused");
        assert_eq!(
            problem,
            FileProblem::NewerFormat { version: format::FORMAT_VERSION + 1, steps: vec!["futureStep".into()] }
        );
        assert!(problem.message().contains("Update required"));
        assert!(problem.message().contains("futureStep"));
    }

    #[test]
    fn a_file_with_a_secret_names_the_steps_to_change() {
        let mut secret = file();
        secret.workflow.steps[1].settings.insert("apiKey".into(), json!("abc"));
        let problem = decode(&encode(&secret).expect("a file")).expect_err("a secret is refused");
        assert_eq!(problem, FileProblem::Secrets(vec!["Ada".into()]));
        assert!(problem.message().contains("Ada"));
    }

    #[test]
    fn connections_and_media_accounts_do_not_go_into_a_file() {
        let settings = BTreeMap::from([
            ("instruction".to_owned(), json!("Translate")),
            ("connection".to_owned(), json!("conn-1")),
            ("mediaAccount".to_owned(), json!("acct-1")),
        ]);
        let portable = portable_settings(&settings);
        assert_eq!(portable.len(), 1);
        assert_eq!(portable["instruction"], "Translate");
    }

    #[test]
    fn a_missing_name_choice_is_reported_and_a_complete_one_is_mapped() {
        let names = vec!["Ada".to_owned(), "Post".to_owned()];
        assert_eq!(resolve_names(&names, &BTreeMap::from([("Ada".into(), "a".into())])), Err(vec!["Post".into()]));
        let chosen = BTreeMap::from([("Ada".into(), " a ".into()), ("Post".into(), "general".into())]);
        assert_eq!(resolve_names(&names, &chosen).expect("all chosen")["Ada"], "a");
    }

    #[test]
    fn an_import_starts_paused_and_keeps_settings_the_file_left_out() {
        assert!(import_starts_paused());
        let mut imported = file().workflow;
        let saved_settings: Entries =
            json!({"model": "kept"}).as_object().expect("an object").clone().into_iter().collect();
        let saved = BTreeMap::from([("Ada".into(), saved_settings)]);
        keep_saved_settings(&mut imported, &saved);
        assert_eq!(imported.steps[1].settings["model"], "kept");
    }

    #[test]
    fn a_file_without_a_name_is_not_valid() {
        let mut nameless = file();
        nameless.workflow.name.clear();
        assert!(validate(&nameless).is_err());
    }
}
