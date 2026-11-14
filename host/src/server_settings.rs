//! Server-wide settings every client reads: how the agents' updates between steps read ("Personality"),
//! whether Claude agents load the person's own Claude Code setup, and who may control this computer over
//! remote desktop (and whether each session is allowed at the computer first).
//!
//! Ported from OpenBot's server personality, "Use my Claude Code setup" and remote desktop policy. The
//! values live in the `kv` table as one JSON document; a missing, unreadable or newer document reads as
//! the defaults: Natural updates (OpenBot's default), Claude's setup off, and every paired device
//! allowed without a question.

use crate::store::Store;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

const KEY: &str = "server.settings";

/// How an agent's updates between steps read in every client.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStyle {
    /// A short progress note only when a finding changes the plan or before a long step, then a verdict
    /// first answer. OpenBot's "Natural" style.
    #[default]
    Natural,
    /// The agent's own streamed text as it comes, with no extra guidance. OpenBot's "Classic" style.
    Classic,
}

impl ResponseStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::Classic => "classic",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "natural" => Some(Self::Natural),
            "classic" => Some(Self::Classic),
            _ => None,
        }
    }
}

/// Who may control this computer through remote desktop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteAccess {
    /// Every device in the account, which is every paired phone today.
    #[default]
    Everyone,
    /// Owners and admins. Codync has one owner and no member roles yet, so this reads as `everyone`.
    Admins,
    /// The owner only: this computer's own screen, never a paired phone.
    Owner,
}

impl RemoteAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Everyone => "everyone",
            Self::Admins => "admins",
            Self::Owner => "owner",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "everyone" => Some(Self::Everyone),
            "admins" => Some(Self::Admins),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
}

/// Who a caller is to this computer's account, for [`allows_remote`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteRole {
    /// The person at this computer.
    Owner,
    /// Another computer or device the owner paired.
    Device,
}

/// Remote desktop policy: who may start a session, and whether the person at this computer allows each one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteControl {
    #[serde(default)]
    pub access: RemoteAccess,
    /// The person at this computer allows or denies each session; one nobody allows in 30 s does not start.
    #[serde(default)]
    pub ask_on_host: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub response_style: ResponseStyle,
    /// Claude agents load the person's `~/.claude` (CLAUDE.md, skills, agents, hooks, plugins). Off by default.
    #[serde(default)]
    pub claude_user_setup: bool,
    #[serde(default)]
    pub remote_control: RemoteControl,
}

impl Settings {
    /// The saved settings, with a missing or unreadable document read as the defaults.
    pub fn load(store: &Store) -> Self {
        store
            .kv_get(KEY)
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .map_or_else(Self::default, |value| Self::decode(&value))
    }

    /// Every field on its own, so one unknown value from a newer app doesn't reset the others.
    pub fn decode(value: &Value) -> Self {
        let mut settings = Self::default();
        if let Some(style) = value["responseStyle"].as_str().and_then(ResponseStyle::parse) {
            settings.response_style = style;
        }
        if let Some(on) = value["claudeUserSetup"].as_bool() {
            settings.claude_user_setup = on;
        }
        let remote = &value["remoteControl"];
        if let Some(access) = remote["access"].as_str().and_then(RemoteAccess::parse) {
            settings.remote_control.access = access;
        }
        if let Some(ask) = remote["askOnHost"].as_bool() {
            settings.remote_control.ask_on_host = ask;
        }
        settings
    }

    /// Applies the fields `body` names, refusing a value the host doesn't know.
    pub fn apply(&mut self, body: &Value) -> Result<()> {
        if let Some(value) = body.get("responseStyle").filter(|v| !v.is_null()) {
            let raw = value.as_str().unwrap_or_default();
            self.response_style = ResponseStyle::parse(raw)
                .ok_or_else(|| anyhow::anyhow!("Unknown response style \"{raw}\": use natural or classic."))?;
        }
        if let Some(value) = body.get("claudeUserSetup").filter(|v| !v.is_null()) {
            self.claude_user_setup =
                value.as_bool().ok_or_else(|| anyhow::anyhow!("claudeUserSetup must be true or false."))?;
        }
        if let Some(remote) = body.get("remoteControl").filter(|v| !v.is_null()) {
            let remote = remote.as_object().ok_or_else(|| anyhow::anyhow!("remoteControl must be an object."))?;
            self.apply_remote(remote)?;
        }
        Ok(())
    }

    fn apply_remote(&mut self, remote: &Map<String, Value>) -> Result<()> {
        if let Some(value) = remote.get("access").filter(|v| !v.is_null()) {
            let raw = value.as_str().unwrap_or_default();
            self.remote_control.access = RemoteAccess::parse(raw)
                .ok_or_else(|| anyhow::anyhow!("Unknown remote access \"{raw}\": use everyone, admins or owner."))?;
        }
        if let Some(value) = remote.get("askOnHost").filter(|v| !v.is_null()) {
            self.remote_control.ask_on_host =
                value.as_bool().ok_or_else(|| anyhow::anyhow!("askOnHost must be true or false."))?;
        }
        Ok(())
    }

    pub fn save(&self, store: &Store) -> Result<()> {
        store.kv_set(KEY, &serde_json::to_string(&self.to_json())?)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "responseStyle": self.response_style.as_str(),
            "claudeUserSetup": self.claude_user_setup,
            "remoteControl": {
                "access": self.remote_control.access.as_str(),
                "askOnHost": self.remote_control.ask_on_host,
            },
        })
    }

    /// The instructions the "Natural" style adds to a bot's prompt; empty for "Classic".
    pub fn style_section(&self) -> &'static str {
        match self.response_style {
            ResponseStyle::Classic => "",
            ResponseStyle::Natural => {
                "Updates between steps, the Natural style: write one short line when what you find changes \
the plan, and one before a long step: what you learned and what comes next. Don't narrate every tool \
call. When you use a tool first, write one line saying what you understood and what you check first. \
In your final answer, lead with the verdict, explain a technical term in a few words the first time you \
use it, and say what you could not check and whether a read-only task still wrote files."
            }
        }
    }
}

/// Whether a caller in `role` may start a remote desktop session under `access`.
pub fn allows_remote(access: RemoteAccess, role: RemoteRole) -> bool {
    match access {
        // A paired device is the same person as the owner; member roles don't exist yet, so "owners and
        // admins" allows them exactly as "everyone" does.
        RemoteAccess::Everyone | RemoteAccess::Admins => true,
        RemoteAccess::Owner => role == RemoteRole::Owner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open(std::path::Path::new(":memory:")).unwrap()
    }

    #[test]
    fn defaults_are_natural_updates_without_the_claude_setup_and_without_a_question() {
        let settings = Settings::default();
        assert_eq!(settings.response_style, ResponseStyle::Natural);
        assert!(!settings.claude_user_setup, "the person's Claude Code setup stays off until they turn it on");
        assert_eq!(settings.remote_control.access, RemoteAccess::Everyone);
        assert!(!settings.remote_control.ask_on_host);
        assert!(Settings::load(&store()) == Settings::default(), "no document reads as the defaults");
    }

    #[test]
    fn a_saved_document_round_trips_and_a_broken_one_reads_as_the_defaults() {
        let store = store();
        let mut settings =
            Settings { response_style: ResponseStyle::Classic, claude_user_setup: true, ..Settings::default() };
        settings.remote_control = RemoteControl { access: RemoteAccess::Owner, ask_on_host: true };
        settings.save(&store).unwrap();
        assert!(Settings::load(&store) == settings);
        store.kv_set(KEY, "not json").unwrap();
        assert!(Settings::load(&store) == Settings::default());
        store.kv_set(KEY, &json!({"responseStyle": "future"}).to_string()).unwrap();
        assert_eq!(Settings::load(&store).response_style, ResponseStyle::Natural);
    }

    #[test]
    fn applying_a_patch_changes_only_what_it_names_and_refuses_unknown_values() {
        let mut settings = Settings::default();
        settings.apply(&json!({"claudeUserSetup": true})).unwrap();
        assert!(settings.claude_user_setup && settings.response_style == ResponseStyle::Natural);
        settings.apply(&json!({"remoteControl": {"askOnHost": true}})).unwrap();
        assert!(settings.remote_control.ask_on_host);
        assert_eq!(settings.remote_control.access, RemoteAccess::Everyone);
        assert!(settings.apply(&json!({"responseStyle": "poetic"})).is_err());
        assert!(settings.apply(&json!({"remoteControl": {"access": "friends"}})).is_err());
        assert_eq!(settings.response_style, ResponseStyle::Natural, "a refused patch changes nothing");
    }

    #[test]
    fn remote_access_gates_this_computer_only_and_admins_read_as_everyone_until_roles_exist() {
        assert!(allows_remote(RemoteAccess::Everyone, RemoteRole::Device));
        assert!(allows_remote(RemoteAccess::Admins, RemoteRole::Device));
        assert!(!allows_remote(RemoteAccess::Owner, RemoteRole::Device));
        assert!(allows_remote(RemoteAccess::Owner, RemoteRole::Owner));
    }

    #[test]
    fn natural_updates_carry_guidance_and_classic_carries_none() {
        let mut settings = Settings::default();
        assert!(settings.style_section().contains("one short line"));
        settings.response_style = ResponseStyle::Classic;
        assert!(settings.style_section().is_empty());
    }
}
