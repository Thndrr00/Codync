//! The user's queued messages before their turn comes: handled now (`steer`) or taken back (`unqueue`).

use super::turn_rules::{self, SteerStage, SteerTiming};
use super::{Actor, Cmd, Done, Queued};
use crate::analytics::{self, Steer, TurnSource};
use crate::chat::team::RequestStatus;
use crate::chat::uploads;
use crate::store::Lane;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;

/// How long the agent may take to say whether a steered message joined its turn.
const STEER_ANSWER: Duration = Duration::from_secs(15);

/// Ported from `OpenBot` 0.69.0 (`src/backend/agent/developer-instructions.ts`, `steerNote`): a provider
/// that gets the message as plain user text has no sign that the turn was in progress, and the model can
/// then drop its work for the new message.
const STEER_NOTE: &str = "<openbot_steer>\nThe user sent the next message while you work on this turn. It adds to your current work and does not cancel it. If it asks a question, answer the question first in a short line. If it changes or corrects the work, apply the change from this point. Then continue the unfinished work; do not start it again. Stop the current work only when the message tells you to stop or to do different work.\n</openbot_steer>";

/// Claude Code and Grok Build tell their model that a message arrived while it worked, and Grok reads
/// only the first text block of a steer: their steers carry no note (`OpenBot`'s `PROVIDERS_THAT_FRAME_STEERS`).
fn frames_steers(backend: &str) -> bool {
    matches!(backend, "claude" | "grok")
}

/// The steer's prompts: the note as a block of its own when the backend writes none, then the user's text
/// (which stays the last block, so a provider that reads one block reads the message). A slash command
/// already says what it does.
fn steer_prompts(backend: &str, text: &str) -> Vec<Value> {
    let mut prompts = Vec::with_capacity(2);
    if !frames_steers(backend) && !text.starts_with('/') {
        prompts.push(json!({"type": "text", "text": STEER_NOTE}));
    }
    prompts.push(json!({"type": "text", "text": text}));
    prompts
}

/// A queued message handed to the agent to join its running turn, until the agent answers.
pub(super) struct InFlight {
    /// The user took it back meanwhile (`unqueue`): it goes unless the agent took it.
    taken_back: bool,
}

/// What the agent's answer to a steer made of the message.
fn steer_outcome(answer: &Result<Value>) -> Steer {
    match answer {
        Ok(v) if v["outcome"] == "injected" => Steer::Joined,
        Ok(v) if v["outcome"] == "promptRequired" => Steer::Next,
        _ => Steer::Failed,
    }
}

/// Whether the message was written by a person, not a routine, a teammate or the host (triage-15730).
/// Also used by the queue so a message held for an open card is only a person's (triage-15776).
pub(super) fn from_person(data: &Value) -> bool {
    data["from"] != "host" && !matches!(data["source"].as_str(), Some("routine" | "scheduled_task" | "bot" | "host"))
}

impl Actor {
    /// Where the user's queued message `entry_id` waits.
    fn queued_at(&self, entry_id: &str) -> Option<usize> {
        self.queue.iter().position(|q| matches!(q, Queued::User { entry_id: id, .. } if id == entry_id))
    }

    /// The running turn answers the user's own messages in `lane`: not a routine's, a room's, another bot's or the
    /// host's, nor a compaction, nor one the user already stopped.
    fn answers_user_in(&self, lane: &Lane) -> bool {
        self.turn.is_some()
            && !self.stop_requested
            && self.lane == *lane
            && self.compacting.is_none()
            && self.source() == TurnSource::User
    }

    /// `steer`: the agent takes the message into its running turn when it can (`_session/steering`). Otherwise the
    /// user's turn stops for it, or, behind other work, it goes first in line.
    pub(super) async fn steer(&mut self, entry_id: String) {
        // Already with the agent, or held for this turn's session (v0.78.0).
        if self.steers_in_flight.contains_key(&entry_id) || self.deferred_steer.as_deref() == Some(entry_id.as_str()) {
            return;
        }
        let Some(at) = self.queued_at(&entry_id) else { return };
        let Some(Queued::User { lane, text, .. }) = self.queue.get(at) else { return };
        let (lane, text) = (lane.clone(), text.clone());
        let data = self.hub.store.entry(&entry_id).map(|e| e.data).unwrap_or_default();
        let source = data["source"].as_str().unwrap_or_default().to_owned();
        let is_user = from_person(&data);
        // A card opened since this message was queued still owns it: hold it there instead of steering
        // into (or stopping) the turn that could not sign in (triage-15776).
        if super::queue::message_during_card(self.card_open(&lane), is_user) == super::queue::CardMessage::Hold
            && self.hold_for_card(&entry_id, &lane)
        {
            self.queue.remove(at);
            return;
        }
        // A step of a Claude agent is not a place for another bot's or a routine's message (triage-15730).
        let step_running = self.cfg.backend == "claude" && self.watchdog.tool_running();
        if !turn_rules::interrupt_allowed(&source, is_user, step_running) {
            self.move_first(at);
            analytics::message_steered(&self.hub, &self.cfg.backend, Steer::MovedUp);
            return;
        }
        // OpenBot v0.78.0: a Stop the provider ignored leaves its flag set; a message now resumes that turn.
        if turn_rules::steer_after_stop(self.stop_requested, self.provider_still_running()) {
            self.stop_requested = false;
            self.cancel_deadline = None;
        }
        let own = self.answers_user_in(&lane);
        if own {
            let stage = if self.turn.is_some() { SteerStage::Starting } else { SteerStage::BetweenTurns };
            match turn_rules::steer_during_start(stage, self.turn_session.is_some()) {
                // The turn has not opened its session yet: deliver the steer the moment it does (v0.78.0).
                SteerTiming::HoldForSession => {
                    self.deferred_steer = Some(entry_id);
                    return;
                }
                SteerTiming::Now => {
                    self.await_steer_ready().await;
                    if self.hand_to_agent(entry_id, &text) {
                        return;
                    }
                }
                SteerTiming::Wait => {}
            }
        }
        self.move_first(at);
        let how = if own {
            self.cancel_turn().await;
            Steer::Interrupted
        } else {
            Steer::MovedUp
        };
        analytics::message_steered(&self.hub, &self.cfg.backend, how);
    }

    /// Antigravity needs its first update before it takes a steer (OpenBot v0.78.0; Devin has no Codync
    /// backend). Wait briefly, draining updates the actor already holds so the first one can arrive.
    async fn await_steer_ready(&mut self) {
        const TRIES: u32 = 6;
        const GAP: Duration = Duration::from_millis(50);
        for _ in 0..TRIES {
            if turn_rules::steer_ready(&self.cfg.backend, self.watchdog.saw_update()) {
                return;
            }
            loop {
                let Some(inc) = self.conn.as_mut().and_then(|c| c.rx.try_recv().ok()) else { break };
                self.on_incoming(Some(inc)).await;
            }
            tokio::time::sleep(GAP).await;
        }
    }

    /// Whether a provider still runs the turn a Stop asked to end (v0.78.0).
    fn provider_still_running(&self) -> bool {
        self.conn.is_some() && self.hub.runtime(&self.id()).started_at.is_some()
    }

    /// A steer held while this turn opened its session joins the live turn now (OpenBot v0.78.0).
    pub(super) fn session_opened(&mut self) {
        let Some(entry_id) = self.deferred_steer.take() else { return };
        let Some(at) = self.queued_at(&entry_id) else { return };
        let Some(Queued::User { text, .. }) = self.queue.get(at) else { return };
        let text = text.clone();
        self.hand_to_agent(entry_id, &text);
    }

    /// Hands the message to the agent to join its running turn (`steered` gets the answer); false when it can't steer.
    fn hand_to_agent(&mut self, entry_id: String, text: &str) -> bool {
        let (Some(conn), Some(sid)) = (self.conn.as_ref().filter(|c| c.sessions.steering), self.turn_session.clone())
        else {
            return false;
        };
        let (acp, hub, bot) = (conn.acp.clone(), self.hub.clone(), self.cfg.id.clone());
        let params = json!({
            "sessionId": sid,
            "prompt": steer_prompts(&self.cfg.backend, text),
            // A turn that just ended doesn't take it: it stays queued for the next one.
            "_meta": {"steering": {"idleBehavior": "promptRequired"}},
        });
        // What the agent writes from here may answer it: a reply of its own, after the message in the chat.
        self.close_seg();
        self.steers_in_flight.insert(entry_id.clone(), InFlight { taken_back: false });
        tokio::spawn(async move {
            let answer = tokio::time::timeout(STEER_ANSWER, acp.request("_session/steering", params)).await;
            let outcome = answer.unwrap_or_else(|_| Err(anyhow!("the agent didn't answer the steer")));
            // Refused only once the bot was deleted meanwhile: no one is left to tell.
            let _ = hub.send_cmd(&bot, Cmd::Steered { entry_id, outcome });
        });
        true
    }

    /// Moves what waits at `at` first in line; a compaction that is due still goes before it.
    fn move_first(&mut self, at: usize) {
        let Some(item) = self.queue.remove(at) else { return };
        let first = self.queue.iter().position(|q| !matches!(q, Queued::Compact { .. })).unwrap_or(self.queue.len());
        self.queue.insert(first, item);
        self.renumber_waiting();
    }

    /// Hands the waiting messages' turn numbers out again in their new order: a batch runs as its last message's turn
    /// (`start_turn`), and the clients take the highest for the running one.
    fn renumber_waiting(&self) {
        let waiting: Vec<_> = self
            .queue
            .iter()
            .filter_map(|q| match q {
                Queued::User { entry_id, .. } => self.hub.store.entry(entry_id),
                _ => None,
            })
            .collect();
        let mut turns: Vec<i64> = waiting.iter().map(|e| e.turn).collect();
        turns.sort_unstable();
        for (e, turn) in waiting.iter().zip(turns) {
            if e.turn != turn {
                self.hub.set_entry_turn(&e.id, Some(turn), &e.data);
            }
        }
    }

    /// The agent ended the running turn. Its answer to a steer travels apart from this, and a message it took belongs
    /// to this turn, so the end normally waits for every answer (at most `STEER_ANSWER`). A final answer ends the
    /// working line at once: a steer still waiting for the agent's answer stays queued for the next turn instead of
    /// keeping the bot busy and holding that next message (triage-12470).
    pub(super) async fn turn_ended(&mut self, done: Done, done_tx: &mpsc::UnboundedSender<Done>) {
        if super::turn_rules::final_answer_ends_turn(&done) {
            self.steers_in_flight.clear();
            self.held_done = None;
        }
        if self.steers_in_flight.is_empty() {
            self.on_turn_done(done, done_tx).await;
        } else {
            self.held_done = Some(done);
        }
    }

    /// The agent's answer to a steer; the last one lets a held turn end (`turn_ended`).
    pub(super) async fn steer_answered(
        &mut self,
        entry_id: &str,
        outcome: &Result<Value>,
        done_tx: &mpsc::UnboundedSender<Done>,
    ) {
        self.steered(entry_id, outcome);
        if self.steers_in_flight.is_empty()
            && let Some(done) = self.held_done.take()
        {
            self.on_turn_done(done, done_tx).await;
        }
    }

    /// A message the agent took is the running turn's, sent; one it left for later goes next; any other stays queued,
    /// unless the user took it back meanwhile.
    fn steered(&mut self, entry_id: &str, outcome: &Result<Value>) {
        let Some(steer) = self.steers_in_flight.remove(entry_id) else { return };
        if let Err(error) = outcome {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't steer; the message waits");
        }
        let how = steer_outcome(outcome);
        analytics::message_steered(&self.hub, &self.cfg.backend, how);
        // A Stop meanwhile dropped it from the queue (and marked it cancelled).
        let Some(at) = self.queued_at(entry_id) else { return };
        if how != Steer::Joined {
            if steer.taken_back {
                self.unqueue(entry_id);
            } else {
                // A message read by a busy Claude whose session closed has nowhere to go now: requeue it
                // instead of losing it when the session closes (16361). An agent's own continued reply
                // (Kimi Code) is delivered by the joined path below, not requeued (16364).
                let late = turn_rules::late_message_action(self.turn_session.is_none(), self.turn.is_some());
                if how == Steer::Next || late == turn_rules::LateMessage::Requeue {
                    self.move_first(at);
                }
            }
            return;
        }
        let Some(Queued::User { text, .. }) = self.queue.remove(at) else { return };
        // The host's copy, never `removed`, also brings it back to a client that took it back meanwhile.
        if let Some(mut e) = self.hub.store.entry(entry_id) {
            e.data["status"] = json!(RequestStatus::Sent);
            self.hub.set_entry_turn(entry_id, self.turn, &e.data);
        }
        self.joined = true;
        self.turn_entries.push(entry_id.to_owned());
        if let Some(turn_text) = &mut self.turn_text {
            // A provider that echoes a background answer twice (Gemini) must not show it again (16149).
            let seen = turn_text.contains(text.trim()) && !text.trim().is_empty();
            if turn_rules::background_answer_seen(&text, seen) == turn_rules::BackgroundAnswer::Show {
                turn_text.push_str("\n\n");
                turn_text.push_str(&text);
            }
        }
        self.relay_joined(entry_id, &text);
        // `send --wait` learns a started message's turn from the bot's next event, and a status and activity that stay
        // the same send none.
        self.hub.emit_bot(&self.cfg.id);
    }

    /// `unqueue`: the message never goes out, the chat no longer shows it, and its files leave the bot's folder.
    pub(super) fn unqueue(&mut self, entry_id: &str) {
        if self.deferred_steer.as_deref() == Some(entry_id) {
            self.deferred_steer = None;
        }
        // Already with the agent, which may take it: `steered` decides.
        if let Some(steer) = self.steers_in_flight.get_mut(entry_id) {
            steer.taken_back = true;
            return;
        }
        let Some(at) = self.queued_at(entry_id) else { return };
        self.queue.remove(at);
        if let Some(mut e) = self.hub.store.entry(entry_id) {
            e.data["status"] = json!(RequestStatus::Cancelled);
            e.data["removed"] = true.into();
            self.hub.set_entry(entry_id, &e.data);
            self.delete_uploads(&e.data);
            // The roster previews the chat's newest message.
            self.hub.emit_bot(&self.cfg.id);
        }
    }

    /// Deletes the files of a message (its saved `data`) that never went out.
    fn delete_uploads(&self, data: &Value) {
        let ids: Vec<String> = data["attachments"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|a| a["id"].as_str().map(str::to_owned))
            .collect();
        if ids.is_empty() {
            return;
        }
        let (root, bot) = (uploads::root(&self.cfg), self.cfg.id.clone());
        tokio::task::spawn_blocking(move || {
            for id in ids {
                if let Err(error) = uploads::remove(&root, &id) {
                    tracing::warn!(%bot, error = format!("{error:#}"), "couldn't delete a removed message's file");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agents_answer_says_what_became_of_the_message() {
        assert_eq!(steer_outcome(&Ok(json!({"outcome": "injected"}))), Steer::Joined);
        assert_eq!(steer_outcome(&Ok(json!({"outcome": "promptRequired"}))), Steer::Next);
        assert_eq!(steer_outcome(&Ok(json!({"outcome": "somethingNew"}))), Steer::Failed);
        assert_eq!(steer_outcome(&Err(anyhow!("gone"))), Steer::Failed);
        assert_eq!(json!(Steer::MovedUp), "moved_up", "as the event reports it");
    }

    #[test]
    fn a_steer_tells_a_backend_without_its_own_note_that_the_work_continues() {
        let prompts = steer_prompts("codex", "also update the README");
        assert_eq!(prompts.len(), 2);
        assert!(prompts[0]["text"].as_str().unwrap().contains("adds to your current work"));
        assert_eq!(prompts[1]["text"], "also update the README", "the person's text stays last");
        for backend in ["claude", "grok"] {
            assert_eq!(
                *steer_prompts(backend, "also update the README"),
                vec![json!({"type": "text", "text": "also update the README"})],
                "{backend} writes its own note"
            );
        }
        assert_eq!(
            *steer_prompts("codex", "/compact"),
            vec![json!({"type": "text", "text": "/compact"})],
            "a slash command needs none"
        );
    }
}
