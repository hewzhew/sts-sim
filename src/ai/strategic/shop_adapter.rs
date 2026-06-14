use super::{
    compile_decision, ledger_from_snapshot, CandidateAction, CandidateDelta, CandidateRole,
    LedgerDelta, OpportunityCost, PressureKind, StrategicDebt, StrategicDecisionSite,
    StrategicDeckFacts, StrategicJob, StrategicSnapshot, VerdictHint,
};
use crate::ai::card_component_marginal_value_v1::{
    evaluate_card_component_marginal_value_v1, CardComponentMarginalContextV1,
};
use crate::ai::card_reward_policy_v1::{card_reward_semantic_profile_v1, CardRewardSemanticRoleV1};
use crate::ai::decision_tags_v1::{
    strings_have_tag, TAG_COLLECTOR_ANSWER, TAG_ENGINE_CLOSURE, TAG_STARTUP_ACCESS,
};
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
use crate::ai::shop_policy_v1::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPolicyClassV1, ShopPurchaseTargetV1,
};
use crate::state::rewards::RewardCard;

pub fn strategic_trace_for_shop(context: &ShopDecisionContextV1) -> super::StrategicDecisionTrace {
    let snapshot = snapshot_from_shop_context(context);
    let ledger = ledger_from_snapshot(&snapshot);
    let deltas = context
        .candidates
        .iter()
        .map(|candidate| candidate_delta_from_shop_candidate(context, candidate))
        .collect::<Vec<_>>();
    compile_decision(snapshot, ledger, context.candidates.len(), deltas)
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
        ShopPolicyClassV1::CursePurge | ShopPolicyClassV1::StarterStrikePurge => {
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
            delta.verdict_hint = purchase_verdict_hint(candidate.purchase_priority);
            if candidate.card.is_some() {
                add_shop_card_purchase_deltas(context, candidate, &mut delta);
                delta.negative.push(LedgerDelta {
                    kind: PressureKind::DeckDebt(StrategicDebt::CycleTime),
                    amount: 0.12,
                    reason: "shop_card_adds_cycle_card".to_string(),
                });
            } else {
                delta.positive.push(LedgerDelta {
                    kind: PressureKind::EconomyNeed,
                    amount: purchase_priority_amount(candidate.purchase_priority),
                    reason: "shop_purchase_converts_gold".to_string(),
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

    delta
}

fn add_shop_card_purchase_deltas(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
) {
    let priority_amount = purchase_priority_amount(candidate.purchase_priority);

    if candidate_has_evidence(candidate, TAG_COLLECTOR_ANSWER) {
        delta.role = CandidateRole::BossAnswer;
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::StatusControl),
            amount: priority_amount.max(0.45),
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
            amount: priority_amount.max(0.45),
            reason: TAG_ENGINE_CLOSURE.to_string(),
        });
    }

    if candidate_has_evidence(candidate, TAG_STARTUP_ACCESS) {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::Lubricant;
        }
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::DrawEnergy),
            amount: priority_amount.max(0.35),
            reason: TAG_STARTUP_ACCESS.to_string(),
        });
    }

    add_default_shop_card_semantic_deltas(candidate, delta, priority_amount);
    add_shop_card_component_deltas(context, candidate, delta);
}

fn add_default_shop_card_semantic_deltas(
    candidate: &ShopCandidateEvidenceV1,
    delta: &mut CandidateDelta,
    priority_amount: f32,
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
        delta.role = CandidateRole::Transition;
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::Frontload),
            amount: priority_amount,
            reason: "shop_card_adds_frontload".to_string(),
        });
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
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::Block),
            amount: priority_amount.max(0.25),
            reason: "shop_card_adds_block_or_mitigation".to_string(),
        });
    }
    if profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnergySource)
    {
        if delta.role == CandidateRole::ResourceConversion {
            delta.role = CandidateRole::Lubricant;
        }
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::DrawEnergy),
            amount: priority_amount.max(0.25),
            reason: "shop_card_adds_draw_or_energy".to_string(),
        });
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
        delta.positive.push(LedgerDelta {
            kind: PressureKind::MissingJob(StrategicJob::ExhaustAccess),
            amount: priority_amount.max(0.25),
            reason: "shop_card_adds_exhaust_access".to_string(),
        });
    }
    if delta.positive.is_empty() {
        delta.positive.push(LedgerDelta {
            kind: PressureKind::EconomyNeed,
            amount: priority_amount,
            reason: "shop_card_converts_gold_without_specific_job".to_string(),
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
        &component_context_from_shop_context(context),
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
    delta.positive.extend(component_delta.positive);
    delta.negative.extend(component_delta.negative);
    delta.notes.extend(component_delta.notes);
    delta.evidence.extend(component_delta.evidence);
}

fn component_context_from_shop_context(
    context: &ShopDecisionContextV1,
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
        formation_needs: context.strategy.formation_summary().needs,
        startup: DeckStartupProfileV1 {
            feel_no_pain_count: deck.exhaust_payoffs,
            exhaust_engine_count: deck.exhaust_generators,
            exhaust_payoff_count: deck.exhaust_payoffs,
            status_generator_count: deck.status_generators,
            status_digest_count: deck.status_payoffs,
            strong_draw_count: deck.draw_sources,
            persistent_strength_source_count: deck.strength_sources,
            strength_payoff_count: deck.strength_payoffs,
            ..Default::default()
        },
    }
}

fn candidate_has_evidence(candidate: &ShopCandidateEvidenceV1, tag: &str) -> bool {
    strings_have_tag(&candidate.evidence, tag)
}

fn purchase_verdict_hint(priority: Option<i32>) -> VerdictHint {
    match priority.unwrap_or_default() {
        value if value >= 900 => VerdictHint::StrongTake,
        value if value >= 650 => VerdictHint::ContextTake,
        value if value >= 250 => VerdictHint::Speculative,
        _ => VerdictHint::SkipPreferred,
    }
}

fn purchase_priority_amount(priority: Option<i32>) -> f32 {
    (priority.unwrap_or_default().max(0) as f32 / 1000.0).clamp(0.0, 1.0)
}
