//! MIP client (YAP §7, consuming mrpack-incremental-protocol.md): pack
//! state tracking, manifest fetch, incremental diff, download orchestration
//! and atomic apply. Sub-modules split the responsibilities.
pub mod apply;
pub mod diff;
pub mod manifest;
pub mod publish;
pub mod state;
pub mod update;
