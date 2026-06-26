//! Runtime layer.
//!
//! Campaign scheduling, branch experiments, journals, and replay belong here.
//! Runtime may allocate budget and record provenance, but it must not invent
//! hidden strategic verdicts.

pub mod branch_campaign;
