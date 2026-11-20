//! Agent templates: a bot's setup saved on this computer and optionally shared by a link.
//!
//! A template carries a name, a short summary, instructions and safe defaults
//! (agent, model, avatar, skill and connector ids). It never carries a secret,
//! a key, an account, an image or a computer path: those are dropped on the way
//! in, so a saved or exported template is safe to show and to share. Installing
//! one goes through `agent::new_bot` and `Hub::create_bot`, the same path as any
//! new bot, so the result has a fresh workspace and none of the source bot's
//! credentials.
//!
//! The share-id and origin rules mirror OpenBot nightly's
//! `packages/contracts/src/agent-template-links.ts` (22 safe characters, one
//! fixed web origin, the `codync://` custom scheme, no userinfo/query/fragment).

mod ids;
mod model;
mod ops;
mod sanitize;

pub use ids::{is_template_id, new_template_id, parse_template_ref, share_url};
pub use model::{VisibilityChange, detail, export_template, import_template, publication, summary, transition};
pub use ops::{delete, get, install, list, preview, publish, rename, save, unpublish};
pub use sanitize::redact;

pub(crate) use ids::checked_id;
pub(crate) use sanitize::{avatar, clean_instructions, clean_line, positive_or, safe_catalog_id, safe_ids};

// Limits shared by the store, the importer and the operations.
pub(crate) const ID_LEN: usize = 22;
pub(crate) const MAX_NAME: usize = 80;
pub(crate) const MAX_SUMMARY: usize = 280;
pub(crate) const MAX_INSTRUCTIONS: usize = 16_000;
pub(crate) const MAX_ITEMS: usize = 32;
pub(crate) const MAX_ID: usize = 64;
pub(crate) const DEFAULT_NAME: &str = "Bot";
pub(crate) const DEFAULT_COLOR: &str = "blue";
pub(crate) const DEFAULT_SHAPE: &str = "blob";
pub(crate) const SHARE_ORIGIN: &str = "https://codync.dev";
pub(crate) const SHARE_PATH: &str = "/agents/";
