//! Extra webhook-routine methods: the sender's own Standard Webhooks secret, a signed test
//! delivery, and the short history of deliveries that didn't run.
//!
//! Routed from `api/mod.rs::route` (a separate dispatch line, kept out of this change's scope).

use super::devices::Caller;
use crate::hub::Hub;
use anyhow::{Result, bail};
use serde_json::Value;
use std::sync::Arc;

pub const METHODS: &[&str] = &["routineSenderSecret", "testRoutineWebhook", "routineIgnored"];

pub fn call(hub: &Arc<Hub>, _caller: &Caller, method: &str, b: &Value) -> Result<Value> {
    let bot = super::str_arg(b, "botId")?;
    let id = super::str_arg(b, "id")?;
    match method {
        "routineSenderSecret" => {
            // Missing `secret` clears it; the value never goes into a reply or a log.
            let secret = b.get("secret").and_then(Value::as_str).map(str::to_owned);
            hub.routines.set_sender_secret(&hub.store, bot, id, secret)
        }
        "testRoutineWebhook" => hub.routines.test_webhook(hub, bot, id),
        "routineIgnored" => Ok(hub.routines.ignored(bot, id)),
        _ => bail!("unknown webhook method"),
    }
}
