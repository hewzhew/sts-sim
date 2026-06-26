//! Strategy layer.
//!
//! This is the intended owner of high-level interpretation: jobs, gates,
//! package state, deck debt, branch assessment, and candidate assessment.
//! Scene policies should call into this layer instead of carrying private
//! strategic models.

pub mod assessment;
pub mod deck_debt;
pub mod formation;
pub mod gates;
pub mod jobs;
pub mod package_state;
pub mod package_transition;
