//! Integration-layer testing surfaces.
//!
//! `testing::fixtures` owns replay/spec fixtures and fixture assembly.
//! `testing::harness` owns integration-side analysis helpers.
//! `testing::support` owns test-only helpers.

pub mod fixtures;
pub(crate) mod harness;
