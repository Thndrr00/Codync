//! One Bot Framework activity: what it says, who may say it, the conversation a reply goes to, and
//! the approval action an Adaptive Card submit carries.

use super::Config;
use serde_json::Value;

/// Who wrote an activity, as Teams names them.
pub(super) struct Sender {
    pub id: String,
    pub name: String,
    pub aad: String,
    pub tenant: String,
}

pub(super) fn sender(activity: &Value) -> Sender {
    Sender {
        id: activity["from"]["id"].as_str().unwrap_or_default().to_owned(),
        name: activity["from"]["name"].as_str().unwrap_or_default().to_owned(),
        aad: activity["from"]["aadObjectId"].as_str().unwrap_or_default().to_owned(),
        tenant: tenant(activity),
    }
}

pub(super) fn tenant(activity: &Value) -> String {
    activity["channelData"]["tenant"]["id"]
        .as_str()
        .or_else(|| activity["conversation"]["tenantId"].as_str())
        .unwrap_or_default()
        .to_owned()
}

pub(super) fn conversation(activity: &Value) -> &str {
    activity["conversation"]["id"].as_str().unwrap_or_default()
}

pub(super) fn service_url(activity: &Value) -> &str {
    activity["serviceUrl"].as_str().unwrap_or_default()
}

/// A message is answered when its sender and tenant are allowed. A tenant list that is present
/// bounds every user; a user list that is present bounds every tenant.
pub(super) fn allowed(c: &Config, activity: &Value) -> bool {
    let sender = sender(activity);
    if conversation(activity).is_empty() || service_url(activity).is_empty() {
        return false;
    }
    if !c.allowed_tenants.is_empty()
        && !c.allowed_tenants.iter().any(|tenant| tenant == "*" || tenant.eq_ignore_ascii_case(&sender.tenant))
    {
        return false;
    }
    if c.allowed_users.is_empty() {
        return !c.allowed_tenants.is_empty();
    }
    c.allowed_users.iter().any(|user| {
        user.as_str() == "*"
            || user.eq_ignore_ascii_case(&sender.aad)
            || user.eq_ignore_ascii_case(&sender.id)
            || user.eq_ignore_ascii_case(&sender.name)
    })
}

/// The allowed entry that lets this sender write, for the queue's later re-check: the exact
/// `allowedUsers` value (an AAD id, a UPN, a Teams id or a display name), `*` when the list is
/// empty and the tenant allowed it, or `None` when nobody did.
pub(super) fn identity(c: &Config, activity: &Value) -> Option<String> {
    let sender = sender(activity);
    if c.allowed_users.is_empty() {
        return Some("*".to_owned());
    }
    c.allowed_users
        .iter()
        .find(|user| {
            user.as_str() == "*"
                || user.eq_ignore_ascii_case(&sender.aad)
                || user.eq_ignore_ascii_case(&sender.id)
                || user.eq_ignore_ascii_case(&sender.name)
        })
        .cloned()
}

/// The words of a message, without the bot's `<at>` mention.
pub(super) fn text(activity: &Value) -> String {
    let raw = activity["text"].as_str().unwrap_or_default();
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find("<at>") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        match after.find("</at>") {
            Some(end) => rest = &after[end + 5..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A mention of this bot in a group or channel conversation. A personal conversation needs none.
pub(super) fn mentions_bot(activity: &Value) -> bool {
    let personal = activity["conversation"]["conversationType"].as_str().is_none_or(|kind| kind == "personal");
    if personal {
        return true;
    }
    let bot = activity["recipient"]["id"].as_str().unwrap_or_default();
    activity["entities"].as_array().is_some_and(|entities| {
        entities.iter().any(|entity| {
            entity["type"] == "mention"
                && (entity["mentioned"]["id"].as_str() == Some(bot)
                    || entity["mentioned"]["name"].as_str().is_some_and(|name| name == "Codync"))
        })
    }) || activity["text"].as_str().is_some_and(|text| text.contains("<at>"))
}

/// The `codyncAction` an Adaptive Card submit carries, if this activity is one.
pub(super) fn action(activity: &Value) -> Option<&str> {
    activity["value"]["codyncAction"].as_str().filter(|action| !action.is_empty())
}

/// The names of the files that came with the message (Teams downloads nothing here).
pub(super) fn attachments(activity: &Value) -> Vec<String> {
    activity["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|attachment| attachment["name"].as_str().filter(|name| !name.trim().is_empty()).map(str::to_owned))
        .take(10)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> Config {
        Config {
            id: "c1".into(),
            app_id: "app".into(),
            tenant_id: "tenant".into(),
            app_password: "secret".into(),
            managed_identity: false,
            bot_id: "b".into(),
            allowed_users: vec!["ana@example.com".into()],
            allowed_tenants: vec!["tenant-1".into()],
            memory: String::new(),
            icon: String::new(),
            generation: "g".into(),
        }
    }

    #[test]
    fn a_personal_message_strips_the_mention_and_names_its_sender() {
        let activity = json!({
            "type": "message", "text": "<at>Codync</at> deploy please",
            "from": {"id": "29:1", "name": "Ana", "aadObjectId": "ana@example.com"},
            "conversation": {"id": "c1", "tenantId": "tenant-1", "conversationType": "personal"},
            "serviceUrl": "https://smba.example",
        });
        assert_eq!(text(&activity), "deploy please");
        assert!(mentions_bot(&activity));
        let who = sender(&activity);
        assert_eq!((who.id.as_str(), who.aad.as_str(), who.tenant.as_str()), ("29:1", "ana@example.com", "tenant-1"));
        assert!(allowed(&config(), &activity));
    }

    #[test]
    fn a_group_needs_a_mention_and_the_right_tenant() {
        let group = json!({
            "text": "hello", "from": {"id": "29:2", "aadObjectId": "beto@example.com"},
            "conversation": {"id": "c2", "tenantId": "tenant-2", "conversationType": "channel"},
            "serviceUrl": "https://smba.example",
        });
        assert!(!mentions_bot(&group));
        assert!(!allowed(&config(), &group), "another tenant");
        let mentioned = json!({
            "text": "<at>Codync</at> hello", "from": {"aadObjectId": "ana@example.com"},
            "conversation": {"id": "c2", "tenantId": "tenant-1", "conversationType": "channel"},
            "serviceUrl": "https://smba.example",
            "entities": [{"type": "mention", "mentioned": {"id": "28:bot"}}],
            "recipient": {"id": "28:bot"},
        });
        assert!(mentions_bot(&mentioned));
        assert!(allowed(&config(), &mentioned));
    }

    #[test]
    fn an_approval_is_the_submit_action() {
        assert_eq!(action(&json!({"value": {"codyncAction": "p|e1|yes|abcdef"}})), Some("p|e1|yes|abcdef"));
        assert_eq!(action(&json!({"value": {"codyncAction": ""}})), None);
        assert_eq!(action(&json!({"text": "hi"})), None);
        assert_eq!(attachments(&json!({"attachments": [{"name": "a.pdf"}]})), ["a.pdf"]);
    }

    #[test]
    fn the_identity_kept_for_the_queue_is_the_allowed_entry() {
        let activity = json!({"from": {"id": "29:1", "name": "Ana", "aadObjectId": "ana@example.com"}});
        assert_eq!(identity(&config(), &activity).as_deref(), Some("ana@example.com"));
        let mut open = config();
        open.allowed_users.clear();
        assert_eq!(identity(&open, &activity).as_deref(), Some("*"));
        let mut other = config();
        other.allowed_users = vec!["beto@example.com".into()];
        assert_eq!(identity(&other, &activity), None);
    }
}
