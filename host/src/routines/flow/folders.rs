//! Workflow folders of this computer, as used by the canvas's "All workflows" menu.
//!
//! A folder is navigation convenience only — it never changes which links a routine runs. The host
//! owns the folders (they are not a device preference): one stamped kv blob holds every folder and
//! where each workflow (routine) sits, so every client reads the same tree and catches up through
//! the global rev. The model, its limits and its refusal reasons are ported from OpenBot nightly's
//! workflow-folder contract; this module owns the data and the pure validation, while the API checks
//! workflow ids against the host's routine registry before applying an action.

use crate::LockExt;
use crate::store::Store;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Mutex;

/// The one kv slot holding this computer's workflow folders.
pub const SLOT: &str = "workflowFolders";
/// How many folders one computer may hold.
pub const MAX_FOLDERS: usize = 200;
/// How long a folder name may be, in characters.
pub const MAX_NAME_CHARS: usize = 80;
/// How many workflow placements one computer may hold.
pub const MAX_PLACEMENTS: usize = 10_000;
/// How many workflows one action may create or place at once.
pub const MAX_WORKFLOWS_PER_ACTION: usize = 1_000;
/// How long an id may be, in characters.
pub const MAX_ID_CHARS: usize = 200;
/// A guard against a corrupt or hand-edited blob growing without bound. The limits above bound a
/// well-formed snapshot well below this.
pub const MAX_BLOB_BYTES: usize = 4 * 1024 * 1024;

/// Serializes the load → apply → save of one action: two clients filing workflows at the same time
/// must not lose one another's change.
static LOCK: Mutex<()> = Mutex::new(());

/// One named folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowFolder {
    pub id: String,
    pub name: String,
}

/// The folders of this computer and where each workflow sits. `placement` maps a workflow (routine)
/// id to a folder id; a workflow with no entry is in "All workflows". `revision` is the global rev
/// the host stamped when it saved this snapshot, and is set by `save`, never by a client.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowFoldersSnapshot {
    pub revision: i64,
    pub folders: Vec<WorkflowFolder>,
    pub placement: BTreeMap<String, String>,
}

/// One change a client asks the host to make. `workflowIds` may carry up to
/// `MAX_WORKFLOWS_PER_ACTION` workflows, so filing a whole folder is one action and one rev stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkflowFolderAction {
    Create {
        id: String,
        name: String,
        workflow_ids: Vec<String>,
    },
    Rename {
        id: String,
        name: String,
    },
    Delete {
        id: String,
    },
    /// `folder_id: None` takes the workflows out of their folder (files them in "All workflows").
    Place {
        workflow_ids: Vec<String>,
        folder_id: Option<String>,
    },
}

/// Why an action or a stored snapshot was refused. Kept as fixed values so a client can show the
/// same reason in every language; the text never carries a folder or workflow name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderRefusal {
    MalformedSnapshot,
    MalformedAction,
    UnknownAction,
    InvalidActionKeys,
    InvalidId,
    InvalidName,
    EmptyName,
    NameTooLong,
    DuplicateFolderId,
    DuplicateName,
    TooManyFolders,
    TooManyPlacements,
    TooManyWorkflows,
    UnknownFolder,
    UnknownWorkflow,
    InvalidRevision,
}

impl FolderRefusal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MalformedSnapshot => "the workflow folders are malformed",
            Self::MalformedAction => "the folder action is malformed",
            Self::UnknownAction => "unknown folder action",
            Self::InvalidActionKeys => "the folder action has unexpected or missing keys",
            Self::InvalidId => "a folder or workflow id is invalid",
            Self::InvalidName => "a folder name has invalid characters",
            Self::EmptyName => "a folder name can't be empty",
            Self::NameTooLong => "a folder name is too long",
            Self::DuplicateFolderId => "two folders have the same id",
            Self::DuplicateName => "two folders have the same name",
            Self::TooManyFolders => "a computer holds at most 200 folders",
            Self::TooManyPlacements => "there are too many workflow placements",
            Self::TooManyWorkflows => "one action can place at most 1000 workflows",
            Self::UnknownFolder => "that folder does not exist",
            Self::UnknownWorkflow => "that workflow does not exist",
            Self::InvalidRevision => "the workflow folders have an invalid revision",
        }
    }
}

impl fmt::Display for FolderRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for FolderRefusal {}

/// A name key: names are compared without surrounding whitespace and without case, so "Photos" and
/// "photos " are the same folder to a person.
fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().count() <= MAX_ID_CHARS && !id.chars().any(char::is_control)
}

fn clean_name(name: &str) -> Result<String, FolderRefusal> {
    let name = name.trim();
    if name.is_empty() {
        return Err(FolderRefusal::EmptyName);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(FolderRefusal::NameTooLong);
    }
    if name.chars().any(char::is_control) {
        return Err(FolderRefusal::InvalidName);
    }
    Ok(name.to_owned())
}

/// Checks a whole snapshot: exact fields have already been checked by `serde`; this checks the
/// limits, the ids and names, and that every placement points at a folder that exists. Liveness of
/// the workflows is checked by the API against the host's routine registry.
fn validate(snapshot: &WorkflowFoldersSnapshot) -> Result<(), FolderRefusal> {
    if snapshot.revision < 0 {
        return Err(FolderRefusal::InvalidRevision);
    }
    if snapshot.folders.len() > MAX_FOLDERS {
        return Err(FolderRefusal::TooManyFolders);
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for folder in &snapshot.folders {
        if !valid_id(&folder.id) {
            return Err(FolderRefusal::InvalidId);
        }
        let name = clean_name(&folder.name)?;
        if !ids.insert(folder.id.as_str()) {
            return Err(FolderRefusal::DuplicateFolderId);
        }
        if !names.insert(name_key(&name)) {
            return Err(FolderRefusal::DuplicateName);
        }
    }
    if snapshot.placement.len() > MAX_PLACEMENTS {
        return Err(FolderRefusal::TooManyPlacements);
    }
    for (workflow, folder) in &snapshot.placement {
        if !valid_id(workflow) || !valid_id(folder) {
            return Err(FolderRefusal::InvalidId);
        }
        if !ids.contains(folder.as_str()) {
            return Err(FolderRefusal::UnknownFolder);
        }
    }
    Ok(())
}

/// Decodes and fully checks a snapshot read from kv. An old or malformed blob is refused rather than
/// half applied; the caller then uses an empty snapshot.
pub fn decode_snapshot(raw: &str) -> Result<WorkflowFoldersSnapshot, FolderRefusal> {
    if raw.len() > MAX_BLOB_BYTES {
        return Err(FolderRefusal::MalformedSnapshot);
    }
    let snapshot: WorkflowFoldersSnapshot = serde_json::from_str(raw).map_err(|_| FolderRefusal::MalformedSnapshot)?;
    validate(&snapshot)?;
    Ok(snapshot)
}

/// Requires an action object to carry exactly these keys. `optional` keys may be left out; neither
/// an unknown key nor a missing required one is accepted.
fn exact_keys(object: &Map<String, Value>, required: &[&str], optional: &[&str]) -> Result<(), FolderRefusal> {
    if object.len() < required.len()
        || object.len() > required.len() + optional.len()
        || required.iter().any(|key| !object.contains_key(*key))
        || object.keys().any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(FolderRefusal::InvalidActionKeys);
    }
    Ok(())
}

fn id_value(object: &Map<String, Value>, key: &str) -> Result<String, FolderRefusal> {
    let id = object.get(key).and_then(Value::as_str).ok_or(FolderRefusal::MalformedAction)?;
    if !valid_id(id) {
        return Err(FolderRefusal::InvalidId);
    }
    Ok(id.to_owned())
}

fn name_value(object: &Map<String, Value>) -> Result<String, FolderRefusal> {
    let name = object.get("name").and_then(Value::as_str).ok_or(FolderRefusal::MalformedAction)?;
    clean_name(name)
}

fn workflow_ids(object: &Map<String, Value>) -> Result<Vec<String>, FolderRefusal> {
    let Some(value) = object.get("workflowIds") else { return Ok(Vec::new()) };
    let ids = value.as_array().ok_or(FolderRefusal::MalformedAction)?;
    if ids.len() > MAX_WORKFLOWS_PER_ACTION {
        return Err(FolderRefusal::TooManyWorkflows);
    }
    ids.iter()
        .map(|id| {
            let id = id.as_str().ok_or(FolderRefusal::MalformedAction)?;
            if !valid_id(id) {
                return Err(FolderRefusal::InvalidId);
            }
            Ok(id.to_owned())
        })
        .collect()
}

/// Parses one action from the API body, with strict keys before any value is read. `action` is the
/// discriminator: `create` (`id`, `name`, optional `workflowIds`), `rename` (`folderId`, `name`),
/// `delete` (`folderId`), or `place` (`workflowIds`, `folderId`, where `null` means unfiled).
pub fn parse_action(value: &Value) -> Result<WorkflowFolderAction, FolderRefusal> {
    let object = value.as_object().ok_or(FolderRefusal::MalformedAction)?;
    let kind = object.get("action").and_then(Value::as_str).ok_or(FolderRefusal::MalformedAction)?;
    match kind {
        "create" => {
            exact_keys(object, &["action", "id", "name"], &["workflowIds"])?;
            let (id, name, workflow_ids) = (id_value(object, "id")?, name_value(object)?, workflow_ids(object)?);
            Ok(WorkflowFolderAction::Create { id, name, workflow_ids })
        }
        "rename" => {
            exact_keys(object, &["action", "folderId", "name"], &[])?;
            let (id, name) = (id_value(object, "folderId")?, name_value(object)?);
            Ok(WorkflowFolderAction::Rename { id, name })
        }
        "delete" => {
            exact_keys(object, &["action", "folderId"], &[])?;
            Ok(WorkflowFolderAction::Delete { id: id_value(object, "folderId")? })
        }
        "place" => {
            exact_keys(object, &["action", "workflowIds", "folderId"], &[])?;
            let workflow_ids = workflow_ids(object)?;
            let folder_id = match object.get("folderId") {
                Some(Value::String(id)) if valid_id(id) => Some(id.to_owned()),
                Some(Value::String(_)) => return Err(FolderRefusal::InvalidId),
                Some(Value::Null) => None,
                _ => return Err(FolderRefusal::MalformedAction),
            };
            Ok(WorkflowFolderAction::Place { workflow_ids, folder_id })
        }
        _ => Err(FolderRefusal::UnknownAction),
    }
}

/// Files `workflows` into `folder`, refusing anything unknown or over the cap. `None` unfiled them.
fn place(
    placement: &mut BTreeMap<String, String>,
    workflows: &[String],
    folder: Option<&str>,
    folders: &[WorkflowFolder],
    known: &BTreeSet<String>,
) -> Result<(), FolderRefusal> {
    for workflow in workflows {
        if !known.contains(workflow) {
            return Err(FolderRefusal::UnknownWorkflow);
        }
        match folder {
            Some(id) if folders.iter().any(|existing| existing.id == id) => {
                placement.insert(workflow.clone(), id.to_owned());
            }
            Some(_) => return Err(FolderRefusal::UnknownFolder),
            None => {
                placement.remove(workflow);
            }
        }
    }
    if placement.len() > MAX_PLACEMENTS {
        return Err(FolderRefusal::TooManyPlacements);
    }
    Ok(())
}

/// Applies one action to a snapshot, returning a new one and refusing anything that would break a
/// limit, duplicate a folder name or name something that doesn't exist. `known` holds the live
/// workflow (routine) ids; the caller prunes stale placements before applying.
pub fn apply(
    snapshot: &WorkflowFoldersSnapshot,
    action: &WorkflowFolderAction,
    known: &BTreeSet<String>,
) -> Result<WorkflowFoldersSnapshot, FolderRefusal> {
    let mut next = snapshot.clone();
    match action {
        WorkflowFolderAction::Create { id, name, workflow_ids } => {
            if next.folders.len() >= MAX_FOLDERS {
                return Err(FolderRefusal::TooManyFolders);
            }
            if next.folders.iter().any(|folder| &folder.id == id) {
                return Err(FolderRefusal::DuplicateFolderId);
            }
            if next.folders.iter().any(|folder| name_key(&folder.name) == name_key(name)) {
                return Err(FolderRefusal::DuplicateName);
            }
            next.folders.push(WorkflowFolder { id: id.clone(), name: name.clone() });
            place(&mut next.placement, workflow_ids, Some(id), &next.folders, known)?;
        }
        WorkflowFolderAction::Rename { id, name } => {
            if !next.folders.iter().any(|folder| &folder.id == id) {
                return Err(FolderRefusal::UnknownFolder);
            }
            if next.folders.iter().any(|folder| &folder.id != id && name_key(&folder.name) == name_key(name)) {
                return Err(FolderRefusal::DuplicateName);
            }
            for folder in &mut next.folders {
                if &folder.id == id {
                    folder.name = name.clone();
                    break;
                }
            }
        }
        WorkflowFolderAction::Delete { id } => {
            if !next.folders.iter().any(|folder| &folder.id == id) {
                return Err(FolderRefusal::UnknownFolder);
            }
            next.folders.retain(|folder| &folder.id != id);
            next.placement.retain(|_, folder| folder != id);
        }
        WorkflowFolderAction::Place { workflow_ids, folder_id } => {
            place(&mut next.placement, workflow_ids, folder_id.as_deref(), &next.folders, known)?;
        }
    }
    Ok(next)
}

/// Drops placements of workflows that no longer exist. Folders stay: removing one is an explicit
/// `delete` action. Used on load and before applying an action, so a stale id can't block a canvas.
pub fn pruned(snapshot: &WorkflowFoldersSnapshot, known: &BTreeSet<String>) -> WorkflowFoldersSnapshot {
    let mut next = snapshot.clone();
    next.placement.retain(|workflow, _| known.contains(workflow));
    next
}

/// This computer's folders, or none before the first save and for an old or malformed blob.
pub fn load(store: &Store) -> Option<WorkflowFoldersSnapshot> {
    let raw = store.kv_read(SLOT).ok().flatten()?;
    decode_snapshot(&raw).ok()
}

/// Saves a snapshot at the next global rev, returning the stamped copy clients read back. One
/// transaction keeps the blob and the rev in step, as `store::kv_set_stamped` does for other blobs.
fn save(store: &Store, snapshot: &WorkflowFoldersSnapshot) -> Result<WorkflowFoldersSnapshot> {
    validate(snapshot).map_err(anyhow::Error::from)?;
    let mut stamped = snapshot.clone();
    store.kv_set_stamped(SLOT, |rev| {
        stamped.revision = rev;
        let value = serde_json::to_string(&stamped)?;
        if value.len() > MAX_BLOB_BYTES {
            bail!("the workflow folders on this computer are too large");
        }
        Ok(value)
    })?;
    Ok(stamped)
}

/// Loads, changes and saves under one guard, so two simultaneous actions both survive. `known` is
/// used to prune workflows deleted since the last save before the change is applied.
pub fn mutate(
    store: &Store,
    known: &BTreeSet<String>,
    change: impl FnOnce(&WorkflowFoldersSnapshot) -> Result<WorkflowFoldersSnapshot, FolderRefusal>,
) -> Result<WorkflowFoldersSnapshot> {
    let _guard = LOCK.locked();
    let current = pruned(&load(store).unwrap_or_default(), known);
    let next = change(&current)?;
    save(store, &next)
}

#[cfg(test)]
mod tests;
