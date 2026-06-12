mod approvals;
mod policy;
mod types;
mod upgrade_priority;

#[cfg(test)]
mod tests;

pub use policy::{build_campfire_decision_context_v1, plan_campfire_decision_v1};
pub use types::{
    CampfireCandidateEvidenceV1, CampfireDecisionContextV1, CampfireDecisionV1,
    CampfirePolicyActionV1, CampfirePolicyClassV1, CampfirePolicyConfigV1,
};
pub use upgrade_priority::{
    campfire_smith_upgrade_priority_v1, campfire_smith_upgrade_strategy_tag_v1,
};
