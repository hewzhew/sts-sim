use crate::bot::card_disposition::{
    combat_copy_score_for_uuid, combat_exhaust_score_for_uuid, combat_retention_score_for_uuid,
};
use crate::combat::CombatState;
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
                        crate::combat::Intent::Attack { hits, .. }
                        | crate::combat::Intent::AttackBuff { hits, .. }
                        | crate::combat::Intent::AttackDebuff { hits, .. }
                        | crate::combat::Intent::AttackDefend { hits, .. } => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CombatCard, CombatState, Intent};
    use crate::content::monsters::EnemyId;
    use crate::state::core::{GridSelectReason, PileType};
    use crate::testing::support::test_support::{combat_with_attacking_monster, CombatTestExt};

    fn test_combat() -> CombatState {
        combat_with_attacking_monster(EnemyId::JawWorm, 36, 12)
            .with_energy(2)
            .with_player_hp(40)
    }

    #[test]
    fn stance_choice_exposes_wrath_and_calm_instead_of_proceed() {
        let combat = test_combat();
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::StanceChoice),
            &combat,
        );
        assert!(moves
            .iter()
            .any(|m| matches!(m, ClientInput::SubmitDiscoverChoice(0))));
        assert!(moves
            .iter()
            .any(|m| matches!(m, ClientInput::SubmitDiscoverChoice(1))));
        assert!(!moves.iter().any(|m| matches!(m, ClientInput::Proceed)));
    }

    #[test]
    fn liquid_memories_prioritizes_offering_from_discard() {
        let mut combat = test_combat();
        let offering = CombatCard::new(crate::content::cards::CardId::Offering, 11);
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 12);
        combat.zones.discard_pile = vec![strike.clone(), offering.clone()];
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: PileType::Discard,
                candidate_uuids: vec![strike.uuid, offering.uuid],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: GridSelectReason::DiscardToHand,
            }),
            &combat,
        );
        assert!(matches!(
            moves.first(),
            Some(ClientInput::SubmitGridSelect(selected)) if selected == &vec![offering.uuid]
        ));
    }

    #[test]
    fn zero_min_grid_select_only_emits_empty_once() {
        let mut combat = test_combat();
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 13);
        combat.zones.discard_pile = vec![strike.clone()];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: PileType::Discard,
                candidate_uuids: vec![strike.uuid],
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: GridSelectReason::DiscardToHand,
            }),
            &combat,
        );

        let empty_count = moves
            .iter()
            .filter(|m| matches!(m, ClientInput::SubmitGridSelect(selected) if selected.is_empty()))
            .count();
        assert_eq!(empty_count, 1);
    }

    #[test]
    fn discard_to_hand_does_not_duplicate_two_card_grid_selection() {
        let mut combat = test_combat();
        let offering = CombatCard::new(crate::content::cards::CardId::Offering, 14);
        let shrug = CombatCard::new(crate::content::cards::CardId::ShrugItOff, 15);
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 16);
        combat.zones.discard_pile = vec![strike, shrug.clone(), offering.clone()];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: PileType::Discard,
                candidate_uuids: vec![offering.uuid, shrug.uuid],
                min_cards: 2,
                max_cards: 2,
                can_cancel: false,
                reason: GridSelectReason::DiscardToHand,
            }),
            &combat,
        );

        let pair_count = moves
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    ClientInput::SubmitGridSelect(selected)
                        if selected == &vec![offering.uuid, shrug.uuid]
                )
            })
            .count();
        assert_eq!(pair_count, 1);
    }

    #[test]
    fn zero_min_exhaust_selection_offers_positive_exhaust_choices() {
        let mut combat = test_combat();
        let slimed = CombatCard::new(crate::content::cards::CardId::Slimed, 21);
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 22);
        combat.zones.hand = vec![strike, slimed.clone()];
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: vec![21, 22],
                min_cards: 0,
                max_cards: 99,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::Exhaust,
            }),
            &combat,
        );
        assert!(moves.iter().any(|m| matches!(
            m,
            ClientInput::SubmitHandSelect(selected) if selected == &vec![slimed.uuid]
        )));
    }

    #[test]
    fn multi_card_hand_select_generates_more_than_one_combination() {
        let mut combat = test_combat();
        combat.zones.hand = vec![
            CombatCard::new(crate::content::cards::CardId::Strike, 23),
            CombatCard::new(crate::content::cards::CardId::Defend, 24),
            CombatCard::new(crate::content::cards::CardId::Slimed, 25),
        ];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: combat.zones.hand.iter().map(|c| c.uuid).collect(),
                min_cards: 2,
                max_cards: 2,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Discard,
            }),
            &combat,
        );

        let pair_count = moves
            .iter()
            .filter(|m| matches!(m, ClientInput::SubmitHandSelect(selected) if selected.len() == 2))
            .count();
        assert!(pair_count >= 2);
    }

    #[test]
    fn optional_put_on_draw_pile_offers_non_empty_choice() {
        let mut combat = test_combat();
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 26);
        let defend = CombatCard::new(crate::content::cards::CardId::Defend, 27);
        combat.zones.hand = vec![strike.clone(), defend];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: vec![strike.uuid],
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::PutOnDrawPile,
            }),
            &combat,
        );

        assert!(moves.iter().any(|m| matches!(
            m,
            ClientInput::SubmitHandSelect(selected) if selected == &vec![strike.uuid]
        )));
    }

    #[test]
    fn dual_wield_copy_prefers_premium_keeper_over_basic_attack() {
        let mut combat = test_combat();
        let mut strike = CombatCard::new(crate::content::cards::CardId::Strike, 51);
        strike.upgrades = 1;
        let demon_form = CombatCard::new(crate::content::cards::CardId::DemonForm, 52);
        combat.zones.hand = vec![strike, demon_form.clone()];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: combat.zones.hand.iter().map(|c| c.uuid).collect(),
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Copy { amount: 1 },
            }),
            &combat,
        );

        assert!(matches!(
            moves.first(),
            Some(ClientInput::SubmitHandSelect(selected)) if selected == &vec![demon_form.uuid]
        ));
    }

    #[test]
    fn exhaust_selection_preserves_setup_power_when_junk_exists() {
        let mut combat = test_combat();
        let wound = CombatCard::new(crate::content::cards::CardId::Wound, 61);
        let demon_form = CombatCard::new(crate::content::cards::CardId::DemonForm, 62);
        let defend = CombatCard::new(crate::content::cards::CardId::Defend, 63);
        combat.zones.hand = vec![wound.clone(), demon_form, defend];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: combat.zones.hand.iter().map(|c| c.uuid).collect(),
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Exhaust,
            }),
            &combat,
        );

        assert!(matches!(
            moves.first(),
            Some(ClientInput::SubmitHandSelect(selected)) if selected == &vec![wound.uuid]
        ));
    }

    #[test]
    fn gambling_chip_does_not_offer_full_mulligan_when_hand_has_clear_keeper() {
        let mut combat = test_combat();
        combat.turn.energy = 3;
        let offering = CombatCard::new(crate::content::cards::CardId::Offering, 31);
        let strike_a = CombatCard::new(crate::content::cards::CardId::Strike, 32);
        let strike_b = CombatCard::new(crate::content::cards::CardId::Strike, 33);
        let defend_a = CombatCard::new(crate::content::cards::CardId::Defend, 34);
        let defend_b = CombatCard::new(crate::content::cards::CardId::Defend, 35);
        combat.zones.hand = vec![
            offering.clone(),
            strike_a.clone(),
            strike_b.clone(),
            defend_a.clone(),
            defend_b.clone(),
        ];

        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: combat.zones.hand.iter().map(|c| c.uuid).collect(),
                min_cards: 0,
                max_cards: 99,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::GamblingChip,
            }),
            &combat,
        );

        assert!(!moves.iter().any(|m| matches!(
            m,
            ClientInput::SubmitHandSelect(selected)
                if selected.len() == 5 && selected.contains(&offering.uuid)
        )));
    }

    #[test]
    fn gambling_chip_offers_full_mulligan_for_status_clogged_hand() {
        let mut combat = test_combat();
        combat.turn.energy = 3;
        combat.entities.monsters[0].current_intent = Intent::Buff;
        combat.entities.monsters[0].intent_dmg = 0;
        combat.zones.draw_pile = vec![CombatCard::new(crate::content::cards::CardId::Offering, 90)];
        combat.zones.hand = vec![
            CombatCard::new(crate::content::cards::CardId::Slimed, 41),
            CombatCard::new(crate::content::cards::CardId::Slimed, 42),
            CombatCard::new(crate::content::cards::CardId::Burn, 43),
            CombatCard::new(crate::content::cards::CardId::Dazed, 44),
            CombatCard::new(crate::content::cards::CardId::Strike, 45),
        ];

        let all_uuids: Vec<u32> = combat.zones.hand.iter().map(|c| c.uuid).collect();
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: all_uuids.clone(),
                min_cards: 0,
                max_cards: 99,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::GamblingChip,
            }),
            &combat,
        );

        assert!(moves.iter().any(|m| matches!(
            m,
            ClientInput::SubmitHandSelect(selected) if selected.len() == all_uuids.len()
        )));
    }

    #[test]
    fn upgraded_blind_is_emitted_as_non_targeted_play() {
        let mut combat = test_combat();
        let mut blind = CombatCard::new(crate::content::cards::CardId::Blind, 41);
        blind.upgrades = 1;
        combat.zones.hand = vec![blind];

        let moves = get_legal_moves(&EngineState::CombatPlayerTurn, &combat);

        assert!(moves.iter().any(|m| matches!(
            m,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            }
        )));
        assert!(!moves.iter().any(|m| matches!(
            m,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(_),
            }
        )));
    }

    #[test]
    fn upgraded_trip_is_emitted_as_non_targeted_play() {
        let mut combat = test_combat();
        let mut trip = CombatCard::new(crate::content::cards::CardId::Trip, 42);
        trip.upgrades = 1;
        combat.zones.hand = vec![trip];

        let moves = get_legal_moves(&EngineState::CombatPlayerTurn, &combat);

        assert!(moves.iter().any(|m| matches!(
            m,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            }
        )));
        assert!(!moves.iter().any(|m| matches!(
            m,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(_),
            }
        )));
    }
}
