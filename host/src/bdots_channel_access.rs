#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from mercury-agent's `utils/config.ts`: channel access lists, pairing codes,
//! provider configuration and cloud normalization. Composes helpers already ported into this host.
//! Only `botInsights` calls this.

use serde_json::{Map, Value, json};

/// Channels whose access is stored in config (Telegram, Signal, Discord, Slack).
pub const CHANNELS: [&str; 4] = ["telegram", "signal", "discord", "slack"];

/// Roles a pending request may be approved into.
pub const ROLES: [&str; 2] = ["admin", "member"];

/// Creator filled in for an older config that had only an owner (ensureCreatorField).
pub const DEFAULT_CREATOR: &str = "Cosmic Stack";

/// Current cloud API URL (MERCURY_CLOUD_API_URL).
pub const MERCURY_CLOUD_API_URL: &str = "https://api.mercury.dev";

/// Legacy seed API URL, replaced on load (LEGACY_SEED_API_URL).
pub const LEGACY_SEED_API_URL: &str = "https://api.atomicchat.ai";

/// Most users kept in one channel's access list.
pub const MAX_ACCESS_USERS: usize = 1_000;

/// Whether a pairing code is a short uppercase alphanumeric token.
fn is_pairing_code(code: &str) -> bool {
    (4..=12).contains(&code.len()) && code.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

/// Access object of one channel, or an empty object.
fn channel_of(config: &Value, channel: &str) -> Value {
    config["channels"][channel]
        .as_object()
        .cloned()
        .map(Value::Object)
        .unwrap_or_else(|| json!({"admins":[],"members":[],"pending":[]}))
}

/// Admin list of a channel (getTelegramAdmins / getSignalAdmins).
pub fn admins(config: &Value, channel: &str) -> Vec<Value> {
    channel_of(config, channel)["admins"].as_array().cloned().unwrap_or_default()
}

/// Member list of a channel (getTelegramApprovedUsers).
pub fn members(config: &Value, channel: &str) -> Vec<Value> {
    channel_of(config, channel)["members"].as_array().cloned().unwrap_or_default()
}

/// Pending requests of a channel (getTelegramPendingRequests).
pub fn pending(config: &Value, channel: &str) -> Vec<Value> {
    channel_of(config, channel)["pending"].as_array().cloned().unwrap_or_default()
}

/// Find an entry in a list by a field value.
fn find_in(list: &[Value], key: &str, value: &str) -> Option<Value> {
    list.iter().find(|entry| entry[key].as_str() == Some(value)).cloned()
}

/// Find an approved user by id or phone (findTelegramApprovedUser).
pub fn find_user(config: &Value, channel: &str, key: &str, value: &str) -> Value {
    let list = admins(config, channel);
    let list = if list.is_empty() { members(config, channel) } else { list };
    find_in(&list, key, value).or_else(|| find_in(&members(config, channel), key, value)).unwrap_or(Value::Null)
}

/// Find a pending request by id or phone (findTelegramPendingRequest).
pub fn find_pending(config: &Value, channel: &str, key: &str, value: &str) -> Value {
    find_in(&pending(config, channel), key, value).unwrap_or(Value::Null)
}

/// Find a pending request by pairing code (findTelegramPendingRequestByPairingCode).
pub fn find_pending_by_code(config: &Value, channel: &str, code: &str) -> Value {
    find_in(&pending(config, channel), "pairingCode", code).unwrap_or(Value::Null)
}

/// Whether a channel has at least one admin (hasTelegramAdmins).
pub fn has_admins(config: &Value, channel: &str) -> bool {
    !admins(config, channel).is_empty()
}

/// Human summary of a channel's access (getTelegramAccessSummary).
pub fn access_summary(config: &Value, channel: &str) -> String {
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    format!(
        "{}, {}, {} pending",
        plural(admins(config, channel).len(), "admin"),
        plural(members(config, channel).len(), "member"),
        pending(config, channel).len()
    )
}

/// Add or refresh a pending request (addTelegramPendingRequest).
pub fn add_pending(config: &Value, channel: &str, key: &str, request: &Value, now: &str) -> Value {
    let mut out = config.clone();
    let mut list = pending(&out, channel);
    let value = request[key].as_str().unwrap_or("");
    if let Some(existing) = list.iter_mut().find(|entry| entry[key].as_str() == Some(value)) {
        for field in ["username", "firstName", "name", "pairingCode", "chatId"] {
            if let Some(replacement) = request.get(field).filter(|value| !value.is_null()) {
                existing[field] = replacement.clone();
            }
        }
    } else {
        let mut entry = request.clone();
        if !entry.get("requestedAt").is_some_and(Value::is_string) {
            entry["requestedAt"] = Value::String(now.to_owned());
        }
        list.push(entry);
    }
    out["channels"][channel]["pending"] = Value::Array(list);
    out
}

/// Approve a pending request into a role (approveTelegramPendingRequest).
pub fn approve_pending(config: &Value, channel: &str, key: &str, value: &str, role: &str, now: &str) -> Value {
    let Some(request) = find_pending(config, channel, key, value).as_object().cloned() else {
        return config.clone();
    };
    let mut out = config.clone();
    out["channels"][channel]["pending"] =
        Value::Array(pending(&out, channel).into_iter().filter(|entry| entry[key].as_str() != Some(value)).collect());
    out["channels"][channel]["admins"] =
        Value::Array(admins(&out, channel).into_iter().filter(|entry| entry[key].as_str() != Some(value)).collect());
    out["channels"][channel]["members"] =
        Value::Array(members(&out, channel).into_iter().filter(|entry| entry[key].as_str() != Some(value)).collect());
    let mut user = request;
    user["approvedAt"] = Value::String(now.to_owned());
    let target = if role == "admin" { "admins" } else { "members" };
    let mut list = channel_of(&out, channel)[target].as_array().cloned().unwrap_or_default();
    list.push(Value::Object(user));
    out["channels"][channel][target] = Value::Array(list);
    out
}

/// Reject and drop a pending request (rejectTelegramPendingRequest).
pub fn reject_pending(config: &Value, channel: &str, key: &str, value: &str) -> Value {
    let mut out = config.clone();
    out["channels"][channel]["pending"] =
        Value::Array(pending(&out, channel).into_iter().filter(|entry| entry[key].as_str() != Some(value)).collect());
    out
}

/// Promote a member to admin (promoteTelegramUserToAdmin).
pub fn promote_to_admin(config: &Value, channel: &str, key: &str, value: &str) -> Value {
    let Some(user) = find_in(&members(config, channel), key, value) else {
        return config.clone();
    };
    let mut out = reject_pending(config, channel, key, value);
    out["channels"][channel]["members"] =
        Value::Array(members(&out, channel).into_iter().filter(|entry| entry[key].as_str() != Some(value)).collect());
    let mut admins_list = admins(&out, channel);
    admins_list.push(user);
    out["channels"][channel]["admins"] = Value::Array(admins_list);
    out
}

/// Demote an admin to member (demoteTelegramAdmin).
pub fn demote_admin(config: &Value, channel: &str, key: &str, value: &str) -> Value {
    let Some(user) = find_in(&admins(config, channel), key, value) else {
        return config.clone();
    };
    let mut out = config.clone();
    out["channels"][channel]["admins"] =
        Value::Array(admins(&out, channel).into_iter().filter(|entry| entry[key].as_str() != Some(value)).collect());
    let mut list = members(&out, channel);
    list.push(user);
    out["channels"][channel]["members"] = Value::Array(list);
    out
}

/// Remove a user from both roles (removeTelegramUser).
pub fn remove_user(config: &Value, channel: &str, key: &str, value: &str) -> Value {
    let mut out = config.clone();
    for role in ROLES {
        let plural = format!("{role}s");
        out["channels"][channel][&plural] = Value::Array(
            channel_of(&out, channel)[&plural]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| entry[key].as_str() != Some(value))
                .collect(),
        );
    }
    out
}

/// Clear all access and pairing fields (clearTelegramAccess).
pub fn clear_access(config: &Value, channel: &str) -> Value {
    let mut out = config.clone();
    for field in ["admins", "members", "pending"] {
        out["channels"][channel][field] = Value::Array(Vec::new());
    }
    for field in ["pairedUserId", "pairedChatId", "pairedUsername", "pairedPhoneNumber"] {
        if let Some(map) = out["channels"][channel].as_object_mut() {
            map.remove(field);
        }
    }
    out
}

/// Clear only pairing fields (clearTelegramPairing).
pub fn clear_pairing(config: &Value, channel: &str) -> Value {
    let mut out = config.clone();
    for field in ["pairedUserId", "pairedChatId", "pairedUsername", "pairedPhoneNumber"] {
        if let Some(map) = out["channels"][channel].as_object_mut() {
            map.remove(field);
        }
    }
    out
}

/// Whether a provider has everything it needs (isProviderConfigured).
pub fn is_provider_configured(provider: &Value) -> bool {
    if provider["enabled"] != true {
        return false;
    }
    let name = provider["name"].as_str().unwrap_or("");
    let has = |field: &str| provider[field].as_str().is_some_and(|value| !value.is_empty());
    match name {
        "mercuryCloud" | "chatgptWeb" => has("model"),
        "ollamaLocal" | "atomicChat" | "openaiCompat" | "litellm" | "lmStudio" => has("baseUrl") && has("model"),
        "ollamaCloud" => has("apiKey") && has("baseUrl"),
        "atlascloud" | "runcrate" | "cloudgpu" => has("apiKey") && has("baseUrl") && has("model"),
        _ => has("apiKey"),
    }
}

/// Names of configured providers (getActiveProviders).
pub fn active_providers(config: &Value) -> Vec<String> {
    config["providers"]
        .as_object()
        .map(|map| {
            map.values()
                .filter(|provider| is_provider_configured(provider))
                .filter_map(|provider| provider["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether the owner has completed setup (isSetupComplete).
pub fn is_setup_complete(config: &Value) -> bool {
    config["identity"]["owner"].as_str().is_some_and(|owner| !owner.is_empty())
}

/// Fill the creator field of an older config (ensureCreatorField).
pub fn ensure_creator(config: &Value) -> Value {
    let mut out = config.clone();
    if out["identity"]["creator"].as_str().unwrap_or("").is_empty() && is_setup_complete(&out) {
        out["identity"]["creator"] = Value::String(DEFAULT_CREATOR.to_owned());
    }
    out
}

/// Normalize cloud URLs and derive the OpenAI-compatible provider (normalizeCloudConfig).
pub fn normalize_cloud_config(config: &Value) -> Value {
    let mut out = config.clone();
    let api = out["cloud"]["apiUrl"].as_str().unwrap_or("");
    if api.is_empty() || api == LEGACY_SEED_API_URL {
        out["cloud"]["apiUrl"] = Value::String(MERCURY_CLOUD_API_URL.to_owned());
    }
    let api = out["cloud"]["apiUrl"].as_str().unwrap_or(MERCURY_CLOUD_API_URL);
    let ws = out["cloud"]["wsUrl"].as_str().unwrap_or("");
    if ws.is_empty() {
        let derived = api.replacen("https://", "wss://", 1).replacen("http://", "ws://", 1);
        out["cloud"]["wsUrl"] = Value::String(format!("{derived}/ws"));
    }
    out["providers"]["mercuryCloud"]["baseUrl"] = out["cloud"]["apiUrl"].clone();
    if let Some(key) = out["cloud"]["accessKey"].as_str().filter(|value| !value.is_empty()) {
        out["providers"]["mercuryCloud"]["apiKey"] = Value::String(key.to_owned());
    }
    out
}

/// Move a legacy `allowedUsers` list into members (migrateLegacyAccess).
pub fn migrate_legacy_access(config: &Value, channel: &str) -> Value {
    let mut out = config.clone();
    let legacy = channel_of(&out, channel)["allowedUsers"].as_array().cloned().unwrap_or_default();
    if legacy.is_empty() {
        return out;
    }
    let mut list = members(&out, channel);
    for user in legacy {
        if !list.iter().any(|entry| entry["userId"] == user["userId"]) {
            list.push(user);
        }
    }
    if let Some(map) = out["channels"][channel].as_object_mut() {
        map.remove("allowedUsers");
    }
    out["channels"][channel]["members"] = Value::Array(list);
    out
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(channels: usize, admins: usize, pending: usize, configured: bool) -> Vec<(&'static str, String)> {
    vec![
        ("channels", count_label(channels)),
        ("admins", count_label(admins)),
        ("pending", count_label(pending)),
        ("configured", if configured { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_lists() {
        assert!(is_pairing_code("A1B2C3"));
        assert!(!is_pairing_code("abc"));
        let config = json!({"channels":{"telegram":{
            "admins":[{"userId":"1"}],"members":[{"userId":"2"}],
            "pending":[{"userId":"3","pairingCode":"A1B2C3"}],
        }}});
        assert!(has_admins(&config, "telegram"));
        assert_eq!(admins(&config, "telegram").len(), 1);
        assert_eq!(members(&config, "telegram").len(), 1);
        assert_eq!(pending(&config, "telegram").len(), 1);
        assert_eq!(find_user(&config, "telegram", "userId", "2")["userId"], "2");
        assert_eq!(find_pending_by_code(&config, "telegram", "A1B2C3")["userId"], "3");
        assert!(access_summary(&config, "telegram").contains("1 admin"));
    }

    #[test]
    fn pending_lifecycle() {
        let config = json!({"channels":{"telegram":{"admins":[],"members":[],"pending":[]}}});
        let added = add_pending(
            &config,
            "telegram",
            "userId",
            &json!({"userId":"5","chatId":9,"pairingCode":"ZZ11"}),
            "2026-01-01",
        );
        assert_eq!(pending(&added, "telegram").len(), 1);
        assert_eq!(pending(&added, "telegram")[0]["requestedAt"], "2026-01-01");
        let approved = approve_pending(&added, "telegram", "userId", "5", "admin", "2026-01-02");
        assert!(pending(&approved, "telegram").is_empty());
        assert_eq!(admins(&approved, "telegram")[0]["approvedAt"], "2026-01-02");
        let demoted = demote_admin(&approved, "telegram", "userId", "5");
        assert!(admins(&demoted, "telegram").is_empty());
        assert_eq!(members(&demoted, "telegram").len(), 1);
        let promoted = promote_to_admin(&demoted, "telegram", "userId", "5");
        assert_eq!(admins(&promoted, "telegram").len(), 1);
        let rejected = reject_pending(&added, "telegram", "userId", "5");
        assert!(pending(&rejected, "telegram").is_empty());
        let removed = remove_user(&promoted, "telegram", "userId", "5");
        assert!(admins(&removed, "telegram").is_empty() && members(&removed, "telegram").is_empty());
        assert_eq!(clear_pairing(&removed, "telegram"), removed);
    }

    #[test]
    fn clearing_and_migration() {
        let config = json!({"channels":{"signal":{
            "admins":[],"members":[],"pending":[],"pairedUserId":"1","allowedUsers":[{"userId":"7"}],
        }}});
        let migrated = migrate_legacy_access(&config, "signal");
        assert_eq!(members(&migrated, "signal")[0]["userId"], "7");
        assert!(migrated["channels"]["signal"].get("allowedUsers").is_none());
        let cleared = clear_access(&migrated, "signal");
        assert!(members(&cleared, "signal").is_empty());
        assert!(cleared["channels"]["signal"].get("pairedUserId").is_none());
    }

    #[test]
    fn providers_and_cloud() {
        assert!(is_provider_configured(&json!({"name":"mercuryCloud","enabled":true,"model":"m"})));
        assert!(!is_provider_configured(&json!({"name":"mercuryCloud","enabled":false,"model":"m"})));
        assert!(is_provider_configured(&json!({"name":"ollamaLocal","enabled":true,"baseUrl":"http://x","model":"m"})));
        let config = json!({"providers":{
            "a":{"name":"openai","enabled":true,"apiKey":"k"},
            "b":{"name":"mercuryCloud","enabled":false,"model":"m"},
        }});
        assert_eq!(active_providers(&config), vec!["openai"]);
        assert!(!is_setup_complete(&config));
        let created = ensure_creator(&json!({"identity":{"owner":"Ana"}}));
        assert_eq!(created["identity"]["creator"], DEFAULT_CREATOR);
        let cloud = normalize_cloud_config(
            &json!({"cloud":{"apiUrl":LEGACY_SEED_API_URL,"wsUrl":"","accessKey":"k"},"providers":{}}),
        );
        assert_eq!(cloud["cloud"]["apiUrl"], MERCURY_CLOUD_API_URL);
        assert_eq!(cloud["providers"]["mercuryCloud"]["apiKey"], "k");
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(4, 1, 2, true).len(), 4);
    }
}
