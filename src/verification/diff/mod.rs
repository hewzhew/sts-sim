//! Verification adapters around external protocol state and replay data.
//!
//! `verification::protocol` owns Java/protocol-facing parsing and snapshot shaping.
//! `verification::diff::replay` owns replay execution and comparison.
//! `verification::diff::state_sync` owns protocol-to-runtime state construction.

pub mod replay;
pub mod state_sync;
