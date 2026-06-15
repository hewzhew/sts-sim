use super::types::{ShopPlanComponentKindV1, ShopPlanComponentScoreV1, ShopPlanComponentV1};

pub(crate) fn score_shop_plan_components_v1(
    components: &[ShopPlanComponentV1],
) -> ShopPlanComponentScoreV1 {
    if components.is_empty() {
        return ShopPlanComponentScoreV1::neutral("component score has no components");
    }

    let mut positive = 0.0;
    let mut negative = 0.0;
    let mut has_legacy_estimate = false;
    let mut has_non_legacy_signal = false;

    for component in components {
        match component.kind {
            ShopPlanComponentKindV1::DeckCleanup => {
                positive += 90.0 * component.amount.max(0.0);
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::RelicValue => {
                positive += 180.0 * component.amount.max(0.0);
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::PotionFill => {
                positive += 80.0 * component.amount.max(0.0);
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::BossAnswer => {
                positive += 100.0 * component.amount.max(0.0);
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::LegacyEstimate => {
                has_legacy_estimate = true;
            }
            ShopPlanComponentKindV1::BranchExploration => {
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::DeckBloatCost => {
                negative += 70.0 * component.amount.max(0.0);
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::GoldSpend => {
                negative += component.amount.max(0.0) / 4.0;
                has_non_legacy_signal = true;
            }
            ShopPlanComponentKindV1::StopReason => {
                negative += 10.0 * component.amount.max(0.0);
            }
        }
    }

    let confidence = (0.25_f32
        + if has_legacy_estimate { 0.20 } else { 0.0 }
        + if has_non_legacy_signal { 0.15 } else { 0.0 })
    .min(0.75_f32);
    let net = positive - negative;
    ShopPlanComponentScoreV1 {
        positive,
        negative,
        net,
        confidence,
        explanation: format!(
            "component shadow score positive={positive:.1} negative={negative:.1} net={net:.1}"
        ),
    }
}
