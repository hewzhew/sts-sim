mod compiler;
mod component_scorer;
mod conversion;
mod evaluator;
mod policy;
mod portfolio;
mod strategy_tags;
mod types;

#[cfg(test)]
mod tests;

pub use compiler::{
    compile_shop_decision_v1, compiled_shop_decision_has_executable_conversion_branch_v1,
};
pub use conversion::{
    build_shop_need_profile_v1, shop_card_conversion_priority_v1, shop_conversion_pressure_v1,
    shop_potion_conversion_priority_for_v1, shop_potion_conversion_priority_v1,
    shop_relic_conversion_priority_for_v1, shop_relic_conversion_priority_v1,
};
pub use policy::{
    build_shop_decision_context_v1, shop_potion_purchase_block_reason_v1,
    shop_potion_purchase_is_allowed_v1,
};
pub use types::{
    CompiledShopDecisionV1, ShopCandidateEvidenceV1, ShopCompileModeV1, ShopDecisionContextV1,
    ShopDecisionSourceV1, ShopNeedProfileV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1,
    ShopPlanComponentKindV1, ShopPlanComponentScoreV1, ShopPlanComponentV1, ShopPlanEvaluationV1,
    ShopPlanKindV1, ShopPlanSourceV1, ShopPlanStepV1, ShopPlanV1, ShopPlanVerdictV1,
    ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};
