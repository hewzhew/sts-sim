use crate::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1,
    replay_card_reward_decision_v1, CardRewardEvidenceGapV1, CardRewardPlanEffectV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1, PublicRewardDecisionPacketV1,
};
use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::content::cards::CardId;
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
    assert!(decision
        .value_estimates
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::UncalibratedPrior));
    assert!(!decision.autopilot_gate.value_source_eligible);
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
    assert!(!decision.autopilot_gate.value_source_eligible);
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
    assert!(!decision.autopilot_gate.value_source_eligible);
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

    assert_eq!(decision.value_estimates.len(), 2);
    assert!(decision
        .value_estimates
        .iter()
        .all(|estimate| estimate.source == CardRewardValueSourceV1::UncalibratedImpactPrior));
    assert!(decision
        .value_estimates
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::UncalibratedPrior));
    assert!(decision.value_estimates.iter().all(|estimate| {
        estimate.survival_delta == 0.0
            && estimate.progress_delta == 0.0
            && estimate.deck_consistency_delta == 0.0
            && estimate.uncertainty == 1.0
    }));
}

#[test]
fn uncalibrated_prior_values_are_consumed_by_gate_but_cannot_certify_pick() {
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
        .contains(&CardRewardEvidenceGapV1::UncalibratedValueEstimate));
}

#[test]
fn uncalibrated_prior_blocks_even_when_old_rule_would_have_matched() {
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
        decision.value_estimates[0].source,
        CardRewardValueSourceV1::UncalibratedImpactPrior
    );
    assert_eq!(
        decision.value_estimates[0].status,
        CardRewardValueStatusV1::UncalibratedPrior
    );
    assert!(decision
        .autopilot_gate
        .blocked_reasons
        .contains(&CardRewardEvidenceGapV1::UncalibratedValueEstimate));
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
fn replay_harness_exports_value_loop_gate_state_without_selecting() {
    let mut context = context_for_cards(vec![RewardCard::new(CardId::Shockwave, 0)]);
    context.route = Some(route_with_combat_pressure());
    let packet = PublicRewardDecisionPacketV1::from_context(&context);

    let replay =
        replay_card_reward_decision_v1(&packet, &CardRewardPolicyConfigV1::default(), None);

    assert_eq!(replay.candidates.len(), 1);
    assert_eq!(replay.value_estimates.len(), 1);
    assert_eq!(
        replay.value_estimates[0].source,
        CardRewardValueSourceV1::UncalibratedImpactPrior
    );
    assert_eq!(
        replay.value_estimates[0].status,
        CardRewardValueStatusV1::UncalibratedPrior
    );
    assert!(!replay.autopilot_gate.value_source_eligible);
    assert!(replay.selected_candidate_id.is_none());
    assert!(replay
        .stop_reason
        .contains("missing or unresolved evidence"));
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
            crate::ai::card_reward_policy_v1::CardRewardCandidateEvidenceV1 {
                index,
                card: facts.card,
                name: facts.name,
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
        route_policy: "test_route_evidence",
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

fn route_with_upgrade_budget() -> CardRewardRouteEvidenceV1 {
    CardRewardRouteEvidenceV1 {
        route_policy: "test_route_evidence",
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
