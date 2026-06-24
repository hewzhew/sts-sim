use crate::engine::targeting;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, PendingChoice};
use crate::state::selection::{SelectionResolution, SelectionScope};
use crate::state::EngineState;

pub fn engine_local_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
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
            PendingChoice::ScrySelect { card_uuids, .. } => {
                extend_scry_moves(&mut moves, card_uuids.len());
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
        },
        _ => {
            moves.push(ClientInput::Proceed);
        }
    }

    moves
}

pub fn get_legal_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    engine_local_moves(engine, combat)
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

fn extend_scry_moves(moves: &mut Vec<ClientInput>, candidate_count: usize) {
    moves.push(ClientInput::SubmitScryDiscard(Vec::new()));
    for target_size in 1..=candidate_count {
        let mut selected = Vec::with_capacity(target_size);
        collect_scry_index_combinations(moves, candidate_count, target_size, 0, &mut selected);
    }
}

fn collect_scry_index_combinations(
    moves: &mut Vec<ClientInput>,
    candidate_count: usize,
    target_size: usize,
    start: usize,
    selected: &mut Vec<usize>,
) {
    if selected.len() == target_size {
        moves.push(ClientInput::SubmitScryDiscard(selected.clone()));
        return;
    }

    let remaining_needed = target_size - selected.len();
    if candidate_count.saturating_sub(start) < remaining_needed {
        return;
    }

    let max_start = candidate_count.saturating_sub(remaining_needed);
    for idx in start..=max_start {
        selected.push(idx);
        collect_scry_index_combinations(moves, candidate_count, target_size, idx + 1, selected);
        selected.pop();
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
        moves.push(selection_input(SelectionScope::Hand, selection));
    }
}

fn push_unique_grid_select(moves: &mut Vec<ClientInput>, selection: Vec<u32>) {
    if !contains_grid_select(moves, &selection) {
        moves.push(selection_input(SelectionScope::Grid, selection));
    }
}

fn contains_hand_select(moves: &[ClientInput], selection: &[u32]) -> bool {
    contains_selection(moves, SelectionScope::Hand, selection)
}

fn contains_grid_select(moves: &[ClientInput], selection: &[u32]) -> bool {
    contains_selection(moves, SelectionScope::Grid, selection)
}

fn selection_input(scope: SelectionScope, selection: Vec<u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(scope, selection))
}

fn contains_selection(moves: &[ClientInput], scope: SelectionScope, selection: &[u32]) -> bool {
    moves.iter().any(|move_input| match move_input {
        ClientInput::SubmitSelection(resolution) if resolution.scope == scope => {
            resolution.selected_card_uuids() == selection
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::test_support::{blank_test_combat, test_monster};

    fn build_fixture_combat() -> CombatState {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        combat
    }

    #[test]
    fn engine_local_moves_skip_unusable_potions() {
        let mut combat = build_fixture_combat();
        combat.entities.potions = vec![
            Some(Potion::with_affordance_truth(
                PotionId::FairyPotion,
                1,
                false,
                true,
                false,
            )),
            None,
            None,
        ];
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
            Some(Potion::with_affordance_truth(
                PotionId::FairyPotion,
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
            Some(Potion::with_affordance_truth(
                PotionId::SmokeBomb,
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
            Some(Potion::with_affordance_truth(
                PotionId::SmokeBomb,
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
            Some(Potion::with_affordance_truth(
                PotionId::LiquidMemories,
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
            Some(Potion::with_affordance_truth(
                PotionId::LiquidMemories,
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
    fn pending_scry_legal_moves_cover_keep_and_all_discard_subsets() {
        let mut combat = build_fixture_combat();
        combat.zones.draw_pile = vec![
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Strike, 10),
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Defend, 20),
        ];
        let engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
            cards: vec![
                crate::content::cards::CardId::Strike,
                crate::content::cards::CardId::Defend,
            ],
            card_uuids: vec![10, 20],
        });

        let inputs = engine_local_moves(&engine, &combat);

        assert_eq!(
            inputs,
            vec![
                ClientInput::SubmitScryDiscard(vec![]),
                ClientInput::SubmitScryDiscard(vec![0]),
                ClientInput::SubmitScryDiscard(vec![1]),
                ClientInput::SubmitScryDiscard(vec![0, 1]),
            ]
        );
        assert!(
            !inputs.contains(&ClientInput::Proceed),
            "pending Scry must not fall back to a fake proceed action"
        );
    }

    #[test]
    fn pending_choice_legal_moves_never_use_fake_proceed_fallback() {
        let mut combat = build_fixture_combat();
        combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Strike,
            10,
        )];
        combat.zones.discard_pile = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Defend,
            20,
        )];
        combat.zones.draw_pile = vec![
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Strike, 30),
            crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Defend, 40),
        ];
        let choices = vec![
            PendingChoice::HandSelect {
                candidate_uuids: vec![10],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::HandSelectReason::Discard,
            },
            PendingChoice::GridSelect {
                source_pile: crate::state::PileType::Discard,
                candidate_uuids: vec![20],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::GridSelectReason::MoveToDrawPile,
            },
            PendingChoice::DiscoverySelect(crate::state::DiscoveryChoiceState {
                cards: vec![crate::content::cards::CardId::Strike],
                colorless: false,
                card_type: None,
                amount: 1,
                can_skip: true,
            }),
            PendingChoice::ScrySelect {
                cards: vec![
                    crate::content::cards::CardId::Strike,
                    crate::content::cards::CardId::Defend,
                ],
                card_uuids: vec![30, 40],
            },
            PendingChoice::CardRewardSelect {
                cards: vec![crate::content::cards::CardId::ShrugItOff],
                destination: crate::runtime::action::CardDestination::Hand,
                can_skip: true,
            },
            PendingChoice::ForeignInfluenceSelect {
                cards: vec![crate::content::cards::CardId::Headbutt],
                upgraded: false,
            },
            PendingChoice::ChooseOneSelect {
                choices: vec![crate::state::ChooseOneCardChoice {
                    card_id: crate::content::cards::CardId::InfernalBlade,
                    upgrades: 0,
                }],
            },
            PendingChoice::StanceChoice,
        ];

        for choice in choices {
            let inputs = engine_local_moves(&EngineState::PendingChoice(choice.clone()), &combat);
            assert!(
                !inputs.is_empty(),
                "pending choice {choice:?} should expose at least one legal input"
            );
            assert!(
                !inputs.contains(&ClientInput::Proceed),
                "pending choice {choice:?} must not fall back to fake proceed: {inputs:?}"
            );
        }
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
}
