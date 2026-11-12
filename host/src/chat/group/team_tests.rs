use super::*;
use crate::store::{TeamMember, TeamSettings};
use serde_json::json;

fn bot(id: &str, name: &str) -> BotConfig {
    serde_json::from_value(json!({"id": id, "name": name, "backend": "claude", "cwd": "/tmp"})).expect("valid bot")
}

fn team_group(members: &[&str], chief: Option<&str>) -> BotConfig {
    let mut cfg: BotConfig =
        serde_json::from_value(json!({"id": "g", "name": "Team", "kind": "group", "members": members}))
            .expect("valid group");
    cfg.chief_bot_id = chief.map(str::to_owned);
    cfg
}

fn member(id: &str, reports_to: Option<&str>) -> TeamMember {
    TeamMember { bot_id: id.into(), reports_to: reports_to.map(str::to_owned), ..TeamMember::default() }
}

#[test]
fn a_team_hierarchy_must_name_its_own_members_and_never_loop() {
    let g = team_group(&["a", "b", "c"], Some("a"));
    let ok = TeamSettings {
        mode: "team".into(),
        voice: "lead_and_hands".into(),
        members: vec![member("a", None), member("b", Some("a")), member("c", Some("b"))],
    };
    validate_team(&g, &ok).expect("a chain is valid");

    let stranger = TeamSettings { members: vec![member("z", None)], ..ok.clone() };
    assert!(validate_team(&g, &stranger).is_err(), "a bot outside the group");

    let self_report = TeamSettings { members: vec![member("b", Some("b"))], ..ok.clone() };
    assert!(validate_team(&g, &self_report).is_err(), "reporting to itself");

    let looped = TeamSettings {
        members: vec![member("a", Some("c")), member("b", Some("a")), member("c", Some("b"))],
        ..ok.clone()
    };
    assert!(validate_team(&g, &looped).is_err(), "a reporting loop");

    let bad_voice = TeamSettings { voice: "everyone".into(), ..ok };
    assert!(validate_team(&g, &bad_voice).is_err(), "an unknown voice");
}

#[test]
fn only_team_mode_answers_as_one() {
    let mut g = team_group(&["a", "b"], Some("a"));
    assert!(!team_mode(&g), "no setup keeps the round table");
    g.team = Some(TeamSettings { mode: "roundtable".into(), ..TeamSettings::default() });
    assert!(!team_mode(&g));
    g.team = Some(TeamSettings { mode: "team".into(), ..TeamSettings::default() });
    assert!(team_mode(&g));
}

#[test]
fn a_lead_naming_a_member_hands_it_the_floor() {
    let g = team_group(&["a", "b", "c"], Some("a"));
    let members = [bot("a", "Alice"), bot("b", "Bob"), bot("c", "Cara")];
    assert!(names("@Bob, take the release notes", &members[1]));
    assert!(!names("@bob", &members[2]));
    assert_eq!(team_voice(&g), "lead", "the default voice is the lead alone");
    let mut with_hands = g.clone();
    with_hands.team = Some(TeamSettings { voice: "lead_and_hands".into(), ..TeamSettings::default() });
    assert_eq!(team_voice(&with_hands), "lead_and_hands");
}
