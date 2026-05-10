use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
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
    amount: usize,
    card_uuids: &[u32],
    input: ClientInput,
) -> Result<(), &'static str> {
    match input {
        ClientInput::SubmitScryDiscard(indices) => {
            if card_uuids.len() != amount {
                return Err("Scry candidate count mismatch");
            }
            let mut selected = Vec::new();
            for index in indices {
                if index >= card_uuids.len() {
                    return Err("Invalid discard indices");
                }
                let uuid = card_uuids[index];
                if selected.contains(&uuid) {
                    return Err("Duplicate scry discard index");
                }
                selected.push(uuid);
            }

            for uuid in selected {
                if let Some(pos) = combat_state
                    .zones
                    .draw_pile
                    .iter()
                    .position(|card| card.uuid == uuid)
                {
                    let card = combat_state.zones.draw_pile.remove(pos);
                    combat_state.add_card_to_discard_pile_top(card);
                } else {
                    return Err("Scry candidate no longer in draw pile");
                }
            }

            *engine_state = EngineState::CombatProcessing;
            Ok(())
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
                            combat_state.add_card_to_discard_pile_top(card);
                        }
                    }
                    // Queue draw actions for same number of cards
                    if num_selected > 0 {
                        let action = ActionInfo {
                            action: Action::DrawCards(num_selected as u32),
                            insertion_mode: AddTo::Top,
                        };
                        combat_state.queue_actions(smallvec::smallvec![action]);
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
                            combat_state.add_card_to_discard_pile_top(card);
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
                            combat_state.add_card_to_draw_pile_top(card);
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
                            combat_state.add_card_to_draw_pile_bottom(card);
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
                            crate::engine::action_handlers::cards::handle_make_copy_in_hand(
                                Box::new(card),
                                amount,
                                combat_state,
                            );
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
                            card.set_cost_for_turn_java(0);
                            if combat_state.zones.hand.len() < 10 {
                                combat_state.zones.hand.push(card);
                            }
                        }
                    }
                }
                GridSelectReason::MoveToDrawPile => {
                    // Headbutt-style movement: selected card returns to the top of draw pile.
                    for uuid in &uuids {
                        let pile = match source_pile {
                            PileType::Discard => &mut combat_state.zones.discard_pile,
                            PileType::Exhaust => &mut combat_state.zones.exhaust_pile,
                            _ => &mut combat_state.zones.discard_pile,
                        };
                        if let Some(pos) = pile.iter().position(|c| c.uuid == *uuid) {
                            let card = pile.remove(pos);
                            combat_state.add_card_to_draw_pile_top(card);
                        }
                    }
                }
                GridSelectReason::Exhume { upgrade } => {
                    for uuid in &uuids {
                        crate::engine::action_handlers::cards::handle_exhume_card(
                            *uuid,
                            upgrade,
                            combat_state,
                        );
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

#[cfg(test)]
mod tests {
    use super::{handle_hand_select, handle_scry};
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState, HandSelectReason};
    use crate::test_support::blank_test_combat;

    #[test]
    fn scry_discards_selected_top_candidates_by_index() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![1, 2],
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Bash, 3),
        ];

        handle_scry(
            &mut engine_state,
            &mut combat_state,
            2,
            &[1, 2],
            ClientInput::SubmitScryDiscard(vec![1]),
        )
        .expect("scry selection should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(
            combat_state
                .zones
                .draw_pile
                .iter()
                .map(|card| card.id)
                .collect::<Vec<_>>(),
            vec![CardId::Strike, CardId::Bash]
        );
        assert_eq!(combat_state.zones.discard_pile.len(), 1);
        assert_eq!(combat_state.zones.discard_pile[0].id, CardId::Defend);
    }

    #[test]
    fn scry_rejects_duplicate_indices_without_mutating_piles() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![1, 2],
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];

        let result = handle_scry(
            &mut engine_state,
            &mut combat_state,
            2,
            &[1, 2],
            ClientInput::SubmitScryDiscard(vec![0, 0]),
        );

        assert!(result.is_err());
        assert_eq!(combat_state.zones.draw_pile.len(), 2);
        assert!(combat_state.zones.discard_pile.is_empty());
    }

    #[test]
    fn hand_select_copy_uses_generated_card_uuid_counter_and_hand_overflow_rules() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect {
                reason: HandSelectReason::Copy { amount: 3 },
                candidate_uuids: vec![10],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.card_uuid_counter = 100;
        combat_state.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        for uuid in 11..20 {
            combat_state
                .zones
                .hand
                .push(CombatCard::new(CardId::Defend, uuid));
        }

        handle_hand_select(
            &mut engine_state,
            &mut combat_state,
            &[10],
            1,
            true,
            false,
            HandSelectReason::Copy { amount: 3 },
            ClientInput::SubmitHandSelect(vec![10]),
        )
        .expect("copy selection should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(combat_state.zones.card_uuid_counter, 103);
        assert_eq!(combat_state.zones.hand.len(), 10);
        assert_eq!(combat_state.zones.discard_pile.len(), 3);
        assert_eq!(
            combat_state
                .zones
                .discard_pile
                .iter()
                .map(|card| (card.id, card.uuid))
                .collect::<Vec<_>>(),
            vec![
                (CardId::Strike, 101),
                (CardId::Strike, 102),
                (CardId::Strike, 103),
            ]
        );
    }
}
