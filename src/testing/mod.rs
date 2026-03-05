//! Testing infrastructure for state comparison and validation.
//!
//! Provides:
//! - `CombatSnapshot`: A serializable snapshot of combat state
//! - `diff_snapshots()`: Compare two snapshots and report divergences
//! - `commod_parser`: Parse CommunicationMod JSON into CombatSnapshot
//! - `action_parser`: Parse CommunicationMod commands into ReplayAction
//! - `replay`: Replay driver for differential testing

pub mod snapshot;
pub mod diff;
pub mod commod_parser;
pub mod action_parser;
pub mod replay;
pub mod hydrator;
pub mod step_verifier;
pub mod timing_known;
