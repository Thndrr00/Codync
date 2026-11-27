//! The device pref for opt-in learning of site facts from browser teaching (OpenBot inventory #14).
//! Stored in the host kv next to the other device prefs; no new subsystem.

use crate::hub::Hub;
use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;

pub const METHODS: &[&str] = &["browsingLearning", "setBrowsingLearning"];

/// `true` only when the person explicitly turned learning on.
pub fn call(hub: &Arc<Hub>, method: &str, body: &Value) -> Result<Value> {
    match method {
        "browsingLearning" => Ok(json!({"enabled": crate::chat::teach::learn_from_browsing(&hub.store)})),
        "setBrowsingLearning" => {
            let on = body["enabled"].as_bool().ok_or_else(|| anyhow::anyhow!("enabled must be a boolean"))?;
            crate::chat::teach::set_learn_from_browsing(&hub.store, on)?;
            Ok(json!({"enabled": on}))
        }
        _ => Err(anyhow::anyhow!("unknown browsing-learning method")),
    }
}
