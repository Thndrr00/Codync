//! Acceptance and refusal of block documents (card K3).

use super::*;
use serde_json::json;

fn form() -> Value {
    json!({
        "version": 1,
        "title": "Book the trip",
        "body": [
            {"type": "heading", "text": "Madrid"},
            {"type": "text", "text": "Two nights, from Tuesday."},
            {"type": "callout", "tone": "warning", "text": "The price may change."},
            {"type": "field", "field": {
                "id": "nights", "label": "Nights", "type": "number", "required": true, "value": 2
            }},
            {"type": "field", "field": {
                "id": "room", "label": "Room", "type": "select", "required": true,
                "options": [{"value": "single", "label": "Single"}, {"value": "double", "label": "Double"}]
            }},
            {"type": "field", "field": {"id": "note", "label": "Note", "type": "multiline"}},
            {"type": "divider"},
            {"type": "table", "columns": ["Item", "Price"], "rows": [["Room", 120], ["Taxi", null]]},
            {"type": "chart", "chartType": "bar", "labels": ["Mon", "Tue"], "values": [2, 5]}
        ],
        "actions": [
            {"id": "approve", "label": "Approve", "kind": "primary"},
            {"id": "deny", "label": "Deny", "kind": "destructive"}
        ]
    })
}

#[test]
fn a_full_form_is_accepted_and_normalized() {
    let block = parse(&form()).unwrap();
    assert_eq!(block.doc["version"], 1);
    assert_eq!(block.doc["title"], "Book the trip");
    assert_eq!(block.doc["body"].as_array().unwrap().len(), 9);
    assert_eq!(block.fields.len(), 3);
    assert_eq!(block.fields[1].options.len(), 2);
    assert_eq!(block.actions.len(), 2);
    assert_eq!(block.actions[1].kind, "destructive");
    assert_eq!(block.doc["body"][3]["field"]["value"], 2);
    assert_eq!(block.doc["body"][7]["rows"][1][1], serde_json::Value::Null);
}

#[test]
fn a_form_without_actions_gets_one_submit() {
    let block = parse(&json!({
        "version": 1,
        "title": "Your name",
        "body": [{"type": "field", "field": {"id": "name", "label": "Name", "type": "text"}}]
    }))
    .unwrap();
    assert_eq!(block.actions.len(), 1);
    assert_eq!(block.actions[0].id, "submit");
    assert_eq!(block.actions[0].label, "Submit");
}

#[test]
fn a_display_only_block_offers_nothing() {
    let block = parse(&json!({
        "version": 1,
        "title": "This week",
        "body": [{"type": "chart", "chartType": "line", "labels": ["Mon"], "values": [1]}]
    }))
    .unwrap();
    assert!(block.actions.is_empty());
}

#[test]
fn unknown_components_props_and_types_are_refused() {
    for (document, expected) in [
        (json!({"version": 1, "title": "x", "body": [{"type": "iframe", "src": "https://x"}]}), "the components are"),
        (
            json!({"version": 1, "title": "x", "body": [{"type": "text", "text": "hi", "colour": "red"}]}),
            "unknown prop \"colour\"",
        ),
        (
            json!({"version": 1, "title": "x", "body": [{"type": "field", "field": {"id": "a", "label": "A", "type": "file"}}]}),
            "the field types are",
        ),
        (json!({"version": 2, "title": "x", "body": [{"type": "divider"}]}), "must be 1"),
        (json!({"title": "x", "body": [{"type": "divider"}]}), "must be 1"),
        (json!({"version": 1, "title": "x", "body": []}), "1 to"),
    ] {
        let error = parse(&document).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn markup_scripts_and_control_characters_are_refused() {
    for body in [
        json!([{"type": "text", "text": "<b>bold</b>"}]),
        json!([{"type": "heading", "text": "<script>alert(1)</script>"}]),
        json!([{"type": "text", "text": "see javascript:alert(1)"}]),
        json!([{"type": "text", "text": "bell\u{7}"}]),
    ] {
        assert!(parse(&json!({"version": 1, "title": "x", "body": body})).is_err());
    }
}

#[test]
fn sizes_are_bounded() {
    let many = (0..=MAX_BODY).map(|index| json!({"type": "text", "text": format!("line {index}")})).collect::<Vec<_>>();
    assert!(parse(&json!({"version": 1, "title": "x", "body": many})).is_err());
    let wide = json!({
        "version": 1,
        "title": "x",
        "body": [{"type": "table", "columns": (0..=MAX_TABLE_COLUMNS).map(|i| format!("c{i}")).collect::<Vec<_>>(),
                  "rows": []}]
    });
    assert!(parse(&wide).is_err());
    let long = json!({
        "version": 1,
        "title": "x",
        "body": [{"type": "chart", "chartType": "bar",
                  "labels": (0..=MAX_CHART_POINTS).map(|i| format!("p{i}")).collect::<Vec<_>>(),
                  "values": (0..=MAX_CHART_POINTS).map(|i| i as f64).collect::<Vec<_>>()}]
    });
    assert!(parse(&long).is_err());
    let title = "x".repeat(MAX_TITLE + 1);
    assert!(parse(&json!({"version": 1, "title": title, "body": [{"type": "divider"}]})).is_err());
}

#[test]
fn table_rows_must_match_the_columns() {
    let error = parse(&json!({
        "version": 1,
        "title": "x",
        "body": [{"type": "table", "columns": ["A", "B"], "rows": [["only one"]]}]
    }))
    .unwrap_err()
    .to_string();
    assert!(error.contains("the table has 2 columns"), "{error}");
}

#[test]
fn chart_values_must_match_its_labels() {
    assert!(
        parse(&json!({
            "version": 1, "title": "x",
            "body": [{"type": "chart", "chartType": "bar", "labels": ["a", "b"], "values": [1]}]
        }))
        .is_err()
    );
}

#[test]
fn answers_are_checked_against_the_form() {
    let block = parse(&form()).unwrap();
    let answers = block.answers(&json!({"nights": 3, "room": "double", "note": "Late arrival"})).unwrap();
    assert_eq!(answers["nights"], 3);
    assert_eq!(answers["room"], "double");
    assert_eq!(answers["note"], "Late arrival");

    assert!(block.answers(&json!({"room": "double"})).is_err(), "a required field is missing");
    assert!(block.answers(&json!({"nights": 3, "room": "suite"})).is_err(), "not one of the choices");
    assert!(block.answers(&json!({"nights": "three", "room": "double"})).is_err(), "wrong type");
    assert!(block.answers(&json!({"nights": 2, "room": "double", "extra": 1})).is_err(), "unknown field");
    assert!(block.answers(&json!("not an object")).is_err());
}
