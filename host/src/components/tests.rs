//! The gallery's own checks: what a card accepts, what the playground saves, what a bot may show.

use super::catalogue::{self, VERSION};
use super::{card, instances, schema};
use crate::hub::Hub;
use crate::store::{BotConfig, Store};
use serde_json::{Value, json};

fn test_hub() -> (std::sync::Arc<Hub>, tempdir::Dir) {
    let dir = tempdir::Dir::new();
    let store = Store::open(&dir.path().join("test.db")).unwrap();
    let bot: BotConfig = serde_json::from_value(json!({
        "id": "bot", "name": "Builder", "backend": "claude", "cwd": dir.path(),
    }))
    .unwrap();
    store.save_bot(&bot).unwrap();
    let identity = crate::remote::identity::Identity::load_or_create(&dir.path()).unwrap();
    (Hub::new(store, "test".into(), identity, "token".into(), 19224), dir)
}

fn form() -> Value {
    json!({
        "name": "form",
        "title": "Name the release",
        "props": {
            "fields": [
                {"id": "version", "label": "Version", "type": "text", "required": true},
                {"id": "notes", "label": "Notes", "type": "multiline"}
            ],
            "submit": "Start the release"
        }
    })
}

#[test]
fn a_form_card_round_trips_the_answers_the_person_gave() {
    let component = schema::parse(&form()).unwrap();
    let card = card::card(&component);
    assert_eq!(card["toolKind"], "component");
    assert_eq!(card["status"], "pending");
    assert_eq!(card["options"][0]["optionId"], "submit");
    assert_eq!(card["options"][0]["name"], "Start the release");
    assert_eq!(card["component"]["props"]["fields"][0]["id"], "version");

    // What the client sends: the button's id and the fields the card asked for.
    let accepted =
        card::answers(&card["component"], Some(json!({"version": "3.4.0", "notes": "Friday"}))).unwrap().unwrap();
    assert_eq!(accepted, json!({"version": "3.4.0", "notes": "Friday"}));

    // A required field left blank, a field the card never asked for, and the wrong type are refused.
    assert!(card::answers(&card["component"], Some(json!({"notes": "x"}))).is_err());
    assert!(card::answers(&card["component"], Some(json!({"version": "3.4.0", "extra": "x"}))).is_err());
    assert!(card::answers(&card["component"], Some(json!({"version": 3}))).is_err());
    // A cancel passes through.
    assert!(card::answers(&card["component"], None).unwrap().is_none());
}

#[test]
fn a_display_only_card_is_shown_at_once_and_carries_no_actions() {
    let component = schema::parse(&json!({
        "version": VERSION,
        "name": "metric",
        "props": {"label": "Builds", "value": "42", "trend": "up", "series": [1, 2, 3]}
    }))
    .unwrap();
    let card = card::card(&component);
    assert_eq!(card["status"], "shown");
    assert!(card["options"].as_array().unwrap().is_empty());
    assert!(card["block"].is_null(), "a component card is not a declarative block");
    assert_eq!(card["component"]["props"]["value"], "42");
}

#[test]
fn a_card_names_the_field_that_broke_the_catalogue() {
    let refused =
        |props: Value| schema::parse(&json!({"version": VERSION, "name": "table", "props": props})).unwrap_err();
    // A column count that doesn't match the rows.
    let error = refused(json!({"columns": ["A", "B"], "rows": [["one", "two"], ["three"]]}));
    assert!(error.to_string().contains("props.rows[1]"), "{error}");
    // A prop the component doesn't have.
    let error = refused(json!({"columns": ["A"], "rows": [["one"]], "colour": "red"}));
    assert!(error.to_string().contains("colour"), "{error}");
    // A required prop that is missing.
    let error =
        schema::parse(&json!({"version": VERSION, "name": "metric", "props": {"label": "Builds"}})).unwrap_err();
    assert!(error.to_string().contains("props.value"), "{error}");
    // Markup never reaches a client.
    let error = refused(json!({"columns": ["<b>A</b>"], "rows": [["one"]]}));
    assert!(error.to_string().contains("plain text"), "{error}");
    // A component that isn't in the catalogue is named, with the ones that are.
    let error = schema::parse(&json!({"version": VERSION, "name": "iframe", "props": {}})).unwrap_err();
    assert!(error.to_string().contains("metric"), "{error}");
}

#[test]
fn the_guide_lists_every_component_and_its_required_props() {
    let guide = catalogue::guide();
    for spec in &catalogue::ALL {
        assert!(guide.contains(spec.name), "{guide}");
    }
    assert!(guide.contains("value*"), "{guide}");
    assert!(guide.contains("Allowed shape"), "{guide}");
}

#[test]
fn withholding_a_component_keeps_it_from_that_bot_and_no_other() {
    let (hub, _dir) = test_hub();
    let props = json!({"label": "Builds", "value": "42"});
    assert!(card::document(&hub, Some("bot"), "metric", None, &props, None).is_ok());

    let saved =
        instances::call(&hub, "setComponentAccess", &json!({"bot": "bot", "component": "metric", "published": false}))
            .unwrap();
    assert_eq!(saved["published"], false);

    let error = card::document(&hub, Some("bot"), "metric", None, &props, None).unwrap_err();
    assert!(error.to_string().contains("withheld"), "{error}");
    // The playground's preview (no bot) and any other bot still have it.
    assert!(card::document(&hub, None, "metric", None, &props, None).is_ok());
    assert!(card::document(&hub, Some("another"), "metric", None, &props, None).is_ok());

    // Publishing it again gives the bot the card back.
    instances::call(&hub, "setComponentAccess", &json!({"bot": "bot", "component": "metric", "published": true}))
        .unwrap();
    assert!(card::document(&hub, Some("bot"), "metric", None, &props, None).is_ok());
}

#[test]
fn the_gallery_lists_every_component_with_its_schema_sample_and_bots() {
    let (hub, _dir) = test_hub();
    let gallery = instances::call(&hub, "listComponents", &json!({})).unwrap();
    assert_eq!(gallery["version"], VERSION);
    let components = gallery["components"].as_array().unwrap();
    assert_eq!(components.len(), catalogue::ALL.len());
    let table = components.iter().find(|entry| entry["name"] == "table").unwrap();
    assert_eq!(table["schema"]["required"], json!(["columns", "rows"]));
    assert!(table["sample"]["columns"].is_array());
    assert_eq!(table["data"], json!(["routines.list", "threads.recent", "bots.roster"]));
    assert_eq!(table["available"], true);
    let bots = gallery["bots"].as_array().unwrap();
    assert_eq!(bots[0]["name"], "Builder");
    assert!(bots[0]["withheld"].as_array().unwrap().is_empty());
}

#[test]
fn a_published_card_is_in_the_gallery_and_a_draft_is_not() {
    let (hub, _dir) = test_hub();
    let draft = instances::call(
        &hub,
        "saveComponent",
        &json!({"name": "table", "title": "Routines", "props": {
            "columns": ["Routine", "When", "On"],
            "rows": [["Morning digest", "Weekdays at 08:00", "On"]]
        }}),
    )
    .unwrap();
    assert_eq!(draft["instance"]["published"], false);
    let id = draft["instance"]["id"].as_str().unwrap().to_owned();

    // Saving again keeps the same row, and publishing puts it in the gallery for everyone.
    let published = instances::call(&hub, "publishComponent", &json!({"id": id, "published": true})).unwrap();
    assert_eq!(published["instance"]["published"], true);
    assert_eq!(published["instance"]["title"], "Routines");
    let gallery = instances::call(&hub, "listComponents", &json!({})).unwrap();
    assert_eq!(gallery["instances"].as_array().unwrap().len(), 1);
    assert_eq!(gallery["instances"][0]["valid"], true);

    // A card with a prop the catalogue refuses is never saved.
    let refused = instances::call(&hub, "saveComponent", &json!({"name": "metric", "props": {"label": "x"}}));
    assert!(refused.is_err());

    instances::call(&hub, "deleteComponent", &json!({"id": id})).unwrap();
    let gallery = instances::call(&hub, "listComponents", &json!({})).unwrap();
    assert!(gallery["instances"].as_array().unwrap().is_empty());
}

#[test]
fn a_data_function_fills_a_card_from_this_computer() {
    let (hub, _dir) = test_hub();
    // The bot's own roster is real host state the card didn't carry.
    let component = card::document(
        &hub,
        Some("bot"),
        "table",
        Some("Bots"),
        &json!({}),
        Some(&json!([{"function": "bots.roster"}])),
    )
    .unwrap();
    assert_eq!(component.doc["source"], "bots.roster");
    assert_eq!(component.doc["props"]["columns"], json!(["Bot", "Kind"]));
    let rows = component.doc["props"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "Builder");
    // A prop the caller wrote wins over the resolved one.
    let component = card::document(
        &hub,
        Some("bot"),
        "table",
        None,
        &json!({"columns": ["Name"]}),
        Some(&json!([{"function": "bots.roster"}])),
    )
    .unwrap();
    assert_eq!(component.doc["props"]["columns"], json!(["Name"]));
}

#[test]
fn a_card_refuses_a_data_function_it_was_not_granted() {
    let (hub, _dir) = test_hub();
    let error = card::document(
        &hub,
        Some("bot"),
        "metric",
        None,
        &json!({"label": "x", "value": "1"}),
        Some(&json!([{"function": "bots.roster"}])),
    )
    .unwrap_err();
    assert!(error.to_string().contains("can't use"), "{error}");
    // A limit is clamped to what the function allows, never trusted.
    assert!(catalogue::grants("table").contains(&"routines.list"));
    let error = card::document(
        &hub,
        Some("bot"),
        "table",
        None,
        &json!({"columns": ["A"], "rows": [["one"]]}),
        Some(&json!([{"function": "routines.list", "limit": "all"}])),
    )
    .unwrap_err();
    assert!(error.to_string().contains("whole number"), "{error}");
}

#[test]
fn every_sample_passes_its_own_schema() {
    for spec in &catalogue::ALL {
        let component = schema::build(spec, None, None, &catalogue::sample(spec))
            .unwrap_or_else(|error| panic!("{}'s sample is refused: {error}", spec.name));
        assert_eq!(component.doc["name"], spec.name);
        assert_eq!(component.doc["version"], VERSION);
    }
}

#[test]
fn the_tool_lists_the_components_it_can_show() {
    let tool = card::tool();
    assert_eq!(tool["name"], "show_component");
    let choices = tool["inputSchema"]["properties"]["name"]["enum"].as_array().unwrap();
    assert_eq!(choices.len(), catalogue::ALL.len());
    assert!(tool["description"].as_str().unwrap().contains("nothing here is HTML"));
}

/// A small directory that removes itself, so the tests share no file with each other.
mod tempdir {
    pub struct Dir(std::path::PathBuf);

    impl Dir {
        pub fn new() -> Self {
            let path = std::env::temp_dir().join(format!("codync-components-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        pub fn path(&self) -> std::path::PathBuf {
            self.0.clone()
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
