//! Integration-layer testing surfaces.
//!
//! `testing::fixtures` owns replay/spec fixtures and fixture assembly.
//! `testing::harness` owns integration-side analysis helpers.
//! `testing::protocol` and `testing::state_sync` own legacy live-capture import
//! support used by fixtures, not by the AI/search runtime.
//! `testing::support` owns test-only helpers.

pub mod fixtures;
pub(crate) mod harness;
pub(crate) mod protocol;
pub(crate) mod replay_support;
pub(crate) mod state_sync;
pub mod support;
