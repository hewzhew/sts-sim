use crate::bot::card_taxonomy::{is_strength_enabler, is_strength_payoff};
use crate::runtime::combat::CombatState;
use crate::content::cards::{self, CardId, CardType};

use super::helpers::{
    effective_block, effective_damage, is_block_core_card, is_draw_core_card,
    is_exhaust_engine_card, is_exhaust_outlet_card, is_keeper_priority_card, is_setup_power_card,
    is_status_or_curse_card, monster_is_attacking, total_incoming_damage,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExhaustDispositionStats {
    pub junk_count: i32,
    pub protected_count: i32,
    pub core_count: i32,
    pub near_core_count: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandCardRole {
    CoreKeeper,
    SequencedPiece,
    SituationalResource,
    LowValueFuel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardRoleContext {
    pub total_incoming: i32,
    pub unblocked_incoming: i32,
    pub missing_hp: i32,
    pub energy: i32,
    pub has_attacking_target: bool,
    pub playable_attack_count: i32,
    pub followup_attack_count: i32,
    pub strength_payoff_count: i32,
    pub has_exhaust_outlet: bool,
    pub has_exhaust_engine: bool,
    pub status_or_curse_count: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CombatCardEval {
    id: CardId,
    can_play_now: bool,
    block: i32,
    damage: i32,
    starter_basic: bool,
    status_or_curse: bool,
    keeper_priority: bool,
    setup_power: bool,
    draw_core: bool,
    exhaust_outlet: bool,
    exhaust_engine: bool,
    block_core: bool,
    strength_enabler: bool,
    strength_payoff: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CombatDispositionInputs {
    followup_attack_count: i32,
    has_strength_followup: bool,
    low_value_non_attacks: i32,
}

pub(crate) fn build_context(combat: &CombatState) -> CardRoleContext {
    let total_incoming = total_incoming_damage(combat);
    let unblocked_incoming = (total_incoming - combat.entities.player.block).max(0);
    let hand_attack_count = combat
        .zones
        .hand
        .iter()
        .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
        .filter(|card| cards::get_card_definition(card.id).card_type == CardType::Attack)
        .count() as i32;
    let strength_payoff_count = combat
        .zones
        .hand
        .iter()
        .filter(|card| is_strength_payoff(card.id))
        .count() as i32;
    let has_exhaust_outlet = combat
        .zones
        .hand
        .iter()
        .any(|card| is_exhaust_outlet_card(card.id));
    let has_exhaust_engine = combat
        .zones
        .hand
        .iter()
        .any(|card| is_exhaust_engine_card(card.id));
    let status_or_curse_count = combat
        .zones
        .hand
        .iter()
        .filter(|card| is_status_or_curse_card(card.id))
        .count() as i32;

    CardRoleContext {
        total_incoming,
        unblocked_incoming,
        missing_hp: (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0),
        energy: combat.turn.energy as i32,
        has_attacking_target: combat.entities.monsters.iter().any(monster_is_attacking),
        playable_attack_count: hand_attack_count,
        followup_attack_count: hand_attack_count,
        strength_payoff_count,
        has_exhaust_outlet,
        has_exhaust_engine,
        status_or_curse_count,
    }
}

pub(crate) fn classify_hand_card_with_context(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
) -> HandCardRole {
    let Some(eval) = evaluate_hand_card(combat, hand_index) else {
        return HandCardRole::LowValueFuel;
    };
    let inputs = derive_inputs_for_hand(combat, hand_index, context);
    classify_eval(combat, hand_index, context, &eval, &inputs)
}

pub(crate) fn keeper_score_with_context(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
) -> i32 {
    let Some(eval) = evaluate_hand_card(combat, hand_index) else {
        return i32::MIN / 4;
    };
    let inputs = derive_inputs_for_hand(combat, hand_index, context);
    keeper_score_for_eval(combat, hand_index, context, &eval, &inputs)
}

pub(crate) fn fuel_score_with_context(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
) -> i32 {
    let Some(eval) = evaluate_hand_card(combat, hand_index) else {
        return i32::MIN / 4;
    };
    let inputs = derive_inputs_for_hand(combat, hand_index, context);
    fuel_score_for_eval(combat, hand_index, context, &eval, &inputs)
}

pub(crate) fn combat_copy_score_for_uuid(combat: &CombatState, uuid: u32) -> i32 {
    let context = build_context(combat);
    combat
        .zones
        .hand
        .iter()
        .position(|card| card.uuid == uuid)
        .map(|idx| combat_copy_score(combat, idx, &context))
        .unwrap_or(i32::MIN / 4)
}

pub(crate) fn combat_exhaust_score_for_uuid(combat: &CombatState, uuid: u32) -> i32 {
    let context = build_context(combat);
    combat
        .zones
        .hand
        .iter()
        .position(|card| card.uuid == uuid)
        .map(|idx| combat_exhaust_score(combat, idx, &context))
        .unwrap_or(i32::MIN / 4)
}

pub(crate) fn combat_retention_score_for_uuid(combat: &CombatState, uuid: u32) -> i32 {
    let context = build_context(combat);
    combat
        .zones
        .hand
        .iter()
        .position(|card| card.uuid == uuid)
        .map(|idx| keeper_score_with_context(combat, idx, &context))
        .unwrap_or(i32::MIN / 4)
}

pub(crate) fn exhaust_disposition_stats(
    combat: &CombatState,
    candidate_uuids: &[u32],
) -> ExhaustDispositionStats {
    let mut stats = ExhaustDispositionStats::default();
    for uuid in candidate_uuids {
        let exhaust = combat_exhaust_score_for_uuid(combat, *uuid);
        let retention = combat_retention_score_for_uuid(combat, *uuid);
        if exhaust > 0 {
            stats.junk_count += 1;
        }
        if retention > 0 {
            stats.protected_count += 1;
        }
        if retention >= 8_000 {
            stats.core_count += 1;
        } else if retention >= 2_500 {
            stats.near_core_count += 1;
        }
    }
    stats
}

pub(crate) fn count_remaining_low_value_exhaust_candidates(
    combat: &CombatState,
    candidate_uuids: &[u32],
    excluded_uuids: &[u32],
) -> i32 {
    candidate_uuids
        .iter()
        .filter(|uuid| !excluded_uuids.contains(uuid))
        .filter(|uuid| combat_exhaust_score_for_uuid(combat, **uuid) > 0)
        .count() as i32
}

pub(crate) fn best_exhaust_candidate_uuid(
    combat: &CombatState,
    candidate_uuids: &[u32],
) -> Option<u32> {
    candidate_uuids
        .iter()
        .max_by_key(|uuid| combat_exhaust_score_for_uuid(combat, **uuid))
        .copied()
}

fn combat_copy_score(combat: &CombatState, hand_index: usize, context: &CardRoleContext) -> i32 {
    let Some(eval) = evaluate_hand_card(combat, hand_index) else {
        return i32::MIN / 4;
    };
    let inputs = derive_inputs_for_hand(combat, hand_index, context);
    copy_score_for_eval(combat, hand_index, context, &eval, &inputs)
}

fn combat_exhaust_score(combat: &CombatState, hand_index: usize, context: &CardRoleContext) -> i32 {
    let Some(eval) = evaluate_hand_card(combat, hand_index) else {
        return i32::MIN / 4;
    };
    let inputs = derive_inputs_for_hand(combat, hand_index, context);
    exhaust_score_for_eval(combat, hand_index, context, &eval, &inputs)
}

fn classify_eval(
    combat: &CombatState,
    _hand_index: usize,
    context: &CardRoleContext,
    eval: &CombatCardEval,
    inputs: &CombatDispositionInputs,
) -> HandCardRole {
    if eval.status_or_curse {
        return HandCardRole::LowValueFuel;
    }

    if eval.can_play_now && context.unblocked_incoming > 0 && eval.block > 0 {
        return HandCardRole::CoreKeeper;
    }

    match eval.id {
        CardId::Offering
        | CardId::BattleTrance
        | CardId::Shockwave
        | CardId::Disarm
        | CardId::Apotheosis => {
            return if eval.can_play_now {
                HandCardRole::SequencedPiece
            } else {
                HandCardRole::CoreKeeper
            };
        }
        CardId::SpotWeakness => {
            return if context.has_attacking_target && inputs.has_strength_followup {
                HandCardRole::SequencedPiece
            } else {
                HandCardRole::SituationalResource
            };
        }
        CardId::Rage => {
            return if eval.can_play_now && inputs.followup_attack_count >= 2 {
                HandCardRole::SequencedPiece
            } else if eval.can_play_now && inputs.followup_attack_count >= 1 {
                HandCardRole::SituationalResource
            } else {
                HandCardRole::LowValueFuel
            };
        }
        CardId::SecondWind => {
            let fuel_ready = context.status_or_curse_count > 0 || inputs.low_value_non_attacks > 0;
            return if fuel_ready && (context.unblocked_incoming > 0 || context.has_exhaust_engine) {
                HandCardRole::SequencedPiece
            } else {
                HandCardRole::SituationalResource
            };
        }
        _ => {}
    }

    if eval.keeper_priority {
        return HandCardRole::CoreKeeper;
    }

    if eval.setup_power && eval.can_play_now {
        return if combat.turn.turn_count <= 2 {
            HandCardRole::SequencedPiece
        } else {
            HandCardRole::SituationalResource
        };
    }

    if eval.draw_core && eval.can_play_now {
        return HandCardRole::SequencedPiece;
    }

    if eval.exhaust_outlet {
        return HandCardRole::SituationalResource;
    }

    if eval.starter_basic {
        if context.unblocked_incoming > 0 && eval.block > 0 {
            return HandCardRole::CoreKeeper;
        }
        if eval.damage > 0 && context.total_incoming <= 0 {
            return HandCardRole::SituationalResource;
        }
        return HandCardRole::LowValueFuel;
    }

    if !eval.can_play_now {
        return HandCardRole::LowValueFuel;
    }

    if eval.block_core && eval.block > 0 {
        return HandCardRole::SituationalResource;
    }

    if eval.strength_payoff && inputs.has_strength_followup {
        return HandCardRole::SituationalResource;
    }

    HandCardRole::SituationalResource
}

fn keeper_score_for_eval(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
    eval: &CombatCardEval,
    inputs: &CombatDispositionInputs,
) -> i32 {
    let mut score = match classify_eval(combat, hand_index, context, eval, inputs) {
        HandCardRole::CoreKeeper => 12_000,
        HandCardRole::SequencedPiece => 8_000,
        HandCardRole::SituationalResource => 2_000,
        HandCardRole::LowValueFuel => -3_000,
    };

    if eval.status_or_curse {
        return -16_000;
    }
    if context.unblocked_incoming > 0 && eval.block > 0 && eval.can_play_now {
        score += context.unblocked_incoming.min(eval.block) * 220 + 1_200;
    }
    if eval.strength_enabler && context.has_attacking_target {
        score += 1_600 + context.strength_payoff_count * 500;
    }
    if eval.id == CardId::Rage {
        score += inputs.followup_attack_count * 1_400;
    }
    if eval.setup_power && combat.turn.turn_count <= 2 {
        score += 1_400;
    }
    if eval.starter_basic && context.unblocked_incoming <= 0 && eval.damage < 8 && eval.block < 6 {
        score -= 3_200;
    }
    if !eval.can_play_now {
        score -= 1_200;
    }
    if eval.draw_core {
        score += 900;
    }
    score
}

fn fuel_score_for_eval(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
    eval: &CombatCardEval,
    inputs: &CombatDispositionInputs,
) -> i32 {
    let mut score = match classify_eval(combat, hand_index, context, eval, inputs) {
        HandCardRole::LowValueFuel => 6_000,
        HandCardRole::SituationalResource => 1_200,
        HandCardRole::SequencedPiece => -4_500,
        HandCardRole::CoreKeeper => -9_000,
    };

    if eval.status_or_curse {
        return 20_000;
    }
    if !eval.can_play_now {
        score += 1_800;
    }
    if eval.starter_basic && context.unblocked_incoming <= 0 {
        score += 2_000;
    }
    if (eval.starter_basic || eval.status_or_curse) && context.has_exhaust_outlet {
        score += 1_200;
    }
    if context.unblocked_incoming > 0 && eval.block > 0 && eval.can_play_now {
        score -= context.unblocked_incoming.min(eval.block) * 300 + 2_400;
    }
    if matches!(eval.id, CardId::SpotWeakness | CardId::Rage) {
        score -= 1_800;
    }
    score
}

fn copy_score_for_eval(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
    eval: &CombatCardEval,
    inputs: &CombatDispositionInputs,
) -> i32 {
    let mut score = keeper_score_for_eval(combat, hand_index, context, eval, inputs);

    if eval.status_or_curse {
        return i32::MIN / 4;
    }

    score -= fuel_score_for_eval(combat, hand_index, context, eval, inputs) / 4;

    if eval.starter_basic {
        score -= 2_800;
    }
    if let Some(card) = combat.zones.hand.get(hand_index) {
        if card.upgrades > 0 {
            score += 500 + card.upgrades as i32 * 220;
        }
    }
    if eval.setup_power {
        score += if combat.meta.is_boss_fight {
            1_800
        } else if combat.meta.is_elite_fight {
            1_100
        } else {
            700
        };
    }
    if eval.draw_core {
        score += 650;
    }
    if eval.strength_enabler && context.strength_payoff_count >= 1 {
        score += 850;
    }
    if eval.exhaust_engine && context.has_exhaust_outlet {
        score += 950;
    }

    score
}

fn exhaust_score_for_eval(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
    eval: &CombatCardEval,
    inputs: &CombatDispositionInputs,
) -> i32 {
    let retention = keeper_score_for_eval(combat, hand_index, context, eval, inputs);
    let fuel = fuel_score_for_eval(combat, hand_index, context, eval, inputs);
    let mut score = fuel - retention / 2;

    if eval.status_or_curse {
        score += 6_000;
    }
    if let Some(card) = combat.zones.hand.get(hand_index) {
        if card.upgrades > 0 {
            score -= 350 + card.upgrades as i32 * 180;
        }
    }
    if eval.keeper_priority {
        score -= 1_400;
    }
    if eval.setup_power {
        score -= if combat.meta.is_boss_fight || combat.meta.is_elite_fight {
            2_000
        } else {
            1_100
        };
    }
    if eval.draw_core {
        score -= 700;
    }

    score
}

fn derive_inputs_for_hand(
    combat: &CombatState,
    hand_index: usize,
    context: &CardRoleContext,
) -> CombatDispositionInputs {
    let followup_attack_count = count_followup_attacks(combat, hand_index);
    let low_value_non_attacks = combat
        .zones
        .hand
        .get(hand_index)
        .filter(|card| card.id == CardId::SecondWind)
        .map(|_| count_low_value_non_attacks(combat, hand_index, context))
        .unwrap_or(0);
    CombatDispositionInputs {
        followup_attack_count,
        has_strength_followup: context.strength_payoff_count > 0
            || followup_attack_count > 0
            || context.playable_attack_count > 1,
        low_value_non_attacks,
    }
}

fn count_followup_attacks(combat: &CombatState, excluding_hand_index: usize) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            *idx != excluding_hand_index
                && crate::content::cards::can_play_card(card, combat).is_ok()
                && cards::get_card_definition(card.id).card_type == CardType::Attack
        })
        .count() as i32
}

fn count_low_value_non_attacks(
    combat: &CombatState,
    excluding_hand_index: usize,
    context: &CardRoleContext,
) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            *idx != excluding_hand_index
                && cards::get_card_definition(card.id).card_type != CardType::Attack
                && fuel_score_with_context(combat, *idx, context) > 0
        })
        .count() as i32
}

fn evaluate_hand_card(combat: &CombatState, hand_index: usize) -> Option<CombatCardEval> {
    let card = combat.zones.hand.get(hand_index)?;
    let def = cards::get_card_definition(card.id);
    Some(CombatCardEval {
        id: card.id,
        can_play_now: crate::content::cards::can_play_card(card, combat).is_ok(),
        block: effective_block(card, &def, combat),
        damage: effective_damage(card, &def),
        starter_basic: cards::is_starter_basic(card.id),
        status_or_curse: is_status_or_curse_card(card.id),
        keeper_priority: is_keeper_priority_card(card.id),
        setup_power: is_setup_power_card(card.id),
        draw_core: is_draw_core_card(card.id),
        exhaust_outlet: is_exhaust_outlet_card(card.id),
        exhaust_engine: is_exhaust_engine_card(card.id),
        block_core: is_block_core_card(card.id),
        strength_enabler: is_strength_enabler(card.id),
        strength_payoff: is_strength_payoff(card.id),
    })
}
