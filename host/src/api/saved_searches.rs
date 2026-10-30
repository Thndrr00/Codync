use super::{Caller, str_arg};
use crate::{hub::Hub, store::SearchScope};
use anyhow::Result;
use serde_json::{Value, json};

pub(super) fn call(hub: &Hub, caller: &Caller, method: &str, body: &Value) -> Result<Value> {
    let device = caller.device_key();
    let searches = match method {
        "saveSearch" => {
            let scope: SearchScope = serde_json::from_value(body.get("scope").cloned().unwrap_or(json!("all")))?;
            hub.store.change_saved_searches(device, Some((str_arg(body, "query")?, scope)), None)?
        }
        "deleteSavedSearch" => hub.store.change_saved_searches(device, None, Some(str_arg(body, "id")?))?,
        _ => hub.store.saved_searches(device)?,
    };
    Ok(json!({"searches": searches}))
}
