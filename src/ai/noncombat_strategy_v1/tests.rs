use crate::content::cards::CardId;

use super::{
    build_run_strategy_snapshot_v1, candidate_plan_delta_v1, StrategyCandidateFactsV1,
    StrategyDeckFactsV1, StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1,
    StrategyPlanEffectV1, StrategyPlanIdV1, StrategyPlanSupportV1, StrategyRouteFutureV1,
    StrategyRoutePackageIdV1,
};

#[test]
fn run_strategy_snapshot_keeps_multiple_plan_hypotheses() {
    let snapshot = build_run_strategy_snapshot_v1(
        StrategyDeckFactsV1 {
            deck_size: 11,
            attacks: 7,
            skills: 4,
            powers: 0,
            starter_strikes: 5,
            starter_defends: 4,
            strength_sources: 1,
            strength_payoffs: 0,
            weak_sources: 0,
            draw_sources: 0,
            energy_sources: 0,
            vulnerable_sources: 1,
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
            attacks: 6,
            skills: 4,
            powers: 0,
            starter_strikes: 5,
            starter_defends: 4,
            strength_sources: 0,
            strength_payoffs: 0,
            weak_sources: 0,
            draw_sources: 0,
            energy_sources: 0,
            vulnerable_sources: 1,
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

#[test]
fn starter_shell_formation_marks_frontload_and_scaling_needs() {
    let snapshot = build_run_strategy_snapshot_v1(
        StrategyDeckFactsV1 {
            deck_size: 10,
            attacks: 6,
            skills: 4,
            powers: 0,
            starter_strikes: 5,
            starter_defends: 4,
            strength_sources: 0,
            strength_payoffs: 0,
            weak_sources: 0,
            draw_sources: 0,
            energy_sources: 0,
            vulnerable_sources: 1,
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
            min_fires: 1,
            max_fires: 2,
            first_fire_floor: Some(6),
            max_early_pressure: 3,
            need_heal: 0.4,
            avoid_damage: 0.5,
        }),
    );

    assert_eq!(
        snapshot.formation.stage,
        StrategyDeckFormationStageV1::StarterShell
    );
    assert!(snapshot
        .formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Frontload));
    assert!(snapshot
        .formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Scaling));
}

#[test]
fn supported_engine_formation_marks_plan_committed() {
    let snapshot = build_run_strategy_snapshot_v1(
        StrategyDeckFactsV1 {
            deck_size: 14,
            attacks: 8,
            skills: 4,
            powers: 2,
            starter_strikes: 4,
            starter_defends: 3,
            strength_sources: 2,
            strength_payoffs: 1,
            weak_sources: 1,
            draw_sources: 1,
            energy_sources: 1,
            vulnerable_sources: 1,
            route_upgrade_payoffs: 0,
            important_cards_unupgraded: 0,
            exhaust_generators: 0,
            exhaust_payoffs: 0,
            status_generators: 0,
            status_payoffs: 0,
            total_attack_damage: 86,
            total_block: 32,
        },
        Some(StrategyRouteFutureV1 {
            min_fires: 2,
            max_fires: 3,
            first_fire_floor: Some(5),
            max_early_pressure: 1,
            need_heal: 0.1,
            avoid_damage: 0.1,
        }),
    );

    assert_eq!(
        snapshot.formation.stage,
        StrategyDeckFormationStageV1::PlanCommitted
    );
    assert!(snapshot
        .formation
        .strengths
        .contains(&StrategyPlanIdV1::StrengthScaling));
    assert!(!snapshot
        .formation
        .needs
        .contains(&StrategyDeckFormationNeedV1::Scaling));
}

#[test]
fn route_packages_link_upgrade_commitment_to_visible_fire_budget() {
    let snapshot = build_run_strategy_snapshot_v1(
        StrategyDeckFactsV1 {
            deck_size: 10,
            attacks: 6,
            skills: 4,
            powers: 0,
            starter_strikes: 5,
            starter_defends: 4,
            strength_sources: 0,
            strength_payoffs: 0,
            weak_sources: 0,
            draw_sources: 0,
            energy_sources: 0,
            vulnerable_sources: 1,
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

    assert_eq!(
        snapshot
            .route_package(StrategyRoutePackageIdV1::UpgradeCommitment)
            .map(|package| package.support),
        Some(StrategyPlanSupportV1::Strong)
    );
}

#[test]
fn route_packages_mark_combat_patch_window_under_pressure() {
    let snapshot = build_run_strategy_snapshot_v1(
        StrategyDeckFactsV1 {
            deck_size: 10,
            attacks: 6,
            skills: 4,
            powers: 0,
            starter_strikes: 5,
            starter_defends: 4,
            strength_sources: 0,
            strength_payoffs: 0,
            weak_sources: 0,
            draw_sources: 0,
            energy_sources: 0,
            vulnerable_sources: 1,
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
            min_fires: 1,
            max_fires: 2,
            first_fire_floor: Some(6),
            max_early_pressure: 3,
            need_heal: 0.4,
            avoid_damage: 0.5,
        }),
    );

    assert_eq!(
        snapshot
            .route_package(StrategyRoutePackageIdV1::CombatPatchWindow)
            .map(|package| package.support),
        Some(StrategyPlanSupportV1::Strong)
    );
}

#[test]
fn route_packages_protect_committed_core_plan() {
    let snapshot = build_run_strategy_snapshot_v1(
        StrategyDeckFactsV1 {
            deck_size: 14,
            attacks: 8,
            skills: 4,
            powers: 2,
            starter_strikes: 4,
            starter_defends: 3,
            strength_sources: 2,
            strength_payoffs: 1,
            weak_sources: 1,
            draw_sources: 1,
            energy_sources: 1,
            vulnerable_sources: 1,
            route_upgrade_payoffs: 0,
            important_cards_unupgraded: 0,
            exhaust_generators: 0,
            exhaust_payoffs: 0,
            status_generators: 0,
            status_payoffs: 0,
            total_attack_damage: 86,
            total_block: 32,
        },
        Some(StrategyRouteFutureV1 {
            min_fires: 2,
            max_fires: 3,
            first_fire_floor: Some(5),
            max_early_pressure: 1,
            need_heal: 0.1,
            avoid_damage: 0.1,
        }),
    );

    assert_eq!(
        snapshot
            .route_package(StrategyRoutePackageIdV1::CorePlanProtection)
            .map(|package| package.support),
        Some(StrategyPlanSupportV1::Strong)
    );
}

#[test]
fn strategy_snapshot_v2_unifies_archetype_route_and_resource_packages() {
    let snapshot = super::build_run_strategy_snapshot_v2(
        StrategyDeckFactsV1 {
            deck_size: 14,
            attacks: 8,
            skills: 4,
            powers: 2,
            starter_strikes: 4,
            starter_defends: 3,
            strength_sources: 2,
            strength_payoffs: 1,
            weak_sources: 1,
            draw_sources: 1,
            energy_sources: 1,
            vulnerable_sources: 1,
            route_upgrade_payoffs: 0,
            important_cards_unupgraded: 0,
            exhaust_generators: 0,
            exhaust_payoffs: 0,
            status_generators: 0,
            status_payoffs: 0,
            total_attack_damage: 86,
            total_block: 32,
        },
        Some(StrategyRouteFutureV1 {
            min_fires: 2,
            max_fires: 3,
            first_fire_floor: Some(5),
            max_early_pressure: 1,
            need_heal: 0.1,
            avoid_damage: 0.1,
        }),
        None,
    );

    assert_eq!(
        snapshot
            .package(super::StrategyPackageIdV2::StrengthScaling)
            .map(|package| package.domain),
        Some(super::StrategyPackageDomainV2::Archetype)
    );
    assert_eq!(
        snapshot
            .package(super::StrategyPackageIdV2::CorePlanProtection)
            .map(|package| package.domain),
        Some(super::StrategyPackageDomainV2::Route)
    );
    assert_eq!(
        snapshot
            .package(super::StrategyPackageIdV2::HpSafety)
            .map(|package| package.domain),
        Some(super::StrategyPackageDomainV2::Resource)
    );
    assert!(snapshot.packages.len() >= snapshot.v1.plans.len() + snapshot.v1.route_packages.len());
}

#[test]
fn strategy_snapshot_v2_from_run_state_tracks_resource_facts() {
    let mut run_state = crate::state::run::RunState::new(123, 0, false, "Ironclad");
    run_state.current_hp = 20;
    run_state.max_hp = 80;
    run_state.gold = 160;
    run_state.add_card_to_deck_without_interception_from(
        crate::content::cards::CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    run_state.potions[0] = Some(crate::content::potions::Potion::new(
        crate::content::potions::PotionId::FirePotion,
        7,
    ));

    let snapshot = super::build_run_strategy_snapshot_from_run_state_v2(&run_state);

    assert_eq!(snapshot.resources.current_hp, 20);
    assert_eq!(snapshot.resources.max_hp, 80);
    assert_eq!(snapshot.resources.gold, 160);
    assert_eq!(snapshot.resources.curses, 1);
    assert_eq!(snapshot.resources.potion_slots, 3);
    assert_eq!(snapshot.resources.potion_count, 1);
    assert_eq!(snapshot.resources.empty_potion_slots, 2);
    assert_eq!(
        snapshot
            .package(super::StrategyPackageIdV2::HpSafety)
            .map(|package| package.support),
        Some(StrategyPlanSupportV1::Strong)
    );
    assert_eq!(
        snapshot
            .package(super::StrategyPackageIdV2::ShopRemoveWindow)
            .map(|package| package.support),
        Some(StrategyPlanSupportV1::Strong)
    );
}
