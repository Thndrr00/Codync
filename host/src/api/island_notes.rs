//! An explicit user capture intent, independent of the bot's currently executing lane.
use super::str_arg;
use crate::hub::Hub;
use anyhow::{Result, ensure};
use serde_json::Value;
use std::sync::Arc;

pub(super) fn append(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let text = str_arg(body, "text")?;
    ensure!(text.chars().count() <= 4_000, "A captured note must be at most 4,000 characters");
    crate::chat::scratchpad::append_main(hub, str_arg(body, "botId")?, text)
}
