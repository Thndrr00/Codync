//! AG-UI (Agent User Interaction Protocol), in both directions.
//!
//! - [`server`]: `POST /agui/{bot}` streams one Codync bot turn as an AG-UI run.
//! - [`shim`]: `codync-host agui-acp` presents a remote AG-UI endpoint to a bot as an ACP agent.
//!
//! The protocol is implemented from its public documentation; no AG-UI package is used. The event mapping is in
//! [`mapping`] (Codync → AG-UI) and [`shim`] (AG-UI → ACP). See `docs/features/agui.md`.

pub mod endpoint;
pub mod events;
mod input;
mod mapping;
pub mod remote;
pub mod server;
pub mod shim;

pub use events::APPROVAL;
pub use server::{MAX_BODY, serve};

/// The backend id of a bot that runs a remote AG-UI agent.
pub const BACKEND: &str = "agui";
