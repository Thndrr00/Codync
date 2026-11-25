//! The "Reset usage" method for the Providers rows: spends one of the provider's extra credits
//! and ends the usage-limit hold on this computer. Routed from `api/mod.rs::route`.

use super::devices::Caller;
use crate::hub::Hub;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;

pub const METHOD: &str = "resetUsageCredits";

pub fn call(hub: &Arc<Hub>, _caller: &Caller, b: &Value) -> Result<Value> {
    crate::usage::reset_usage(hub, super::str_arg(b, "id")?)
}
