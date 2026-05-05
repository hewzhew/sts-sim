use crate::bot::deck_ops::{self, DeckOperationKind};
use crate::bot::deck_scoring::score_card_offer;
use crate::bot::shared::{
    analyze_run_needs, best_potion_replacement, score_shop_potion, RunNeedSnapshot,
};
use crate::content::cards;
use crate::content::relics::{get_relic_tier, RelicId, RelicTier};
use crate::shop::ShopState;
use crate::state::run::RunState;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopAction {
    BuyCard(usize),
    BuyRelic(usize),
    BuyPotion(usize),
    PurgeCard(usize),
    DiscardPotion(usize),
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopOptionKind {
    Card,
    Relic,
    Potion,
    Purge,
    Leave,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopOptionScore {
    pub kind: ShopOptionKind,
    pub index: Option<usize>,
    pub label: String,
    pub raw_score: i32,
    pub normalized_score: i32,
    pub price: i32,
    pub benefit_score: i32,
    pub penalty_score: i32,
    pub price_penalty: i32,
    pub situational_bonus: i32,
    pub rationale_key: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShopDecisionDiagnostics {
    pub chosen_action: ShopAction,
    pub reserve_gold: i32,
    pub top_options: Vec<ShopOptionScore>,
}

#[derive(Clone, Copy)]
struct ShopContext<'a> {
    run_state: &'a RunState,
    need: RunNeedSnapshot,
    reserve_gold: i32,
    profile: crate::bot::DeckProfile,
    potion_slots_full: bool,
}

#[derive(Clone, Copy)]
struct ShopEvaluation {
    benefit_score: i32,
    penalty_score: i32,
    situational_bonus: i32,
    rationale_key: &'static str,
}

pub fn decide(run_state: &RunState, shop: &ShopState) -> (ShopAction, ShopDecisionDiagnostics) {
    let need = analyze_run_needs(run_state);
    let context = ShopContext {
        run_state,
        reserve_gold: need.gold_reserve,
        need,
        profile: crate::bot::deck_profile(run_state),
        potion_slots_full: run_state.potions.iter().all(|slot| slot.is_some()),
    };
    let mut options = build_shop_options(&context, shop);
    options.sort_by(|lhs, rhs| {
        rhs.normalized_score
            .cmp(&lhs.normalized_score)
            .then_with(|| lhs.price.cmp(&rhs.price))
            .then_with(|| lhs.label.cmp(&rhs.label))
    });

    let chosen_action = options
        .first()
        .map(|best| choose_action_from_option(best, &context))
        .unwrap_or(ShopAction::Leave);

    (
        chosen_action,
        ShopDecisionDiagnostics {
            chosen_action,
            reserve_gold: context.reserve_gold,
            top_options: options.into_iter().take(8).collect(),
        },
    )
}

fn build_shop_options(context: &ShopContext<'_>, shop: &ShopState) -> Vec<ShopOptionScore> {
    let mut options = Vec::new();

    if shop.purge_available {
        let purge = deck_ops::assess(context.run_state, DeckOperationKind::Remove);
        if let Some(candidate) = purge.best_candidate.as_ref() {
            let evaluation = ShopEvaluation {
                benefit_score: purge.total_score.max(0) + context.need.purge_pressure / 6,
                penalty_score: 0,
                situational_bonus: if context.need.deck_size >= 18 { 6 } else { 0 },
                rationale_key: purge.rationale_key,
            };
            options.push(build_option(
                ShopOptionKind::Purge,
                candidate.target_index,
                candidate.label.clone(),
                shop.purge_cost,
                evaluation,
                context.reserve_gold,
            ));
        }
    }

    for (index, card) in shop.cards.iter().enumerate() {
        if !card.can_buy {
            continue;
        }
        let evaluation = evaluate_shop_card(context, card.card_id);
        options.push(build_option(
            ShopOptionKind::Card,
            Some(index),
            cards::get_card_definition(card.card_id).name.to_string(),
            card.price,
            evaluation,
            context.reserve_gold,
        ));
    }

    for (index, relic) in shop.relics.iter().enumerate() {
        if !relic.can_buy {
            continue;
        }
        let evaluation = evaluate_shop_relic(context, relic.relic_id);
        options.push(build_option(
            ShopOptionKind::Relic,
            Some(index),
            format!("{:?}", relic.relic_id),
            relic.price,
            evaluation,
            context.reserve_gold,
        ));
    }

    for (index, potion) in shop.potions.iter().enumerate() {
        let replaceable_blocked_offer = context.potion_slots_full
            && potion.blocked_reason.as_deref() == Some("potion_slots_full");
        if !potion.can_buy && !replaceable_blocked_offer {
            continue;
        }
        let evaluation = evaluate_shop_potion(context, potion.potion_id);
        options.push(build_option(
            ShopOptionKind::Potion,
            Some(index),
            format!("{:?}", potion.potion_id),
            potion.price,
            evaluation,
            context.reserve_gold,
        ));
    }

    options.push(build_option(
        ShopOptionKind::Leave,
        None,
        "Leave".to_string(),
        0,
        evaluate_leave(context),
        context.reserve_gold,
    ));

    options
}

fn build_option(
    kind: ShopOptionKind,
    index: Option<usize>,
    label: String,
    price: i32,
    evaluation: ShopEvaluation,
    reserve_gold: i32,
) -> ShopOptionScore {
    let raw_score =
        evaluation.benefit_score + evaluation.situational_bonus - evaluation.penalty_score;
    let price_penalty = normalized_price(price, reserve_gold);
    ShopOptionScore {
        kind,
        index,
        label,
        raw_score,
        normalized_score: raw_score - price_penalty,
        price,
        benefit_score: evaluation.benefit_score,
        penalty_score: evaluation.penalty_score,
        price_penalty,
        situational_bonus: evaluation.situational_bonus,
        rationale_key: evaluation.rationale_key,
    }
}

fn choose_action_from_option(best: &ShopOptionScore, context: &ShopContext<'_>) -> ShopAction {
    match (best.kind, best.index) {
        (ShopOptionKind::Card, Some(index)) => ShopAction::BuyCard(index),
        (ShopOptionKind::Relic, Some(index)) => ShopAction::BuyRelic(index),
        (ShopOptionKind::Potion, Some(index)) => {
            if context.potion_slots_full {
                if let Some(discard_idx) =
                    best_potion_replacement(context.run_state, best.raw_score, |held| {
                        score_shop_potion(context.run_state, held)
                    })
                {
                    ShopAction::DiscardPotion(discard_idx)
                } else {
                    ShopAction::Leave
                }
            } else {
                ShopAction::BuyPotion(index)
            }
        }
        (ShopOptionKind::Purge, Some(index)) => ShopAction::PurgeCard(index),
        _ => ShopAction::Leave,
    }
}

fn evaluate_shop_card(
    context: &ShopContext<'_>,
    card_id: crate::content::cards::CardId,
) -> ShopEvaluation {
    let base_score = score_card_offer(card_id, context.run_state);
    let gap_bonus = shop_gap_bonus(card_id, &context.need);
    let early_window_bonus = if context.run_state.act_num == 1 && context.need.deck_size <= 14 {
        6
    } else {
        0
    };
    let penalty_score = clutter_penalty(card_id, context);
    let rationale_key = if gap_bonus > early_window_bonus && gap_bonus > 0 {
        "buy_card_gap_patch"
    } else if early_window_bonus > 0 {
        "buy_card_early_curve"
    } else {
        "buy_card_offer_value"
    };

    ShopEvaluation {
        benefit_score: base_score + gap_bonus,
        penalty_score,
        situational_bonus: early_window_bonus,
        rationale_key,
    }
}

fn evaluate_shop_relic(context: &ShopContext<'_>, relic_id: RelicId) -> ShopEvaluation {
    let tier_benefit = match get_relic_tier(relic_id) {
        RelicTier::Rare => 42,
        RelicTier::Shop => 36,
        RelicTier::Uncommon => 30,
        _ => 24,
    };

    match relic_id {
        RelicId::MembershipCard => ShopEvaluation {
            benefit_score: tier_benefit + 34,
            penalty_score: 0,
            situational_bonus: 8,
            rationale_key: "buy_relic_discount_engine",
        },
        RelicId::ChemicalX => ShopEvaluation {
            benefit_score: tier_benefit + 8 + context.profile.x_cost_payoffs * 20,
            penalty_score: 0,
            situational_bonus: 0,
            rationale_key: "buy_relic_x_payoff",
        },
        RelicId::OrangePellets => ShopEvaluation {
            benefit_score: tier_benefit + 18,
            penalty_score: 0,
            situational_bonus: if context.need.survival_pressure >= 120 {
                6
            } else {
                0
            },
            rationale_key: "buy_relic_debuff_cleanse",
        },
        RelicId::MedicalKit => ShopEvaluation {
            benefit_score: tier_benefit + 16,
            penalty_score: 0,
            situational_bonus: if context.profile.status_generators > 0 {
                6
            } else {
                0
            },
            rationale_key: "buy_relic_status_patch",
        },
        RelicId::Toolbox => ShopEvaluation {
            benefit_score: tier_benefit + 12,
            penalty_score: 0,
            situational_bonus: 0,
            rationale_key: "buy_relic_opening_help",
        },
        RelicId::ClockworkSouvenir | RelicId::Cauldron | RelicId::QuestionCard => ShopEvaluation {
            benefit_score: tier_benefit + 14,
            penalty_score: 0,
            situational_bonus: 0,
            rationale_key: "buy_relic_generic_value",
        },
        RelicId::FrozenEye => ShopEvaluation {
            benefit_score: tier_benefit
                + if context.need.survival_pressure >= 120 {
                    16
                } else {
                    8
                },
            penalty_score: 0,
            situational_bonus: 0,
            rationale_key: "buy_relic_planning_safety",
        },
        RelicId::SmilingMask => ShopEvaluation {
            benefit_score: tier_benefit
                + if context.need.purge_pressure >= 100 {
                    22
                } else {
                    8
                },
            penalty_score: 0,
            situational_bonus: 0,
            rationale_key: "buy_relic_purge_discount",
        },
        RelicId::HandDrill | RelicId::DiscerningMonocle => ShopEvaluation {
            benefit_score: tier_benefit,
            penalty_score: 10,
            situational_bonus: 0,
            rationale_key: "buy_relic_low_impact",
        },
        _ => ShopEvaluation {
            benefit_score: tier_benefit,
            penalty_score: if context.run_state.gold < 120 { 6 } else { 0 },
            situational_bonus: 0,
            rationale_key: "buy_relic_baseline",
        },
    }
}

fn evaluate_shop_potion(
    context: &ShopContext<'_>,
    potion_id: crate::content::potions::PotionId,
) -> ShopEvaluation {
    let benefit_score = score_shop_potion(context.run_state, potion_id);
    let slot_penalty = if context.potion_slots_full { 6 } else { 0 };
    let urgency_bonus = if context.need.survival_pressure >= 120 {
        6
    } else {
        0
    };

    ShopEvaluation {
        benefit_score,
        penalty_score: slot_penalty,
        situational_bonus: urgency_bonus,
        rationale_key: if context.potion_slots_full {
            "buy_potion_replace_held"
        } else {
            "buy_potion_baseline"
        },
    }
}

fn evaluate_leave(context: &ShopContext<'_>) -> ShopEvaluation {
    let reserve_pressure = if context.run_state.gold <= context.reserve_gold {
        28
    } else if context.run_state.gold <= context.reserve_gold + 50 {
        18
    } else {
        8
    };
    let clutter_relief = if context.need.purge_pressure >= 100 && context.run_state.gold < 75 {
        8
    } else {
        0
    };
    let urgency_penalty = if context.need.survival_pressure >= 120 && context.run_state.gold >= 120
    {
        12
    } else {
        0
    };
    let rationale_key = if reserve_pressure >= 24 {
        "leave_shop_preserve_gold"
    } else if clutter_relief > 0 {
        "leave_shop_wait_for_purge"
    } else {
        "leave_shop"
    };

    ShopEvaluation {
        benefit_score: reserve_pressure + clutter_relief,
        penalty_score: urgency_penalty,
        situational_bonus: 0,
        rationale_key,
    }
}

fn normalized_price(price: i32, reserve_gold: i32) -> i32 {
    price / 12 + reserve_gold / 8
}

fn shop_gap_bonus(card_id: crate::content::cards::CardId, need: &RunNeedSnapshot) -> i32 {
    let signals = crate::bot::noncombat_card_signals::signals(card_id);
    let mut bonus = 0;
    if need.damage_gap > 0 {
        bonus += signals.damage_patch_strength.min(need.damage_gap / 2 + 6);
    }
    if need.block_gap > 0 {
        bonus += signals.block_patch_strength.min(need.block_gap / 2 + 6);
    }
    if need.control_gap > 0 {
        bonus += signals.control_patch_strength.min(need.control_gap / 2 + 4);
    }
    bonus
}

fn clutter_penalty(card_id: crate::content::cards::CardId, context: &ShopContext<'_>) -> i32 {
    let signals = crate::bot::noncombat_card_signals::signals(card_id);
    let late_penalty = if context.run_state.act_num >= 2 {
        signals.filler_attack_risk * 6
    } else {
        signals.filler_attack_risk * 3
    };
    let density_penalty = if context.need.deck_size >= 18
        && context.need.damage_gap + context.need.block_gap + context.need.control_gap == 0
    {
        signals.filler_attack_risk * 6
    } else {
        0
    };
    late_penalty + density_penalty
}
