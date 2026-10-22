//! Conversation features on top of bots: messages to the user, group room turns,
//! bot-to-bot requests, prompt snapshots, long-term memory and automatic bot names.

pub mod context;
pub mod decider;
pub mod files;
pub mod fleet;
pub mod group;
pub mod handoff;
pub mod instruction_history;
pub mod memory;
pub mod naming;
pub mod oneshot;
pub mod outbox;
pub mod profile;
pub mod prompt;
pub mod scratchpad;
pub mod teach;
pub mod team;
pub mod uploads;
