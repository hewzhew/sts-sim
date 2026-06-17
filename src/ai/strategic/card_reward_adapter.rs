use super::{
    compile_decision, ledger_from_snapshot, CandidateAction, CandidateDelta, CandidateRole,
    LedgerDelta, OpportunityCost, PressureKind, StrategicBossTax, StrategicDebt,
    StrategicDecisionSite, StrategicDeckFacts, StrategicJob, StrategicRouteFacts,
    StrategicSnapshot, VerdictHint,
};
use crate::ai::card_component_marginal_value_v1::{
    evaluate_card_component_marginal_value_v1, CardComponentMarginalContextV1,
};
use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1,
    CardRewardEvidenceGapV1, CardRewardSemanticRoleV1,
};
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
use crate::ai::deck_startup_profile_v1::{
    startup_liability_for_candidate_v1, startup_support_for_candidate_v1,
};
use crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1;
use crate::content::monsters::factory::EncounterId;
use crate::state::rewards::RewardCard;

pub fn strategic_trace_for_card_reward(
    context: &CardRewardDecisionContextV1,
) -> super::StrategicDecisionTrace {
    let snapshot = snapshot_from_card_reward_context(context);
    let ledger = ledger_from_snapshot(&snapshot);
    let mut deltas = context
        .candidates
        .iter()
        .map(|candidate| candidate_delta_from_card_reward(context, candidate))
        .collect::<Vec<_>>();
    deltas.push(decline_delta_from_card_reward_context(context));
    compile_decision(snapshot, ledger, context.candidates.len() + 1, deltas)
}

fn snapshot_from_card_reward_context(context: &CardRewardDecisionContextV1) -> StrategicSnapshot {
    StrategicSnapshot {
        site: StrategicDecisionSite::CardReward,
        act: context.run.act,
        floor: context.run.floor,
        boss: context.run.boss.clone(),
        hp: context.run.hp,
        max_hp: context.run.max_hp,
        gold: context.run.gold,
        deck: StrategicDeckFacts {
            deck_size: context.deck.deck_size,
            attacks: context.deck.attacks,
            skills: context.deck.skills,
            powers: context.deck.powers,
            curses: context.deck.curses,
            starter_strikes: context.deck.starter_strikes,
            starter_defends: context.deck.starter_defends,
            draw_sources: context.deck.draw_cards,
            energy_sources: context.deck.energy_sources,
            strength_sources: context.deck.strength_sources,
            temporary_strength_bursts: context.deck.temporary_strength_bursts,
            strength_converters: context.deck.strength_converters,
            convertible_strength_sources: context.deck.convertible_strength_sources,
            strength_payoffs: context.deck.strength_payoffs,
            weak_sources: context.deck.weak_sources,
            vulnerable_sources: context.deck.vulnerable_sources,
            exhaust_generators: context.deck.exhaust_generators,
            exhaust_payoffs: context.deck.exhaust_payoffs,
            status_generators: context.deck.status_generators,
            status_payoffs: context.deck.status_payoffs,
            total_attack_damage: context.deck.total_attack_damage,
            total_block: context.deck.total_block,
        },
        route: context.route.as_ref().map(|route| StrategicRouteFacts {
            need_card_rewards: route.need_card_rewards,
            need_upgrade: route.need_upgrade,
            need_heal: route.need_heal,
            can_take_elite: route.can_take_elite,
            avoid_damage: route.avoid_damage,
            min_elites: route
                .selected_route
                .as_ref()
                .map(|selected| selected.min_elites)
                .unwrap_or_default(),
            max_elites: route
                .selected_route
                .as_ref()
                .map(|selected| selected.max_elites)
                .unwrap_or_default(),
            min_fires: route
                .selected_route
                .as_ref()
                .map(|selected| selected.min_fires)
                .unwrap_or_default(),
            max_fires: route
                .selected_route
                .as_ref()
                .map(|selected| selected.max_fires)
                .unwrap_or_default(),
            first_fire_floor: route
                .selected_route
                .as_ref()
                .and_then(|selected| selected.first_fire_floor),
            warnings: route.warnings.clone(),
        }),
        formation_needs: context
            .strategy
            .formation_summary()
            .needs
            .iter()
            .map(formation_need_for_strategy)
            .collect(),
    }
}

fn candidate_delta_from_card_reward(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
) -> CandidateDelta {
    let action = CandidateAction::TakeCard {
        index: candidate.index,
        card: candidate.card,
    };
    let profile =
        card_reward_semantic_profile_v1(&RewardCard::new(candidate.card, candidate.facts.upgrades));
    let component_report = evaluate_card_component_marginal_value_v1(
        &component_context_from_card_reward_context(context, candidate.same_card_count),
        &profile,
    );
    let mut delta = CandidateDelta::from_component_report(action, &component_report);
    add_candidate_impact_deltas(context, candidate, &mut delta);
    add_candidate_facts_deltas(candidate, &mut delta);
    add_candidate_startup_deltas(context, candidate, &mut delta);
    add_candidate_boss_pressure_deltas(context, &profile, &mut delta);
    delta
}

fn decline_delta_from_card_reward_context(context: &CardRewardDecisionContextV1) -> CandidateDelta {
    let mut delta = if context.has_singing_bowl {
        CandidateDelta::empty(CandidateAction::TakeSingingBowl { max_hp_gain: 2 })
    } else {
        CandidateDelta::empty(CandidateAction::SkipCardReward)
    };
    delta.role = if context.has_singing_bowl {
        CandidateRole::ResourceConversion
    } else {
        CandidateRole::DeckCleaning
    };
    delta.verdict_hint = VerdictHint::Speculative;
    delta
        .evidence
        .push("card_reward_decline_candidate".to_string());
    if context.has_singing_bowl {
        delta
            .evidence
            .push("singing_bowl_max_hp_alternative".to_string());
    } else if context.deck.deck_size >= 24 {
        delta.notes.push(format!(
            "skip_preserves_deck_consistency deck_size={}",
            context.deck.deck_size
        ));
    }
    delta
}

fn add_candidate_impact_deltas(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    if candidate.impact.frontload_damage_delta > 0 {
        if formation_needs_include(context, StrategyDeckFormationNeedV1::Frontload) {
            delta.positive.push(LedgerDelta {
                kind: PressureKind::MissingJob(StrategicJob::Frontload),
                amount: (candidate.impact.frontload_damage_delta as f32 / 20.0).clamp(0.05, 0.60),
                reason: "frontload_damage_delta".to_string(),
            });
        } else {
            delta.notes.push(format!(
                "frontload_delta_not_current_need value={}",
                candidate.impact.frontload_damage_delta
            ));
        }
    }
    if candidate.impact.block_delta > 0 {
        if formation_needs_include(context, StrategyDeckFormationNeedV1::Block) {
            delta.positive.push(LedgerDelta {
                kind: PressureKind::MissingJob(StrategicJob::Block),
                amount: (candidate.impact.block_delta as f32 / 18.0).clamp(0.05, 0.55),
                reason: "block_delta".to_string(),
            });
        } else {
            delta.notes.push(format!(
                "block_delta_not_current_need value={}",
                candidate.impact.block_delta
            ));
        }
    }
    if candidate.impact.draw_delta > 0 || candidate.impact.energy_delta > 0 {
        if formation_needs_include(context, StrategyDeckFormationNeedV1::DrawEnergy)
            || formation_needs_include(context, StrategyDeckFormationNeedV1::Consistency)
        {
            delta.positive.push(LedgerDelta {
                kind: PressureKind::MissingJob(StrategicJob::DrawEnergy),
                amount: ((candidate.impact.draw_delta + candidate.impact.energy_delta) as f32
                    / 4.0)
                    .clamp(0.10, 0.65),
                reason: "draw_or_energy_delta".to_string(),
            });
            if delta.role == CandidateRole::Unknown {
                delta.role = CandidateRole::Lubricant;
            }
        } else {
            delta.notes.push(format!(
                "draw_energy_delta_not_current_need draw={} energy={}",
                candidate.impact.draw_delta, candidate.impact.energy_delta
            ));
        }
    }
    if candidate.impact.added_deck_size > 0 {
        delta.negative.push(LedgerDelta {
            kind: PressureKind::DeckDebt(StrategicDebt::CycleTime),
            amount: 0.12 * candidate.impact.added_deck_size as f32,
            reason: "adds_cycle_card".to_string(),
        });
    }
    for blocker in &candidate.impact.approval_blockers {
        delta.opportunity_costs.push(OpportunityCost {
            label: approval_blocker_label(*blocker),
            severity: 0.35,
        });
    }
    if delta.positive.is_empty() && delta.verdict_hint == VerdictHint::Speculative {
        delta
            .notes
            .push("no explicit positive ledger delta".to_string());
    }
}

fn formation_needs_include(
    context: &CardRewardDecisionContextV1,
    need: StrategyDeckFormationNeedV1,
) -> bool {
    context.strategy.formation_summary().needs.contains(&need)
}

fn add_candidate_facts_deltas(
    candidate: &CardRewardCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    if candidate.facts.enemy_strength_down > 0 {
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::EnemyStrengthDown),
            amount: (candidate.facts.enemy_strength_down as f32 / 4.0).clamp(0.20, 0.75),
            reason: "enemy_strength_down_delta".to_string(),
        });
        if delta.role == CandidateRole::Unknown {
            delta.role = CandidateRole::DefensivePatch;
        }
    }
    if candidate.facts.weak > 0 {
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::Block),
            amount: (candidate.facts.weak as f32 / 5.0).clamp(0.12, 0.45),
            reason: "weak_coverage_delta".to_string(),
        });
        if delta.role == CandidateRole::Unknown {
            delta.role = CandidateRole::DefensivePatch;
        }
    }
    if candidate.facts.vulnerable > 0 {
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::Frontload),
            amount: (candidate.facts.vulnerable as f32 / 5.0).clamp(0.12, 0.45),
            reason: "vulnerable_coverage_delta".to_string(),
        });
    }
}

fn add_candidate_startup_deltas(
    context: &CardRewardDecisionContextV1,
    candidate: &CardRewardCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    if let Some(label) =
        startup_liability_for_candidate_v1(&context.startup, candidate.card, context.run.act)
    {
        delta.negative.push(LedgerDelta {
            kind: startup_liability_kind(label),
            amount: startup_liability_amount(label),
            reason: label.to_string(),
        });
        delta.evidence.push(label.to_string());
    }

    let shape_delta = crate::ai::deck_shape_v1::deck_shape_candidate_delta_v1(
        &context.deck_shape,
        candidate.card,
    );
    for label in shape_delta.labels {
        delta.negative.push(LedgerDelta {
            kind: PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk),
            amount: 0.45,
            reason: label.to_string(),
        });
        delta.evidence.push(label.to_string());
    }

    if let Some(label) = startup_support_for_candidate_v1(&context.startup, candidate.card) {
        delta.positive.push(LedgerDelta {
            kind: startup_support_kind(label),
            amount: startup_support_amount(label),
            reason: label.to_string(),
        });
        delta.evidence.push(label.to_string());
    }
}

fn startup_liability_kind(label: &str) -> PressureKind {
    match label {
        "startup_rejects_status_generator_duplicate_without_digest"
        | "startup_rejects_clash_playability_debt"
        | "startup_rejects_havoc_duplicate_without_payoff"
        | "startup_rejects_more_anger_without_digest"
        | "startup_rejects_dual_wield_without_target_or_payment" => {
            PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk)
        }
        "startup_rejects_strength_payoff_without_strength"
        | "startup_rejects_rupture_without_self_damage"
        | "startup_rejects_corruption_duplicate_without_payoff"
        | "startup_rejects_more_fnp_without_exhaust_engine" => {
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler)
        }
        "startup_rejects_third_fnp_without_setup_payment" => {
            PressureKind::DeckDebt(StrategicDebt::SetupDebt)
        }
        _ => PressureKind::DeckDebt(StrategicDebt::SetupDebt),
    }
}

fn startup_liability_amount(label: &str) -> f32 {
    match label {
        "startup_rejects_corruption_duplicate_without_payoff"
        | "startup_rejects_third_fnp_without_setup_payment" => 0.75,
        "startup_rejects_status_generator_duplicate_without_digest"
        | "startup_rejects_havoc_duplicate_without_payoff"
        | "startup_rejects_more_anger_without_digest" => 0.65,
        "startup_rejects_dual_wield_without_target_or_payment"
        | "startup_rejects_strength_payoff_without_strength"
        | "startup_rejects_rupture_without_self_damage"
        | "startup_rejects_more_fnp_without_exhaust_engine" => 0.55,
        _ => 0.45,
    }
}

fn startup_support_kind(label: &str) -> PressureKind {
    match label {
        "startup_supports_setup_payment" => PressureKind::MissingJob(StrategicJob::DrawEnergy),
        "startup_supports_fnp_exhaust_engine" => {
            PressureKind::MissingJob(StrategicJob::ExhaustAccess)
        }
        "startup_supports_strength_source" | "startup_supports_conditional_strength_source" => {
            PressureKind::MissingJob(StrategicJob::Scaling)
        }
        "startup_supports_rupture_self_damage_source" => {
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler)
        }
        "startup_supports_upgrade_access" => PressureKind::UpgradeNeed,
        _ => PressureKind::MissingJob(StrategicJob::Consistency),
    }
}

fn startup_support_amount(label: &str) -> f32 {
    match label {
        "startup_supports_setup_payment" => 0.60,
        "startup_supports_fnp_exhaust_engine" => 0.55,
        "startup_supports_strength_source" | "startup_supports_conditional_strength_source" => 0.50,
        "startup_supports_rupture_self_damage_source" => 0.45,
        "startup_supports_upgrade_access" => 0.40,
        _ => 0.30,
    }
}

fn add_candidate_boss_pressure_deltas(
    context: &CardRewardDecisionContextV1,
    profile: &crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1,
    delta: &mut CandidateDelta,
) {
    const AUTOMATON_HYPERBEAM_MITIGATION_SIGNAL: f32 = 0.45;
    const AUTOMATON_ORB_CONTROL_SIGNAL: f32 = 0.40;
    const COLLECTOR_MINION_CONTROL_SIGNAL: f32 = 0.55;

    let boss = context.run.boss.as_deref().and_then(parse_boss);
    if boss == Some(EncounterId::Collector)
        && profile_has_role(profile, CardRewardSemanticRoleV1::AoeDamage)
    {
        delta.positive.push(LedgerDelta {
            kind: PressureKind::BossTax(StrategicBossTax::CollectorMinionPlan),
            amount: COLLECTOR_MINION_CONTROL_SIGNAL,
            reason: "collector_aoe_minion_plan".to_string(),
        });
        if delta.role == CandidateRole::Unknown || delta.role == CandidateRole::Transition {
            delta.role = CandidateRole::BossAnswer;
        }
    }

    if boss == Some(EncounterId::Automaton) {
        if profile_has_any_role(
            profile,
            &[
                CardRewardSemanticRoleV1::Block,
                CardRewardSemanticRoleV1::Weak,
                CardRewardSemanticRoleV1::EnemyStrengthDown,
            ],
        ) {
            delta.positive.push(LedgerDelta {
                kind: PressureKind::BossTax(StrategicBossTax::AutomatonHyperbeamPlan),
                amount: AUTOMATON_HYPERBEAM_MITIGATION_SIGNAL,
                reason: "automaton_hyperbeam_mitigation".to_string(),
            });
            if delta.role == CandidateRole::Unknown
                || delta.role == CandidateRole::Transition
                || delta.role == CandidateRole::DefensivePatch
            {
                delta.role = CandidateRole::BossAnswer;
            }
        }

        if profile_has_any_role(
            profile,
            &[
                CardRewardSemanticRoleV1::FrontloadDamage,
                CardRewardSemanticRoleV1::AoeDamage,
                CardRewardSemanticRoleV1::CardDraw,
                CardRewardSemanticRoleV1::EnergySource,
            ],
        ) {
            delta.positive.push(LedgerDelta {
                kind: PressureKind::BossTax(StrategicBossTax::AutomatonOrbControl),
                amount: AUTOMATON_ORB_CONTROL_SIGNAL,
                reason: "automaton_orb_control_or_stasis_recovery".to_string(),
            });
            if delta.role == CandidateRole::Unknown || delta.role == CandidateRole::Transition {
                delta.role = CandidateRole::BossAnswer;
            }
        }
    }
}

fn profile_has_role(
    profile: &crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1,
    role: CardRewardSemanticRoleV1,
) -> bool {
    profile.roles.contains(&role)
}

fn profile_has_any_role(
    profile: &crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1,
    roles: &[CardRewardSemanticRoleV1],
) -> bool {
    roles.iter().any(|role| profile_has_role(profile, *role))
}

fn component_context_from_card_reward_context(
    context: &CardRewardDecisionContextV1,
    same_card_count: usize,
) -> CardComponentMarginalContextV1 {
    CardComponentMarginalContextV1 {
        act: context.run.act,
        floor: context.run.floor,
        boss: context.run.boss.as_deref().and_then(parse_boss),
        hp: context.run.hp,
        max_hp: context.run.max_hp,
        deck_size: context.deck.deck_size,
        powers: context.deck.powers as usize,
        draw_sources: context.deck.draw_cards as usize,
        exhaust_generators: context.deck.exhaust_generators as usize,
        frontload_jobs: context.deck.attacks as usize,
        block_jobs: context.deck.skills as usize,
        same_card_count,
        formation_needs: context.strategy.formation_summary().needs,
        startup: DeckStartupProfileV1 {
            feel_no_pain_count: context.deck.exhaust_payoffs,
            exhaust_engine_count: context.deck.exhaust_generators,
            strong_draw_count: context.deck.draw_cards,
            persistent_strength_source_count: context.deck.strength_sources,
            temporary_strength_burst_count: context.deck.temporary_strength_bursts,
            strength_converter_count: context.deck.strength_converters,
            convertible_strength_source_count: context.deck.convertible_strength_sources,
            self_damage_source_count: 0,
            strength_payoff_count: context.deck.strength_payoffs,
            ..Default::default()
        },
    }
}

fn parse_boss(value: &str) -> Option<EncounterId> {
    match value {
        "AwakenedOne" => Some(EncounterId::AwakenedOne),
        "Automaton" => Some(EncounterId::Automaton),
        "TimeEater" => Some(EncounterId::TimeEater),
        "TheChamp" => Some(EncounterId::TheChamp),
        "TheGuardian" => Some(EncounterId::TheGuardian),
        "Hexaghost" => Some(EncounterId::Hexaghost),
        "SlimeBoss" => Some(EncounterId::SlimeBoss),
        "Collector" => Some(EncounterId::Collector),
        "DonuAndDeca" => Some(EncounterId::DonuAndDeca),
        _ => None,
    }
}

pub(crate) fn formation_need_for_strategy(
    need: &StrategyDeckFormationNeedV1,
) -> super::StrategicJob {
    match need {
        StrategyDeckFormationNeedV1::Frontload => StrategicJob::Frontload,
        StrategyDeckFormationNeedV1::Block => StrategicJob::Block,
        StrategyDeckFormationNeedV1::Scaling => StrategicJob::Scaling,
        StrategyDeckFormationNeedV1::DrawEnergy => StrategicJob::DrawEnergy,
        StrategyDeckFormationNeedV1::Consistency => StrategicJob::Consistency,
    }
}

fn approval_blocker_label(blocker: CardRewardEvidenceGapV1) -> String {
    format!("approval_blocker:{blocker:?}")
}
