use crate::bot::deck_scoring::score_card_offer;
use crate::bot::shared::{
    analyze_run_needs, best_potion_replacement, score_reward_potion, RunNeedSnapshot,
};
use crate::content::cards;
use crate::rewards::state::{RewardCard, RewardItem, RewardState};
use crate::state::run::RunState;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardCardAction {
    Pick(usize),
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardClaimAction {
    Claim(usize),
    DiscardPotion(usize),
    Proceed,
}

#[derive(Clone, Debug, Serialize)]
pub struct RewardCardCandidate {
    pub index: usize,
    pub card_name: String,
    pub card_id: String,
    pub score: i32,
    pub base_score: i32,
    pub gap_bonus: i32,
    pub survival_bonus: i32,
    pub situational_bonus: i32,
    pub benefit_score: i32,
    pub clutter_penalty: i32,
    pub penalty_score: i32,
    pub rationale_key: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct RewardDecisionDiagnostics {
    pub recommended_choice: Option<usize>,
    pub recommended_rationale_key: Option<&'static str>,
    pub best_score: i32,
    pub skip_score: i32,
    pub skip_rationale_key: &'static str,
    pub skip_benefit_score: i32,
    pub skip_penalty_score: i32,
    pub skip_situational_bonus: i32,
    pub force_pick: bool,
    pub can_skip: bool,
    pub candidates: Vec<RewardCardCandidate>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RewardClaimDiagnostics {
    pub chosen_index: Option<usize>,
    pub chosen_kind: &'static str,
    pub blocked_potion_offer_count: usize,
    pub rationale_key: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockedPotionOffer {
    pub potion_id: crate::content::potions::PotionId,
}

#[derive(Clone, Copy)]
struct RewardContext<'a> {
    run_state: &'a RunState,
    need: RunNeedSnapshot,
}

#[derive(Clone, Copy)]
struct RewardCardEvaluation {
    base_score: i32,
    gap_bonus: i32,
    survival_bonus: i32,
    situational_bonus: i32,
    clutter_penalty: i32,
    rationale_key: &'static str,
}

#[derive(Clone, Copy)]
struct SkipEvaluation {
    score: i32,
    benefit_score: i32,
    penalty_score: i32,
    situational_bonus: i32,
    rationale_key: &'static str,
}

pub fn decide_cards(
    run_state: &RunState,
    reward_cards: &[RewardCard],
    can_skip: bool,
) -> (RewardCardAction, RewardDecisionDiagnostics) {
    let context = RewardContext {
        run_state,
        need: analyze_run_needs(run_state),
    };
    let mut candidates = reward_cards
        .iter()
        .enumerate()
        .map(|(index, reward_card)| build_candidate(&context, index, reward_card))
        .collect::<Vec<_>>();
    candidates.sort_by(|lhs, rhs| {
        rhs.score
            .cmp(&lhs.score)
            .then_with(|| lhs.index.cmp(&rhs.index))
    });

    let best_choice = candidates.first().cloned();
    let best_score = best_choice
        .as_ref()
        .map(|candidate| candidate.score)
        .unwrap_or(i32::MIN);
    let force_pick = should_force_pick(&context, best_score);
    let skip = evaluate_skip(&context, can_skip);
    let recommended_choice = if force_pick || !can_skip || best_score >= skip.score {
        best_choice.as_ref().map(|candidate| candidate.index)
    } else {
        None
    };
    let recommended_rationale_key = recommended_choice
        .and_then(|idx| {
            candidates
                .iter()
                .find(|candidate| candidate.index == idx)
                .map(|candidate| candidate.rationale_key)
        })
        .or_else(|| {
            best_choice
                .as_ref()
                .filter(|_| force_pick || !can_skip)
                .map(|candidate| candidate.rationale_key)
        });

    let action = recommended_choice
        .map(RewardCardAction::Pick)
        .unwrap_or(RewardCardAction::Skip);
    (
        action,
        RewardDecisionDiagnostics {
            recommended_choice,
            recommended_rationale_key,
            best_score,
            skip_score: skip.score,
            skip_rationale_key: skip.rationale_key,
            skip_benefit_score: skip.benefit_score,
            skip_penalty_score: skip.penalty_score,
            skip_situational_bonus: skip.situational_bonus,
            force_pick,
            can_skip,
            candidates,
        },
    )
}

pub fn decide_claim(
    run_state: &RunState,
    reward: &RewardState,
    blocked_potion_offers: &[BlockedPotionOffer],
) -> (RewardClaimAction, RewardClaimDiagnostics) {
    if let Some((index, _)) = reward
        .items
        .iter()
        .enumerate()
        .find(|(_, item)| !matches!(item, RewardItem::Potion { .. }))
    {
        return (
            RewardClaimAction::Claim(index),
            RewardClaimDiagnostics {
                chosen_index: Some(index),
                chosen_kind: "claim",
                blocked_potion_offer_count: blocked_potion_offers.len(),
                rationale_key: "claim_non_potion_reward",
            },
        );
    }

    if let Some((index, potion_id)) =
        reward
            .items
            .iter()
            .enumerate()
            .find_map(|(index, item)| match item {
                RewardItem::Potion { potion_id } => Some((index, *potion_id)),
                _ => None,
            })
    {
        if run_state.potions.iter().any(|slot| slot.is_none()) {
            return (
                RewardClaimAction::Claim(index),
                RewardClaimDiagnostics {
                    chosen_index: Some(index),
                    chosen_kind: "claim",
                    blocked_potion_offer_count: blocked_potion_offers.len(),
                    rationale_key: "claim_potion_empty_slot",
                },
            );
        }

        let offered_score = score_reward_potion(run_state, potion_id);
        if let Some(discard_idx) = best_potion_replacement(run_state, offered_score, |held| {
            score_reward_potion(run_state, held)
        }) {
            return (
                RewardClaimAction::DiscardPotion(discard_idx),
                RewardClaimDiagnostics {
                    chosen_index: Some(index),
                    chosen_kind: "discard_potion",
                    blocked_potion_offer_count: blocked_potion_offers.len(),
                    rationale_key: "replace_reward_potion",
                },
            );
        }
    }

    if let Some(offer) = blocked_potion_offers
        .iter()
        .max_by_key(|offer| score_reward_potion(run_state, offer.potion_id))
    {
        let offered_score = score_reward_potion(run_state, offer.potion_id);
        if let Some(discard_idx) = best_potion_replacement(run_state, offered_score, |held| {
            score_reward_potion(run_state, held)
        }) {
            return (
                RewardClaimAction::DiscardPotion(discard_idx),
                RewardClaimDiagnostics {
                    chosen_index: None,
                    chosen_kind: "discard_potion",
                    blocked_potion_offer_count: blocked_potion_offers.len(),
                    rationale_key: "replace_blocked_reward_potion",
                },
            );
        }
    }

    (
        RewardClaimAction::Proceed,
        RewardClaimDiagnostics {
            chosen_index: None,
            chosen_kind: "proceed",
            blocked_potion_offer_count: blocked_potion_offers.len(),
            rationale_key: "reward_proceed",
        },
    )
}

fn build_candidate(
    context: &RewardContext<'_>,
    index: usize,
    reward_card: &RewardCard,
) -> RewardCardCandidate {
    let evaluation = evaluate_card(context, reward_card.id);
    let benefit_score = evaluation.base_score
        + evaluation.gap_bonus
        + evaluation.survival_bonus
        + evaluation.situational_bonus;
    let penalty_score = evaluation.clutter_penalty;

    RewardCardCandidate {
        index,
        card_name: cards::get_card_definition(reward_card.id).name.to_string(),
        card_id: format!("{:?}", reward_card.id),
        score: benefit_score - penalty_score,
        base_score: evaluation.base_score,
        gap_bonus: evaluation.gap_bonus,
        survival_bonus: evaluation.survival_bonus,
        situational_bonus: evaluation.situational_bonus,
        benefit_score,
        clutter_penalty: evaluation.clutter_penalty,
        penalty_score,
        rationale_key: evaluation.rationale_key,
    }
}

fn evaluate_card(
    context: &RewardContext<'_>,
    card_id: crate::content::cards::CardId,
) -> RewardCardEvaluation {
    let base_score = score_card_offer(card_id, context.run_state);
    let gap_bonus = gap_patch_bonus(card_id, &context.need);
    let survival_bonus = survival_bonus(card_id, &context.need);
    let situational_bonus = situational_bonus(card_id, context);
    let clutter_penalty = clutter_penalty(card_id, context);
    let rationale_key = dominant_reward_rationale(gap_bonus, survival_bonus, situational_bonus);

    RewardCardEvaluation {
        base_score,
        gap_bonus,
        survival_bonus,
        situational_bonus,
        clutter_penalty,
        rationale_key,
    }
}

fn evaluate_skip(context: &RewardContext<'_>, can_skip: bool) -> SkipEvaluation {
    if !can_skip {
        return SkipEvaluation {
            score: i32::MIN / 4,
            benefit_score: 0,
            penalty_score: 0,
            situational_bonus: 0,
            rationale_key: "reward_cannot_skip",
        };
    }

    let baseline_discipline = if context.run_state.act_num == 1 {
        16
    } else {
        28
    };
    let clutter_relief = clutter_relief_bonus(&context.need);
    let situational_bonus = if context.run_state.act_num >= 2 && context.need.deck_size >= 15 {
        4
    } else {
        0
    };
    let gap_penalty = gap_skip_penalty(&context.need);
    let score = baseline_discipline + clutter_relief + situational_bonus - gap_penalty;
    let rationale_key = if gap_penalty >= clutter_relief + situational_bonus {
        "reward_skip_risky"
    } else if context.need.deck_size >= 18 {
        "reward_skip_dense_deck"
    } else if context.need.purge_pressure >= 100 {
        "reward_skip_clogged_deck"
    } else {
        "reward_skip_baseline"
    };

    SkipEvaluation {
        score,
        benefit_score: baseline_discipline + clutter_relief,
        penalty_score: gap_penalty,
        situational_bonus,
        rationale_key,
    }
}

fn gap_patch_bonus(card_id: crate::content::cards::CardId, need: &RunNeedSnapshot) -> i32 {
    let signals = crate::bot::noncombat_card_signals::signals(card_id);
    let mut bonus = 0;
    if need.damage_gap > 0 {
        bonus += signals.damage_patch_strength.min(need.damage_gap / 2 + 8);
    }
    if need.block_gap > 0 {
        bonus += signals.block_patch_strength.min(need.block_gap / 2 + 8);
    }
    if need.control_gap > 0 {
        bonus += signals.control_patch_strength.min(need.control_gap / 2 + 6);
    }
    bonus
}

fn survival_bonus(card_id: crate::content::cards::CardId, need: &RunNeedSnapshot) -> i32 {
    let signals = crate::bot::noncombat_card_signals::signals(card_id);
    if need.survival_pressure >= 140 {
        signals.frontload_patch_strength / 2
    } else if need.survival_pressure >= 100 {
        signals.frontload_patch_strength / 3
    } else {
        0
    }
}

fn situational_bonus(card_id: crate::content::cards::CardId, context: &RewardContext<'_>) -> i32 {
    let signals = crate::bot::noncombat_card_signals::signals(card_id);
    let early_pick_window = context.run_state.act_num == 1
        && context.run_state.floor_num <= 16
        && context.run_state.master_deck.len() <= 14;
    if !early_pick_window {
        return 0;
    }

    let early_curve_bonus = (signals.damage_patch_strength + signals.frontload_patch_strength) / 3
        - signals.filler_attack_risk;
    early_curve_bonus.max(0)
}

fn clutter_penalty(card_id: crate::content::cards::CardId, context: &RewardContext<'_>) -> i32 {
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

fn dominant_reward_rationale(
    gap_bonus: i32,
    survival_bonus: i32,
    situational_bonus: i32,
) -> &'static str {
    if survival_bonus >= gap_bonus && survival_bonus >= situational_bonus && survival_bonus > 0 {
        "reward_survival_patch"
    } else if gap_bonus >= situational_bonus && gap_bonus > 0 {
        "reward_gap_patch"
    } else if situational_bonus > 0 {
        "reward_early_curve"
    } else {
        "reward_best_offer"
    }
}

fn should_force_pick(context: &RewardContext<'_>, best_score: i32) -> bool {
    context.run_state.act_num == 1
        && context.run_state.floor_num <= 16
        && context.run_state.master_deck.len() <= 14
        && (best_score >= 18
            || context.need.damage_gap + context.need.block_gap + context.need.control_gap >= 24)
}

fn clutter_relief_bonus(need: &RunNeedSnapshot) -> i32 {
    let mut bonus = 0;
    if need.deck_size >= 18 {
        bonus += 10;
    }
    if need.purge_pressure >= 100 {
        bonus += 8;
    }
    bonus
}

fn gap_skip_penalty(need: &RunNeedSnapshot) -> i32 {
    let mut penalty = 0;
    if need.damage_gap + need.block_gap + need.control_gap >= 24 {
        penalty += 12;
    }
    if need.survival_pressure >= 140 {
        penalty += 10;
    }
    penalty
}
