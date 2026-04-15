use crate::bot::card_disposition::{
    combat_copy_score_for_uuid, combat_exhaust_score_for_uuid, combat_retention_score_for_uuid,
};
use crate::runtime::combat::CombatState;
use crate::engine::targeting;
use crate::state::core::{ClientInput, GridSelectReason, HandSelectReason, PendingChoice};
use crate::state::EngineState;

use super::hand_select::{
    score_discard_candidate, score_discard_to_hand_candidate, score_exhaust_candidate,
    score_put_on_draw_pile_candidate,
};

pub(crate) fn get_legal_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    let mut moves = Vec::new();

    match engine {
        EngineState::CombatPlayerTurn => {
            moves.push(ClientInput::EndTurn);
            moves.extend(crate::bot::potions::candidate_potion_moves(combat));

            for (i, card) in combat.zones.hand.iter().enumerate() {
                if crate::content::cards::can_play_card(card, combat).is_ok() {
                    let target_type = crate::content::cards::effective_target(card);
                    if let Some(validation) = targeting::validation_for_card_target(target_type) {
                        for target in targeting::candidate_targets(combat, validation) {
                            moves.push(ClientInput::PlayCard {
                                card_index: i,
                                target: Some(target),
                            });
                        }
                    } else {
                        moves.push(ClientInput::PlayCard {
                            card_index: i,
                            target: None,
                        });
                    }
                }
            }
        }
        EngineState::PendingChoice(choice) => match choice {
            PendingChoice::HandSelect {
                min_cards,
                max_cards,
                candidate_uuids,
                reason,
                ..
            } => {
                if matches!(reason, HandSelectReason::GamblingChip) && *min_cards == 0 {
                    moves.extend(gambling_chip_moves(combat, candidate_uuids, *max_cards));
                } else {
                    extend_hand_select_moves(
                        &mut moves,
                        combat,
                        candidate_uuids,
                        *min_cards,
                        *max_cards,
                        *reason,
                    );
                }
            }
            PendingChoice::GridSelect {
                min_cards,
                candidate_uuids,
                max_cards,
                reason,
                ..
            } => {
                extend_grid_select_moves(
                    &mut moves,
                    combat,
                    candidate_uuids,
                    *min_cards,
                    *max_cards,
                    *reason,
                );
            }
            PendingChoice::DiscoverySelect(_) => {
                moves.push(ClientInput::SubmitDiscoverChoice(0));
                moves.push(ClientInput::SubmitDiscoverChoice(1));
                moves.push(ClientInput::SubmitDiscoverChoice(2));
            }
            PendingChoice::CardRewardSelect { .. } => {
                moves.push(ClientInput::SubmitCardChoice(vec![0]));
            }
            PendingChoice::StanceChoice => {
                let hp_percent = (combat.entities.player.current_hp * 100)
                    / combat.entities.player.max_hp.max(1);
                let expected_inc_damage: i32 = combat
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
                    .map(|m| match m.current_intent {
                        crate::runtime::combat::Intent::Attack { hits, .. }
                        | crate::runtime::combat::Intent::AttackBuff { hits, .. }
                        | crate::runtime::combat::Intent::AttackDebuff { hits, .. }
                        | crate::runtime::combat::Intent::AttackDefend { hits, .. } => {
                            (m.intent_dmg * (hits as i32)).max(0)
                        }
                        _ => 0,
                    })
                    .sum();
                let unblocked = (expected_inc_damage - combat.entities.player.block).max(0);
                let playable_attacks = combat
                    .zones
                    .hand
                    .iter()
                    .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
                    .filter(|card| {
                        crate::content::cards::get_card_definition(card.id).card_type
                            == crate::content::cards::CardType::Attack
                    })
                    .count();

                let prefer_calm = unblocked > 0 || hp_percent <= 45 || playable_attacks == 0;
                if prefer_calm {
                    moves.push(ClientInput::SubmitDiscoverChoice(1));
                    moves.push(ClientInput::SubmitDiscoverChoice(0));
                } else {
                    moves.push(ClientInput::SubmitDiscoverChoice(0));
                    moves.push(ClientInput::SubmitDiscoverChoice(1));
                }
            }
            _ => {
                moves.push(ClientInput::Proceed);
            }
        },
        _ => {
            moves.push(ClientInput::Proceed);
        }
    }

    moves
}

fn gambling_chip_moves(
    combat: &CombatState,
    candidate_uuids: &[u32],
    max_cards: u8,
) -> Vec<ClientInput> {
    let mut moves = vec![ClientInput::SubmitHandSelect(Vec::new())];
    let mut scored = candidate_uuids
        .iter()
        .map(|uuid| {
            let discard_score = score_discard_candidate(combat, *uuid);
            let retention = combat_retention_score_for_uuid(combat, *uuid);
            let exhaust = combat_exhaust_score_for_uuid(combat, *uuid).max(0);
            (*uuid, discard_score + exhaust / 3 - retention / 4)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    let protected_count = candidate_uuids
        .iter()
        .filter(|uuid| combat_retention_score_for_uuid(combat, **uuid) >= 7_500)
        .count();
    let safe_cap = candidate_uuids
        .len()
        .saturating_sub(protected_count)
        .min(max_cards as usize);
    if safe_cap == 0 {
        return moves;
    }

    let strong = scored
        .iter()
        .filter(|(_, score)| *score >= 3_000)
        .map(|(uuid, _)| *uuid)
        .collect::<Vec<_>>();
    let medium = scored
        .iter()
        .filter(|(_, score)| *score >= 1_400)
        .map(|(uuid, _)| *uuid)
        .collect::<Vec<_>>();

    for take in 1..=strong.len().min(safe_cap.min(3)) {
        moves.push(ClientInput::SubmitHandSelect(strong[..take].to_vec()));
    }

    let cautious_take = medium.len().min(safe_cap.min(4));
    if cautious_take > 0 {
        let selection = medium[..cautious_take].to_vec();
        if !contains_hand_select(&moves, &selection) {
            moves.push(ClientInput::SubmitHandSelect(selection));
        }
    }

    if should_allow_full_gambling_chip_mulligan(combat, &scored, safe_cap) {
        let full = scored
            .iter()
            .take(safe_cap)
            .map(|(uuid, _)| *uuid)
            .collect::<Vec<_>>();
        if !full.is_empty() && !contains_hand_select(&moves, &full) {
            moves.push(ClientInput::SubmitHandSelect(full));
        }
    }

    moves
}

fn extend_hand_select_moves(
    moves: &mut Vec<ClientInput>,
    combat: &CombatState,
    candidate_uuids: &[u32],
    min_cards: u8,
    max_cards: u8,
    reason: HandSelectReason,
) {
    let mut ordered = candidate_uuids.to_vec();
    ordered
        .sort_by_key(|uuid| std::cmp::Reverse(score_hand_select_candidate(combat, *uuid, reason)));

    let filtered = match reason {
        HandSelectReason::Discard | HandSelectReason::Exhaust if min_cards == 0 => ordered
            .into_iter()
            .filter(|uuid| score_hand_select_candidate(combat, *uuid, reason) > 0)
            .collect::<Vec<_>>(),
        _ => ordered,
    };

    let effective_max = candidate_uuids.len().min(max_cards as usize);
    if min_cards == 0 {
        push_unique_hand_select(moves, Vec::new());
    }
    if filtered.is_empty() || effective_max == 0 {
        return;
    }

    let combo_pool = filtered
        .iter()
        .copied()
        .take(selection_pool_cap(min_cards, max_cards, filtered.len()))
        .collect::<Vec<_>>();
    let min_select = if min_cards == 0 {
        1
    } else {
        min_cards as usize
    };
    let max_select = selection_generation_max(min_cards, max_cards, combo_pool.len());

    for selection in generate_ranked_combinations(&combo_pool, min_select, max_select, 16) {
        push_unique_hand_select(moves, selection);
    }
}

fn extend_grid_select_moves(
    moves: &mut Vec<ClientInput>,
    combat: &CombatState,
    candidate_uuids: &[u32],
    min_cards: u8,
    max_cards: u8,
    reason: GridSelectReason,
) {
    let mut ordered = candidate_uuids.to_vec();
    ordered
        .sort_by_key(|uuid| std::cmp::Reverse(score_grid_select_candidate(combat, *uuid, reason)));

    let effective_max = candidate_uuids.len().min(max_cards as usize);
    if min_cards == 0 {
        push_unique_grid_select(moves, Vec::new());
    }
    if ordered.is_empty() || effective_max == 0 {
        return;
    }

    let combo_pool = ordered
        .iter()
        .copied()
        .take(selection_pool_cap(min_cards, max_cards, ordered.len()))
        .collect::<Vec<_>>();
    let min_select = if min_cards == 0 {
        1
    } else {
        min_cards as usize
    };
    let max_select = selection_generation_max(min_cards, max_cards, combo_pool.len());

    for selection in generate_ranked_combinations(&combo_pool, min_select, max_select, 16) {
        push_unique_grid_select(moves, selection);
    }
}

fn score_hand_select_candidate(combat: &CombatState, uuid: u32, reason: HandSelectReason) -> i32 {
    match reason {
        HandSelectReason::PutOnDrawPile | HandSelectReason::PutToBottomOfDraw => {
            score_put_on_draw_pile_candidate(combat, uuid)
        }
        HandSelectReason::Exhaust => score_exhaust_candidate(combat, uuid),
        HandSelectReason::Discard | HandSelectReason::GamblingChip => {
            score_discard_candidate(combat, uuid)
        }
        HandSelectReason::Copy { .. } => combat_copy_score_for_uuid(combat, uuid),
        HandSelectReason::Retain | HandSelectReason::Upgrade => {
            combat_retention_score_for_uuid(combat, uuid)
        }
    }
}

fn score_grid_select_candidate(combat: &CombatState, uuid: u32, reason: GridSelectReason) -> i32 {
    match reason {
        GridSelectReason::DiscardToHand => score_discard_to_hand_candidate(combat, uuid),
        GridSelectReason::MoveToDrawPile
        | GridSelectReason::Exhume { .. }
        | GridSelectReason::SkillFromDeckToHand
        | GridSelectReason::AttackFromDeckToHand => 0,
    }
}

fn selection_pool_cap(min_cards: u8, max_cards: u8, available: usize) -> usize {
    let baseline = if min_cards == 0 {
        (max_cards as usize).saturating_add(3)
    } else {
        (min_cards as usize).saturating_add(4)
    };
    available.min(baseline.clamp(4, 8))
}

fn selection_generation_max(min_cards: u8, max_cards: u8, available: usize) -> usize {
    let effective_max = available.min(max_cards as usize);
    if min_cards == 0 {
        effective_max.min(4)
    } else {
        effective_max.min((min_cards as usize).saturating_add(2))
    }
}

fn generate_ranked_combinations(
    ordered: &[u32],
    min_size: usize,
    max_size: usize,
    max_results: usize,
) -> Vec<Vec<u32>> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    if ordered.is_empty() || min_size == 0 || min_size > max_size {
        return out;
    }

    for target_size in min_size..=max_size.min(ordered.len()) {
        collect_ranked_combinations(ordered, target_size, 0, &mut current, &mut out, max_results);
        if out.len() >= max_results {
            break;
        }
    }

    out
}

fn collect_ranked_combinations(
    ordered: &[u32],
    target_size: usize,
    start: usize,
    current: &mut Vec<u32>,
    out: &mut Vec<Vec<u32>>,
    max_results: usize,
) {
    if out.len() >= max_results {
        return;
    }
    if current.len() == target_size {
        out.push(current.clone());
        return;
    }
    if start >= ordered.len() {
        return;
    }

    let remaining_needed = target_size - current.len();
    let max_start = ordered.len().saturating_sub(remaining_needed);
    for idx in start..=max_start {
        current.push(ordered[idx]);
        collect_ranked_combinations(ordered, target_size, idx + 1, current, out, max_results);
        current.pop();
        if out.len() >= max_results {
            return;
        }
    }
}

fn push_unique_hand_select(moves: &mut Vec<ClientInput>, selection: Vec<u32>) {
    if !contains_hand_select(moves, &selection) {
        moves.push(ClientInput::SubmitHandSelect(selection));
    }
}

fn push_unique_grid_select(moves: &mut Vec<ClientInput>, selection: Vec<u32>) {
    if !contains_grid_select(moves, &selection) {
        moves.push(ClientInput::SubmitGridSelect(selection));
    }
}

fn should_allow_full_gambling_chip_mulligan(
    combat: &CombatState,
    scored: &[(u32, i32)],
    safe_cap: usize,
) -> bool {
    if combat.zones.draw_pile.is_empty() || safe_cap == 0 {
        return false;
    }

    let strong_keepers = scored
        .iter()
        .filter(|(uuid, _)| combat_retention_score_for_uuid(combat, *uuid) >= 7_500)
        .count();
    let bad_cards = scored.iter().filter(|(_, score)| *score >= 1_400).count();
    let average_score = scored.iter().map(|(_, score)| *score).sum::<i32>() / scored.len() as i32;

    strong_keepers == 0
        && bad_cards >= scored.len().saturating_sub(1)
        && average_score >= 1_600
        && combat.turn.energy >= 2
}

fn contains_hand_select(moves: &[ClientInput], selection: &[u32]) -> bool {
    moves.iter().any(|move_input| {
        matches!(
            move_input,
            ClientInput::SubmitHandSelect(existing) if existing == selection
        )
    })
}

fn contains_grid_select(moves: &[ClientInput], selection: &[u32]) -> bool {
    moves.iter().any(|move_input| {
        matches!(
            move_input,
            ClientInput::SubmitGridSelect(existing) if existing == selection
        )
    })
}
