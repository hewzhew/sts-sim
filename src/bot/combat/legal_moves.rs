use crate::bot::card_disposition::{
    combat_copy_score_for_uuid, combat_exhaust_score_for_uuid, combat_retention_score_for_uuid,
};
use crate::engine::targeting;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, GridSelectReason, HandSelectReason, PendingChoice};
use crate::state::EngineState;

use super::hand_select::{
    score_discard_candidate, score_discard_to_hand_candidate, score_exhaust_candidate,
    score_put_on_draw_pile_candidate,
};

pub(crate) fn engine_local_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    let mut moves = Vec::new();

    match engine {
        EngineState::CombatPlayerTurn => {
            moves.push(ClientInput::EndTurn);
            for (potion_index, maybe_potion) in combat.entities.potions.iter().enumerate() {
                let Some(potion) = maybe_potion.as_ref() else {
                    continue;
                };
                if !potion.can_use {
                    continue;
                }
                if potion.id == crate::content::potions::PotionId::FairyPotion {
                    continue;
                }
                if potion.id == crate::content::potions::PotionId::SmokeBomb
                    && combat.meta.is_boss_fight
                {
                    continue;
                }
                if potion.id == crate::content::potions::PotionId::LiquidMemories
                    && combat.zones.discard_pile.is_empty()
                {
                    continue;
                }
                if let Some(validation) =
                    targeting::validation_for_potion_target(potion.requires_target)
                {
                    for target in targeting::candidate_targets(combat, validation) {
                        moves.push(ClientInput::UsePotion {
                            potion_index,
                            target: Some(target),
                        });
                    }
                } else {
                    moves.push(ClientInput::UsePotion {
                        potion_index,
                        target: None,
                    });
                }
            }

            let velvet_choker_locked = combat
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::VelvetChoker)
                && combat.turn.counters.cards_played_this_turn >= 6;
            for (i, card) in combat.zones.hand.iter().enumerate() {
                if velvet_choker_locked {
                    continue;
                }
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
            PendingChoice::CardRewardSelect {
                cards, can_skip, ..
            } => {
                for index in 0..cards.len() {
                    moves.push(ClientInput::SubmitDiscoverChoice(index));
                }
                if *can_skip {
                    moves.push(ClientInput::Cancel);
                }
            }
            PendingChoice::StanceChoice => {
                let hp_percent = (combat.entities.player.current_hp * 100)
                    / combat.entities.player.max_hp.max(1);
                let expected_inc_damage: i32 = combat
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
                    .map(|monster| {
                        crate::projection::combat::monster_preview_total_damage_in_combat(
                            combat, monster,
                        )
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

pub(crate) fn get_legal_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    engine_local_moves(engine, combat)
}

pub(crate) fn protocol_root_moves(
    snapshot: &crate::protocol::java::CombatAffordanceSnapshot,
) -> Vec<ClientInput> {
    snapshot.protocol_root_inputs()
}

pub fn legal_moves_for_audit(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    get_legal_moves(engine, combat)
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
    use crate::diff::state_sync::build_combat_state_from_snapshots;
    use crate::protocol::java::{
        build_combat_affordance_snapshot, build_live_observation_snapshot,
        build_live_truth_snapshot,
    };
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn load_fixture_root() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("protocol_truth_samples")
            .join("sentry_livecomm")
            .join("frame.json");
        let text = std::fs::read_to_string(path).expect("fixture");
        serde_json::from_str(&text).expect("fixture json")
    }

    fn build_fixture_combat() -> CombatState {
        let root = load_fixture_root();
        let game_state = root.get("game_state").expect("game_state");
        let truth = build_live_truth_snapshot(game_state);
        let observation = build_live_observation_snapshot(game_state);
        let relics = game_state.get("relics").unwrap_or(&Value::Null);
        build_combat_state_from_snapshots(&truth, &observation, relics)
    }

    #[test]
    fn engine_local_moves_skip_unusable_potions() {
        let root = load_fixture_root();
        let game_state = root.get("game_state").expect("game_state");
        let mut truth = build_live_truth_snapshot(game_state);
        truth["potions"] = json!([
            {
                "id": "FairyPotion",
                "name": "Fairy in a Bottle",
                "uuid": "fairy-1",
                "can_use": false,
                "can_discard": true,
                "requires_target": false
            },
            {
                "id": "Potion Slot",
                "name": "Potion Slot",
                "can_use": false,
                "can_discard": false,
                "requires_target": false
            },
            {
                "id": "Potion Slot",
                "name": "Potion Slot",
                "can_use": false,
                "can_discard": false,
                "requires_target": false
            }
        ]);
        let observation = build_live_observation_snapshot(game_state);
        let relics = game_state.get("relics").unwrap_or(&Value::Null);
        let combat = build_combat_state_from_snapshots(&truth, &observation, relics);
        let inputs = engine_local_moves(&EngineState::CombatPlayerTurn, &combat);
        assert!(
            !inputs
                .iter()
                .any(|input| matches!(input, ClientInput::UsePotion { .. })),
            "engine-local enumeration should not emit can_use=false potion actions"
        );
    }

    #[test]
    fn engine_local_moves_skip_passive_fairy_potion_even_if_local_affordance_is_stale() {
        let mut combat = build_fixture_combat();
        combat.entities.potions = vec![
            Some(crate::content::potions::Potion::with_affordance_truth(
                crate::content::potions::PotionId::FairyPotion,
                1,
                true,
                true,
                false,
            )),
            None,
            None,
        ];

        let inputs = engine_local_moves(&EngineState::CombatPlayerTurn, &combat);
        assert!(
            !inputs.iter().any(|input| matches!(
                input,
                ClientInput::UsePotion {
                    potion_index: 0,
                    ..
                }
            )),
            "Fairy in a Bottle is passive and should not be a manual root action"
        );
    }

    #[test]
    fn engine_local_moves_skip_smoke_bomb_during_boss_combat() {
        let mut combat = build_fixture_combat();
        combat.meta.is_boss_fight = true;
        combat.entities.potions = vec![
            Some(crate::content::potions::Potion::with_affordance_truth(
                crate::content::potions::PotionId::SmokeBomb,
                1,
                true,
                true,
                false,
            )),
            None,
            None,
        ];

        let inputs = engine_local_moves(&EngineState::CombatPlayerTurn, &combat);
        assert!(
            !inputs.iter().any(|input| matches!(
                input,
                ClientInput::UsePotion {
                    potion_index: 0,
                    ..
                }
            )),
            "Smoke Bomb is not usable in boss combat"
        );
    }

    #[test]
    fn engine_local_moves_skip_liquid_memories_with_empty_discard_pile() {
        let mut combat = build_fixture_combat();
        combat.zones.discard_pile.clear();
        combat.entities.potions = vec![
            Some(crate::content::potions::Potion::with_affordance_truth(
                crate::content::potions::PotionId::LiquidMemories,
                1,
                true,
                true,
                false,
            )),
            None,
            None,
        ];

        let inputs = engine_local_moves(&EngineState::CombatPlayerTurn, &combat);
        assert!(
            !inputs.iter().any(|input| matches!(
                input,
                ClientInput::UsePotion {
                    potion_index: 0,
                    ..
                }
            )),
            "Liquid Memories needs a discard-pile target"
        );
    }

    #[test]
    fn engine_local_moves_skip_cards_when_velvet_choker_locked() {
        let mut combat = build_fixture_combat();
        combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Apparition,
            90_001,
        )];
        combat.turn.energy = 3;
        combat.turn.counters.cards_played_this_turn = 6;
        combat
            .entities
            .player
            .relics
            .push(crate::content::relics::RelicState::new(
                crate::content::relics::RelicId::VelvetChoker,
            ));

        let inputs = engine_local_moves(&EngineState::CombatPlayerTurn, &combat);
        assert!(
            !inputs
                .iter()
                .any(|input| matches!(input, ClientInput::PlayCard { .. })),
            "Velvet Choker prevents playing more than six cards in a turn, so card actions must not enter the legal mask"
        );
        assert!(
            inputs
                .iter()
                .any(|input| matches!(input, ClientInput::EndTurn)),
            "EndTurn should remain legal under Velvet Choker"
        );
    }

    #[test]
    fn protocol_root_moves_only_echo_protocol_affordance() {
        let combat = build_fixture_combat();
        let action_space = json!({
            "combat_action_space": {
                "screen_type": "NONE",
                "actions": [
                    {
                        "action_id": "end_turn",
                        "kind": "end_turn",
                        "command": "END",
                        "target_required": false,
                        "target_options": []
                    },
                    {
                        "action_id": "play-0",
                        "kind": "play_card",
                        "command": "PLAY 0 0",
                        "target_required": true,
                        "target_options": [0],
                        "target_index": 0,
                        "hand_index": 0,
                        "card_uuid": "card-uuid-1",
                        "card_id": "Strike_R"
                    }
                ]
            }
        });
        let snapshot = build_combat_affordance_snapshot(&action_space, &combat)
            .expect("affordance parse")
            .expect("action space");
        let inputs = protocol_root_moves(&snapshot);
        assert_eq!(inputs.len(), 2);
        assert!(inputs.contains(&ClientInput::EndTurn));
        assert!(
            inputs.iter().any(|input| matches!(
                input,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(_)
                }
            )),
            "protocol-root chooser should consume protocol-exported play actions"
        );
        assert!(
            !inputs
                .iter()
                .any(|input| matches!(input, ClientInput::UsePotion { .. })),
            "protocol-root chooser must not synthesize extra potion actions"
        );
    }
}
