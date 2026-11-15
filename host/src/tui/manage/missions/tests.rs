use super::super::super::app::{App, Msg, Overlay};
use super::*;
use crossterm::event::KeyModifiers;

fn app() -> App {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut app = App::new(crate::tui::net::Client::new("http://127.0.0.1:1", None), tx, String::new());
    app.on_msg(Msg::Event(json!({"type": "bot", "bot": {"id": "bot", "name": "Missions"}})));
    app
}

fn sheet() -> MissionSheet {
    MissionSheet {
        bot: "bot".into(),
        items: None,
        cursor: 0,
        step: 0,
        focus_steps: false,
        finished: false,
        edit: None,
        busy: false,
        error: None,
    }
}

fn mission(id: &str, status: &str, steps: &Value) -> Value {
    json!({"id": id, "botId": "bot", "title": format!("mission {id}"), "goal": "g", "status": status,
           "checkpoint": "", "createdAt": 0, "updatedAt": 1, "steps": steps})
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn a_loaded_list_clamps_the_cursor_and_the_step() {
    let mut app = app();
    app.overlays.push(Overlay::Missions(Box::new(sheet())));
    if let Some(Overlay::Missions(m)) = app.overlays.last_mut() {
        m.cursor = 5;
        m.step = 9;
    }
    app.missions_loaded("bot", false, &json!({"missions": [mission("a", "active", &json!([]))]}));
    let Some(Overlay::Missions(m)) = app.overlays.last() else { panic!("the sheet stays open") };
    assert_eq!(m.cursor, 0);
    assert_eq!(m.step, 0);
    assert_eq!(m.items.as_ref().map(Vec::len), Some(1));
    assert_eq!(m.items.as_ref().unwrap()[0].title, "mission a");
}

#[test]
fn a_list_for_another_filter_does_not_replace_this_one() {
    let mut app = app();
    app.overlays.push(Overlay::Missions(Box::new(sheet())));
    app.missions_loaded("bot", true, &json!({"missions": [mission("a", "done", &json!([]))]}));
    let Some(Overlay::Missions(m)) = app.overlays.last() else { panic!() };
    assert!(m.items.is_none(), "another filter's list is not this sheet's");
}

#[test]
fn keys_move_focus_and_close() {
    let mut app = app();
    app.overlays.push(Overlay::Missions(Box::new(sheet())));
    let step = json!([{"id": "s", "missionId": "a", "position": 0, "text": "one", "status": "pending", "at": 0}]);
    app.missions_loaded("bot", false, &json!({"missions": [mission("a", "active", &step)]}));
    let Some(Overlay::Missions(m)) = app.overlays.pop() else { panic!() };
    let Some(Overlay::Missions(m)) = app.missions_key(*m, key(KeyCode::Tab)) else { panic!() };
    assert!(m.focus_steps);
    let Some(Overlay::Missions(m)) = app.missions_key(*m, key(KeyCode::Tab)) else { panic!() };
    assert!(!m.focus_steps);
    assert!(app.missions_key(*m, key(KeyCode::Esc)).is_none());
}

#[test]
fn an_editor_takes_the_keys_and_esc_drops_it() {
    let mut app = app();
    app.overlays.push(Overlay::Missions(Box::new(sheet())));
    app.missions_loaded("bot", false, &json!({"missions": [mission("a", "active", &json!([]))]}));
    let Some(Overlay::Missions(m)) = app.overlays.pop() else { panic!() };
    let Some(Overlay::Missions(m)) = app.missions_key(*m, key(KeyCode::Char('c'))) else {
        panic!("c opens the checkpoint")
    };
    assert!(m.edit.is_some());
    let Some(Overlay::Missions(next)) = app.missions_key(*m, key(KeyCode::Esc)) else { panic!() };
    assert!(next.edit.is_none(), "esc drops the editor, not the sheet");
}

#[test]
fn a_takes_a_mission_on_and_back() {
    assert_eq!(step_status(MissionStatus::Proposed), "active");
    assert_eq!(step_status(MissionStatus::Waiting), "active");
    assert_eq!(step_status(MissionStatus::Done), "active");
    assert_eq!(step_status(MissionStatus::Active), "waiting");
}

#[test]
fn steps_have_their_own_glyph() {
    assert_eq!(mark(MissionStepStatus::Pending), "·");
    assert_eq!(mark(MissionStepStatus::Done), "✓");
    assert_eq!(mark(MissionStepStatus::Skipped), "–");
    assert_eq!(mark(MissionStepStatus::Active), "▸");
}
