//! Closed setup milestones through the existing analytics opt-out and transport.
use super::{Event, capture};
use crate::hub::Hub;
use serde_json::json;

pub(crate) fn ready(hub: &Hub) {
    milestone(hub, "ready", Event::SetupReady);
}
pub(crate) fn activated(hub: &Hub) {
    milestone(hub, "activated", Event::SetupActivated);
}
fn milestone(hub: &Hub, name: &str, event: Event) {
    if !hub.analytics.state() {
        return;
    }
    match hub.store.claim_setup_milestone(name) {
        Ok(true) => capture(hub, event, json!({"step":"ready"}).as_object().cloned().unwrap_or_default()),
        Ok(false) => {}
        Err(error) => tracing::debug!(%error,"Setup milestone was not saved"),
    }
}
pub(crate) fn failed(hub: &Hub, channel: bool) {
    if !hub.analytics.state() {
        return;
    }
    let class = if channel { "channel_start_failed" } else { "assistant_run_failed" };
    match hub.store.claim_setup_milestone(class) {
        Ok(true) => capture(
            hub,
            Event::SetupFailed,
            json!({"step":"ready","error_class":class}).as_object().cloned().unwrap_or_default(),
        ),
        Ok(false) => {}
        Err(error) => tracing::debug!(%error,"Setup failure milestone was not saved"),
    }
}
