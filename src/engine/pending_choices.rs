use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::state::core::{ClientInput, EngineState, GridSelectReason, HandSelectReason, PileType};

pub fn handle_scry(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    _amount: usize,
    input: ClientInput,
) -> Result<(), &'static str> {
    match input {
        ClientInput::SubmitScryDiscard(indices) => {
            // Simplified stub
            if indices.len() <= combat_state.draw_pile.len() {
                *engine_state = EngineState::CombatProcessing;
                Ok(())
            } else {
                Err("Invalid discard indices")
            }
        }
        _ => Err("Invalid input for Scry"),
    }
}

pub fn handle_hand_select(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    candidate_uuids: &[u32],
    count: usize,
    requires_exact: bool,
    cancellable: bool,
    reason: HandSelectReason,
    input: ClientInput,
) -> Result<(), &'static str> {
    match input {
        ClientInput::Cancel => {
            if cancellable {
                *engine_state = EngineState::CombatProcessing;
                Ok(())
            } else {
                Err("Cannot cancel this selection")
            }
        }
        ClientInput::SubmitHandSelect(uuids) => {
            if uuids.iter().any(|uuid| !candidate_uuids.contains(uuid)) {
                return Err("Selected card is not in the frozen hand-select candidate set");
            }
            if requires_exact && uuids.len() != count {
                return Err("Must select exact number of cards");
            }
            if uuids.len() > count {
                return Err("Too many cards selected");
            }

            match reason {
                HandSelectReason::GamblingChip => {
                    // Java GamblingChipAction: discard selected cards, then draw equal count
                    let num_selected = uuids.len();
                    // Move selected cards from hand to discard
                    for uuid in &uuids {
                        if let Some(pos) = combat_state.hand.iter().position(|c| c.uuid == *uuid) {
                            let card = combat_state.hand.remove(pos);
                            combat_state.discard_pile.push(card);
                        }
                    }
                    // Queue draw actions for same number of cards
                    if num_selected > 0 {
                        let action = ActionInfo {
                            action: Action::DrawCards(num_selected as u32),
                            insertion_mode: AddTo::Top,
                        };
                        crate::engine::core::queue_actions(
                            &mut combat_state.action_queue,
                            smallvec::smallvec![action],
                        );
                    }
                }
                HandSelectReason::Exhaust => {
                    // Java ExhaustAction: exhaust selected cards from hand
                    for uuid in &uuids {
                        if let Some(pos) = combat_state.hand.iter().position(|c| c.uuid == *uuid) {
                            let card = combat_state.hand.remove(pos);
                            combat_state.exhaust_pile.push(card);
                        }
                    }
                }
                HandSelectReason::Discard => {
                    // Discard selected cards from hand
                    for uuid in &uuids {
                        if let Some(pos) = combat_state.hand.iter().position(|c| c.uuid == *uuid) {
                            let card = combat_state.hand.remove(pos);
                            combat_state.discard_pile.push(card);
                        }
                    }
                }
                HandSelectReason::PutOnDrawPile => {
                    // Move selected cards from hand to top of draw pile
                    for uuid in &uuids {
                        if let Some(pos) = combat_state.hand.iter().position(|c| c.uuid == *uuid) {
                            let card = combat_state.hand.remove(pos);
                            combat_state.draw_pile.insert(0, card);
                        }
                    }
                }
                HandSelectReason::PutToBottomOfDraw => {
                    // Forethought: move to bottom of draw pile, mark free_to_play_once
                    for uuid in &uuids {
                        if let Some(pos) = combat_state.hand.iter().position(|c| c.uuid == *uuid) {
                            let mut card = combat_state.hand.remove(pos);
                            card.cost_for_turn = Some(0); // free_to_play_once
                            combat_state.draw_pile.push(card);
                        }
                    }
                }
                HandSelectReason::Retain => {
                    // Retain: mark selected cards as retained (skip discard at turn end)
                    // Currently a stub — retain flag not in CombatCard
                }
                HandSelectReason::Copy { amount } => {
                    // Dual Wield: copy selected card(s) into hand
                    for uuid in &uuids {
                        if let Some(pos) = combat_state.hand.iter().position(|c| c.uuid == *uuid) {
                            let card = combat_state.hand[pos].clone();
                            for _ in 0..amount {
                                let mut copy = card.clone();
                                copy.uuid = 60000 + combat_state.hand.len() as u32;
                                combat_state.hand.push(copy);
                            }
                        }
                    }
                }
                HandSelectReason::Upgrade => {
                    // Armaments upgraded: upgrade selected card in hand
                    for uuid in &uuids {
                        if let Some(card) = combat_state.hand.iter_mut().find(|c| c.uuid == *uuid) {
                            card.upgrades += 1;
                        }
                    }
                }
            }

            *engine_state = EngineState::CombatProcessing;
            Ok(())
        }
        _ => Err("Invalid input for HandSelect"),
    }
}

pub fn handle_grid_select(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    candidate_uuids: &[u32],
    source_pile: PileType,
    min_cards: u8,
    max_cards: u8,
    can_cancel: bool,
    reason: GridSelectReason,
    input: ClientInput,
) -> Result<(), &'static str> {
    match input {
        ClientInput::Cancel => {
            if can_cancel {
                *engine_state = EngineState::CombatProcessing;
                Ok(())
            } else {
                Err("Cannot cancel this selection")
            }
        }
        ClientInput::SubmitGridSelect(uuids) => {
            if uuids.iter().any(|uuid| !candidate_uuids.contains(uuid)) {
                return Err("Selected card is not in the frozen grid-select candidate set");
            }
            if uuids.len() < min_cards as usize {
                return Err("Must select at least the minimum number of cards");
            }
            if uuids.len() > max_cards as usize {
                return Err("Too many cards selected");
            }
            match reason {
                GridSelectReason::DiscardToHand => {
                    // Java BetterDiscardPileToHandAction: move from discard to hand, setCostForTurn(0)
                    for uuid in &uuids {
                        if let Some(pos) = combat_state
                            .discard_pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                        {
                            let mut card = combat_state.discard_pile.remove(pos);
                            card.cost_for_turn = Some(0);
                            if combat_state.hand.len() < 10 {
                                combat_state.hand.push(card);
                            }
                        }
                    }
                }
                GridSelectReason::MoveToDrawPile => {
                    // Move from source pile to draw pile (random position)
                    for uuid in &uuids {
                        let pile = match source_pile {
                            PileType::Discard => &mut combat_state.discard_pile,
                            PileType::Exhaust => &mut combat_state.exhaust_pile,
                            _ => &mut combat_state.discard_pile,
                        };
                        if let Some(pos) = pile.iter().position(|c| c.uuid == *uuid) {
                            let card = pile.remove(pos);
                            combat_state.draw_pile.push(card);
                        }
                    }
                }
                GridSelectReason::Exhume { upgrade } => {
                    // Exhume: move from exhaust to hand
                    for uuid in &uuids {
                        if let Some(pos) = combat_state
                            .exhaust_pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                        {
                            let mut card = combat_state.exhaust_pile.remove(pos);
                            if upgrade {
                                card.upgrades += 1;
                            }
                            if combat_state.hand.len() < 10 {
                                combat_state.hand.push(card);
                            }
                        }
                    }
                }
                GridSelectReason::SkillFromDeckToHand | GridSelectReason::AttackFromDeckToHand => {
                    // SecretTechnique/SecretWeapon: move from draw pile to hand
                    for uuid in &uuids {
                        if let Some(pos) =
                            combat_state.draw_pile.iter().position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.draw_pile.remove(pos);
                            if combat_state.hand.len() < 10 {
                                combat_state.hand.push(card);
                            }
                        }
                    }
                }
            }

            *engine_state = EngineState::CombatProcessing;
            Ok(())
        }
        _ => Err("Invalid input for GridSelect"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::combat::{CombatCard, CombatPhase, CombatState, PlayerEntity, RelicBuses, StanceId};
    use crate::state::PendingChoice;
    use std::collections::{HashMap, VecDeque};

    fn test_combat() -> CombatState {
        CombatState {
            ascension_level: 0,
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            draw_pile: Vec::new(),
            hand: vec![CombatCard::new(crate::content::cards::CardId::Strike, 101)],
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            player: PlayerEntity {
                id: 0,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: Vec::new(),
                stance: StanceId::Neutral,
                relics: Vec::new(),
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters: Vec::new(),
            potions: vec![None, None, None],
            power_db: HashMap::new(),
            action_queue: VecDeque::<Action>::new(),
            counters: Default::default(),
            card_uuid_counter: 1,
            rng: crate::rng::RngPool::new(5),
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        }
    }

    #[test]
    fn grid_select_rejects_cancel_when_not_cancellable() {
        let mut engine_state = EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: vec![101],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::DiscardToHand,
        });
        let mut combat = test_combat();

        let err = handle_grid_select(
            &mut engine_state,
            &mut combat,
            &[101],
            PileType::Discard,
            1,
            1,
            false,
            GridSelectReason::DiscardToHand,
            ClientInput::Cancel,
        )
        .unwrap_err();

        assert_eq!(err, "Cannot cancel this selection");
    }

    #[test]
    fn hand_select_rejects_unknown_uuid() {
        let mut engine_state = EngineState::PendingChoice(PendingChoice::HandSelect {
            candidate_uuids: vec![101],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: HandSelectReason::Upgrade,
        });
        let mut combat = test_combat();

        let err = handle_hand_select(
            &mut engine_state,
            &mut combat,
            &[101],
            1,
            true,
            false,
            HandSelectReason::Upgrade,
            ClientInput::SubmitHandSelect(vec![999]),
        )
        .unwrap_err();

        assert_eq!(
            err,
            "Selected card is not in the frozen hand-select candidate set"
        );
    }
}
