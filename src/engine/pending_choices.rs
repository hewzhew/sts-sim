use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::state::core::{ClientInput, EngineState, GridSelectReason, HandSelectReason, PileType};
use crate::state::selection::{
    DomainCardSnapshot, DomainEvent, DomainEventSource, SelectionResolution, SelectionScope,
    SelectionTargetRef,
};

fn selection_to_uuids(
    input: ClientInput,
    expected_scope: SelectionScope,
) -> Result<Vec<u32>, &'static str> {
    match input {
        ClientInput::SubmitSelection(SelectionResolution { scope, selected }) => {
            if scope != expected_scope {
                return Err("Selection scope does not match pending choice");
            }
            let uuids = selected
                .into_iter()
                .map(|target| match target {
                    SelectionTargetRef::CardUuid(uuid) => Ok(uuid),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(uuids)
        }
        ClientInput::SubmitHandSelect(uuids) if expected_scope == SelectionScope::Hand => Ok(uuids),
        ClientInput::SubmitGridSelect(uuids) if expected_scope == SelectionScope::Grid => Ok(uuids),
        _ => Err("Invalid input for selection"),
    }
}

fn snapshot_cards_from_hand(combat_state: &CombatState, uuids: &[u32]) -> Vec<DomainCardSnapshot> {
    uuids
        .iter()
        .filter_map(|uuid| {
            combat_state
                .zones
                .hand
                .iter()
                .find(|card| card.uuid == *uuid)
                .map(|card| DomainCardSnapshot {
                    id: card.id,
                    upgrades: card.upgrades,
                    uuid: card.uuid,
                })
        })
        .collect()
}

pub fn handle_scry(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    _amount: usize,
    input: ClientInput,
) -> Result<(), &'static str> {
    match input {
        ClientInput::SubmitScryDiscard(indices) => {
            // Simplified stub
            if indices.len() <= combat_state.zones.draw_pile.len() {
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
        input => {
            let uuids = selection_to_uuids(input, SelectionScope::Hand)?;
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
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.zones.hand.remove(pos);
                            combat_state.zones.discard_pile.push(card);
                        }
                    }
                    // Queue draw actions for same number of cards
                    if num_selected > 0 {
                        let action = ActionInfo {
                            action: Action::DrawCards(num_selected as u32),
                            insertion_mode: AddTo::Top,
                        };
                        crate::engine::core::queue_actions(
                            &mut combat_state.engine.action_queue,
                            smallvec::smallvec![action],
                        );
                    }
                }
                HandSelectReason::Exhaust => {
                    let exhausted_cards = snapshot_cards_from_hand(combat_state, &uuids);
                    // Java ExhaustAction: exhaust selected cards from hand
                    for uuid in &uuids {
                        crate::engine::action_handlers::cards::handle_exhaust_card(
                            *uuid,
                            PileType::Hand,
                            combat_state,
                        );
                    }
                    if !exhausted_cards.is_empty() {
                        combat_state.emit_event(DomainEvent::CardsExhausted {
                            cards: exhausted_cards,
                            source: DomainEventSource::Selection(reason.into()),
                        });
                    }
                }
                HandSelectReason::Discard => {
                    // Discard selected cards from hand
                    for uuid in &uuids {
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.zones.hand.remove(pos);
                            combat_state.zones.discard_pile.push(card);
                        }
                    }
                }
                HandSelectReason::PutOnDrawPile => {
                    // Move selected cards from hand to top of draw pile
                    for uuid in &uuids {
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.zones.hand.remove(pos);
                            combat_state.zones.draw_pile.insert(0, card);
                        }
                    }
                }
                HandSelectReason::PutToBottomOfDraw => {
                    // Forethought: move to bottom of draw pile; only cards with cost > 0 become free once.
                    for uuid in &uuids {
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let mut card = combat_state.zones.hand.remove(pos);
                            if card.get_cost() > 0 {
                                card.free_to_play_once = true;
                            }
                            combat_state.zones.draw_pile.push(card);
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
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.zones.hand[pos].clone();
                            for _ in 0..amount {
                                let mut copy = card.clone();
                                copy.uuid = 60000 + combat_state.zones.hand.len() as u32;
                                combat_state.zones.hand.push(copy);
                            }
                        }
                    }
                }
                HandSelectReason::Upgrade => {
                    // Armaments upgraded: upgrade selected card in hand
                    for uuid in &uuids {
                        if let Some(card) =
                            combat_state.zones.hand.iter_mut().find(|c| c.uuid == *uuid)
                        {
                            let before = DomainCardSnapshot {
                                id: card.id,
                                upgrades: card.upgrades,
                                uuid: card.uuid,
                            };
                            card.upgrades += 1;
                            combat_state.emit_event(DomainEvent::CardUpgraded {
                                before,
                                after: before.upgraded(),
                                source: DomainEventSource::Selection(reason.into()),
                            });
                        }
                    }
                }
            }

            combat_state.emit_event(DomainEvent::SelectionResolved {
                scope: SelectionScope::Hand,
                reason: reason.into(),
                selected: uuids
                    .iter()
                    .copied()
                    .map(SelectionTargetRef::CardUuid)
                    .collect(),
                source: DomainEventSource::Selection(reason.into()),
            });
            *engine_state = EngineState::CombatProcessing;
            Ok(())
        }
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
        input => {
            let uuids = selection_to_uuids(input, SelectionScope::Grid)?;
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
                            .zones
                            .discard_pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                        {
                            let mut card = combat_state.zones.discard_pile.remove(pos);
                            card.cost_for_turn = Some(0);
                            if combat_state.zones.hand.len() < 10 {
                                combat_state.zones.hand.push(card);
                            }
                        }
                    }
                }
                GridSelectReason::MoveToDrawPile => {
                    // Move from source pile to draw pile (random position)
                    for uuid in &uuids {
                        let pile = match source_pile {
                            PileType::Discard => &mut combat_state.zones.discard_pile,
                            PileType::Exhaust => &mut combat_state.zones.exhaust_pile,
                            _ => &mut combat_state.zones.discard_pile,
                        };
                        if let Some(pos) = pile.iter().position(|c| c.uuid == *uuid) {
                            let card = pile.remove(pos);
                            combat_state.zones.draw_pile.push(card);
                        }
                    }
                }
                GridSelectReason::Exhume { upgrade } => {
                    // Exhume: move from exhaust to hand
                    for uuid in &uuids {
                        if let Some(pos) = combat_state
                            .zones
                            .exhaust_pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                        {
                            let mut card = combat_state.zones.exhaust_pile.remove(pos);
                            if upgrade {
                                card.upgrades += 1;
                            }
                            if combat_state.zones.hand.len() < 10 {
                                combat_state.zones.hand.push(card);
                            }
                        }
                    }
                }
                GridSelectReason::SkillFromDeckToHand | GridSelectReason::AttackFromDeckToHand => {
                    // SecretTechnique/SecretWeapon: move from draw pile to hand
                    for uuid in &uuids {
                        if let Some(pos) = combat_state
                            .zones
                            .draw_pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.zones.draw_pile.remove(pos);
                            if combat_state.zones.hand.len() < 10 {
                                combat_state.zones.hand.push(card);
                            }
                        }
                    }
                }
            }

            combat_state.emit_event(DomainEvent::SelectionResolved {
                scope: SelectionScope::Grid,
                reason: reason.into(),
                selected: uuids
                    .iter()
                    .copied()
                    .map(SelectionTargetRef::CardUuid)
                    .collect(),
                source: DomainEventSource::Selection(reason.into()),
            });
            *engine_state = EngineState::CombatProcessing;
            Ok(())
        }
    }
}

