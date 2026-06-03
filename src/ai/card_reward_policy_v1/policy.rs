use crate::content::cards::{get_card_definition, CardType};
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::facts::{
    deck_needs, draw_value, effective_cost, is_aoe_card, premium_value, rarity_value, risk_penalty,
    scaling_value,
};
use super::types::{
    CardRewardCandidateScoreV1, CardRewardDecisionV1, CardRewardPolicyActionV1,
    CardRewardPolicyConfigV1, CardRewardScoreTermsV1, DeckNeedsV1,
};

pub fn plan_card_reward_decision_v1(
    run_state: &RunState,
    cards: &[RewardCard],
    config: &CardRewardPolicyConfigV1,
) -> CardRewardDecisionV1 {
    let needs = deck_needs(run_state, config);
    let mut candidates = cards
        .iter()
        .enumerate()
        .map(|(index, card)| score_candidate(index, card, &needs))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.index.cmp(&right.index))
    });

    let action = match candidates.as_slice() {
        [] => CardRewardPolicyActionV1::Stop {
            reason: "no visible card reward candidates".to_string(),
        },
        [best] if best.score >= config.min_auto_pick_score => {
            pick_action(best, 0.75, "single card clears score gate")
        }
        [best] => CardRewardPolicyActionV1::Stop {
            reason: format!(
                "single card {} score {:.2} does not clear auto-pick score gate {:.2}",
                best.name, best.score, config.min_auto_pick_score
            ),
        },
        [best, second, ..]
            if best.score >= config.min_auto_pick_score
                && best.score - second.score >= config.min_auto_pick_margin =>
        {
            pick_action(
                best,
                confidence_from_margin(best.score - second.score),
                "best card clears score and margin gates",
            )
        }
        [best, second, ..] => CardRewardPolicyActionV1::Stop {
            reason: format!(
                "best card {} score {:.2} is not separated enough from {} score {:.2}",
                best.name, best.score, second.name, second.score
            ),
        },
    };

    CardRewardDecisionV1 {
        action,
        candidates,
        label_role: "behavior_policy_not_teacher",
    }
}

fn pick_action(
    best: &CardRewardCandidateScoreV1,
    confidence: f32,
    reason: &'static str,
) -> CardRewardPolicyActionV1 {
    CardRewardPolicyActionV1::Pick {
        index: best.index,
        card: best.card,
        confidence,
        reason: reason.to_string(),
    }
}

fn confidence_from_margin(margin: f32) -> f32 {
    (0.65 + margin / 10.0).clamp(0.65, 0.95)
}

fn score_candidate(
    index: usize,
    reward_card: &RewardCard,
    needs: &DeckNeedsV1,
) -> CardRewardCandidateScoreV1 {
    let def = get_card_definition(reward_card.id);
    let upgrades = f32::from(reward_card.upgrades);
    let damage = (def.base_damage + def.upgrade_damage * i32::from(reward_card.upgrades)).max(0);
    let block = (def.base_block + def.upgrade_block * i32::from(reward_card.upgrades)).max(0);
    let cost = effective_cost(reward_card.id);
    let mut notes = Vec::new();

    let mut terms = CardRewardScoreTermsV1 {
        frontload: frontload_score(def.card_type, damage, cost, needs.need_frontload),
        block: block_score(block, cost, needs.need_block),
        draw: draw_value(reward_card.id) * needs.need_draw,
        scaling: scaling_value(reward_card.id) * needs.need_scaling,
        aoe: if is_aoe_card(reward_card.id) {
            1.4
        } else {
            0.0
        },
        exhaust_synergy: exhaust_synergy_score(reward_card.id, needs),
        rarity: rarity_value(reward_card.id),
        premium: premium_value(reward_card.id),
        risk: risk_penalty(reward_card.id, needs),
        bloat: bloat_penalty(def.card_type, needs),
    };

    if terms.premium > 0.0 {
        notes.push("premium");
    }
    if terms.draw > 0.0 {
        notes.push("draw");
    }
    if terms.scaling > 0.0 {
        notes.push("scaling");
    }
    if terms.risk < 0.0 {
        notes.push("conditional-risk");
    }
    if upgrades > 0.0 {
        terms.premium += 0.35 * upgrades;
        notes.push("upgraded");
    }

    CardRewardCandidateScoreV1 {
        index,
        card: reward_card.id,
        name: def.name,
        card_type: def.card_type,
        score: terms.total(),
        terms,
        notes,
    }
}

fn frontload_score(card_type: CardType, damage: i32, cost: f32, need_frontload: f32) -> f32 {
    if card_type != CardType::Attack || damage <= 0 {
        return 0.0;
    }
    let efficiency = damage as f32 / cost;
    (efficiency / 3.0).min(4.5) * need_frontload
}

fn block_score(block: i32, cost: f32, need_block: f32) -> f32 {
    if block <= 0 {
        return 0.0;
    }
    let efficiency = block as f32 / cost;
    (efficiency / 3.5).min(3.6) * need_block
}

fn exhaust_synergy_score(card_id: crate::content::cards::CardId, needs: &DeckNeedsV1) -> f32 {
    if !needs.has_exhaust_payoff {
        return 0.0;
    }
    match card_id {
        crate::content::cards::CardId::FiendFire
        | crate::content::cards::CardId::SecondWind
        | crate::content::cards::CardId::SeverSoul
        | crate::content::cards::CardId::BurningPact => 1.1,
        _ => 0.0,
    }
}

fn bloat_penalty(card_type: CardType, needs: &DeckNeedsV1) -> f32 {
    if !needs.is_late_deck {
        return 0.0;
    }
    match card_type {
        CardType::Attack | CardType::Skill => -0.8,
        CardType::Power => -0.4,
        CardType::Status | CardType::Curse => -2.0,
    }
}
