//! ComfyUI's `/ws`, for the node and the step a running job is on. `/queue` and `/history` say
//! whether a job runs and what it made, but not how far it got; this listener fills the card and the
//! phone in while it works, and stops as soon as the job ends.

use super::comfy_job::Jobs;
use futures::StreamExt as _;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// The socket may be quiet for a while: a big checkpoint takes time to load before the first step.
const SILENCE: Duration = Duration::from_secs(5);

/// Follows one prompt until it is no longer active. A failed connection is not a job failure: the
/// history poller still records the result, only without a step count.
pub async fn follow(port: u16, prompt_id: String, jobs: Arc<Jobs>) {
    let url = format!("ws://127.0.0.1:{port}/ws?clientId=codync");
    let Ok((mut socket, _)) = tokio_tungstenite::connect_async(url).await else {
        tracing::debug!("ComfyUI's /ws did not answer; the job runs without a progress bar");
        return;
    };
    let mut node: Option<String> = None;
    loop {
        if !jobs.is_current(&prompt_id) {
            return;
        }
        match tokio::time::timeout(SILENCE, socket.next()).await {
            Ok(Some(Ok(message))) => {
                if let Ok(text) = message.into_text() {
                    handle(text.as_str(), &prompt_id, &jobs, &mut node);
                }
            }
            Ok(Some(Err(error))) => {
                tracing::debug!(error = %error, "reading ComfyUI's /ws");
                return;
            }
            Ok(None) => return,
            // Quiet is normal: keep waiting while the job is active.
            Err(_) => {}
        }
    }
}

fn handle(text: &str, prompt_id: &str, jobs: &Jobs, node: &mut Option<String>) {
    let Ok(event) = serde_json::from_str::<Value>(text) else { return };
    let data = &event["data"];
    match event["type"].as_str() {
        Some("executing") => {
            if let Some(id) = data["node"].as_str() {
                *node = Some(id.to_owned());
            } else {
                // `node: null`: the run reached its end; the history poller records the result.
                *node = None;
            }
        }
        Some("progress") => {
            let current = data["node"].as_str().map(str::to_owned).or_else(|| node.clone());
            let Some(current) = current else { return };
            let step = data["value"].as_u64().unwrap_or_default();
            let total = data["max"].as_u64().unwrap_or_default();
            jobs.progress(prompt_id, &current, step, total);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_progress_event_names_the_node_that_ran_before_it() {
        let jobs = Jobs::new(tokio::sync::broadcast::channel(8).0);
        // Nothing is active: the event is dropped, which is also what happens after a cancel.
        let mut node = None;
        handle(r#"{"type":"executing","data":{"node":"5","prompt_id":"p"}}"#, "p", &jobs, &mut node);
        assert_eq!(node.as_deref(), Some("5"));
        handle(r#"{"type":"progress","data":{"value":3,"max":20}}"#, "p", &jobs, &mut node);
        handle(r#"{"type":"executing","data":{"node":null}}"#, "p", &jobs, &mut node);
        assert_eq!(node, None);
    }
}
