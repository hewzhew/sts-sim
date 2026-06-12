use super::{
    compile_decision, ledger_from_snapshot, CandidateAction, CandidateDelta, CandidateRole,
    LedgerDelta, OpportunityCost, PressureKind, StrategicDebt, StrategicDecisionSite,
    StrategicDeckFacts, StrategicJob, StrategicSnapshot, VerdictHint,
};
use crate::ai::shop_policy_v1::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPolicyClassV1, ShopPurchaseTargetV1,
};

pub fn strategic_trace_for_shop(context: &ShopDecisionContextV1) -> super::StrategicDecisionTrace {
    let snapshot = snapshot_from_shop_context(context);
    let ledger = ledger_from_snapshot(&snapshot);
    let deltas = context
        .candidates
        .iter()
        .map(candidate_delta_from_shop_candidate)
        .collect::<Vec<_>>();
    compile_decision(snapshot, ledger, context.candidates.len(), deltas)
}

fn snapshot_from_shop_context(context: &ShopDecisionContextV1) -> StrategicSnapshot {
    let deck = &context.strategy.v1.deck;
    StrategicSnapshot {
        site: StrategicDecisionSite::Shop,
        act: context.need.act,
        floor: context.need.floor,
        boss: None,
        hp: 0,
        max_hp: 0,
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

fn candidate_delta_from_shop_candidate(candidate: &ShopCandidateEvidenceV1) -> CandidateDelta {
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
                delta.positive.push(LedgerDelta {
                    kind: PressureKind::MissingJob(StrategicJob::Frontload),
                    amount: purchase_priority_amount(candidate.purchase_priority),
                    reason: "shop_card_purchase_priority".to_string(),
                });
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
