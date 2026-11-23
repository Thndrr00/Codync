//! `shareLocation`: the place a person typed into a location card. The host owns the answer and the card;
//! this method only carries the text to it. `shareLocation` is not an analytics event, so the place is
//! never logged (OpenBot nightly v0.77.0, "the server gives the location to the agent and does not save it").

use super::str_arg;
use crate::hub::Hub;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::sync::Arc;

/// The methods this module answers.
pub const METHODS: [&str; 1] = ["shareLocation"];

/// Answers a location card. The text goes to the waiting voice call, or — once it has stopped waiting —
/// becomes the user's next message to that agent.
pub fn call(hub: &Arc<Hub>, method: &str, b: &Value) -> Result<Value> {
    match method {
        "shareLocation" => {
            let entry_id = str_arg(b, "entryId")?;
            let text = str_arg(b, "text")?;
            if !crate::external::location::share(hub, entry_id, text) {
                bail!("That location card is no longer answerable");
            }
            Ok(json!({"shared": true}))
        }
        _ => unreachable!("only METHODS are routed here"),
    }
}
