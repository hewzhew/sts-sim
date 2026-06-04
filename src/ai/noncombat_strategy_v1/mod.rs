mod candidate;
mod snapshot;
mod types;

pub use candidate::candidate_plan_delta_v1;
pub use snapshot::build_run_strategy_snapshot_v1;
pub use types::{
    DeckPlanHypothesisV1, RunStrategySnapshotV1, StrategyCandidateFactsV1,
    StrategyCandidatePlanDeltaV1, StrategyDeckFactsV1, StrategyPlanEffectV1, StrategyPlanIdV1,
    StrategyPlanPressureV1, StrategyPlanSupportV1, StrategyRouteFutureV1,
};

#[cfg(test)]
mod tests {
    use crate::content::cards::CardId;

    use super::{
        build_run_strategy_snapshot_v1, candidate_plan_delta_v1, StrategyCandidateFactsV1,
        StrategyDeckFactsV1, StrategyPlanEffectV1, StrategyPlanIdV1, StrategyPlanSupportV1,
        StrategyRouteFutureV1,
    };

    #[test]
    fn run_strategy_snapshot_keeps_multiple_plan_hypotheses() {
        let snapshot = build_run_strategy_snapshot_v1(
            StrategyDeckFactsV1 {
                deck_size: 11,
                strength_sources: 1,
                strength_payoffs: 0,
                weak_sources: 0,
                route_upgrade_payoffs: 0,
                important_cards_unupgraded: 1,
                exhaust_generators: 0,
                exhaust_payoffs: 0,
                status_generators: 0,
                status_payoffs: 0,
                total_attack_damage: 42,
                total_block: 20,
            },
            Some(StrategyRouteFutureV1 {
                min_fires: 2,
                max_fires: 4,
                first_fire_floor: Some(5),
                max_early_pressure: 2,
                need_heal: 0.2,
                avoid_damage: 0.3,
            }),
        );

        assert_eq!(
            snapshot
                .plan(StrategyPlanIdV1::StrengthScaling)
                .map(|plan| plan.support),
            Some(StrategyPlanSupportV1::Strong)
        );
        assert_eq!(
            snapshot
                .plan(StrategyPlanIdV1::WeakControl)
                .map(|plan| plan.support),
            Some(StrategyPlanSupportV1::Plausible)
        );
        assert!(snapshot.plans.len() >= 6);
    }

    #[test]
    fn candidate_plan_delta_uses_strategy_snapshot_not_card_score() {
        let snapshot = build_run_strategy_snapshot_v1(
            StrategyDeckFactsV1 {
                deck_size: 10,
                strength_sources: 0,
                strength_payoffs: 0,
                weak_sources: 0,
                route_upgrade_payoffs: 0,
                important_cards_unupgraded: 1,
                exhaust_generators: 0,
                exhaust_payoffs: 0,
                status_generators: 0,
                status_payoffs: 0,
                total_attack_damage: 36,
                total_block: 15,
            },
            Some(StrategyRouteFutureV1 {
                min_fires: 3,
                max_fires: 4,
                first_fire_floor: Some(4),
                max_early_pressure: 1,
                need_heal: 0.0,
                avoid_damage: 0.1,
            }),
        );

        let searing = candidate_plan_delta_v1(
            StrategyCandidateFactsV1 {
                card: CardId::SearingBlow,
                damage_total: 12,
                weak: 0,
                strength_gain: 0,
            },
            &snapshot,
        );
        let clothesline = candidate_plan_delta_v1(
            StrategyCandidateFactsV1 {
                card: CardId::Clothesline,
                damage_total: 12,
                weak: 2,
                strength_gain: 0,
            },
            &snapshot,
        );

        assert!(searing.effects.contains(&StrategyPlanEffectV1::UpgradeSink));
        assert!(clothesline
            .effects
            .contains(&StrategyPlanEffectV1::WeakCoverage));
        assert_eq!(searing.support, StrategyPlanSupportV1::Strong);
    }
}
