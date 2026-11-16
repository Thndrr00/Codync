//! The data functions a card may name: the closed allow list, what each one hands back, and the
//! bounds that keep a card small.
//!
//! A card's props are written by whoever asked for it (the bot, or the playground). A data function
//! is the one part the host fills itself, from this computer's own state: a bot never reads a list
//! through a tool and then retypes it, and a published card never carries someone's data around.
//! Each function is granted per component in [`super::catalogue`], is scoped to the bot whose chat
//! the card is in, and returns at most the rows its entry below allows. The host resolves it at the
//! moment the card is shown, so the card is a picture of the state then.

use super::catalogue::{self, Spec};
use crate::hub::Hub;
use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

/// Every data function, and the most rows it returns whatever a request asks for.
pub const FUNCTIONS: [(&str, usize); 4] =
    [("bots.roster", 50), ("routines.list", 50), ("routines.status", 20), ("threads.recent", 20)];

/// The requests one card may make.
pub const MAX_REQUESTS: usize = 4;

/// Fills the props a card's `data` requests resolve, leaving every prop the caller wrote as it is.
/// Returns the function that filled something, for the card's `source`.
pub fn fill(
    hub: &Hub,
    bot: &str,
    spec: &Spec,
    requests: Option<&Value>,
    props: &mut Map<String, Value>,
) -> Result<Option<String>> {
    let mut source = None;
    for (function, limit) in planned(spec, requests)? {
        let Value::Object(values) = resolve(hub, bot, function, limit)? else {
            bail!("\"{function}\" didn't resolve to props.");
        };
        for (key, value) in values {
            // A prop the caller wrote wins over the one the host resolved.
            props.entry(key).or_insert(value);
        }
        source.get_or_insert_with(|| function.to_owned());
    }
    Ok(source)
}

/// The requests a card asked for, checked against its grant list and this host's allow list. No
/// state is read, so the playground can check a draft before anyone's data is involved.
pub fn check(spec: &Spec, requests: Option<&Value>) -> Result<()> {
    planned(spec, requests).map(|_| ())
}

/// The (function, limit) pairs a card asked for, checked and clamped.
fn planned<'a>(spec: &'a Spec, requests: Option<&'a Value>) -> Result<Vec<(&'a str, usize)>> {
    let Some(requests) = requests.filter(|value| !value.is_null()) else { return Ok(Vec::new()) };
    let Some(requests) = requests.as_array() else { bail!("`data` must be a list of {{function, limit?}}.") };
    if requests.is_empty() || requests.len() > MAX_REQUESTS {
        bail!("`data` must have 1 to {MAX_REQUESTS} requests.");
    }
    let allowed = allowed_list(spec);
    let mut planned = Vec::with_capacity(requests.len());
    for (index, request) in requests.iter().enumerate() {
        let Some(request) = request.as_object() else { bail!("`data[{index}]` must be an object.") };
        if let Some(key) = request.keys().find(|key| !matches!(key.as_str(), "function" | "limit")) {
            bail!("`data[{index}]` has an unknown key \"{key}\": only function and limit.");
        }
        let function = request.get("function").and_then(Value::as_str).unwrap_or_default();
        if !spec.data.contains(&function) {
            bail!("a {} card can't use \"{function}\"; it may use: {allowed}.", spec.name);
        }
        let Some((_, most)) = FUNCTIONS.iter().find(|(name, _)| *name == function) else {
            bail!("\"{function}\" isn't a data function this computer resolves; it resolves: {}.", function_list());
        };
        let limit = match request.get("limit") {
            None | Some(Value::Null) => *most,
            Some(value) => {
                let Some(asked) = value.as_u64() else { bail!("`data[{index}].limit` must be a whole number.") };
                usize::try_from(asked).unwrap_or(usize::MAX).clamp(1, *most)
            }
        };
        planned.push((function, limit));
    }
    Ok(planned)
}

/// The functions a card may name, as a refusal phrases them.
fn allowed_list(spec: &Spec) -> String {
    if spec.data.is_empty() { "nothing (it takes its props in full)".to_owned() } else { spec.data.join(", ") }
}

fn function_list() -> String {
    FUNCTIONS.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
}

/// What one function hands back: props, keyed as the components that grant it expect them.
fn resolve(hub: &Hub, bot: &str, function: &str, limit: usize) -> Result<Value> {
    Ok(match function {
        "bots.roster" => {
            let rows: Vec<Value> = hub
                .store
                .bots()?
                .into_iter()
                .filter(|row| !row.deleted)
                .take(limit)
                .map(|row| {
                    let kind = if row.config.is_group() { "Group" } else { "Agent" };
                    json!([row.config.name, kind])
                })
                .collect();
            json!({"columns": ["Bot", "Kind"], "rows": rows})
        }
        "routines.list" => {
            let list = hub.routines.list(bot, |_| false);
            let rows: Vec<Value> = routines(&list)
                .take(limit)
                .map(|routine| {
                    let on = if routine["enabled"].as_bool().unwrap_or(false) { "On" } else { "Off" };
                    json!([name(routine), when(routine), on])
                })
                .collect();
            json!({"columns": ["Routine", "When", "On"], "rows": rows})
        }
        "routines.status" => {
            let list = hub.routines.list(bot, |_| false);
            let items: Vec<Value> = routines(&list)
                .take(limit)
                .map(|routine| json!({"label": name(routine), "done": routine["enabled"].as_bool().unwrap_or(false)}))
                .collect();
            json!({"items": items})
        }
        "threads.recent" => {
            let rows: Vec<Value> = hub
                .store
                .threads(bot, false)?
                .into_iter()
                .take(limit)
                .map(|thread| json!([thread.title, thread.unread]))
                .collect();
            json!({"columns": ["Thread", "Unread"], "rows": rows})
        }
        other => bail!("\"{other}\" isn't a data function this computer resolves; it resolves: {}.", function_list()),
    })
}

/// A bot's routines, as `routines.list` reports them.
fn routines(list: &Value) -> impl Iterator<Item = &Value> {
    list["routines"].as_array().into_iter().flatten()
}

fn name(routine: &Value) -> &str {
    routine["name"].as_str().unwrap_or_default()
}

/// When a routine runs, in the routine editor's own words; none when it has no trigger.
fn when(routine: &Value) -> String {
    let parts: Vec<&str> =
        routine["triggerDescriptions"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    if parts.is_empty() { "No trigger".to_owned() } else { parts.join(", ") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_granted_function_is_one_the_host_resolves() {
        for spec in &catalogue::ALL {
            for function in spec.data {
                assert!(
                    FUNCTIONS.iter().any(|(name, _)| name == function),
                    "{} grants {function}, which no resolver answers",
                    spec.name
                );
            }
        }
        assert_eq!(FUNCTIONS.len(), 4);
    }

    #[test]
    fn a_component_refuses_a_function_it_does_not_grant() {
        let spec = catalogue::find("metric").unwrap();
        assert!(!catalogue::grants("metric").contains(&"routines.list"));
        assert!(catalogue::grants("table").contains(&"routines.list"));
        assert!(allowed_list(spec).contains("nothing"));
    }
}
