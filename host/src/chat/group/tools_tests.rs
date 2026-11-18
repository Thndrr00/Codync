use super::tools::*;
use crate::chat::group::MAX_TITLE_CHARS;
use crate::chat::profile::MAX_INSTRUCTIONS_CHARS;
use crate::hub::Hub;
use crate::remote::identity::Identity;
use crate::store::{BotConfig, Store};
use serde_json::json;
use std::sync::Arc;

fn hub(dir: &std::path::Path) -> Arc<Hub> {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    for (id, manage) in [("lead", true), ("dev", false), ("ops", false)] {
        let cfg: BotConfig = serde_json::from_value(json!({"id": id, "name": id, "backend": "claude",
            "cwd": dir.to_string_lossy(), "manageTeam": manage}))
        .unwrap();
        store.save_bot(&cfg).unwrap();
    }
    let hub = Hub::new(store, "group-tools-test".into(), Identity::load_or_create(dir).unwrap(), "test".into(), 0);
    hub.start().unwrap();
    hub
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("codync-{name}-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn notices(hub: &Hub, bot: &str) -> Vec<String> {
    hub.store
        .history(bot, i64::MAX, 20)
        .unwrap()
        .into_iter()
        .filter_map(|e| e.data["text"].as_str().map(str::to_owned))
        .collect()
}

#[tokio::test]
async fn a_manager_creates_a_channel_and_changes_it() {
    let dir = temp("group-tools");
    let hub = hub(&dir);
    let made = call(
        &hub,
        "lead",
        "create_channel",
        &json!({"name": " Launch ", "title": " Ship it ", "instructions": "Plan the launch.",
            "memberAgentIds": ["lead", "dev"], "leadAgentId": "dev"}),
    )
    .await
    .unwrap();
    let channel = &made["channel"];
    let id = channel["id"].as_str().unwrap();
    assert_eq!(
        (channel["name"].as_str(), channel["title"].as_str(), channel["instructions"].as_str()),
        (Some("Launch"), Some("Ship it"), Some("Plan the launch."))
    );
    assert_eq!(channel["leadAgentId"], json!("dev"));
    assert_eq!(channel["members"][1], json!({"agentId": "dev", "name": "dev"}));
    let cfg = hub.store.bot(id).unwrap().unwrap().config;
    assert!(cfg.is_group());
    assert_eq!(
        (cfg.members.clone(), cfg.chief_bot_id.as_deref(), cfg.created_by.as_deref(), cfg.archived),
        (vec!["lead".to_owned(), "dev".to_owned()], Some("dev"), Some("lead"), false)
    );

    // The same open channel again is the one to change, not a second one.
    let again =
        call(&hub, "lead", "create_channel", &json!({"name": "Launch", "memberAgentIds": ["dev", "lead"]})).await;
    assert!(again.unwrap_err().to_string().contains("already exists"));

    // The lead it created changes it by name, case aside.
    let changed = call(
        &hub,
        "lead",
        "update_channel",
        &json!({"channelId": "lAuNcH", "title": "Ship it today", "addMemberAgentIds": ["ops"],
            "removeMemberAgentIds": ["dev"], "archived": true}),
    )
    .await
    .unwrap();
    assert_eq!(
        changed["channel"]["members"],
        json!([{"agentId": "lead", "name": "lead"}, {"agentId": "ops", "name": "ops"}])
    );
    let cfg = hub.store.bot(id).unwrap().unwrap().config;
    assert_eq!(
        (cfg.members.clone(), cfg.title.as_deref(), cfg.archived, cfg.chief_bot_id.clone()),
        (vec!["lead".to_owned(), "ops".to_owned()], Some("Ship it today"), true, None),
        "the lead left: the channel has none"
    );
    assert_eq!(notices(&hub, "lead"), ["Created channel Launch", "Updated Launch: title, members, lead, archived"]);

    // An archived channel is out of the way: the same bots get a fresh one.
    let fresh = call(&hub, "lead", "create_channel", &json!({"name": "Again", "memberAgentIds": ["lead", "ops"]}))
        .await
        .unwrap();
    assert_ne!(fresh["channel"]["id"].as_str().unwrap(), id);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_channel_the_manager_cannot_see_is_refused_with_a_reason() {
    let dir = temp("group-tools-refused");
    let hub = hub(&dir);
    let secret: BotConfig = serde_json::from_value(json!({"id": "g", "name": "Secret", "kind": "group",
        "members": ["dev", "ops"]}))
    .unwrap();
    hub.store.save_bot(&secret).unwrap();

    let refused = |bot: &'static str, name: &'static str, args: Value| {
        let hub = hub.clone();
        async move { call(&hub, bot, name, &args).await.unwrap_err().to_string() }
    };
    // Only a bot the user lets manage the team has these tools at all.
    assert!(
        refused("dev", "update_channel", json!({"channelId": "Secret", "title": "x"}))
            .await
            .contains("may not manage channels")
    );
    assert!(
        refused("lead", "update_channel", json!({"channelId": "secret", "title": "x"}))
            .await
            .contains("can't see the channel Secret")
    );
    assert!(
        refused("lead", "update_channel", json!({"channelId": "nope", "title": "x"}))
            .await
            .contains("no channel has the id or name nope")
    );

    let made = call(&hub, "lead", "create_channel", &json!({"name": "Mine", "memberAgentIds": ["lead", "dev"]}))
        .await
        .unwrap();
    let id = made["channel"]["id"].as_str().unwrap().to_owned();
    assert!(
        refused("lead", "create_channel", json!({"name": "x", "memberAgentIds": ["ghost"]}))
            .await
            .contains("no bot has the id ghost")
    );
    assert!(
        refused("lead", "create_channel", json!({"name": "x", "memberAgentIds": ["g"]})).await.contains("is a channel")
    );
    assert!(refused("lead", "create_channel", json!({"name": "x"})).await.contains("memberAgentIds is required"));
    assert!(refused("lead", "create_channel", json!({"name": " ", "memberAgentIds": ["lead"]})).await.contains("name"));
    let too_long_title = json!({"channelId": id, "title": "a".repeat(MAX_TITLE_CHARS + 1)});
    assert!(refused("lead", "update_channel", too_long_title).await.contains("at most 120"));
    let bad_lead = json!({"channelId": id, "leadAgentId": "ops"});
    assert!(refused("lead", "update_channel", bad_lead).await.contains("must be a member"));
    let last_member = json!({"channelId": id, "removeMemberAgentIds": ["lead", "dev"]});
    assert!(refused("lead", "update_channel", last_member).await.contains("at least one bot"));
    assert!(refused("lead", "update_channel", json!({"channelId": id})).await.contains("nothing to change"));
    assert_eq!(hub.store.bot(&id).unwrap().unwrap().config.members, ["lead", "dev"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Under Ask the same card as bot management holds the change; one too long for it never reaches the user.
#[tokio::test]
async fn under_ask_a_change_waits_on_its_card() {
    let dir = temp("group-tools-ask");
    let hub = hub(&dir);
    hub.update_bot(&json!({"id": "lead", "permission": "ask"})).unwrap();
    let long =
        json!({"name": "Launch", "memberAgentIds": ["lead"], "instructions": "a".repeat(MAX_INSTRUCTIONS_CHARS)});
    let error = call(&hub, "lead", "create_channel", &long).await.unwrap_err().to_string();
    assert!(error.contains("too long for its approval card"), "{error}");
    assert_eq!(hub.store.bots().unwrap().len(), 3, "a refused change creates nothing");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_manage_tools_list_the_channel_tools() {
    let manage = crate::chat::manage::tools();
    let names: Vec<String> =
        manage.as_array().unwrap().iter().filter_map(|t| t["name"].as_str().map(str::to_owned)).collect();
    assert!(names.contains(&"create_channel".to_owned()) && names.contains(&"update_channel".to_owned()), "{names:?}");

    let channel = tools();
    let by_name = |name: &str| channel.as_array().unwrap().iter().find(|t| t["name"] == name).unwrap().clone();
    let create = by_name("create_channel");
    assert_eq!(create["inputSchema"]["required"], json!(["name", "memberAgentIds"]));
    let update = by_name("update_channel");
    assert_eq!(update["inputSchema"]["required"], json!(["channelId"]));
    for key in ["name", "title", "instructions", "leadAgentId", "archived"] {
        assert!(create.to_string().contains(&format!("\"{key}\"")), "create lacks {key}");
    }
    for key in [
        "name",
        "title",
        "instructions",
        "leadAgentId",
        "archived",
        "channelId",
        "addMemberAgentIds",
        "removeMemberAgentIds",
    ] {
        assert!(update.to_string().contains(&format!("\"{key}\"")), "update lacks {key}");
    }
    // Not yet: a channel's picture and remembered instructions stay the user's to set.
    for absent in ["picture", "keepMemories"] {
        assert!(!channel.to_string().contains(&format!("\"{absent}\"")), "{absent} is a channel tool field");
    }
}
