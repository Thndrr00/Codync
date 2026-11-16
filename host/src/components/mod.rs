//! The component gallery: named, versioned cards the host owns, and the playground that authors them.
//!
//! A card is a catalogue name plus props, checked here against [`catalogue`] and drawn by each client
//! with its own controls. The host never executes a component: nothing in a card is markup, a script
//! or React, which is what separates this from OpenBot's gallery (a plugin folder of React components)
//! and what makes the same card safe to synced to every client.
//!
//! - [`catalogue`] holds the six built-in cards, their props and the data functions each may name.
//! - [`schema`] validates an instance and is the document clients render.
//! - [`data`] resolves a card's data functions from this computer, bounded and scoped to one bot.
//! - [`instances`] is the gallery and the playground's host API.
//! - [`card`] is the `show_component` tool: a card in the chat, and a `form`'s answers back.
//!
//! Withhold flags live beside both: no saved row means a bot may use a component, and a row with
//! `published: 0` keeps it out of that bot's gallery and refuses its `show_component`.

mod card;
mod catalogue;
mod data;
mod instances;
pub mod schema;

pub use card::{ANSWER_TIMEOUT, TOOLS, answers, call, tool};
pub use catalogue::{ALL, Prop, PropSpec, Spec, VERSION, find, guide, names};
pub use instances::{METHODS, call as api_call};
pub use schema::{Component, parse};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
