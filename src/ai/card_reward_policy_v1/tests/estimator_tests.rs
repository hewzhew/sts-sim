use super::*;

#[test]
fn strategy_package_estimator_exports_plan_alignment_without_certifying_autopick() {
    let context = context_for_cards_with_route(
        vec![
            RewardCard::new(CardId::SearingBlow, 0),
            RewardCard::new(CardId::Clothesline, 0),
        ],
        route_with_upgrade_budget(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let strategy_estimates = estimates_for_source(
        &decision.value_estimates,
        CardRewardValueSourceV1::StrategyPackage,
    );

    assert_eq!(strategy_estimates.len(), context.candidates.len());
    assert!(strategy_estimates
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::StrategyPackageEstimate));
    assert!(strategy_estimates
        .iter()
        .all(|estimate| estimate.eligibility.usable_for_value_estimate));
    assert!(strategy_estimates
        .iter()
        .all(|estimate| !estimate.eligibility.usable_for_autopilot_gate));
    assert!(strategy_estimates.iter().any(|estimate| {
        estimate.card == CardId::SearingBlow
            && estimate
                .components
                .iter()
                .any(|component| component.name == "plan_effect_UpgradeSink")
    }));
    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.decision_approval.is_none());
}

#[test]
fn strategy_package_estimator_recognizes_block_engine_payoff() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::Barricade);
    run_state.add_card_to_deck(CardId::Entrench);
    run_state.add_card_to_deck(CardId::FlameBarrier);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::BodySlam, 0),
            RewardCard::new(CardId::HeavyBlade, 0),
        ],
        route_with_combat_pressure(),
    );

    assert_eq!(
        context.strategy.support(StrategyPackageIdV2::BlockEngine),
        StrategyPlanSupportV1::Strong
    );

    let body_slam = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::BodySlam)
        .expect("Body Slam candidate");
    assert_eq!(body_slam.plan_delta.support, StrategyPlanSupportV1::Strong);
    assert!(body_slam
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::BlockPayoff));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let body_slam_strategy_estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BodySlam
        })
        .expect("Body Slam strategy package estimate");

    assert!(body_slam_strategy_estimate.progress_delta > 0.0);
    assert!(body_slam_strategy_estimate
        .components
        .iter()
        .any(|component| component.name == "plan_effect_BlockPayoff"));
    assert!(body_slam_strategy_estimate
        .components
        .iter()
        .any(|component| component.name == "strategy_support_block_engine"));
    assert!(body_slam_strategy_estimate
        .components
        .iter()
        .any(|component| {
            component.name == "strategy_gap_block_engine_block_payoff_filled"
                && component.value > 0.0
        }));
    assert!(body_slam_strategy_estimate
        .components
        .iter()
        .any(|component| {
            component.name == "strategy_package_completion_block_engine" && component.value > 0.0
        }));
    assert!(
        !body_slam_strategy_estimate
            .eligibility
            .usable_for_autopilot_gate
    );
    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
}

#[test]
fn block_engine_completion_aligns_with_long_fight_boss_threats() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.boss_key = Some(EncounterId::TimeEater);
    run_state.add_card_to_deck(CardId::Barricade);
    run_state.add_card_to_deck(CardId::Entrench);
    run_state.add_card_to_deck(CardId::FlameBarrier);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BodySlam, 0)],
        route_without_elites(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BodySlam
        })
        .expect("Body Slam strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_block_engine" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_block_engine_boss_high_incoming"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_block_engine_boss_long_fight"
            && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn block_engine_completion_aligns_with_act2_elites_only_when_route_allows_elites() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.add_card_to_deck(CardId::Barricade);
    run_state.add_card_to_deck(CardId::Entrench);
    run_state.add_card_to_deck(CardId::FlameBarrier);

    let with_elites = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BodySlam, 0)],
        route_with_combat_pressure(),
    );
    let no_elites = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BodySlam, 0)],
        route_without_elites(),
    );

    let with_elites_decision =
        plan_card_reward_decision_v1(&with_elites, &CardRewardPolicyConfigV1::default());
    let no_elites_decision =
        plan_card_reward_decision_v1(&no_elites, &CardRewardPolicyConfigV1::default());
    let with_elites_estimate = with_elites_decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BodySlam
        })
        .expect("Body Slam strategy package estimate with elites");
    let no_elites_estimate = no_elites_decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BodySlam
        })
        .expect("Body Slam strategy package estimate without elites");

    assert!(with_elites_estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_block_engine_elite_high_incoming"
            && component.value > 0.0
    }));
    assert!(with_elites_estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_block_engine_elite_multihit"
            && component.value > 0.0
    }));
    assert!(!no_elites_estimate.components.iter().any(|component| {
        component
            .name
            .starts_with("strategy_threat_alignment_block_engine_elite_")
    }));
}

#[test]
fn strategy_package_estimator_blocks_naked_body_slam() {
    let context = context_for_cards_with_route(
        vec![RewardCard::new(CardId::BodySlam, 0)],
        route_with_combat_pressure(),
    );
    let body_slam = &context.candidates[0];

    assert_eq!(
        context.strategy.support(StrategyPackageIdV2::BlockEngine),
        StrategyPlanSupportV1::Blocked
    );
    assert_eq!(body_slam.plan_delta.support, StrategyPlanSupportV1::Blocked);
    assert!(body_slam
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::BlockPayoff));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let body_slam_strategy_estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BodySlam
        })
        .expect("Body Slam strategy package estimate");

    assert!(body_slam_strategy_estimate.progress_delta <= 0.0);
    assert!(body_slam_strategy_estimate.deck_consistency_delta <= 0.0);
    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
}

#[test]
fn strategy_package_estimator_recognizes_block_engine_missing_pieces() {
    let mut needs_retention = RunState::new(521, 0, false, "Ironclad");
    needs_retention.add_card_to_deck(CardId::BodySlam);
    needs_retention.add_card_to_deck(CardId::Entrench);
    needs_retention.add_card_to_deck(CardId::FlameBarrier);
    let barricade_context = context_for_run_with_route(
        &needs_retention,
        vec![RewardCard::new(CardId::Barricade, 0)],
        route_with_combat_pressure(),
    );
    let barricade = &barricade_context.candidates[0];

    assert_eq!(
        barricade_context
            .strategy
            .support(StrategyPackageIdV2::BlockEngine),
        StrategyPlanSupportV1::Plausible
    );
    assert_eq!(
        barricade.plan_delta.support,
        StrategyPlanSupportV1::Plausible
    );
    assert!(barricade
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::BlockRetention));

    let mut needs_multiplier = RunState::new(521, 0, false, "Ironclad");
    needs_multiplier.add_card_to_deck(CardId::Barricade);
    needs_multiplier.add_card_to_deck(CardId::BodySlam);
    needs_multiplier.add_card_to_deck(CardId::FlameBarrier);
    let entrench_context = context_for_run_with_route(
        &needs_multiplier,
        vec![RewardCard::new(CardId::Entrench, 0)],
        route_with_combat_pressure(),
    );
    let entrench = &entrench_context.candidates[0];

    assert_eq!(
        entrench_context
            .strategy
            .support(StrategyPackageIdV2::BlockEngine),
        StrategyPlanSupportV1::Strong
    );
    assert_eq!(entrench.plan_delta.support, StrategyPlanSupportV1::Strong);
    assert!(entrench
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::BlockMultiplier));
}

#[test]
fn strategy_package_estimator_exports_exhaust_engine_roles() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::FeelNoPain);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BurningPact, 0)],
        route_with_combat_pressure(),
    );

    assert_eq!(
        context.strategy.support(StrategyPackageIdV2::ExhaustEngine),
        StrategyPlanSupportV1::Plausible
    );
    assert!(context.candidates[0]
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::ExhaustGenerator));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BurningPact
        })
        .expect("Burning Pact strategy package estimate");

    assert!(estimate.progress_delta > 0.0);
    assert!(estimate
        .components
        .iter()
        .any(|component| component.name == "strategy_support_exhaust_engine"));
    assert!(estimate
        .components
        .iter()
        .any(|component| component.name == "plan_effect_ExhaustGenerator"));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_exhaust_engine_generator_filled" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_exhaust_engine" && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn exhaust_engine_completion_aligns_with_status_flood_threats() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.boss_key = Some(EncounterId::Hexaghost);
    run_state.add_card_to_deck(CardId::FeelNoPain);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BurningPact, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BurningPact
        })
        .expect("Burning Pact strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_exhaust_engine" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_exhaust_engine_boss_status_flood"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_exhaust_engine_elite_status_flood"
            && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn exhaust_engine_completion_does_not_take_elite_alignment_without_elite_route() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.boss_key = Some(EncounterId::Hexaghost);
    run_state.add_card_to_deck(CardId::FeelNoPain);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BurningPact, 0)],
        route_without_elites(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::BurningPact
        })
        .expect("Burning Pact strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_exhaust_engine_boss_status_flood"
            && component.value > 0.0
    }));
    assert!(!estimate.components.iter().any(|component| {
        component
            .name
            .starts_with("strategy_threat_alignment_exhaust_engine_elite_")
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn strategy_package_estimator_exports_status_package_roles() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::PowerThrough);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Evolve, 0)],
        route_with_combat_pressure(),
    );

    assert_eq!(
        context.strategy.support(StrategyPackageIdV2::StatusPackage),
        StrategyPlanSupportV1::Plausible
    );
    assert!(context.candidates[0]
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::StatusPayoff));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Evolve
        })
        .expect("Evolve strategy package estimate");

    assert!(estimate.progress_delta > 0.0);
    assert!(estimate
        .components
        .iter()
        .any(|component| component.name == "strategy_support_status_package"));
    assert!(estimate
        .components
        .iter()
        .any(|component| component.name == "plan_effect_StatusPayoff"));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_status_package_payoff_filled" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_status_package" && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn status_package_completion_aligns_with_status_flood_and_aoe_threats() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.boss_key = Some(EncounterId::SlimeBoss);
    run_state.add_card_to_deck(CardId::PowerThrough);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Evolve, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Evolve
        })
        .expect("Evolve strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_status_package" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_status_package_boss_status_flood"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_status_package_boss_aoe"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_status_package_elite_status_flood"
            && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn status_package_completion_does_not_take_elite_alignment_without_elite_route() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.boss_key = Some(EncounterId::SlimeBoss);
    run_state.add_card_to_deck(CardId::PowerThrough);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Evolve, 0)],
        route_without_elites(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Evolve
        })
        .expect("Evolve strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_status_package_boss_status_flood"
            && component.value > 0.0
    }));
    assert!(!estimate.components.iter().any(|component| {
        component
            .name
            .starts_with("strategy_threat_alignment_status_package_elite_")
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn route_risk_estimator_values_frontload_more_under_early_route_pressure() {
    let context = context_for_cards_with_route(
        vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Warcry, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let twin = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::RouteRisk
                && estimate.card == CardId::TwinStrike
        })
        .expect("Twin Strike should have a route risk estimate");
    let warcry = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::RouteRisk && estimate.card == CardId::Warcry
        })
        .expect("Warcry should have a route risk estimate");

    assert!(twin.survival_delta > warcry.survival_delta);
    assert!(twin
        .components
        .iter()
        .any(|component| component.name == "route_risk_pressure"));
}

#[test]
fn public_combat_heuristic_values_enter_arbitration_without_certifying_autopick() {
    let context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Warcry, 0),
    ]);

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let combat_probe_estimates = estimates_for_source(
        &decision.value_estimates,
        CardRewardValueSourceV1::PublicCombatHeuristic,
    );

    assert_eq!(combat_probe_estimates.len(), context.candidates.len());
    assert!(combat_probe_estimates
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::PublicCombatHeuristic));
    assert!(combat_probe_estimates
        .iter()
        .all(|estimate| estimate.eligibility.usable_for_value_estimate));
    assert!(combat_probe_estimates
        .iter()
        .all(|estimate| !estimate.eligibility.usable_for_autopilot_gate));
    assert!(decision
        .value_arbitration
        .candidate_reports
        .iter()
        .all(|report| {
            report.selected_source == Some(CardRewardValueSourceV1::PublicCombatHeuristic)
        }));
    assert!(!decision.autopilot_gate.value_source_eligible);
    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
}

#[test]
fn public_combat_heuristic_suppresses_elite_encounter_responses_when_route_has_no_elites() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    run_state.boss_key = Some(EncounterId::Collector);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Disarm, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_without_elites(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let disarm = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Disarm
        })
        .expect("Disarm public combat heuristic estimate");

    assert!(!disarm.components.iter().any(|component| {
        component.name.starts_with("elite_encounter_") || component.name.starts_with("elite_pool_")
    }));
}
