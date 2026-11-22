//! Unit tests for the workflow folder model, action parsing and stamped storage.

use super::*;
use serde_json::json;

fn snapshot() -> WorkflowFoldersSnapshot {
    WorkflowFoldersSnapshot {
        revision: 7,
        folders: vec![
            WorkflowFolder { id: "f1".into(), name: "Pictures".into() },
            WorkflowFolder { id: "f2".into(), name: "Orders".into() },
        ],
        placement: BTreeMap::from([("w1".to_owned(), "f1".to_owned())]),
    }
}

fn known() -> BTreeSet<String> {
    ["w1", "w2", "w3"].into_iter().map(str::to_owned).collect()
}

fn place_action(workflows: &[&str], folder: Option<&str>) -> WorkflowFolderAction {
    WorkflowFolderAction::Place {
        workflow_ids: workflows.iter().map(|id| (*id).to_owned()).collect(),
        folder_id: folder.map(str::to_owned),
    }
}

fn create(id: &str, name: &str) -> WorkflowFolderAction {
    WorkflowFolderAction::Create { id: id.into(), name: name.into(), workflow_ids: Vec::new() }
}

#[test]
fn creating_a_folder_refuses_duplicate_empty_and_overlong_names() {
    let made = apply(&snapshot(), &create("f3", "  Reports  "), &known()).expect("a new folder");
    assert_eq!(made.folders.last().map(|f| f.name.as_str()), Some("Reports"), "names are trimmed");
    assert_eq!(made.folders.len(), 3);

    let cases = [
        (create("f3", "Pictures"), FolderRefusal::DuplicateName),
        (create("f3", "  pictures "), FolderRefusal::DuplicateName),
        (create("f3", " "), FolderRefusal::EmptyName),
        (create("f3", &"x".repeat(MAX_NAME_CHARS + 1)), FolderRefusal::NameTooLong),
        (create("f1", "Reports"), FolderRefusal::DuplicateFolderId),
    ];
    for (action, refusal) in cases {
        assert_eq!(apply(&snapshot(), &action, &known()).unwrap_err(), refusal, "{action:?}");
    }
}

#[test]
fn creating_a_folder_can_file_workflows_in_it() {
    let made = apply(
        &snapshot(),
        &WorkflowFolderAction::Create {
            id: "f3".into(),
            name: "Reports".into(),
            workflow_ids: vec!["w2".into(), "w3".into()],
        },
        &known(),
    )
    .expect("a folder with members");
    assert_eq!(made.placement.get("w2").map(String::as_str), Some("f3"));
    assert_eq!(made.placement.get("w3").map(String::as_str), Some("f3"));
    assert_eq!(made.placement.get("w1").map(String::as_str), Some("f1"), "other placements stay");
    assert_eq!(
        apply(
            &snapshot(),
            &WorkflowFolderAction::Create { id: "f3".into(), name: "Reports".into(), workflow_ids: vec!["w9".into()] },
            &known(),
        )
        .unwrap_err(),
        FolderRefusal::UnknownWorkflow
    );
}

#[test]
fn renaming_a_folder_keeps_its_placements_and_refuses_unknown_or_duplicate() {
    let renamed =
        apply(&snapshot(), &WorkflowFolderAction::Rename { id: "f1".into(), name: "Gallery".into() }, &known())
            .expect("a rename");
    assert_eq!(renamed.folders[0].name, "Gallery");
    assert_eq!(renamed.placement.get("w1").map(String::as_str), Some("f1"), "placements follow the id");

    assert_eq!(
        apply(&snapshot(), &WorkflowFolderAction::Rename { id: "f9".into(), name: "New".into() }, &known())
            .unwrap_err(),
        FolderRefusal::UnknownFolder
    );
    assert_eq!(
        apply(&snapshot(), &WorkflowFolderAction::Rename { id: "f2".into(), name: "PICTURES".into() }, &known())
            .unwrap_err(),
        FolderRefusal::DuplicateName
    );
    // Renaming itself to a slightly different spelling is fine.
    assert!(
        apply(&snapshot(), &WorkflowFolderAction::Rename { id: "f1".into(), name: " pictures ".into() }, &known())
            .is_ok()
    );
}

#[test]
fn deleting_a_folder_removes_its_placements_but_not_other_folders() {
    let deleted = apply(&snapshot(), &WorkflowFolderAction::Delete { id: "f1".into() }, &known()).expect("a delete");
    assert_eq!(deleted.folders.len(), 1);
    assert!(deleted.placement.is_empty(), "the deleted folder's placements go with it");
    assert_eq!(
        apply(&snapshot(), &WorkflowFolderAction::Delete { id: "f9".into() }, &known()).unwrap_err(),
        FolderRefusal::UnknownFolder
    );
}

#[test]
fn placing_workflows_refuses_unknown_folders_and_unknown_workflows() {
    let placed = apply(&snapshot(), &place_action(&["w2"], Some("f2")), &known()).expect("a placement");
    assert_eq!(placed.placement.get("w2").map(String::as_str), Some("f2"));
    assert_eq!(placed.placement.get("w1").map(String::as_str), Some("f1"), "other placements stay");
    // Moving a workflow replaces its old placement; null files it in "All workflows".
    let moved = apply(&snapshot(), &place_action(&["w1"], Some("f2")), &known()).expect("a move");
    assert_eq!(moved.placement.get("w1").map(String::as_str), Some("f2"));
    assert_eq!(moved.placement.len(), 1);
    let unfiled = apply(&snapshot(), &place_action(&["w1"], None), &known()).expect("an unfiled workflow");
    assert!(unfiled.placement.is_empty());

    assert_eq!(
        apply(&snapshot(), &place_action(&["w2"], Some("f9")), &known()).unwrap_err(),
        FolderRefusal::UnknownFolder
    );
    assert_eq!(
        apply(&snapshot(), &place_action(&["w9"], Some("f1")), &known()).unwrap_err(),
        FolderRefusal::UnknownWorkflow
    );
}

#[test]
fn a_computer_holds_at_most_two_hundred_folders() {
    let mut full = snapshot();
    full.folders =
        (0..MAX_FOLDERS).map(|i| WorkflowFolder { id: format!("f{i}"), name: format!("Folder {i}") }).collect();
    full.placement.clear();
    assert!(validate(&full).is_ok());
    assert_eq!(apply(&full, &create("new", "New"), &known()).unwrap_err(), FolderRefusal::TooManyFolders);
    full.folders.push(WorkflowFolder { id: "extra".into(), name: "Extra".into() });
    assert_eq!(
        decode_snapshot(&serde_json::to_string(&full).expect("json")).unwrap_err(),
        FolderRefusal::TooManyFolders
    );
}

#[test]
fn one_action_carries_at_most_a_thousand_workflows() {
    let ids: Vec<String> = (0..MAX_WORKFLOWS_PER_ACTION).map(|i| format!("w{i}")).collect();
    let action = json!({"action": "place", "workflowIds": ids, "folderId": "f1"});
    assert!(parse_action(&action).is_ok());
    let mut too_many = ids;
    too_many.push("one-too-many".into());
    assert_eq!(
        parse_action(&json!({"action": "place", "workflowIds": too_many, "folderId": "f1"})).unwrap_err(),
        FolderRefusal::TooManyWorkflows
    );
}

#[test]
fn placements_are_capped_at_ten_thousand() {
    let mut full = snapshot();
    let everyone: BTreeSet<String> = (0..MAX_PLACEMENTS).map(|i| format!("w{i}")).collect();
    full.placement = everyone.iter().map(|w| (w.clone(), "f1".to_owned())).collect();
    assert!(validate(&full).is_ok());
    let extra: BTreeSet<String> = everyone.iter().cloned().chain(std::iter::once("extra".to_owned())).collect();
    assert_eq!(
        apply(&full, &place_action(&["extra"], Some("f1")), &extra).unwrap_err(),
        FolderRefusal::TooManyPlacements
    );
}

#[test]
fn decoding_refuses_unknown_missing_and_malformed_fields() {
    let good = serde_json::to_value(snapshot()).expect("json");
    assert!(decode_snapshot(&good.to_string()).is_ok());

    let mut extra = good.clone();
    extra["extra"] = json!(1);
    assert_eq!(decode_snapshot(&extra.to_string()).unwrap_err(), FolderRefusal::MalformedSnapshot);

    let mut missing = good.clone();
    missing.as_object_mut().expect("object").remove("placement");
    assert_eq!(decode_snapshot(&missing.to_string()).unwrap_err(), FolderRefusal::MalformedSnapshot);

    assert_eq!(decode_snapshot("not json").unwrap_err(), FolderRefusal::MalformedSnapshot);
    let bad = serde_json::to_string(&json!({"revision": -1, "folders": [], "placement": {}})).expect("json");
    assert_eq!(decode_snapshot(&bad).unwrap_err(), FolderRefusal::InvalidRevision);
}

#[test]
fn decoding_checks_names_ids_and_placement_targets() {
    let bad_name = json!({"revision": 1, "folders": [{"id":"f1","name":""}], "placement": {}});
    assert_eq!(decode_snapshot(&bad_name.to_string()).unwrap_err(), FolderRefusal::EmptyName);
    let bad_id = json!({"revision": 1, "folders": [{"id":"","name":"A"}], "placement": {}});
    assert_eq!(decode_snapshot(&bad_id.to_string()).unwrap_err(), FolderRefusal::InvalidId);
    let duplicate = json!({"revision": 1, "folders": [
        {"id":"f1","name":"A"}, {"id":"f2","name":"a "}
    ], "placement": {}});
    assert_eq!(decode_snapshot(&duplicate.to_string()).unwrap_err(), FolderRefusal::DuplicateName);
    let stray = json!({"revision": 1, "folders": [], "placement": {"w1":"f9"}});
    assert_eq!(decode_snapshot(&stray.to_string()).unwrap_err(), FolderRefusal::UnknownFolder);
    let too_long =
        json!({"revision": 1, "folders": [{"id":"f1","name":"x".repeat(MAX_NAME_CHARS + 1)}], "placement": {}});
    assert_eq!(decode_snapshot(&too_long.to_string()).unwrap_err(), FolderRefusal::NameTooLong);
}

#[test]
fn parsing_refuses_wrong_action_shapes_and_keys() {
    let made = json!({"action":"create","id":"f3","name":"New","workflowIds":["w1"]});
    assert_eq!(
        parse_action(&made).unwrap(),
        WorkflowFolderAction::Create { id: "f3".into(), name: "New".into(), workflow_ids: vec!["w1".into()] }
    );
    assert_eq!(
        parse_action(&json!({"action":"rename","folderId":"f3","name":"New"})).unwrap(),
        WorkflowFolderAction::Rename { id: "f3".into(), name: "New".into() }
    );
    assert_eq!(
        parse_action(&json!({"action":"delete","folderId":"f3"})).unwrap(),
        WorkflowFolderAction::Delete { id: "f3".into() }
    );
    assert_eq!(
        parse_action(&json!({"action":"place","workflowIds":["w1"],"folderId":null})).unwrap(),
        WorkflowFolderAction::Place { workflow_ids: vec!["w1".into()], folder_id: None }
    );

    assert_eq!(parse_action(&json!({"action":"move","folderId":"f3"})).unwrap_err(), FolderRefusal::UnknownAction);
    assert_eq!(parse_action(&json!({"id":"f3","name":"New"})).unwrap_err(), FolderRefusal::MalformedAction);
    assert_eq!(
        parse_action(&json!({"action":"delete","folderId":"f3","name":"New"})).unwrap_err(),
        FolderRefusal::InvalidActionKeys
    );
    assert_eq!(parse_action(&json!({"action":"create","id":"f3"})).unwrap_err(), FolderRefusal::InvalidActionKeys);
    assert_eq!(
        parse_action(&json!({"action":"place","workflowIds":"w1","folderId":"f1"})).unwrap_err(),
        FolderRefusal::MalformedAction
    );
    assert_eq!(
        parse_action(&json!({"action":"place","workflowIds":["w1"],"folderId":1})).unwrap_err(),
        FolderRefusal::InvalidId
    );
    assert_eq!(parse_action(&json!({"action":"create","id":"f3","name":" "})).unwrap_err(), FolderRefusal::EmptyName);
    assert_eq!(parse_action(&json!({"action":"create","id":"","name":"New"})).unwrap_err(), FolderRefusal::InvalidId);
}

#[test]
fn pruning_drops_workflows_that_no_longer_exist() {
    let mut stale = snapshot();
    stale.placement.insert("gone".into(), "f2".into());
    let pruned = pruned(&stale, &known());
    assert_eq!(pruned.placement.get("w1").map(String::as_str), Some("f1"));
    assert!(!pruned.placement.contains_key("gone"));
    assert_eq!(pruned.folders.len(), 2, "folders are kept for an explicit delete");
}

#[test]
fn a_round_trip_keeps_folders_placements_and_revision() {
    let raw = serde_json::to_string(&snapshot()).expect("json");
    assert_eq!(decode_snapshot(&raw).expect("decoded"), snapshot());
}

#[test]
fn an_oversized_blob_is_refused() {
    let huge = "x".repeat(MAX_BLOB_BYTES + 1);
    assert_eq!(decode_snapshot(&huge).unwrap_err(), FolderRefusal::MalformedSnapshot);
}

#[test]
fn saving_stamps_a_revision_clients_can_read() {
    let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
    let before = store.current_rev();
    assert!(load(&store).is_none(), "a computer without folders omits the field");

    let first =
        mutate(&store, &known(), |current| apply(current, &create("f3", "New"), &known())).expect("the first change");
    assert!(first.revision > before, "the blob carries a fresh global rev");
    let loaded = load(&store).expect("a saved snapshot");
    assert_eq!(loaded, first);
    assert_eq!(store.current_rev(), first.revision, "the rev is the global one");

    let second = mutate(&store, &known(), |current| {
        apply(current, &WorkflowFolderAction::Rename { id: "f3".into(), name: "Renamed".into() }, &known())
    })
    .expect("the second change");
    assert!(second.revision > first.revision, "each write bumps the rev clients catch up on");
    assert_eq!(load(&store).expect("the new snapshot"), second);
}
