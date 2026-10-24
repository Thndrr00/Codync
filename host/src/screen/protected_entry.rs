//! Manual protected entry: no secret value ever crosses an agent or host API payload.
use super::Screen;
use crate::{LockExt, hub::Hub};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

const SLOT: &str = "computer.protected_entry";
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProtectedEntry {
    id: String,
    bot_id: String,
    site: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Begin {
    bot_id: String,
    site: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Finish {
    id: String,
    #[serde(default)]
    keep_session: bool,
}

impl Screen {
    /// Restore before bot actors or HTTP handlers start. An unreadable record stops startup.
    pub(crate) fn restore_protected_entry(&self, store: &crate::store::Store) -> Result<()> {
        let Some(saved) = store.kv_read(SLOT)? else { return Ok(()) };
        let entry: ProtectedEntry = serde_json::from_str(&saved).context(
            "Protected entry state is unreadable; computer tools remain paused until this record is repaired.",
        )?;
        *self.protected.locked() = Some(entry);
        self.takeover(true);
        Ok(())
    }
    pub fn protected_state(&self) -> Value {
        json!(*self.protected.locked())
    }
    pub fn entry_protected(&self) -> bool {
        self.protected.locked().is_some()
    }
}

pub async fn call(hub: &Arc<Hub>, method: &str, body: Value) -> Result<Value> {
    #[cfg(windows)]
    let _operation = hub.screen.protected_operation.lock().await;
    match method {
        "beginProtectedEntry" => {
            let request: Begin = serde_json::from_value(body)?;
            let row = hub
                .store
                .bot(&request.bot_id)?
                .filter(|row| !row.deleted && !row.config.is_group())
                .context("unknown bot")?;
            ensure!(row.config.computer && row.config.computer_permissions.browser, "Desktop permission is required.");
            let site = reqwest::Url::parse(&request.site).context("Enter an HTTPS website URL.")?;
            ensure!(
                site.scheme() == "https"
                    && site.host_str().is_some()
                    && site.username().is_empty()
                    && site.password().is_none(),
                "Enter an HTTPS website without credentials."
            );
            let entry = ProtectedEntry {
                id: uuid::Uuid::new_v4().to_string(),
                bot_id: request.bot_id,
                site: site.origin().ascii_serialization(),
            };
            {
                let mut current = hub.screen.protected.locked();
                if current.is_some() {
                    bail!("Protected entry is already active. Finish it before starting another.");
                }
                hub.store.kv_set(SLOT, &serde_json::to_string(&entry)?)?;
                *current = Some(entry.clone());
            }
            hub.screen.takeover(true);
            #[cfg(windows)]
            hub.screen.driver.cancel().await?;
            for row in hub.store.bots()? {
                if !row.deleted {
                    hub.computer.revoke_all(&row.config.id);
                    // Stop current harness work too; its independent permissions are not a sandbox.
                    if let Err(error) = hub.send_cmd(&row.config.id, crate::agent::bot::Cmd::Stop) {
                        tracing::warn!(%error, "couldn't stop bot for protected entry");
                    }
                }
            }
            Ok(
                json!({"entry":entry,"instruction":"Enter the password or code yourself on the real host. Never paste it into chat. Finish here afterward; bots must capture a fresh snapshot."}),
            )
        }
        "finishProtectedEntry" => {
            let request: Finish = serde_json::from_value(body)?;
            #[cfg(windows)]
            let browser_session =
                super::protected_browser::release(&hub.screen, &request.id, request.keep_session).await?;
            #[cfg(not(windows))]
            let browser_session = {
                ensure!(!request.keep_session, "Private browser handback is only supported on Windows");
                "none"
            };
            {
                let mut current = hub.screen.protected.locked();
                ensure!(
                    current.as_ref().is_some_and(|entry| entry.id == request.id),
                    "Protected entry changed. Refresh before finishing."
                );
                hub.store.kv_delete(SLOT)?;
                *current = None;
            }
            hub.screen.takeover(false);
            #[cfg(windows)]
            super::protected_browser::handed_back(&hub.screen, &request.id).await;
            Ok(json!({"browserSession":browser_session}))
        }
        _ => bail!("unknown protected entry method"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_restarted_host_keeps_manual_entry_until_explicit_finish() {
        let store = crate::store::Store::open(std::path::Path::new(":memory:")).unwrap();
        let entry = ProtectedEntry { id: "entry".into(), bot_id: "bot".into(), site: "https://example.com".into() };
        store.kv_set(SLOT, &serde_json::to_string(&entry).unwrap()).unwrap();
        let (events, _) = tokio::sync::broadcast::channel(8);
        let screen = Screen::new(Some(true), events);
        screen.restore_protected_entry(&store).unwrap();
        assert!(screen.entry_protected());
        assert_eq!(screen.protected_state()["id"], "entry");
        screen.takeover(false);
        assert!(screen.entry_protected());
        assert_eq!(screen.state()["userControl"], true);
    }
    #[test]
    fn unreadable_manual_entry_state_refuses_startup_instead_of_resuming() {
        let store = crate::store::Store::open(std::path::Path::new(":memory:")).unwrap();
        store.kv_set(SLOT, "invalid").unwrap();
        let (events, _) = tokio::sync::broadcast::channel(8);
        let screen = Screen::new(Some(true), events);
        assert!(screen.restore_protected_entry(&store).is_err());
    }
}
