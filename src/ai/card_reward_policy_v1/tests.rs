use crate::ai::card_reward_policy_v1::{
    arbitrate_card_reward_value_estimates_v1, build_card_reward_decision_context_v1,
    card_reward_semantic_profile_v1, plan_card_reward_decision_v1,
    plan_card_reward_decision_with_estimator_inputs_v1, replay_card_reward_decision_v1,
    replay_card_reward_decision_with_estimator_inputs_v1, CardRewardEstimatorInputsV1,
    CardRewardEvidenceGapV1, CardRewardPlanEffectV1, CardRewardPolicyActionV1,
    CardRewardPolicyConfigV1, CardRewardSemanticRoleV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1, PublicRewardDecisionPacketV1,
};
use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::types::{CardRewardRouteEvidenceV1, CardRewardSelectedRouteV1};

#[test]
fn card_facts_are_mechanical_and_do_not_contain_pick_value() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::TwinStrike, 0)],
        None,
    );

    let candidate = &context.candidates[0];

    assert_eq!(candidate.facts.card, CardId::TwinStrike);
    assert_eq!(candidate.facts.damage.damage_per_hit, 5);
    assert_eq!(candidate.facts.damage.hit_count, 2);
    assert_eq!(candidate.facts.damage.total_damage, 10);
    assert!(candidate.facts.pick_dependencies.is_empty());
    assert!(candidate.impact.certification_blockers.is_empty());
}

#[test]
fn semantic_profile_exports_roles_without_card_name_scoring() {
    let body_slam = card_reward_semantic_profile_v1(&RewardCard::new(CardId::BodySlam, 0));
    let barricade = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Barricade, 0));
    let entrench = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Entrench, 0));
    let demon_form = card_reward_semantic_profile_v1(&RewardCard::new(CardId::DemonForm, 0));
    let twin_strike = card_reward_semantic_profile_v1(&RewardCard::new(CardId::TwinStrike, 0));

    assert!(body_slam
        .roles
        .contains(&CardRewardSemanticRoleV1::BlockPayoff));
    assert!(barricade
        .roles
        .contains(&CardRewardSemanticRoleV1::BlockPayoff));
    assert!(entrench
        .roles
        .contains(&CardRewardSemanticRoleV1::BlockPayoff));
    assert!(demon_form
        .roles
        .contains(&CardRewardSemanticRoleV1::ScalingSource));
    assert!(twin_strike
        .roles
        .contains(&CardRewardSemanticRoleV1::FrontloadDamage));
    assert!(!twin_strike
        .roles
        .contains(&CardRewardSemanticRoleV1::PackagePayoff));
    assert_eq!(twin_strike.name, "Twin Strike");
}

#[test]
fn decision_context_marks_missing_route_evidence_as_policy_gap_not_card_fact() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::Shockwave, 0)],
        None,
    );

    assert!(context.route.is_none());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision
        .evidence_gaps
        .contains(&CardRewardEvidenceGapV1::MissingRouteEvidence));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn archetype_dependent_card_requires_matching_deck_evidence() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::HeavyBlade, 0)],
        None,
    );

    let candidate = &context.candidates[0];

    assert!(candidate
        .facts
        .pick_dependencies
        .contains(&crate::ai::card_reward_policy_v1::CardRewardPickDependencyV1::StrengthScaling));
    assert!(candidate
        .impact
        .certification_blockers
        .contains(&CardRewardEvidenceGapV1::UnsatisfiedStrengthScalingEvidence));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn complex_attack_rewards_export_plan_deltas_instead_of_magic_scores() {
    let context = context_for_cards_with_route(
        vec![
            RewardCard::new(CardId::SearingBlow, 0),
            RewardCard::new(CardId::HeavyBlade, 0),
            RewardCard::new(CardId::Clothesline, 0),
        ],
        route_with_upgrade_budget(),
    );

    let searing = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::SearingBlow)
        .expect("Searing Blow candidate");
    let heavy = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::HeavyBlade)
        .expect("Heavy Blade candidate");
    let clothesline = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::Clothesline)
        .expect("Clothesline candidate");

    assert!(searing
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::UpgradeSink));
    assert!(heavy
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::StrengthPayoff));
    assert!(clothesline
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::WeakCoverage));
    assert!(clothesline
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::FrontloadDamage));
}

#[test]
fn card_reward_context_uses_strategy_snapshot_v2_packages() {
    let context = context_for_cards_with_route(
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        route_with_upgrade_budget(),
    );

    assert_eq!(
        context
            .strategy
            .support(StrategyPackageIdV2::UpgradeCommitment),
        StrategyPlanSupportV1::Strong
    );
    assert_eq!(
        context
            .strategy
            .support(StrategyPackageIdV2::PotionCapacity),
        StrategyPlanSupportV1::Strong
    );
}

#[test]
fn noncombat_record_exports_card_reward_plan_evidence() {
    let context = context_for_cards_with_route(
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        route_with_upgrade_budget(),
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    let record = decision.to_noncombat_decision_record_v1();

    let evidence = record
        .evidence
        .items
        .iter()
        .find(|item| item.candidate_id.as_deref() == Some("card_reward:0:SearingBlow"))
        .expect("Searing Blow evidence item");
    assert!(evidence
        .components
        .iter()
        .any(|component| component.name == "plan_effect_UpgradeSink"));
    assert!(record.candidates[0]
        .uncertainty_notes
        .iter()
        .any(|note| note == "plan effect: UpgradeSink"));
}

#[test]
fn searing_blow_exports_upgrade_commitment_but_uncalibrated_gate_stops() {
    let context = context_for_cards_with_route(
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        route_with_upgrade_budget(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert_eq!(
        context
            .strategy
            .support(StrategyPackageIdV2::UpgradeCommitment),
        StrategyPlanSupportV1::Strong
    );
    assert_eq!(
        estimates_for_source(
            &decision.value_estimates,
            CardRewardValueSourceV1::UncalibratedImpactPrior
        )
        .len(),
        1
    );
    let route_risk = estimates_for_source(
        &decision.value_estimates,
        CardRewardValueSourceV1::RouteRisk,
    );
    assert_eq!(route_risk.len(), 1);
    assert!(route_risk
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::RouteRiskEstimate));
    assert!(route_risk
        .iter()
        .all(|estimate| !estimate.eligibility.usable_for_autopilot_gate));
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::SearingBlow
        })
        .expect("Searing Blow strategy package estimate");
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_upgrade_sink_consumer_filled" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_upgrade_sink" && component.value > 0.0
    }));
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn arbitration_keeps_strategy_package_completion_ahead_of_shallow_route_risk() {
    let context = context_for_cards_with_route(
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        route_with_upgrade_budget(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let report = decision
        .value_arbitration
        .candidate_reports
        .iter()
        .find(|report| report.index == 0)
        .expect("candidate arbitration report");

    assert_eq!(
        report.selected_source,
        Some(CardRewardValueSourceV1::StrategyPackage)
    );
    assert!(!report.selected_estimate_gate_eligible);
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn searing_blow_does_not_complete_upgrade_package_without_route_budget() {
    let context = context_for_cards_with_route(
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        route_with_combat_pressure(),
    );

    assert_eq!(
        context.strategy.support(StrategyPackageIdV2::UpgradeSink),
        StrategyPlanSupportV1::Weak
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::SearingBlow
        })
        .expect("Searing Blow strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_upgrade_sink_consumer_filled" && component.value > 0.0
    }));
    assert!(!estimate
        .components
        .iter()
        .any(|component| component.name == "strategy_package_completion_upgrade_sink"));
}

#[test]
fn heavy_blade_exports_strength_plan_but_uncalibrated_gate_stops() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::Inflame);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::HeavyBlade, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert_eq!(
        context
            .strategy
            .support(StrategyPackageIdV2::StrengthScaling),
        StrategyPlanSupportV1::Strong
    );
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::HeavyBlade
        })
        .expect("Heavy Blade strategy package estimate");
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_strength_scaling_payoff_filled" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_strength_scaling" && component.value > 0.0
    }));
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn strength_payoff_completion_aligns_with_long_fight_boss_and_elite_threats() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.boss_key = Some(EncounterId::TimeEater);
    run_state.add_card_to_deck(CardId::Inflame);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::HeavyBlade, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::HeavyBlade
        })
        .expect("Heavy Blade strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_strength_scaling" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_strength_scaling_boss_long_fight"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_strength_scaling_boss_high_incoming"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_strength_scaling_elite_long_fight"
            && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn strength_generator_completion_does_not_take_elite_alignment_without_elite_route() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.boss_key = Some(EncounterId::TimeEater);
    run_state.add_card_to_deck(CardId::HeavyBlade);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Inflame, 0)],
        route_without_elites(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Inflame
        })
        .expect("Inflame strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_strength_scaling" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_strength_scaling_boss_long_fight"
            && component.value > 0.0
    }));
    assert!(!estimate.components.iter().any(|component| {
        component
            .name
            .starts_with("strategy_threat_alignment_strength_scaling_elite_")
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn strategy_package_estimator_recognizes_strength_generator_completion() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::HeavyBlade);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Inflame, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Inflame
        })
        .expect("Inflame strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_strength_scaling_generator_filled" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_strength_scaling" && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn clothesline_exports_weak_frontload_patch_but_uncalibrated_gate_stops() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.floor_num = 1;
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::SearingBlow, 0),
            RewardCard::new(CardId::HeavyBlade, 0),
            RewardCard::new(CardId::Clothesline, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert_eq!(
        context
            .strategy
            .support(StrategyPackageIdV2::CombatPatchWindow),
        StrategyPlanSupportV1::Strong
    );
    assert!(decision
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::Clothesline)
        .expect("Clothesline candidate")
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::WeakCoverage));
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Clothesline
        })
        .expect("Clothesline strategy package estimate");
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_gap_weak_control_generator_filled" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_weak_control" && component.value > 0.0
    }));
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn weak_control_completion_aligns_with_act2_multihit_elite_threats() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Clothesline, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Clothesline
        })
        .expect("Clothesline strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_weak_control" && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_weak_control_elite_high_incoming"
            && component.value > 0.0
    }));
    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_threat_alignment_weak_control_elite_multihit"
            && component.value > 0.0
    }));
    assert!(!estimate.eligibility.usable_for_autopilot_gate);
}

#[test]
fn weak_control_completion_ignores_elite_threats_when_route_has_no_elites() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Clothesline, 0)],
        route_without_elites(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let estimate = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::StrategyPackage
                && estimate.card == CardId::Clothesline
        })
        .expect("Clothesline strategy package estimate");

    assert!(estimate.components.iter().any(|component| {
        component.name == "strategy_package_completion_weak_control" && component.value > 0.0
    }));
    assert!(!estimate.components.iter().any(|component| {
        component
            .name
            .starts_with("strategy_threat_alignment_weak_control_elite_")
    }));
}

#[test]
fn early_transition_attack_exports_frontload_patch_but_uncalibrated_gate_stops() {
    let mut run_state = RunState::new(1552366907, 0, false, "Ironclad");
    run_state.floor_num = 1;
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::SwordBoomerang, 0),
            RewardCard::new(CardId::Warcry, 0),
        ],
        route_with_combat_pressure(),
    );

    let sword_boomerang = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::SwordBoomerang)
        .expect("Sword Boomerang candidate");
    assert!(sword_boomerang.facts.is_random_output);
    assert!(sword_boomerang
        .impact
        .certification_blockers
        .contains(&CardRewardEvidenceGapV1::RandomOutcomeRequiresPolicy));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert!(decision
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::TwinStrike)
        .expect("Twin Strike candidate")
        .plan_delta
        .effects
        .contains(&CardRewardPlanEffectV1::FrontloadDamage));
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn transition_attack_value_gate_stops_when_multiple_deterministic_attacks_match() {
    let mut run_state = RunState::new(1552366907, 0, false, "Ironclad");
    run_state.floor_num = 1;
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::PommelStrike, 0),
            RewardCard::new(CardId::Warcry, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn multi_debuff_control_exports_combat_control_but_uncalibrated_gate_stops() {
    let mut run_state = RunState::new(1552366907, 0, false, "Ironclad");
    run_state.floor_num = 1;
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Shockwave, 0),
            RewardCard::new(CardId::Clash, 0),
            RewardCard::new(CardId::SeverSoul, 0),
        ],
        route_with_combat_pressure(),
    );

    let shockwave = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::Shockwave)
        .expect("Shockwave candidate");
    assert_eq!(shockwave.facts.weak, 3);
    assert_eq!(shockwave.facts.vulnerable, 3);
    assert_eq!(shockwave.facts.enemy_strength_down, 3);

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn weak_frontload_patch_does_not_auto_certify_when_core_plan_is_committed() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.floor_num = 1;
    run_state.add_card_to_deck(CardId::Inflame);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Clothesline, 0)],
        route_with_combat_pressure(),
    );

    assert!(context
        .strategy
        .has_formation_strength(StrategyPackageIdV2::StrengthScaling));
    assert_eq!(
        context
            .strategy
            .support(StrategyPackageIdV2::CorePlanProtection),
        StrategyPlanSupportV1::Strong
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn score_threshold_overrides_cannot_force_a_pick_without_value_gate() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        None,
    );
    let decision = plan_card_reward_decision_v1(
        &context,
        &CardRewardPolicyConfigV1 {
            allow_autopilot_value_gate: true,
        },
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn decision_builds_prior_value_estimates_for_every_candidate() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Shockwave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    let priors = estimates_for_source(
        &decision.value_estimates,
        CardRewardValueSourceV1::UncalibratedImpactPrior,
    );
    assert_eq!(priors.len(), context.candidates.len());
    assert!(priors
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::UncalibratedPrior));
    assert!(priors.iter().all(|estimate| {
        estimate.survival_delta == 0.0
            && estimate.progress_delta == 0.0
            && estimate.deck_consistency_delta == 0.0
            && estimate.uncertainty == 1.0
    }));
    let route_risk = estimates_for_source(
        &decision.value_estimates,
        CardRewardValueSourceV1::RouteRisk,
    );
    assert_eq!(route_risk.len(), context.candidates.len());
    assert!(route_risk
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::RouteRiskEstimate));
    assert!(route_risk
        .iter()
        .all(|estimate| !estimate.eligibility.usable_for_autopilot_gate));
}

#[test]
fn route_risk_values_are_consumed_by_gate_but_cannot_certify_pick_without_promotion() {
    let mut context = context_for_cards(vec![RewardCard::new(CardId::Shockwave, 0)]);
    context.route = Some(route_with_combat_pressure());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert!(decision
        .evidence_gaps
        .contains(&CardRewardEvidenceGapV1::IneligibleValueSource));
    assert_eq!(
        decision.value_arbitration.gate_value_estimates[0].source,
        CardRewardValueSourceV1::StrategyPackage
    );
    assert_eq!(
        decision.value_arbitration.gate_value_estimates[0].status,
        CardRewardValueStatusV1::StrategyPackageEstimate
    );
    assert!(!decision.value_arbitration.candidate_reports[0].selected_estimate_gate_eligible);
}

#[test]
fn route_risk_blocks_even_when_old_rule_would_have_matched_without_promotion() {
    let mut run_state = RunState::new(1552366907, 0, false, "Ironclad");
    run_state.floor_num = 1;
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Shockwave, 0),
            RewardCard::new(CardId::Clash, 0),
            RewardCard::new(CardId::SeverSoul, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert_eq!(
        estimates_for_source(
            &decision.value_estimates,
            CardRewardValueSourceV1::UncalibratedImpactPrior
        )
        .len(),
        context.candidates.len()
    );
    assert!(decision
        .value_arbitration
        .candidate_reports
        .iter()
        .all(|report| !report.selected_estimate_gate_eligible));
    assert!(decision
        .autopilot_gate
        .blocked_reasons
        .contains(&CardRewardEvidenceGapV1::IneligibleValueSource));
    assert!(!decision.autopilot_gate.value_source_eligible);
}

#[test]
fn outcome_calibration_estimates_are_not_autopilot_eligible_without_arbitration_gate() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());
    let value_estimates = context
        .candidates
        .iter()
        .map(|candidate| CardRewardValueEstimateV1 {
            index: candidate.index,
            card: candidate.card,
            source: CardRewardValueSourceV1::OutcomeCalibration,
            status: CardRewardValueStatusV1::OutcomeCalibrated,
            survival_delta: if candidate.card == CardId::TwinStrike {
                2.0
            } else {
                0.5
            },
            progress_delta: 0.0,
            deck_consistency_delta: 0.0,
            uncertainty: 0.1,
            eligibility: Default::default(),
            components: Vec::new(),
        })
        .collect::<Vec<_>>();

    let (action, gate_report, gaps, certificate) = super::gate::pick_gate(
        &context,
        &value_estimates,
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(matches!(action, CardRewardPolicyActionV1::Stop { .. }));
    assert!(certificate.is_none());
    assert!(!gate_report.value_source_eligible);
    assert!(gate_report
        .blocked_reasons
        .contains(&CardRewardEvidenceGapV1::IneligibleValueSource));
    assert!(gaps.contains(&CardRewardEvidenceGapV1::IneligibleValueSource));
}

#[test]
fn outcome_calibration_gate_eligibility_is_estimate_level_not_source_level() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());
    let mut value_estimates = vec![
        test_value_estimate(
            0,
            CardId::TwinStrike,
            CardRewardValueSourceV1::OutcomeCalibration,
            CardRewardValueStatusV1::OutcomeCalibrated,
            2.0,
            0.1,
        ),
        test_value_estimate(
            1,
            CardId::Cleave,
            CardRewardValueSourceV1::OutcomeCalibration,
            CardRewardValueStatusV1::OutcomeCalibrated,
            0.5,
            0.1,
        ),
    ];
    value_estimates[0].eligibility.usable_for_autopilot_gate = true;
    value_estimates[0].eligibility.reasons.clear();
    value_estimates[1].eligibility.usable_for_autopilot_gate = true;
    value_estimates[1].eligibility.reasons.clear();

    let (action, gate_report, gaps, certificate) = super::gate::pick_gate(
        &context,
        &value_estimates,
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(gate_report.value_source_eligible);
    assert!(!gate_report
        .blocked_reasons
        .contains(&CardRewardEvidenceGapV1::IneligibleValueSource));
    assert!(gaps.is_empty());
    assert!(matches!(
        action,
        CardRewardPolicyActionV1::Pick {
            card: CardId::TwinStrike,
            ..
        }
    ));
    assert!(certificate.is_some());
}

#[test]
fn arbitration_reports_estimate_level_gate_eligibility_separately_from_source_allowlist() {
    let mut context = context_for_cards(vec![RewardCard::new(CardId::TwinStrike, 0)]);
    context.route = Some(route_with_combat_pressure());
    let mut value_estimates = vec![test_value_estimate(
        0,
        CardId::TwinStrike,
        CardRewardValueSourceV1::OutcomeCalibration,
        CardRewardValueStatusV1::OutcomeCalibrated,
        2.0,
        0.1,
    )];
    value_estimates[0].eligibility.usable_for_autopilot_gate = true;
    value_estimates[0].eligibility.reasons.clear();

    let arbitration = arbitrate_card_reward_value_estimates_v1(&context, &value_estimates);
    let report = arbitration
        .candidate_reports
        .first()
        .expect("candidate report should exist");

    assert!(!report.autopilot_source_eligible);
    assert!(report.selected_estimate_gate_eligible);
}

#[test]
fn arbitration_prefers_gate_eligible_estimate_over_higher_rank_non_gate_estimate_for_gate() {
    let mut context = context_for_cards(vec![RewardCard::new(CardId::TwinStrike, 0)]);
    context.route = Some(route_with_combat_pressure());
    let route_risk = test_value_estimate(
        0,
        CardId::TwinStrike,
        CardRewardValueSourceV1::RouteRisk,
        CardRewardValueStatusV1::PublicCombatHeuristic,
        10.0,
        0.2,
    );
    let mut outcome_calibration = test_value_estimate(
        0,
        CardId::TwinStrike,
        CardRewardValueSourceV1::OutcomeCalibration,
        CardRewardValueStatusV1::OutcomeCalibrated,
        1.0,
        0.2,
    );
    outcome_calibration.eligibility.usable_for_autopilot_gate = true;
    outcome_calibration.eligibility.reasons.clear();

    let arbitration =
        arbitrate_card_reward_value_estimates_v1(&context, &[route_risk, outcome_calibration]);
    let report = arbitration
        .candidate_reports
        .first()
        .expect("candidate report should exist");

    assert_eq!(
        arbitration.gate_value_estimates[0].source,
        CardRewardValueSourceV1::OutcomeCalibration
    );
    assert_eq!(
        report.selected_source,
        Some(CardRewardValueSourceV1::OutcomeCalibration)
    );
    assert!(report.selected_estimate_gate_eligible);
}

#[test]
fn estimator_arbitration_selects_one_gate_estimate_per_candidate_by_source_quality() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());
    let raw_estimates = vec![
        test_value_estimate(
            0,
            CardId::TwinStrike,
            CardRewardValueSourceV1::UncalibratedImpactPrior,
            CardRewardValueStatusV1::UncalibratedPrior,
            10.0,
            1.0,
        ),
        test_value_estimate(
            0,
            CardId::TwinStrike,
            CardRewardValueSourceV1::OutcomeCalibration,
            CardRewardValueStatusV1::OutcomeCalibrated,
            1.0,
            0.2,
        ),
        test_value_estimate(
            1,
            CardId::Cleave,
            CardRewardValueSourceV1::UncalibratedImpactPrior,
            CardRewardValueStatusV1::UncalibratedPrior,
            0.0,
            1.0,
        ),
    ];

    let arbitration = arbitrate_card_reward_value_estimates_v1(&context, &raw_estimates);

    assert_eq!(arbitration.input_estimate_count, 3);
    assert_eq!(arbitration.gate_value_estimates.len(), 2);
    assert_eq!(
        arbitration.gate_value_estimates[0].source,
        CardRewardValueSourceV1::OutcomeCalibration
    );
    assert_eq!(
        arbitration.gate_value_estimates[1].source,
        CardRewardValueSourceV1::UncalibratedImpactPrior
    );
    let twin_report = arbitration
        .candidate_reports
        .iter()
        .find(|report| report.index == 0)
        .expect("candidate 0 should have arbitration report");
    assert_eq!(twin_report.input_estimate_count, 2);
    assert_eq!(
        twin_report.selected_source,
        Some(CardRewardValueSourceV1::OutcomeCalibration)
    );
    assert!(twin_report.selected_for_gate);
    assert!(!twin_report.autopilot_source_eligible);
    assert_eq!(arbitration.label_role, "diagnostic_not_teacher_label");
}

#[test]
fn card_reward_policy_routes_value_estimates_through_arbitration() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert_eq!(
        estimates_for_source(
            &decision.value_estimates,
            CardRewardValueSourceV1::UncalibratedImpactPrior
        )
        .len(),
        2
    );
    assert_eq!(
        estimates_for_source(
            &decision.value_estimates,
            CardRewardValueSourceV1::StrategyPackage
        )
        .len(),
        2
    );
    assert_eq!(
        estimates_for_source(
            &decision.value_estimates,
            CardRewardValueSourceV1::RouteRisk
        )
        .len(),
        2
    );
    assert_eq!(decision.value_arbitration.input_estimate_count, 8);
    assert_eq!(decision.value_arbitration.gate_value_estimates.len(), 2);
    assert!(decision
        .value_arbitration
        .candidate_reports
        .iter()
        .all(|report| {
            report.selected_for_gate
                && report.selected_source == Some(CardRewardValueSourceV1::RouteRisk)
        }));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn policy_accepts_external_calibrated_estimates_before_arbitration_without_autopicking() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());
    let inputs = CardRewardEstimatorInputsV1 {
        external_value_estimates: vec![test_value_estimate(
            0,
            CardId::TwinStrike,
            CardRewardValueSourceV1::OutcomeCalibration,
            CardRewardValueStatusV1::OutcomeCalibrated,
            2.0,
            0.2,
        )],
    };

    let decision = plan_card_reward_decision_with_estimator_inputs_v1(
        &context,
        &CardRewardPolicyConfigV1::default(),
        &inputs,
    );

    assert_eq!(decision.value_estimates.len(), 9);
    assert_eq!(
        decision.value_arbitration.gate_value_estimates[0].source,
        CardRewardValueSourceV1::OutcomeCalibration
    );
    assert_eq!(
        decision.value_arbitration.gate_value_estimates[1].source,
        CardRewardValueSourceV1::RouteRisk
    );
    assert!(!decision.autopilot_gate.value_source_eligible);
    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn replay_harness_exports_value_loop_gate_state_without_selecting() {
    let mut context = context_for_cards(vec![RewardCard::new(CardId::Shockwave, 0)]);
    context.route = Some(route_with_combat_pressure());
    let packet = PublicRewardDecisionPacketV1::from_context(&context);

    let replay =
        replay_card_reward_decision_v1(&packet, &CardRewardPolicyConfigV1::default(), None);

    assert_eq!(replay.candidates.len(), 1);
    assert_eq!(replay.value_estimates.len(), 4);
    assert_eq!(replay.value_arbitration.input_estimate_count, 4);
    assert_eq!(replay.value_arbitration.gate_value_estimates.len(), 1);
    assert_eq!(
        replay.value_arbitration.gate_value_estimates[0].source,
        CardRewardValueSourceV1::StrategyPackage
    );
    assert_eq!(
        replay.value_arbitration.gate_value_estimates[0].status,
        CardRewardValueStatusV1::StrategyPackageEstimate
    );
    assert!(!replay.autopilot_gate.value_source_eligible);
    assert!(replay.selected_candidate_id.is_none());
    assert!(replay
        .stop_reason
        .contains("missing or unresolved evidence"));
}

#[test]
fn replay_harness_summarizes_selected_package_and_threat_value_components() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.boss_key = Some(EncounterId::TimeEater);
    run_state.add_card_to_deck(CardId::Barricade);
    run_state.add_card_to_deck(CardId::Entrench);
    run_state.add_card_to_deck(CardId::FlameBarrier);
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::BodySlam, 0)],
        route_with_combat_pressure(),
    );
    let packet = PublicRewardDecisionPacketV1::from_context(&context);

    let replay =
        replay_card_reward_decision_v1(&packet, &CardRewardPolicyConfigV1::default(), None);
    let summary = replay.candidates.first().expect("candidate summary");

    assert!(summary
        .value_summary
        .iter()
        .any(|line| line == "selected_value_source=StrategyPackage"));
    assert!(summary
        .value_summary
        .iter()
        .any(|line| line == "selected_value_status=StrategyPackageEstimate"));
    assert!(summary
        .value_summary
        .iter()
        .any(|line| { line == "component=strategy_package_completion_block_engine" }));
    assert!(summary.value_summary.iter().any(|line| {
        line == "component=strategy_threat_alignment_block_engine_boss_long_fight"
    }));
}

#[test]
fn replay_harness_accepts_external_estimator_inputs() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());
    let packet = PublicRewardDecisionPacketV1::from_context(&context);
    let inputs = CardRewardEstimatorInputsV1 {
        external_value_estimates: vec![test_value_estimate(
            0,
            CardId::TwinStrike,
            CardRewardValueSourceV1::OutcomeCalibration,
            CardRewardValueStatusV1::OutcomeCalibrated,
            2.0,
            0.2,
        )],
    };

    let replay = replay_card_reward_decision_with_estimator_inputs_v1(
        &packet,
        &CardRewardPolicyConfigV1::default(),
        &inputs,
        None,
    );

    assert_eq!(replay.value_estimates.len(), 9);
    assert_eq!(replay.value_arbitration.input_estimate_count, 9);
    assert_eq!(
        replay.value_arbitration.gate_value_estimates[0].source,
        CardRewardValueSourceV1::OutcomeCalibration
    );
    assert_eq!(
        replay.value_arbitration.gate_value_estimates[1].source,
        CardRewardValueSourceV1::RouteRisk
    );
    assert!(replay.selected_candidate_id.is_none());
}

#[test]
fn route_risk_estimator_adds_non_gate_value_estimates_when_route_evidence_exists() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Warcry, 0),
    ]);
    assert!(context.route.is_none());
    let without_route =
        plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    assert!(!without_route
        .value_estimates
        .iter()
        .any(|estimate| estimate.source == CardRewardValueSourceV1::RouteRisk));

    context.route = Some(route_with_combat_pressure());
    let with_route = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let route_risk_estimates = with_route
        .value_estimates
        .iter()
        .filter(|estimate| estimate.source == CardRewardValueSourceV1::RouteRisk)
        .collect::<Vec<_>>();

    assert_eq!(route_risk_estimates.len(), 2);
    assert!(route_risk_estimates
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::RouteRiskEstimate));
    assert!(route_risk_estimates
        .iter()
        .all(|estimate| estimate.eligibility.usable_for_value_estimate));
    assert!(route_risk_estimates
        .iter()
        .all(|estimate| !estimate.eligibility.usable_for_autopilot_gate));
}

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
    assert!(decision.pick_certificate.is_none());
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
fn public_combat_heuristic_uses_boss_threat_profile_for_strength_down() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    run_state.boss_key = Some(EncounterId::TheChamp);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Disarm, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
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

    assert!(disarm.survival_delta > 0.0);
    assert!(disarm.components.iter().any(|component| {
        component.name == "boss_threat_strength_down_response" && component.value > 0.0
    }));
    assert!(disarm.components.iter().any(|component| {
        component.name == "strategy_threat_high_incoming_damage" && component.value > 0.0
    }));
    assert!(!disarm.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_does_not_mix_elite_pool_multihit_into_champ_boss_response() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    run_state.boss_key = Some(EncounterId::TheChamp);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Disarm, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
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

    assert!(!disarm
        .components
        .iter()
        .any(|component| { component.name == "boss_threat_multi_hit_strength_down_response" }));
}

#[test]
fn public_combat_heuristic_marks_awakened_one_power_punish() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 40;
    run_state.boss_key = Some(EncounterId::AwakenedOne);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Barricade, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let barricade = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Barricade
        })
        .expect("Barricade public combat heuristic estimate");

    assert!(barricade.components.iter().any(|component| {
        component.name == "boss_threat_power_punish_penalty" && component.value < 0.0
    }));
    assert!(!barricade.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_time_eater_card_play_limit() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 40;
    run_state.boss_key = Some(EncounterId::TimeEater);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Anger, 0),
            RewardCard::new(CardId::Carnage, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let anger = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Anger
        })
        .expect("Anger public combat heuristic estimate");
    let carnage = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Carnage
        })
        .expect("Carnage public combat heuristic estimate");

    assert!(anger.components.iter().any(|component| {
        component.name == "boss_threat_card_play_limit_low_density_penalty" && component.value < 0.0
    }));
    assert!(carnage.components.iter().any(|component| {
        component.name == "boss_threat_card_play_limit_dense_card_response" && component.value > 0.0
    }));
    assert!(!anger.eligibility.usable_for_autopilot_gate);
    assert!(!carnage.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_slime_boss_split_burst() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 10;
    run_state.boss_key = Some(EncounterId::SlimeBoss);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Carnage, 0),
            RewardCard::new(CardId::Warcry, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let carnage = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Carnage
        })
        .expect("Carnage public combat heuristic estimate");

    assert!(carnage.components.iter().any(|component| {
        component.name == "boss_threat_split_burst_response" && component.value > 0.0
    }));
    assert!(!carnage.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_status_flood_payoff() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 10;
    run_state.boss_key = Some(EncounterId::Hexaghost);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Evolve, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let evolve = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Evolve
        })
        .expect("Evolve public combat heuristic estimate");

    assert!(evolve.components.iter().any(|component| {
        component.name == "boss_threat_status_flood_payoff_response" && component.value > 0.0
    }));
    assert!(!evolve.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_multi_hit_strength_down() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 10;
    run_state.boss_key = Some(EncounterId::Hexaghost);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Disarm, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
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

    assert!(disarm.components.iter().any(|component| {
        component.name == "boss_threat_multi_hit_strength_down_response" && component.value > 0.0
    }));
    assert!(!disarm.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_act1_skill_punish() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 4;
    run_state.boss_key = Some(EncounterId::TheGuardian);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Armaments, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let armaments = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Armaments
        })
        .expect("Armaments public combat heuristic estimate");

    assert!(armaments.components.iter().any(|component| {
        component.name == "elite_pool_skill_punish_penalty" && component.value < 0.0
    }));
    assert!(!armaments.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_nob_specific_skill_punish() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 4;
    run_state.boss_key = Some(EncounterId::TheGuardian);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Armaments, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let armaments = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Armaments
        })
        .expect("Armaments public combat heuristic estimate");

    assert!(armaments.components.iter().any(|component| {
        component.name == "elite_encounter_nob_skill_punish_penalty" && component.value < 0.0
    }));
}

#[test]
fn public_combat_heuristic_marks_sentries_specific_status_payoff() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 1;
    run_state.floor_num = 4;
    run_state.boss_key = Some(EncounterId::TheGuardian);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Evolve, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let evolve = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Evolve
        })
        .expect("Evolve public combat heuristic estimate");

    assert!(evolve.components.iter().any(|component| {
        component.name == "elite_encounter_sentries_status_payoff_response" && component.value > 0.0
    }));
}

#[test]
fn public_combat_heuristic_marks_book_specific_strength_down() {
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
        route_with_combat_pressure(),
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

    assert!(disarm.components.iter().any(|component| {
        component.name == "elite_encounter_book_strength_down_response" && component.value > 0.0
    }));
}

#[test]
fn public_combat_heuristic_marks_slavers_specific_aoe_response() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    run_state.boss_key = Some(EncounterId::Collector);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Whirlwind, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let whirlwind = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Whirlwind
        })
        .expect("Whirlwind public combat heuristic estimate");

    assert!(whirlwind.components.iter().any(|component| {
        component.name == "elite_encounter_slavers_aoe_response" && component.value > 0.0
    }));
}

#[test]
fn public_combat_heuristic_marks_reptomancer_specific_aoe_response() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 38;
    run_state.boss_key = Some(EncounterId::DonuAndDeca);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Whirlwind, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let whirlwind = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Whirlwind
        })
        .expect("Whirlwind public combat heuristic estimate");

    assert!(whirlwind.components.iter().any(|component| {
        component.name == "elite_encounter_reptomancer_aoe_response" && component.value > 0.0
    }));
}

#[test]
fn public_combat_heuristic_marks_giant_head_specific_scaling_response() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 3;
    run_state.floor_num = 38;
    run_state.boss_key = Some(EncounterId::DonuAndDeca);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Inflame, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let inflame = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Inflame
        })
        .expect("Inflame public combat heuristic estimate");

    assert!(inflame.components.iter().any(|component| {
        component.name == "elite_encounter_giant_head_scaling_response" && component.value > 0.0
    }));
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

#[test]
fn public_combat_heuristic_marks_long_fight_scaling() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    run_state.boss_key = Some(EncounterId::TheChamp);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Inflame, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let inflame = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Inflame
        })
        .expect("Inflame public combat heuristic estimate");

    assert!(inflame.components.iter().any(|component| {
        component.name == "boss_threat_long_fight_scaling_response" && component.value > 0.0
    }));
    assert!(!inflame.eligibility.usable_for_autopilot_gate);
}

#[test]
fn public_combat_heuristic_marks_setup_window_power_response() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.floor_num = 20;
    run_state.boss_key = Some(EncounterId::Automaton);
    let context = context_for_run_with_route(
        &run_state,
        vec![
            RewardCard::new(CardId::Barricade, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let barricade = decision
        .value_estimates
        .iter()
        .find(|estimate| {
            estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
                && estimate.card == CardId::Barricade
        })
        .expect("Barricade public combat heuristic estimate");

    assert!(barricade.components.iter().any(|component| {
        component.name == "boss_threat_setup_window_response" && component.value > 0.0
    }));
    assert!(!barricade.eligibility.usable_for_autopilot_gate);
}

fn test_value_estimate(
    index: usize,
    card: CardId,
    source: CardRewardValueSourceV1,
    status: CardRewardValueStatusV1,
    survival_delta: f32,
    uncertainty: f32,
) -> CardRewardValueEstimateV1 {
    CardRewardValueEstimateV1 {
        index,
        card,
        source,
        status,
        survival_delta,
        progress_delta: 0.0,
        deck_consistency_delta: 0.0,
        uncertainty,
        eligibility: Default::default(),
        components: Vec::new(),
    }
}

fn estimates_for_source(
    estimates: &[CardRewardValueEstimateV1],
    source: CardRewardValueSourceV1,
) -> Vec<&CardRewardValueEstimateV1> {
    estimates
        .iter()
        .filter(|estimate| estimate.source == source)
        .collect()
}

#[test]
fn singing_bowl_blocks_automatic_card_reward_pick_inside_policy() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state
        .relics
        .push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::SingingBowl,
        ));
    let context = context_for_run_with_route(
        &run_state,
        vec![RewardCard::new(CardId::Clothesline, 0)],
        route_with_combat_pressure(),
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision
        .evidence_gaps
        .contains(&CardRewardEvidenceGapV1::SingingBowlAddsMaxHpChoice));
}

fn context_for_cards(
    cards: Vec<RewardCard>,
) -> crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1 {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    build_card_reward_decision_context_v1(&run_state, cards, None)
}

fn context_for_cards_with_route(
    cards: Vec<RewardCard>,
    route: CardRewardRouteEvidenceV1,
) -> crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1 {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    context_for_run_with_route(&run_state, cards, route)
}

fn context_for_run_with_route(
    run_state: &RunState,
    cards: Vec<RewardCard>,
    route: CardRewardRouteEvidenceV1,
) -> crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1 {
    let deck = super::profile::deck_profile(run_state);
    let route = Some(route);
    let strategy =
        crate::ai::noncombat_strategy_v1::build_run_strategy_snapshot_from_run_state_with_route_v2(
            run_state,
            super::profile::strategy_route_future(route.as_ref()),
        );
    let candidates = cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| {
            let facts = super::facts::card_facts(&card);
            let impact = super::impact::candidate_impact(&facts, &deck, route.as_ref());
            let plan_delta = crate::ai::noncombat_strategy_v1::candidate_plan_delta_v2(
                super::profile::strategy_candidate_facts(&facts),
                &strategy,
            );
            let name = facts.name.clone();
            crate::ai::card_reward_policy_v1::CardRewardCandidateEvidenceV1 {
                index,
                card: facts.card,
                name,
                card_type: facts.card_type,
                facts,
                impact,
                plan_delta,
            }
        })
        .collect();

    crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1 {
        run: super::profile::run_context(run_state),
        deck,
        route,
        strategy,
        has_singing_bowl: run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::SingingBowl),
        candidates,
    }
}

fn route_with_combat_pressure() -> CardRewardRouteEvidenceV1 {
    CardRewardRouteEvidenceV1 {
        route_policy: "test_route_evidence".to_string(),
        selected_route: Some(CardRewardSelectedRouteV1 {
            next_x: 3,
            next_y: 1,
            min_fires: 1,
            max_fires: 2,
            first_fire_floor: Some(6),
            min_elites: 0,
            max_elites: 1,
            min_early_pressure: 1,
            max_early_pressure: 3,
        }),
        candidate_count: 2,
        need_card_rewards: 1.0,
        need_upgrade: 0.5,
        need_heal: 0.1,
        can_take_elite: 0.6,
        avoid_damage: 0.4,
        warnings: Vec::new(),
    }
}

fn route_without_elites() -> CardRewardRouteEvidenceV1 {
    CardRewardRouteEvidenceV1 {
        route_policy: "test_route_no_elites".to_string(),
        selected_route: Some(CardRewardSelectedRouteV1 {
            next_x: 3,
            next_y: 1,
            min_fires: 1,
            max_fires: 2,
            first_fire_floor: Some(6),
            min_elites: 0,
            max_elites: 0,
            min_early_pressure: 1,
            max_early_pressure: 2,
        }),
        candidate_count: 2,
        need_card_rewards: 0.7,
        need_upgrade: 0.4,
        need_heal: 0.2,
        can_take_elite: 0.0,
        avoid_damage: 0.7,
        warnings: Vec::new(),
    }
}

fn route_with_upgrade_budget() -> CardRewardRouteEvidenceV1 {
    CardRewardRouteEvidenceV1 {
        route_policy: "test_route_evidence".to_string(),
        selected_route: Some(CardRewardSelectedRouteV1 {
            next_x: 3,
            next_y: 1,
            min_fires: 3,
            max_fires: 4,
            first_fire_floor: Some(4),
            min_elites: 0,
            max_elites: 1,
            min_early_pressure: 0,
            max_early_pressure: 1,
        }),
        candidate_count: 2,
        need_card_rewards: 0.8,
        need_upgrade: 0.9,
        need_heal: 0.0,
        can_take_elite: 0.7,
        avoid_damage: 0.1,
        warnings: Vec::new(),
    }
}
