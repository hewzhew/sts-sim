//! Strategy layer.
//!
//! This is the intended owner of high-level interpretation: jobs, gates,
//! package state, deck debt, branch assessment, and candidate assessment.
//! Scene policies should call into this layer instead of carrying private
//! strategic models.

pub mod assessment;
pub mod boss_relic_admission;
pub mod campfire_upgrade_quality;
pub mod deck_admission;
pub mod deck_debt;
pub mod deck_role_inventory;
pub mod formation;
pub mod gates;
pub mod jobs;
pub mod package_state;
pub mod package_transition;
pub mod reward_admission;
pub mod reward_quality;
pub mod reward_semantic_probe;
pub mod reward_semantic_review;
pub mod run_strategic_facts;
