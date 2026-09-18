//! YMCL domain adapter protocol (YAP) client: domain registry, capabilities
//! and manifest fetching. See docs/ymcl-adapter-protocol.md in the spec repo.

pub mod auth;
pub mod bundle;
pub mod chrome;
pub mod client;
pub mod data;
#[cfg(feature = "tauri")]
pub mod events;
pub mod manifest;
pub mod mip;
pub mod registry;
pub mod skins;
pub mod yggroot;

pub use crate::state::ymcl_session::{YmclSessionInfo, YmclStoredSession};
