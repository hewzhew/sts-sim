//! Compatibility facade for older control binaries.
//!
//! Oracle and run-control implementation now compile once in
//! `sts_oracle_runtime`; this package no longer owns a second monolithic copy.

pub use sts_oracle_runtime::*;
