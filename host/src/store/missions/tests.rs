use super::*;
use crate::store::tests::temp_store;

fn bot(store: &Store, id: &str) {
    store.save_bot(&serde_json::from_value(serde_json::json!({"id": id, "name": id})).unwrap()).unwrap();
}

#[test]
fn a_mission_keeps_its_steps_and_checkpoint() {
    let store = temp_store();
    bot(&store, "b1");
    let mission = store
        .create_mission("b1", "Ship the release", "cut 2.48.0", &["write notes".to_owned(), "tag".to_owned()])
        .unwrap();
    assert_eq!(mission.status, MissionStatus::Proposed);
    assert_eq!(mission.goal, "cut 2.48.0");
    assert_eq!(mission.steps.len(), 2);
    assert!(mission.steps.iter().all(|s| s.status == MissionStepStatus::Pending));

    let step = mission.steps[0].id.clone();
    let mission = store.set_mission_step(&mission.id, &step, MissionStepStatus::Done).unwrap();
    assert_eq!(mission.steps[0].status, MissionStepStatus::Done);
    let mission = store.set_mission_checkpoint(&mission.id, "notes written; tag left").unwrap();
    assert_eq!(mission.checkpoint, "notes written; tag left");
    let reread = store.mission(&mission.id).unwrap().unwrap();
    assert_eq!(reread.checkpoint, "notes written; tag left");
    assert_eq!(reread.steps.len(), 2);
}

#[test]
fn an_empty_title_is_refused() {
    let store = temp_store();
    bot(&store, "b1");
    assert!(store.create_mission("b1", "   ", "", &[]).is_err());
    assert!(store.create_mission("b1", &"x".repeat(MAX_TITLE + 1), "", &[]).is_err());
}

#[test]
fn a_mission_walks_the_states_and_abandoning_needs_a_reason() {
    let store = temp_store();
    bot(&store, "b1");
    let mission = store.create_mission("b1", "t", "", &[]).unwrap();
    assert!(store.set_mission_status(&mission.id, MissionStatus::Done, None).is_err(), "proposed cannot finish");
    assert!(store.set_mission_status(&mission.id, MissionStatus::Abandoned, None).is_err(), "no reason yet");
    let mission = store.set_mission_status(&mission.id, MissionStatus::Abandoned, Some("the ask went away")).unwrap();
    assert_eq!(mission.stop_reason.as_deref(), Some("the ask went away"));
    let mission = store.set_mission_status(&mission.id, MissionStatus::Active, None).unwrap();
    assert_eq!(mission.status, MissionStatus::Active);
    let mission = store.set_mission_status(&mission.id, MissionStatus::Waiting, None).unwrap();
    let mission = store.set_mission_status(&mission.id, MissionStatus::Done, None).unwrap();
    assert_eq!(mission.status, MissionStatus::Done);
    assert_eq!(store.mission(&mission.id).unwrap().unwrap().status, MissionStatus::Done, "done keeps the history");
}

#[test]
fn a_bot_and_the_fleet_get_a_limit_of_active_missions() {
    let store = temp_store();
    bot(&store, "b1");
    bot(&store, "b2");
    for i in 0..MAX_ACTIVE_PER_BOT {
        let mission = store.create_mission("b1", &format!("m{i}"), "", &[]).unwrap();
        store.set_mission_status(&mission.id, MissionStatus::Active, None).unwrap();
    }
    let extra = store.create_mission("b1", "one too many", "", &[]).unwrap();
    let error = store.set_mission_status(&extra.id, MissionStatus::Active, None).unwrap_err();
    assert!(error.to_string().contains("already runs"), "{error}");
    // Another bot still has room.
    let other = store.create_mission("b2", "elsewhere", "", &[]).unwrap();
    assert!(store.set_mission_status(&other.id, MissionStatus::Active, None).is_ok());
}

#[test]
fn the_fleet_limit_stops_the_next_bot() {
    let store = temp_store();
    let mut bots = 0;
    for i in 0..MAX_ACTIVE_FLEET {
        let id = format!("b{i}");
        bot(&store, &id);
        let mission = store.create_mission(&id, "m", "", &[]).unwrap();
        store.set_mission_status(&mission.id, MissionStatus::Active, None).unwrap();
        bots += 1;
    }
    assert_eq!(bots, MAX_ACTIVE_FLEET);
    bot(&store, "late");
    let mission = store.create_mission("late", "m", "", &[]).unwrap();
    let error = store.set_mission_status(&mission.id, MissionStatus::Active, None).unwrap_err();
    assert!(error.to_string().contains("fleet"), "{error}");
}

#[test]
fn a_restart_parks_the_running_missions_and_keeps_them() {
    let store = temp_store();
    bot(&store, "b1");
    let mission = store.create_mission("b1", "long one", "many turns", &["step".to_owned()]).unwrap();
    store.set_mission_status(&mission.id, MissionStatus::Active, None).unwrap();
    store.set_mission_run(&mission.id, Some("run-1")).unwrap();
    let parked = store.park_interrupted_missions().unwrap();
    assert_eq!(parked, 1);
    let mission = store.mission(&mission.id).unwrap().unwrap();
    assert_eq!(mission.status, MissionStatus::Waiting);
    assert_eq!(mission.run_id, None);
    assert_eq!(mission.stop_reason.as_deref(), Some("the host restarted mid-run"));
    assert_eq!(mission.steps.len(), 1);
}

#[test]
fn listing_narrows_by_bot_title_and_open_state() {
    let store = temp_store();
    bot(&store, "b1");
    bot(&store, "b2");
    let mine = store.create_mission("b1", "deploy the site", "", &[]).unwrap();
    store.create_mission("b1", "write the report", "", &[]).unwrap();
    let other = store.create_mission("b2", "deploy the app", "", &[]).unwrap();
    store.set_mission_status(&other.id, MissionStatus::Active, None).unwrap();
    store.set_mission_status(&other.id, MissionStatus::Done, None).unwrap();

    assert_eq!(store.missions(Some("b1"), None, false).unwrap().len(), 2);
    let found = store.missions(None, Some("deploy"), false).unwrap();
    assert_eq!(found.len(), 2, "the title search spans bots");
    let open = store.missions(None, Some("deploy"), true).unwrap();
    assert_eq!(open.iter().map(|m| m.id.clone()).collect::<Vec<_>>(), vec![mine.id]);
    assert_eq!(store.missions(Some("b2"), None, false).unwrap()[0].id, other.id);
}

#[test]
fn steps_and_links_are_found_only_on_their_mission() {
    let store = temp_store();
    bot(&store, "b1");
    let one = store.create_mission("b1", "one", "", &[]).unwrap();
    let two = store.create_mission("b1", "two", "", &[]).unwrap();
    let one = store.add_mission_step(&one.id, "only here").unwrap();
    let step = one.steps[0].id.clone();
    assert!(store.set_mission_step(&two.id, &step, MissionStepStatus::Active).is_err());

    store.set_mission_approval(&one.id, Some("entry-1")).unwrap();
    let one = store.mission(&one.id).unwrap().unwrap();
    assert_eq!(one.approval_id.as_deref(), Some("entry-1"));
    let one = store.set_mission_approval(&one.id, None).unwrap();
    assert_eq!(one.approval_id, None);
}

#[test]
fn a_step_cap_is_kept() {
    let store = temp_store();
    bot(&store, "b1");
    let mission = store.create_mission("b1", "many", "", &[]).unwrap();
    for i in 0..MAX_STEPS {
        store.add_mission_step(&mission.id, &format!("s{i}")).unwrap();
    }
    assert!(store.add_mission_step(&mission.id, "one too many").is_err());
}
