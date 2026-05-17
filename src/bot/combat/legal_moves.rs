use crate::engine::targeting;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, PendingChoice};
use crate::state::EngineState;

pub(crate) fn engine_local_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    let mut moves = Vec::new();

    match engine {
        EngineState::CombatPlayerTurn => {
            moves.push(ClientInput::EndTurn);
            for (potion_index, maybe_potion) in combat.entities.potions.iter().enumerate() {
                let Some(potion) = maybe_potion.as_ref() else {
                    continue;
                };
                if potion.can_discard {
                    moves.push(ClientInput::DiscardPotion(potion_index));
                }
                if !crate::content::potions::potion_can_use_in_combat_like_java(potion, combat) {
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
                ..
            } => {
                extend_hand_select_moves(&mut moves, candidate_uuids, *min_cards, *max_cards);
            }
            PendingChoice::GridSelect {
                min_cards,
                candidate_uuids,
                max_cards,
                ..
            } => {
                extend_grid_select_moves(&mut moves, candidate_uuids, *min_cards, *max_cards);
            }
            PendingChoice::DiscoverySelect(choice) => {
                for index in 0..choice.cards.len() {
                    moves.push(ClientInput::SubmitDiscoverChoice(index));
                }
                if choice.can_skip {
                    moves.push(ClientInput::Cancel);
                }
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
            PendingChoice::ForeignInfluenceSelect { cards, .. } => {
                for index in 0..cards.len() {
                    moves.push(ClientInput::SubmitDiscoverChoice(index));
                }
            }
            PendingChoice::ChooseOneSelect { choices } => {
                for index in 0..choices.len() {
                    moves.push(ClientInput::SubmitDiscoverChoice(index));
                }
            }
            PendingChoice::StanceChoice => {
                moves.push(ClientInput::SubmitDiscoverChoice(0));
                moves.push(ClientInput::SubmitDiscoverChoice(1));
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

fn extend_hand_select_moves(
    moves: &mut Vec<ClientInput>,
    candidate_uuids: &[u32],
    min_cards: u8,
    max_cards: u8,
) {
    let effective_max = candidate_uuids.len().min(max_cards as usize);
    if min_cards == 0 {
        push_unique_hand_select(moves, Vec::new());
    }
    if candidate_uuids.is_empty() || effective_max == 0 {
        return;
    }

    let combo_pool = candidate_uuids
        .iter()
        .copied()
        .take(selection_pool_cap(
            min_cards,
            max_cards,
            candidate_uuids.len(),
        ))
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
    candidate_uuids: &[u32],
    min_cards: u8,
    max_cards: u8,
) {
    let effective_max = candidate_uuids.len().min(max_cards as usize);
    if min_cards == 0 {
        push_unique_grid_select(moves, Vec::new());
    }
    if candidate_uuids.is_empty() || effective_max == 0 {
        return;
    }

    let combo_pool = candidate_uuids
        .iter()
        .copied()
        .take(selection_pool_cap(
            min_cards,
            max_cards,
            candidate_uuids.len(),
        ))
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

fn selection_pool_cap(min_cards: u8, max_cards: u8, available: usize) -> usize {
    let cap_hint = if min_cards == 0 {
        (max_cards as usize).saturating_add(3)
    } else {
        (min_cards as usize).saturating_add(4)
    };
    available.min(cap_hint.clamp(4, 8))
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
    use crate::content::monsters::EnemyId;
    use crate::diff::state_sync::build_combat_state_from_snapshots;
    use crate::protocol::java::{
        build_combat_affordance_snapshot, build_live_observation_snapshot,
        build_live_truth_snapshot,
    };
    use crate::test_support::test_monster;
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
        assert!(
            inputs
                .iter()
                .any(|input| matches!(input, ClientInput::DiscardPotion(0))),
            "Java PotionPopUp allows discarding an owned potion through canDiscard even when canUse is false"
        );
        assert!(
            !inputs
                .iter()
                .any(|input| matches!(input, ClientInput::DiscardPotion(1 | 2))),
            "empty potion slots are not discardable actions"
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
    fn engine_local_moves_skip_smoke_bomb_when_visible_monster_is_boss() {
        let mut combat = build_fixture_combat();
        combat.meta.is_boss_fight = false;
        combat.entities.monsters = vec![test_monster(EnemyId::SlimeBoss)];
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
            "Java SmokeBomb.canUse blocks by monster EnemyType.BOSS even when a fixture did not set a room boss flag"
        );
    }

    #[test]
    fn engine_local_moves_keeps_liquid_memories_with_empty_discard_pile() {
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
            inputs.iter().any(|input| matches!(
                input,
                ClientInput::UsePotion {
                    potion_index: 0,
                    ..
                }
            )),
            "Liquid Memories is Java-usable with an empty discard pile; the action fizzles after consuming it"
        );
    }

    #[test]
    fn engine_fizzles_liquid_memories_empty_discard_after_consuming_potion() {
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

        let mut engine = EngineState::CombatPlayerTurn;
        let alive = crate::engine::core::tick_until_stable_turn(
            &mut engine,
            &mut combat,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        );

        assert!(alive);
        assert_eq!(engine, EngineState::CombatPlayerTurn);
        assert!(combat.entities.potions[0].is_none());
        let diagnostics = combat.take_engine_diagnostics();
        assert!(
            diagnostics.iter().all(|diagnostic| diagnostic.severity
                != crate::state::selection::EngineDiagnosticSeverity::Error),
            "empty Liquid Memories should not emit an engine error: {diagnostics:?}"
        );
    }

    #[test]
    fn engine_fizzles_empty_warcry_hand_select_without_error() {
        let mut combat = build_fixture_combat();
        combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Warcry,
            90_001,
        )];
        combat.zones.draw_pile.clear();
        combat.zones.discard_pile.clear();
        combat.turn.energy = 1;

        let mut engine = EngineState::CombatPlayerTurn;
        let alive = crate::engine::core::tick_until_stable_turn(
            &mut engine,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );

        assert!(alive);
        assert_eq!(engine, EngineState::CombatPlayerTurn);
        let diagnostics = combat.take_engine_diagnostics();
        assert!(
            diagnostics.iter().all(|diagnostic| diagnostic.severity
                != crate::state::selection::EngineDiagnosticSeverity::Error),
            "empty Warcry with no card to put back should not emit an engine error: {diagnostics:?}"
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
