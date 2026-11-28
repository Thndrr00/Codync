#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from rakazo v0.14.0's connection context
//! (`packages/adapters/src/connection-context.ts`, `packages/adapters/src/integrations/connection-context.ts`
//! in v0.14.0: `selectRunConnections`, `connectedContext`): connection states, recovery of
//! pending/error Composio rows, `providerRef` handling, and channels (`connectorId`s).
//! No I/O; deterministic. Composes helpers already ported in `crate::bdots_connstate`.

use std::collections::BTreeSet;

use serde_json::Value;

/// Connector id whose provider listing drives live recovery.
pub const CONNECTOR_COMPOSIO: &str = "composio";

/// Row status: live and authorized.
pub const STATUS_CONNECTED: &str = "connected";

/// Row status: sign-in in flight; recoverable when the provider is live.
pub const STATUS_PENDING: &str = "pending";

/// Row status: last sync failed; recoverable when the provider is live.
pub const STATUS_ERROR: &str = "error";

/// Row status: revoked; never recovered, never revived with a dead providerRef.
pub const STATUS_REVOKED: &str = "revoked";

/// Connection status of a row (`status`), empty when missing.
pub fn status_of(row: &Value) -> &str {
    row.get("status").and_then(Value::as_str).unwrap_or("")
}

/// Channel (`connectorId`) of a row, empty when missing.
pub fn connector_of(row: &Value) -> &str {
    row.get("connectorId").and_then(Value::as_str).unwrap_or("")
}

/// Provider of a row, empty when missing.
fn provider_of(row: &Value) -> &str {
    row.get("provider").and_then(Value::as_str).unwrap_or("")
}

/// Provider key used for matching: trimmed and lowercased.
pub fn provider_key(provider: &str) -> String {
    provider.trim().to_lowercase()
}

/// Unique key for a connection: `connectorId:provider-key` (provider trimmed + lowercased).
pub fn connection_key(connector_id: &str, provider: &str) -> String {
    format!("{connector_id}:{}", provider_key(provider))
}

/// Keys of every `connected` row, for sibling checks.
pub fn connected_keys(rows: &[Value]) -> BTreeSet<String> {
    rows.iter()
        .filter(|row| status_of(row) == STATUS_CONNECTED)
        .map(|row| connection_key(connector_of(row), provider_of(row)))
        .collect()
}

/// Live provider keys from a Composio listing: trimmed, lowercased, empty dropped.
fn live_providers(connected_composio_providers: &[String]) -> BTreeSet<String> {
    connected_composio_providers
        .iter()
        .map(|provider| provider_key(provider))
        .filter(|provider| !provider.is_empty())
        .collect()
}

/// Whether a non-live row is recoverable (rakazo `selectRunConnections` recovery branch):
/// Composio `pending`/`error`, provider currently live, no connected sibling sharing the slug.
/// `revoked` is never recoverable; a sibling's connected slug blocks reviving a dead providerRef.
pub fn is_recoverable(row: &Value, live_providers: &BTreeSet<String>, connected_keys: &BTreeSet<String>) -> bool {
    if connector_of(row) != CONNECTOR_COMPOSIO {
        return false;
    }
    let status = status_of(row);
    if status != STATUS_PENDING && status != STATUS_ERROR {
        return false;
    }
    let key = provider_key(provider_of(row));
    if !live_providers.contains(&key) {
        return false;
    }
    !connected_keys.contains(&connection_key(CONNECTOR_COMPOSIO, provider_of(row)))
}

/// Connections a run may use (rakazo `selectRunConnections`), original order preserved:
/// keep `connected`; drop `revoked`; recover `pending`/`error` Composio rows only when the
/// provider is live and no connected row shares the slug.
pub fn select_run_connections(rows: &[Value], connected_composio_providers: &[String]) -> Vec<Value> {
    let live = live_providers(connected_composio_providers);
    let keys = connected_keys(rows);
    rows.iter()
        .filter(|row| {
            let status = status_of(row);
            if status == STATUS_CONNECTED {
                return true;
            }
            if status == STATUS_REVOKED {
                return false;
            }
            is_recoverable(row, &live, &keys)
        })
        .cloned()
        .collect()
}

/// `providerRef` (`providerRef ?? undefined`): None when missing or null; empty string kept.
pub fn provider_ref(row: &Value) -> Option<String> {
    row.get("providerRef").and_then(Value::as_str).map(str::to_owned)
}

/// `connectedConnections` entries (rakazo `connectedContext`): id, connectorId, externalId
/// (raw provider), displayName, and providerRef only when set.
pub fn connected_connections(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            let mut entry = serde_json::Map::new();
            entry.insert("id".to_string(), Value::String(provider_id(row)));
            entry.insert("connectorId".to_string(), Value::String(connector_of(row).to_string()));
            entry.insert("externalId".to_string(), Value::String(provider_of(row).to_string()));
            entry.insert("displayName".to_string(), Value::String(display_name(row)));
            if let Some(provider_ref) = provider_ref(row) {
                entry.insert("providerRef".to_string(), Value::String(provider_ref));
            }
            Value::Object(entry)
        })
        .collect()
}

/// Row id (`id`), empty when missing.
fn provider_id(row: &Value) -> String {
    row.get("id").and_then(Value::as_str).unwrap_or("").to_string()
}

/// Row display name (`displayName`), empty when missing.
fn display_name(row: &Value) -> String {
    row.get("displayName").and_then(Value::as_str).unwrap_or("").to_string()
}

/// `connectedProviders` (rakazo `connectedContext`): raw provider of Composio rows, order kept.
pub fn connected_providers(rows: &[Value]) -> Vec<String> {
    rows.iter().filter(|row| connector_of(row) == CONNECTOR_COMPOSIO).map(|row| provider_of(row).to_string()).collect()
}

/// Full `connectedContext` (rakazo `connectedContext`): connectedConnections + connectedProviders.
pub fn connected_context(rows: &[Value]) -> Value {
    serde_json::json!({
        "connectedConnections": connected_connections(rows),
        "connectedProviders": connected_providers(rows),
    })
}

/// Distinct, sorted channels (`connectorId`s) holding a live connection; empties dropped.
/// Reuses `crate::bdots_connstate::is_connected` for what counts as live.
pub fn connected_channels(rows: &[Value]) -> Vec<String> {
    let mut channels: Vec<String> = rows
        .iter()
        .filter(|row| crate::bdots_connstate::is_connected(status_of(row)))
        .map(|row| connector_of(row).to_string())
        .filter(|channel| !channel.is_empty())
        .collect();
    channels.sort();
    channels.dedup();
    channels
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn composio(id: &str, provider: &str, status: &str) -> Value {
        json!({
            "id": id,
            "connectorId": CONNECTOR_COMPOSIO,
            "provider": provider,
            "displayName": provider,
            "status": status,
        })
    }

    #[test]
    fn statuses_channels_and_provider_keys() {
        let row = json!({"connectorId": "composio", "provider": " Gmail ", "status": "connected"});
        assert_eq!(status_of(&row), "connected");
        assert_eq!(connector_of(&row), "composio");
        assert_eq!(provider_key(" Gmail "), "gmail");
        assert_eq!(provider_key("GMAIL"), "gmail");
        assert_eq!(connection_key("composio", " Gmail "), "composio:gmail");
        assert_eq!(status_of(&json!({})), "");
        assert_eq!(connector_of(&json!({})), "");
        assert_eq!(STATUS_CONNECTED, "connected");
        assert_eq!(STATUS_PENDING, "pending");
        assert_eq!(STATUS_ERROR, "error");
        assert_eq!(STATUS_REVOKED, "revoked");
        assert_eq!(CONNECTOR_COMPOSIO, "composio");
    }

    #[test]
    fn connected_keys_collects_only_connected_rows() {
        let rows = vec![
            composio("a", "Gmail", "connected"),
            composio("b", "gmail", "pending"),
            json!({"id": "c", "connectorId": "pipedream", "provider": "gmail", "status": "connected"}),
        ];
        let keys = connected_keys(&rows);
        assert!(keys.contains("composio:gmail"));
        assert!(keys.contains("pipedream:gmail"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn recoverable_pending_and_error_composio_rows() {
        let live: BTreeSet<String> = ["gmail".to_string()].into_iter().collect();
        let empty_keys = BTreeSet::new();

        let pending = composio("p", "gmail", "pending");
        assert!(is_recoverable(&pending, &live, &empty_keys));

        let error = composio("e", "GMAIL", "error");
        assert!(is_recoverable(&error, &live, &empty_keys));

        // revoked is never recoverable
        assert!(!is_recoverable(&composio("r", "gmail", "revoked"), &live, &empty_keys));
        // connected is not a recovery case
        assert!(!is_recoverable(&composio("c", "gmail", "connected"), &live, &empty_keys));
        // non-composio channels are not recovered
        let other = json!({"id": "o", "connectorId": "pipedream", "provider": "gmail", "status": "pending"});
        assert!(!is_recoverable(&other, &live, &empty_keys));
        // provider must be live
        assert!(!is_recoverable(&pending, &BTreeSet::new(), &empty_keys));
        // a connected sibling sharing the slug blocks recovery
        let mut keys = BTreeSet::new();
        keys.insert("composio:gmail".to_string());
        assert!(!is_recoverable(&pending, &live, &keys));
    }

    #[test]
    fn select_run_connections_keeps_connected_drops_revoked_recovers_live() {
        let pending = composio("pending", "youtube", "pending");
        let revoked = composio("revoked", "youtube", "revoked");
        let other = json!({
            "id": "other", "connectorId": "pipedream", "provider": "youtube",
            "displayName": "youtube", "status": "connected",
        });
        let disconnected = json!({
            "id": "disconnected", "connectorId": "pipedream", "provider": "youtube",
            "displayName": "youtube", "status": "error",
        });
        let rows = vec![pending.clone(), revoked.clone(), other.clone(), disconnected.clone()];
        let live = vec!["youtube".to_string()];

        let selected = select_run_connections(&rows, &live);
        assert_eq!(selected, vec![pending.clone(), other.clone()]);

        // with a connected sibling for the same slug, pending is not recovered
        let connected_pending = composio("connected", "youtube", "connected");
        let rows2 = vec![revoked.clone(), connected_pending.clone(), other.clone(), disconnected.clone()];
        let selected2 = select_run_connections(&rows2, &live);
        assert_eq!(selected2, vec![connected_pending, other]);
    }

    #[test]
    fn select_run_connections_trims_and_lowercases_live_providers() {
        let rows = vec![composio("p", "Gmail", "pending")];
        let live = vec!["  GMAIL  ".to_string(), "".to_string(), "gmail".to_string()];
        let selected = select_run_connections(&rows, &live);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0]["id"], json!("p"));
    }

    #[test]
    fn revoked_sibling_with_dead_provider_ref_is_not_revived() {
        let old = json!({
            "id": "youtube-old", "connectorId": "composio", "provider": "youtube",
            "displayName": "YouTube", "status": "revoked", "providerRef": "ca_old",
        });
        let current = json!({
            "id": "youtube-live", "connectorId": "composio", "provider": "youtube",
            "displayName": "YouTube", "status": "connected", "providerRef": "ca_current",
        });
        let rows = vec![old, current.clone()];
        let selected = select_run_connections(&rows, &["youtube".to_string()]);
        assert_eq!(selected, vec![current]);
        assert_eq!(selected[0]["providerRef"], json!("ca_current"));
    }

    #[test]
    fn provider_ref_is_optional_and_keeps_empty_string() {
        assert_eq!(provider_ref(&json!({"providerRef": "ca_1"})), Some("ca_1".to_string()));
        assert_eq!(provider_ref(&json!({"providerRef": ""})), Some(String::new()));
        assert_eq!(provider_ref(&json!({"providerRef": null})), None);
        assert_eq!(provider_ref(&json!({})), None);
    }

    #[test]
    fn connected_context_shape_matches_rakazo() {
        let rows = vec![
            json!({
                "id": "c1", "connectorId": "composio", "provider": "gmail",
                "displayName": "Gmail", "providerRef": "ca_1",
            }),
            json!({
                "id": "c2", "connectorId": "composio", "provider": "slack",
                "displayName": "Slack", "providerRef": null,
            }),
            json!({
                "id": "c3", "connectorId": "pipedream", "provider": "gmail",
                "displayName": "Gmail", "providerRef": "pd_1",
            }),
        ];
        let context = connected_context(&rows);
        assert_eq!(
            context["connectedProviders"],
            json!(["gmail", "slack"]), // only composio, raw provider, order kept
        );
        let connections = context["connectedConnections"].as_array().unwrap();
        assert_eq!(connections.len(), 3);
        assert_eq!(
            connections[0],
            json!({
                "id": "c1", "connectorId": "composio", "externalId": "gmail",
                "displayName": "Gmail", "providerRef": "ca_1",
            })
        );
        // null providerRef -> undefined -> key omitted
        assert_eq!(
            connections[1],
            json!({
                "id": "c2", "connectorId": "composio", "externalId": "slack",
                "displayName": "Slack",
            })
        );
        assert!(!connections[1].as_object().unwrap().contains_key("providerRef"));
        // non-composio connections still listed in connectedConnections
        assert_eq!(connections[2]["connectorId"], json!("pipedream"));
    }

    #[test]
    fn connected_channels_are_distinct_sorted_live_connector_ids() {
        let rows = vec![
            json!({"connectorId": "pipedream", "provider": "x", "status": "connected"}),
            json!({"connectorId": "composio", "provider": "y", "status": "connected"}),
            json!({"connectorId": "composio", "provider": "z", "status": "connected"}),
            json!({"connectorId": "pipedream", "provider": "x", "status": "revoked"}),
            json!({"connectorId": "", "provider": "x", "status": "connected"}),
            json!({"connectorId": "desktop", "provider": "x", "status": "offline"}),
            json!({"connectorId": "ios", "provider": "x", "status": "online"}), // live per bdots_connstate
        ];
        let channels = connected_channels(&rows);
        assert_eq!(channels, vec!["composio".to_string(), "ios".to_string(), "pipedream".to_string()]);
    }
}
