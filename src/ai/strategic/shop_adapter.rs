use super::{
    add_run_debt_candidate_deltas_v1, add_run_debt_pressure_to_ledger,
    add_snecko_cost_conversion_delta_v1, add_startup_profile_pressure_to_ledger, compile_decision,
    ledger_from_snapshot, CandidateAction, CandidateDelta, CandidateRole, LedgerDelta,
    OpportunityCost, PressureHorizon, PressureKind, PressureLedger, RunDebtCandidateSignalsV1,
    StrategicBossTax, StrategicDebt, StrategicDecisionSite, StrategicDeckFacts, StrategicJob,
    StrategicSnapshot, VerdictHint,
};
use crate::ai::acquisition_saturation_v1::{
    apply_acquisition_saturation_to_delta_v1, evaluate_acquisition_saturation_v1,
    AcquisitionSaturationInputV1,
};
use crate::ai::card_component_marginal_value_v1::{
    evaluate_card_component_marginal_value_v1, CardComponentMarginalContextV1,
};
use crate::ai::card_reward_policy_v1::{
    card_facts, card_reward_semantic_profile_v1, CardRewardSemanticRoleV1,
};
use crate::ai::card_semantics_v1::card_mechanics_profile_v1;
use crate::ai::decision_tags_v1::{
    strings_have_tag, TAG_COLLECTOR_ANSWER, TAG_ENGINE_CLOSURE, TAG_STARTUP_ACCESS,
};
use crate::ai::deck_startup_profile_v1::startup_energy_candidate_discounted_by_snecko_v1;
use crate::ai::shop_policy_v1::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPolicyClassV1, ShopPurchaseTargetV1,
};
use crate::state::rewards::RewardCard;

pub fn strategic_trace_for_shop(context: &ShopDecisionContextV1) -> super::StrategicDecisionTrace {
    let snapshot = snapshot_from_shop_context(context);
    let mut ledger = ledger_from_snapshot(&snapshot);
    add_shop_upgrade_pressure_to_ledger(context, &mut ledger);
    add_startup_profile_pressure_to_ledger(&mut ledger, &context.startup);
    add_run_debt_pressure_to_ledger(&mut ledger, &context.run_debt);
    let deltas = context
        .candidates
        .iter()
        .map(|candidate| candidate_delta_from_shop_candidate(context, candidate))
        .collect::<Vec<_>>();
    compile_decision(snapshot, ledger, context.candidates.len(), deltas)
}

fn add_shop_upgrade_pressure_to_ledger(
    context: &ShopDecisionContextV1,
    ledger: &mut PressureLedger,
) {
    if context.upgrade_need.pressure <= 0.0 {
        return;
    }
    ledger.push(
        "upgrade_need:shop_upgrade_debt",
        PressureKind::UpgradeNeed,
        PressureHorizon::ActBoss,
        context.upgrade_need.pressure,
        0.70,
        context.upgrade_need.evidence.clone(),
    );
}

fn snapshot_from_shop_context(context: &ShopDecisionContextV1) -> StrategicSnapshot {
    let deck = &context.strategy.v1.deck;
    StrategicSnapshot {
        site: StrategicDecisionSite::Shop,
        act: context.need.act,
        floor: context.need.floor,
        boss: context.need.boss.map(|boss| format!("{boss:?}")),
        hp: context.need.hp,
        max_hp: context.need.max_hp,
        gold: context.need.gold,
        deck: StrategicDeckFacts {
            deck_size: deck.deck_size,
            attacks: deck.attacks,
            skills: deck.skills,
            powers: deck.powers,
            curses: context.need.has_curse as u8,
            starter_strikes: context.need.strike_count as u8,
            starter_defends: context.need.defend_count as u8,
            draw_sources: deck.draw_sources,
            energy_sources: deck.energy_sources,
            strength_sources: deck.strength_sources,
            temporary_strength_bursts: context.strength.temporary_bursts,
            strength_converters: context.strength.converters,
            convertible_strength_sources: context.strength.convertible_potential_count,
            strength_payoffs: deck.strength_payoffs,
            weak_sources: deck.weak_sources,
            vulnerable_sources: deck.vulnerable_sources,
            exhaust_generators: deck.exhaust_generators,
            exhaust_payoffs: deck.exhaust_payoffs,
            status_generators: deck.status_generators,
            status_payoffs: deck.status_payoffs,
            total_attack_damage: deck.total_attack_damage,
            total_block: deck.total_block,
        },
        route: None,
        formation_needs: context
            .strategy
            .formation_summary()
            .needs
            .iter()
            .map(super::card_reward_adapter::formation_need_for_strategy)
            .collect(),
    }
}

fn candidate_delta_from_shop_candidate(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
) -> CandidateDelta {
    let action = match candidate.purchase_target {
        Some(ShopPurchaseTargetV1::Card { index, card }) => CandidateAction::BuyCard {
            shop_index: index,
            card,
            gold: 0,
        },
        Some(ShopPurchaseTargetV1::Relic { index, relic }) => CandidateAction::BuyRelic {
            shop_index: index,
            relic,
            gold: 0,
        },
        Some(ShopPurchaseTargetV1::Potion { index, potion }) => CandidateAction::BuyPotion {
            shop_index: index,
            potion,
            gold: 0,
        },
        None if candidate.class == ShopPolicyClassV1::Leave => CandidateAction::LeaveShop,
        None => candidate
            .deck_index
            .zip(candidate.card)
            .map(|(deck_index, card)| CandidateAction::RemoveCard {
                deck_index,
                card,
                gold: None,
            })
            .unwrap_or_else(|| CandidateAction::Unknown {
                id: candidate.candidate_id.clone(),
                label: candidate.label.clone(),
            }),
    };

    let mut delta = CandidateDelta::empty(action);
    delta.evidence.extend(candidate.evidence.clone());
    delta.notes.extend(candidate.risks.clone());

    match candidate.class {
        ShopPolicyClassV1::CursePurge
        | ShopPolicyClassV1::StarterStrikePurge
        | ShopPolicyClassV1::StarterDefendPurge => {
            delta.role = CandidateRole::DeckCleaning;
            delta.verdict_hint = VerdictHint::StrongTake;
            delta.positive.push(LedgerDelta {
                kind: PressureKind::DeckDebt(StrategicDebt::CurseOrStarterDensity),
                amount: 0.75,
                reason: "shop_purge_reduces_deck_debt".to_string(),
            });
            delta.positive.push(LedgerDelta {
                kind: PressureKind::DeckDebt(StrategicDebt::CycleTime),
                amount: 0.35,
                reason: "shop_purge_improves_cycle_time".to_string(),
            });
        }
        ShopPolicyClassV1::PurchaseOpportunity => {
            delta.role = CandidateRole::ResourceConversion;
            if candidate.card.is_some() {
                add_shop_card_purchase_deltas(context, candidate, &mut delta);
                delta.negative.push(LedgerDelta {
                    kind: PressureKind::DeckDebt(StrategicDebt::CycleTime),
                    amount: shop_card_add_cycle_debt_amount(context),
                    reason: shop_card_add_cycle_debt_reason(context),
                });
            } else {
                delta.positive.push(LedgerDelta {
                    kind: PressureKind::EconomyNeed,
                    amount: SHOP_PURCHASE_GOLD_CONVERSION_SIGNAL,
                    reason: "shop_purchase_converts_gold_without_shop_priority_estimate"
                        .to_string(),
                });
            }
            delta.opportunity_costs.push(OpportunityCost {
                label: "spends_shop_gold".to_string(),
                severity: 0.25,
            });
        }
        ShopPolicyClassV1::Leave => {
            delta.role = CandidateRole::ResourceConversion;
            delta.verdict_hint = if candidate.risks.is_empty() {
                VerdictHint::ContextTake
            } else {
                VerdictHint::SkipPreferred
            };
            if candidate.risks.is_empty() {
                delta.positive.push(LedgerDelta {
                    kind: PressureKind::EconomyNeed,
                    amount: 0.20,
                    reason: "leave_shop_preserves_gold_without_conversion_pressure".to_string(),
                });
            } else {
                delta.negative.push(LedgerDelta {
                    kind: PressureKind::EconomyNeed,
                    amount: 0.45,
                    reason: "leave_shop_with_unconverted_pressure".to_string(),
                });
            }
        }
        ShopPolicyClassV1::Unknown => {
            delta.verdict_hint = VerdictHint::Speculative;
            delta.negative.push(LedgerDelta {
                kind: PressureKind::EconomyNeed,
                amount: 0.20,
                reason: "shop_candidate_unknown_strategy_role".to_string(),
            });
        }
    }

    add_shop_run_debt_deltas(context, candidate, &mut delta);
    delta
}

fn add_shop_card_purchase_deltas(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    if candidate_has_evidence(candidate, TAG_COLLECTOR_ANSWER) {
        delta.role = CandidateRole::BossAnswer;
        delta.positive.push(LedgerDelta {
            kind: PressureKind::BossTax(StrategicBossTax::CollectorMinionPlan),
            amount: SHOP_CARD_BOSS_ANSWER_SIGNAL,
            reason: TAG_COLLECTOR_ANSWER.to_string(),
        });
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::Block),
            amount: 0.20,
            reason: "collector_answer_reduces_minion_or_debuff_pressure".to_string(),
        });
    }

    if candidate_has_evidence(candidate, TAG_ENGINE_CLOSURE) {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::Enabler;
        }
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::ExhaustAccess),
            amount: SHOP_CARD_ENGINE_CLOSURE_SIGNAL,
            reason: TAG_ENGINE_CLOSURE.to_string(),
        });
    }

    if candidate_has_evidence(candidate, TAG_STARTUP_ACCESS) {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::Lubricant;
        }
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::DrawEnergy),
            amount: SHOP_CARD_STARTUP_ACCESS_SIGNAL,
            reason: TAG_STARTUP_ACCESS.to_string(),
        });
    }

    add_shop_card_component_deltas(context, candidate, delta);
    add_shop_card_acquisition_saturation_deltas(context, candidate, delta);
    add_default_shop_card_semantic_deltas(context, candidate, delta);
}

fn add_shop_run_debt_deltas(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    add_run_debt_candidate_deltas_v1(
        delta,
        &context.run_debt,
        RunDebtCandidateSignalsV1 {
            deck_cleanup_for_hp_loss_control: matches!(
                candidate.class,
                ShopPolicyClassV1::CursePurge
                    | ShopPolicyClassV1::StarterStrikePurge
                    | ShopPolicyClassV1::StarterDefendPurge
            ),
            adds_hp_loss_control: candidate_delta_has_hp_loss_control(delta),
            improves_access_to_control: candidate_delta_has_access_control(delta),
            self_damage_source: candidate
                .card
                .map(|card| card_mechanics_profile_v1(card).self_damage_source)
                .unwrap_or(false),
            same_card_count: candidate.same_card_count,
            adds_card: candidate.card.is_some(),
        },
    );
}

fn candidate_delta_has_hp_loss_control(delta: &CandidateDelta) -> bool {
    delta.positive.iter().any(|entry| {
        if matches!(
            entry.reason.as_str(),
            "shop_card_adds_block_or_mitigation"
                | "mitigates_enemy_damage"
                | "direct_strength_down_answer"
                | "rest_lock_candidate_adds_hp_loss_control"
        ) {
            return true;
        }
        matches!(
            entry.kind,
            PressureKind::MissingJob(StrategicJob::EnemyStrengthDown)
                | PressureKind::BossTax(StrategicBossTax::AutomatonHyperbeamPlan)
                | PressureKind::BossTax(StrategicBossTax::ChampExecutePlan)
                | PressureKind::BossTax(StrategicBossTax::AwakenedPhaseTwoBlock)
        )
    })
}

fn candidate_delta_has_access_control(delta: &CandidateDelta) -> bool {
    delta.positive.iter().any(|entry| {
        matches!(
            entry.kind,
            PressureKind::MissingJob(StrategicJob::DrawEnergy)
                | PressureKind::MissingJob(StrategicJob::ExhaustAccess)
                | PressureKind::DeckDebt(StrategicDebt::CycleTime)
        )
    })
}

fn add_default_shop_card_semantic_deltas(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    let Some(card) = candidate.card else {
        return;
    };
    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, 0));
    if profile.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
    {
        if shop_card_frontload_is_current_need(context) {
            delta.role = CandidateRole::Transition;
            push_default_shop_card_positive_once(
                delta,
                PressureKind::MissingJob(StrategicJob::Frontload),
                SHOP_CARD_DEFAULT_JOB_SIGNAL,
                "shop_card_adds_frontload_current_need",
            );
        } else {
            delta
                .evidence
                .push("shop_card_frontload_not_current_need".to_string());
        }
    }
    if profile.roles.contains(&CardRewardSemanticRoleV1::Block)
        || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
    {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::DefensivePatch;
        }
        push_default_shop_card_positive_once(
            delta,
            PressureKind::MissingJob(StrategicJob::Block),
            SHOP_CARD_DEFAULT_JOB_SIGNAL,
            "shop_card_adds_block_or_mitigation",
        );
    }
    if profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::CycleAccess)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnergySource)
    {
        if profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnergySource)
            && startup_energy_candidate_discounted_by_snecko_v1(&context.startup, card)
        {
            delta
                .notes
                .push("shop_card_draw_energy_discounted_by_snecko".to_string());
        } else {
            if delta.role == CandidateRole::ResourceConversion {
                delta.role = CandidateRole::Lubricant;
            }
            push_default_shop_card_positive_once(
                delta,
                PressureKind::MissingJob(StrategicJob::DrawEnergy),
                SHOP_CARD_DEFAULT_JOB_SIGNAL,
                "shop_card_adds_draw_or_energy",
            );
        }
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::ExhaustGenerator)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::ExhaustPayoff)
    {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::Enabler;
        }
        push_default_shop_card_positive_once(
            delta,
            PressureKind::MissingJob(StrategicJob::ExhaustAccess),
            SHOP_CARD_DEFAULT_JOB_SIGNAL,
            "shop_card_adds_exhaust_access",
        );
    }
    if card_facts(&RewardCard::new(card, 0)).upgrades_cards {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::Enabler;
        }
        push_default_shop_card_positive_once(
            delta,
            PressureKind::UpgradeNeed,
            SHOP_CARD_UPGRADE_ACCESS_SIGNAL,
            "shop_card_upgrades_existing_deck",
        );
    }
    add_snecko_cost_conversion_delta_v1(delta, &context.startup, card);
    if delta.positive.is_empty() {
        delta.positive.push(LedgerDelta {
            kind: PressureKind::EconomyNeed,
            amount: SHOP_CARD_GENERIC_GOLD_CONVERSION_SIGNAL,
            reason: "shop_card_converts_gold_without_specific_job_or_shop_priority_estimate"
                .to_string(),
        });
    }
}

fn add_shop_card_component_deltas(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    let Some(card) = candidate.card else {
        return;
    };
    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, 0));
    let report = evaluate_card_component_marginal_value_v1(
        &component_context_from_shop_context(context, candidate.same_card_count),
        &profile,
    );
    let component_delta = CandidateDelta::from_component_report(delta.action.clone(), &report);

    if matches!(
        delta.role,
        CandidateRole::ResourceConversion | CandidateRole::Unknown
    ) && component_delta.role != CandidateRole::Unknown
    {
        delta.role = component_delta.role;
    }
    delta.verdict_hint = component_delta.verdict_hint;
    delta.positive.extend(component_delta.positive);
    delta.negative.extend(component_delta.negative);
    delta.notes.extend(component_delta.notes);
    delta.evidence.extend(component_delta.evidence);
}

fn push_default_shop_card_positive_once(
    delta: &mut CandidateDelta,
    kind: PressureKind,
    amount: f32,
    reason: &str,
) {
    if delta.positive.iter().any(|entry| entry.kind == kind) {
        delta
            .notes
            .push(format!("shop_card_default_signal_deduped:{reason}"));
        return;
    }
    delta.positive.push(LedgerDelta {
        kind,
        amount,
        reason: reason.to_string(),
    });
}

fn add_shop_card_acquisition_saturation_deltas(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    let Some(card) = candidate.card else {
        return;
    };
    let deck = &context.strategy.v1.deck;
    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, 0));
    let report = evaluate_acquisition_saturation_v1(
        &AcquisitionSaturationInputV1 {
            act: context.need.act,
            floor: context.need.floor,
            deck_size: deck.deck_size,
            frontload_cards: deck.attacks as usize,
            weak_sources: deck.weak_sources as usize,
            block_cards: deck.skills as usize,
            draw_sources: deck.draw_sources.saturating_add(deck.energy_sources) as usize,
            exhaust_generators: deck.exhaust_generators as usize,
            exhaust_payoffs: deck.exhaust_payoffs as usize,
            scaling_sources: deck.strength_sources as usize,
            status_generators: deck.status_generators as usize,
            status_payoffs: deck.status_payoffs as usize,
            block_engine_pieces: context.block_plan.engine_support_score(),
            same_card_count: candidate.same_card_count,
            starter_strikes: context.need.strike_count,
            strength_sources: deck.strength_sources as usize,
        },
        &profile,
    );
    apply_acquisition_saturation_to_delta_v1(delta, &report);
}

fn component_context_from_shop_context(
    context: &ShopDecisionContextV1,
    same_card_count: usize,
) -> CardComponentMarginalContextV1 {
    let deck = &context.strategy.v1.deck;
    CardComponentMarginalContextV1 {
        act: context.need.act,
        floor: context.need.floor,
        boss: context.need.boss,
        hp: context.need.hp,
        max_hp: context.need.max_hp,
        deck_size: deck.deck_size,
        powers: deck.powers as usize,
        draw_sources: deck.draw_sources as usize,
        exhaust_generators: deck.exhaust_generators as usize,
        frontload_jobs: deck.attacks as usize,
        block_jobs: deck.skills as usize,
        same_card_count,
        formation_needs: context.strategy.formation_summary().needs,
        startup: context.startup.clone(),
    }
}

fn shop_card_frontload_is_current_need(context: &ShopDecisionContextV1) -> bool {
    context
        .strategy
        .formation_summary()
        .needs
        .iter()
        .any(|need| {
            matches!(
                need,
                crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1::Frontload
            )
        })
}

fn candidate_has_evidence(candidate: &ShopCandidateEvidenceV1, tag: &str) -> bool {
    strings_have_tag(&candidate.evidence, tag)
}

fn shop_card_add_cycle_debt_amount(context: &ShopDecisionContextV1) -> f32 {
    let deck = &context.strategy.v1.deck;
    if deck.deck_size >= 40 {
        0.78
    } else if deck.deck_size >= 34 {
        0.62
    } else if deck.deck_size >= 28 && deck.draw_sources <= 1 {
        0.42
    } else {
        0.12
    }
}

fn shop_card_add_cycle_debt_reason(context: &ShopDecisionContextV1) -> String {
    let deck = &context.strategy.v1.deck;
    format!(
        "shop_card_adds_cycle_card deck_size={} draw_sources={}",
        deck.deck_size, deck.draw_sources
    )
}

const SHOP_PURCHASE_GOLD_CONVERSION_SIGNAL: f32 = 0.20;
const SHOP_CARD_BOSS_ANSWER_SIGNAL: f32 = 0.55;
const SHOP_CARD_ENGINE_CLOSURE_SIGNAL: f32 = 0.45;
const SHOP_CARD_STARTUP_ACCESS_SIGNAL: f32 = 0.35;
const SHOP_CARD_DEFAULT_JOB_SIGNAL: f32 = 0.30;
const SHOP_CARD_UPGRADE_ACCESS_SIGNAL: f32 = 0.70;
const SHOP_CARD_GENERIC_GOLD_CONVERSION_SIGNAL: f32 = 0.05;
