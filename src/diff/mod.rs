//! Integration adapters around external protocol state and replay data.
//!
//! `diff::protocol` owns Java/protocol-facing parsing and snapshot shaping.
//! `diff::replay` owns replay execution and comparison.
//! `diff::state_sync` owns protocol-to-runtime state construction.

pub mod protocol;
pub mod replay;
pub mod state_sync;
