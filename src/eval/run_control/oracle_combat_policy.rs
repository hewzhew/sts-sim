//! Shared exact-planner policy surface.
//!
//! Complete-line advisors and external lookahead proposals are deliberately
//! absent: they could influence production stopping without owning the exact
//! contract frontier. The maintained run-control surface exposes only typed
//! action and state guidance to the exact planners.

pub(super) use sts_combat_knowledge::ExistingCombatKnowledgePolicy;
pub use sts_combat_knowledge::{
    authorized_potion_trial_policy_v1, existing_combat_guide_service_bias_v1,
    existing_combat_knowledge_policy_v1,
};
