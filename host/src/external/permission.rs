//! A terminal agent asking to use a tool: an approval card on every client, and the hook held open for the answer.

use super::event::Event;
use super::session::{bot_for, turn_of};
use super::{DECISION_WAIT, Decision, External, reply, tool};
use crate::LockExt;
use crate::hub::Hub;
use crate::remote::push::{self, AlertKind};
use crate::store::{BotConfig, EntryKind, Lane};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::oneshot;

const ALLOW: &str = "allow";
const DENY: &str = "deny";

fn card(ev: &Event, bot: &str, cwd: &str) -> Value {
    let title = tool::title(&ev.tool, &ev.input);
    let command = if tool::kind(&ev.tool) == "execute" {
        ev.input.get("command").or_else(|| ev.input.get("CommandLine")).and_then(Value::as_str).map(str::to_owned)
    } else {
        None
    };
    json!({
        "title": title,
        "toolKind": tool::kind(&ev.tool),
        "command": command,
        "detail": "",
        "diffs": ev.diffs(),
        "cwd": cwd,
        "options": [
            {"optionId": ALLOW, "name": "Allow", "kind": "allow_once"},
            {"optionId": DENY, "name": "Deny", "kind": "reject_once"},
        ],
        "status": "pending",
        "selected": Value::Null,
        "author": bot,
    })
}

/// Closes the card when the hook goes away unanswered: the person answered in the terminal, the agent gave up, or
/// the wait ran out. Whatever the card still says is no longer true.
struct Waiting {
    hub: Arc<Hub>,
    entry: String,
    bot: String,
}

impl Drop for Waiting {
    fn drop(&mut self) {
        if self.hub.external.state.locked().waiting.remove(&self.entry).is_none() {
            return; // answered: the answer already settled the card
        }
        if let Some(mut entry) = self.hub.store.entry(&self.entry) {
            entry.data["status"] = "expired".into();
            self.hub.set_entry(&entry.id, &entry.data);
        }
        // Another question in this chat may still be open: it keeps the bot in NeedsInput instead
        // of this hook going away burying it (OpenBot nightly triage-15338).
        super::question::after_card(&self.hub, &self.bot, &self.entry, "Asking in the terminal…");
    }
}

/// The person's decision, or `None` when nobody decides in time and the terminal should ask as usual.
pub(super) async fn request(hub: &Arc<Hub>, ev: &Event) -> Option<Decision> {
    // A question isn't an approval; the terminal shows it. Neither is a request from an agent that can't be answered.
    if ev.tool == "AskUserQuestion" || !reply::takes_decisions(&ev.agent) {
        return None;
    }
    let (bot, config) = target(hub, ev)?;
    let data = card(ev, &bot, &config.cwd);
    // Forbidden for every bot: the terminal is told to refuse, and no card asks.
    if data["command"].as_str().is_some_and(|command| crate::agent::blocklist::blocked(command).is_some()) {
        return Some(Decision::Deny);
    }
    let title = data["title"].as_str().unwrap_or("Use a tool").to_owned();
    wait_card(hub, ev, &bot, &config, &data, &title).await.map(|(decision, _)| decision)
}

/// The bot standing for the event's session and its settings.
pub(super) fn target(hub: &Arc<Hub>, ev: &Event) -> Option<(String, BotConfig)> {
    let bot = bot_for(hub, ev)?;
    let config = hub.store.bot(&bot).ok().flatten()?.config;
    Some((bot, config))
}

/// Shows `data` as a card, tells the person and holds until it is answered; `None` when nobody answers in time.
/// The entry id comes back with the decision so a plan card's free-text change, committed on the card, can be read.
pub(super) async fn wait_card(
    hub: &Arc<Hub>,
    ev: &Event,
    bot: &str,
    config: &BotConfig,
    data: &Value,
    title: &str,
) -> Option<(Decision, String)> {
    let entry = hub.add_entry(&Lane::main(bot), EntryKind::Permission, turn_of(hub, &ev.session), data)?;

    let (tx, rx) = oneshot::channel();
    hub.external.state.locked().waiting.insert(entry.id.clone(), tx);
    let waiting = Waiting { hub: hub.clone(), entry: entry.id.clone(), bot: bot.to_owned() };
    super::question::on_post(hub, &entry, title);
    hub.set_runtime(bot, |r| {
        r.lane = Some(Lane::main(bot));
    });
    push::notify(hub, config, Some(bot), &format!("{} needs you", config.name), title, AlertKind::NeedsInput);

    let answer = tokio::time::timeout(DECISION_WAIT, rx).await;
    drop(waiting);
    let decision = answer.ok()?.ok()?;
    Some((decision, entry.id))
}

/// See [`External::answer`].
/// triage-15940: a plan card's free-text "what to change" arrives as `change:<text>` in the one
/// option id `respondPermission` carries. Only a plan card, a matching prefix and non-blank text
/// count; every other card and option is untouched. Pure.
fn plan_change(plan: bool, option_id: Option<&str>) -> Option<&str> {
    plan.then(|| option_id.and_then(|id| id.strip_prefix(super::question::CHANGE_PREFIX)))
        .flatten()
        .filter(|text| !text.trim().is_empty())
}

pub(super) fn answer(external: &External, hub: &Hub, entry_id: &str, option_id: Option<&str>) -> bool {
    let Some(mut entry) = hub.store.entry(entry_id) else { return is_ours(hub, entry_id) };
    let plan = entry.data["plan"].as_bool() == Some(true);
    // Keep the free text on the card so the waiting hook can forward it as the plan's reason.
    let change = plan_change(plan, option_id);
    // An answer must be one of the options the card showed; anything else leaves the card waiting.
    let known = |id: &str| entry.data["options"].as_array().is_some_and(|o| o.iter().any(|o| o["optionId"] == id));
    if option_id.is_some_and(|id| change.is_none() && !known(id)) {
        return external.state.locked().waiting.contains_key(entry_id) || is_ours(hub, entry_id);
    }
    let provider = entry.data["answerAgent"].as_str().unwrap_or_default();
    if super::question::multi_answer_allowed(provider, &entry.data) {
        return answer_multi(external, hub, entry, entry_id, option_id);
    }
    let Some(waiting) = external.state.locked().waiting.remove(entry_id) else {
        return is_ours(hub, entry_id);
    };
    let decision = if change.is_some() || (plan && option_id == Some(super::question::STOP)) {
        // Stop planning and a free-text change both deny the plan; the reason on the card tells them apart.
        Some(Decision::Deny)
    } else {
        match option_id {
            Some(ALLOW) => Some(Decision::Allow),
            Some(DENY) => Some(Decision::Deny),
            Some(index) => index.parse().ok().map(Decision::Pick),
            None => None,
        }
    };
    if let Some(text) = change {
        entry.data["answerText"] = text.trim().to_owned().into();
    } else if plan && option_id == Some(super::question::STOP) {
        entry.data["answerText"] = json!(super::question::STOP_REASON);
    }
    entry.data["status"] = if decision.is_some() { "answered" } else { "cancelled" }.into();
    entry.data["selected"] = option_id.into();
    hub.set_entry(entry_id, &entry.data);
    let bot = entry.data["author"].as_str().unwrap_or(&entry.bot_id).to_owned();
    // Answering this card cannot close another question in the same chat; if one is still open it
    // takes over NeedsInput (OpenBot nightly triage-15338).
    super::question::after_card(hub, &bot, entry_id, "Continuing…");
    // The hook may already be gone (answered in the terminal): nobody is left to tell.
    let _ = waiting.send(decision);
    true
}

/// A multi-answer card takes taps on its options, deduplicated and capped, until the person taps Done.
/// Only then does the hook get the indexes; a cancelled card gives it no decision.
fn answer_multi(
    external: &External,
    hub: &Hub,
    mut entry: crate::store::Entry,
    entry_id: &str,
    option_id: Option<&str>,
) -> bool {
    if !external.state.locked().waiting.contains_key(entry_id) {
        return is_ours(hub, entry_id);
    }
    let mut selected: Vec<String> = entry.data["selected"]
        .as_array()
        .map(|ids| ids.iter().filter_map(|id| id.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    let done = option_id == Some(super::question::DONE);
    if !done && option_id.is_none() {
        return settle_multi(external, hub, entry, entry_id, Vec::new(), Vec::new(), false);
    }
    // Keep accumulating: the card stays pending until Done. Extra or repeated taps change nothing.
    if !done
        && let Some(id) = option_id
        && !selected.iter().any(|p| p == id)
        && selected.len() < super::question::MAX_SELECTED
    {
        selected.push(id.to_owned());
        entry.data["selected"] = json!(selected);
        hub.set_entry(entry_id, &entry.data);
        return true;
    }
    if !done {
        return true;
    }
    let picks = selected.iter().filter_map(|id| id.parse::<usize>().ok()).collect();
    settle_multi(external, hub, entry, entry_id, selected, picks, true)
}

/// Settles a multi-answer card: close it, wake the bot and send the indexes (or nothing on a cancel).
fn settle_multi(
    external: &External,
    hub: &Hub,
    mut entry: crate::store::Entry,
    entry_id: &str,
    selected: Vec<String>,
    picks: Vec<usize>,
    done: bool,
) -> bool {
    let waiting = external.state.locked().waiting.remove(entry_id);
    let decision = (!picks.is_empty()).then_some(Decision::Picks(picks));
    entry.data["status"] = if decision.is_some() { "answered" } else { "cancelled" }.into();
    // Keep `selected` a string on the settled card: `respondPermission` treats the same answer
    // again as a retry, not as a second, different choice. The picked options live beside it.
    entry.data["selected"] = if done { super::question::DONE.into() } else { Value::Null };
    if done {
        entry.data["selectedOptions"] = json!(selected);
    }
    hub.set_entry(entry_id, &entry.data);
    let bot = entry.data["author"].as_str().unwrap_or(&entry.bot_id).to_owned();
    super::question::after_card(hub, &bot, entry_id, "Continuing…");
    if let Some(waiting) = waiting {
        let _ = waiting.send(decision);
    }
    true
}

/// A card of a terminal session that is no longer waiting (answered, expired, or the hook went away): a late tap
/// is a no-op, as it is for a bot's own card.
fn is_ours(hub: &Hub, entry_id: &str) -> bool {
    hub.store
        .entry(entry_id)
        .and_then(|e| hub.store.bot(e.data["author"].as_str().unwrap_or(&e.bot_id)).ok().flatten())
        .is_some_and(|row| super::is_external(&row.config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plan_card_carries_a_free_text_change() {
        assert_eq!(plan_change(true, Some("change:use SQLite")), Some("use SQLite"));
        assert_eq!(plan_change(true, Some("change:  ")), None, "blank text is not an answer");
        assert_eq!(plan_change(true, Some("change:")), None);
        assert_eq!(plan_change(true, Some("allow")), None, "a normal option is not text");
        assert_eq!(plan_change(false, Some("change:use SQLite")), None, "other cards keep their options");
    }
}
