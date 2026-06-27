mod evaluator;
mod policy;
mod types;

#[cfg(test)]
mod tests;

pub use policy::{
    boss_relic_candidate_order_key_v1, boss_relic_skip_order_key_v1,
    build_boss_relic_decision_context_v1, plan_boss_relic_decision_v1,
    render_boss_relic_candidate_compact_v1,
};
pub use types::{
    BossRelicCandidateEvidenceV1, BossRelicDecisionContextV1, BossRelicDecisionV1,
    BossRelicOrderKeyV1, BossRelicOrderTierV1, BossRelicPolicyActionV1, BossRelicPolicyClassV1,
    BossRelicPolicyConfigV1,
};
