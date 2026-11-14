//! Declarative UI blocks (card K3): a bot describes a small piece of interface, the host checks it
//! against Codync's own catalogue and posts it as an approval card, and every client draws it with
//! its own controls.
//!
//! The schema is versioned (`VERSION`) and lives here, not in a client: `schema::parse` accepts only
//! the components, props and sizes listed in [`schema::COMPONENTS`], refuses markup and control
//! characters, and answers an invalid document with a reason the agent can act on. A block with
//! actions (a form's Submit, an approval's Approve/Deny) waits for the user through the same answer
//! path every other approval card uses, and its fields come back to the host with the choice.

mod card;
mod props;
pub mod schema;

pub use card::{ANSWER_TIMEOUT, TOOLS, answers, call, tool};
pub use schema::{Block, VERSION};

/// Why a schema was refused, plus what's allowed, for the agent to fix it in one step.
pub fn guide() -> &'static str {
    "Allowed shape: {\"version\": 1, \"title\": \"…\", \"body\": [components], \"actions\": [{id, label, kind}]}. \
     Components: heading {text}, text {text}, divider, callout {tone: info|warning, text}, \
     field {field: {id, label, type: text|multiline|number|date|time|select|toggle, required, options, value}}, \
     table {columns, rows}, chart {chartType: bar|line, labels, values}. Plain text only: no HTML, \
     Markdown or scripts."
}
