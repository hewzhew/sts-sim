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

fn reject_duplicate_uuids(uuids: &[u32], message: &'static str) -> Result<(), &'static str> {
    for (idx, uuid) in uuids.iter().enumerate() {
        if uuids[..idx].contains(uuid) {
            return Err(message);
        }
    }
    Ok(())
}

fn pile_contains_all(pile: &[crate::runtime::combat::CombatCard], uuids: &[u32]) -> bool {
    uuids
        .iter()
        .all(|uuid| pile.iter().any(|card| card.uuid == *uuid))
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

            if !pile_contains_all(&combat_state.zones.draw_pile, &selected) {
                return Err("Scry candidate no longer in draw pile");
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
            reject_duplicate_uuids(&uuids, "Duplicate hand selection")?;
            if !pile_contains_all(&combat_state.zones.hand, &uuids) {
                return Err("Selected hand card no longer in hand");
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
                    // Java ForethoughtAction checks AbstractCard.cost, not
                    // costForTurn. Temporary turn-cost reductions do not stop
                    // the selected card from becoming free next time it is
                    // drawn.
                    for uuid in &uuids {
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let mut card = combat_state.zones.hand.remove(pos);
                            if card.combat_cost_without_turn_override_java() > 0 {
                                card.free_to_play_once = true;
                            }
                            combat_state.add_card_to_draw_pile_bottom(card);
                        }
                    }
                }
                HandSelectReason::Retain => {
                    // Java RetainCardsAction/MeditateAction sets AbstractCard.retain
                    // for one end-of-turn discard pass. RestoreRetainedCardsAction
                    // clears that flag after the card survives the turn.
                    for uuid in &uuids {
                        if let Some(card) =
                            combat_state.zones.hand.iter_mut().find(|c| c.uuid == *uuid)
                        {
                            card.retain_override = Some(true);
                        }
                    }
                }
                HandSelectReason::Copy { amount } => {
                    // Java DualWieldAction reads selectedCards after the hand
                    // select screen has removed the selected original from
                    // hand. Its selected branch queues one extra copy to
                    // replace that original, so Rust must remove the selected
                    // card before applying `amount`.
                    for uuid in &uuids {
                        if let Some(pos) =
                            combat_state.zones.hand.iter().position(|c| c.uuid == *uuid)
                        {
                            let card = combat_state.zones.hand.remove(pos);
                            crate::engine::action_handlers::cards::handle_make_copy_in_hand(
                                Box::new(card),
                                amount,
                                combat_state,
                            );
                        }
                    }
                }
                HandSelectReason::Upgrade => {
                    if candidate_uuids.len() > 1 {
                        let mut remaining_candidates = Vec::new();
                        let mut selected_cards = Vec::new();
                        let mut non_candidates = Vec::new();

                        for card in combat_state.zones.hand.drain(..) {
                            if uuids.contains(&card.uuid) {
                                selected_cards.push(card);
                            } else if candidate_uuids.contains(&card.uuid) {
                                remaining_candidates.push(card);
                            } else {
                                non_candidates.push(card);
                            }
                        }

                        combat_state.zones.hand = remaining_candidates;
                        for mut card in selected_cards {
                            let before = DomainCardSnapshot {
                                id: card.id,
                                upgrades: card.upgrades,
                                uuid: card.uuid,
                            };
                            card.upgrades += 1;
                            combat_state.zones.hand.push(card);
                            combat_state.emit_event(DomainEvent::CardUpgraded {
                                before,
                                after: before.upgraded(),
                                source: DomainEventSource::Selection(reason.into()),
                            });
                        }
                        for card in non_candidates {
                            combat_state.zones.hand.push(card);
                        }
                    } else {
                        // Java ArmamentsAction's single-upgradeable branch upgrades in place.
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
            reject_duplicate_uuids(&uuids, "Duplicate grid selection")?;
            if uuids.len() < min_cards as usize {
                return Err("Must select at least the minimum number of cards");
            }
            if uuids.len() > max_cards as usize {
                return Err("Too many cards selected");
            }
            match reason {
                GridSelectReason::DiscardToHand => {
                    // Java BetterDiscardPileToHandAction: move from discard to hand, setCostForTurn(0)
                    if !matches!(source_pile, PileType::Discard) {
                        return Err("Discard-to-hand selection must source from discard pile");
                    }
                    if !pile_contains_all(&combat_state.zones.discard_pile, &uuids) {
                        return Err("Grid candidate no longer in discard pile");
                    }
                    for uuid in &uuids {
                        if combat_state.zones.hand.len() < 10 {
                            let pos = combat_state
                                .zones
                                .discard_pile
                                .iter()
                                .position(|c| c.uuid == *uuid)
                                .expect("validated discard-to-hand selection must still exist");
                            let mut card = combat_state.zones.discard_pile.remove(pos);
                            card.set_cost_for_turn_java(0);
                            combat_state.zones.hand.push(card);
                        }
                    }
                }
                GridSelectReason::MoveToDrawPile => {
                    // Headbutt-style movement: selected card returns to the top of draw pile.
                    let candidates_still_present =
                        match source_pile {
                            PileType::Discard => {
                                pile_contains_all(&combat_state.zones.discard_pile, &uuids)
                            }
                            PileType::Exhaust => {
                                pile_contains_all(&combat_state.zones.exhaust_pile, &uuids)
                            }
                            _ => return Err(
                                "Move-to-draw selection must source from discard or exhaust pile",
                            ),
                        };
                    if !candidates_still_present {
                        return Err("Grid candidate no longer in source pile");
                    }
                    for uuid in &uuids {
                        let pile = match source_pile {
                            PileType::Discard => &mut combat_state.zones.discard_pile,
                            PileType::Exhaust => &mut combat_state.zones.exhaust_pile,
                            _ => unreachable!("source pile was validated above"),
                        };
                        let pos = pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                            .expect("validated move-to-draw selection must still exist");
                        let card = pile.remove(pos);
                        combat_state.add_card_to_draw_pile_top(card);
                    }
                }
                GridSelectReason::Exhume { upgrade } => {
                    if !matches!(source_pile, PileType::Exhaust) {
                        return Err("Exhume selection must source from exhaust pile");
                    }
                    if !pile_contains_all(&combat_state.zones.exhaust_pile, &uuids) {
                        return Err("Grid candidate no longer in exhaust pile");
                    }
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
                    if !matches!(source_pile, PileType::Draw) {
                        return Err("Deck-to-hand selection must source from draw pile");
                    }
                    if !pile_contains_all(&combat_state.zones.draw_pile, &uuids) {
                        return Err("Grid candidate no longer in draw pile");
                    }
                    for uuid in &uuids {
                        let pos = combat_state
                            .zones
                            .draw_pile
                            .iter()
                            .position(|c| c.uuid == *uuid)
                            .expect("validated deck-to-hand selection must still exist");
                        let card = combat_state.zones.draw_pile.remove(pos);
                        if combat_state.zones.hand.len() < 10 {
                            combat_state.zones.hand.push(card);
                        } else {
                            combat_state.add_card_to_discard_pile_top(card);
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
    use super::{handle_grid_select, handle_hand_select, handle_scry};
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ClientInput, EngineState, GridSelectReason, HandSelectReason, PileType,
    };
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
    fn scry_rejects_stale_candidate_without_partial_mutation() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![10, 20],
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 10)];

        let result = handle_scry(
            &mut engine_state,
            &mut combat_state,
            2,
            &[10, 20],
            ClientInput::SubmitScryDiscard(vec![0, 1]),
        );

        assert_eq!(result, Err("Scry candidate no longer in draw pile"));
        assert_eq!(
            combat_state
                .zones
                .draw_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![10]
        );
        assert!(combat_state.zones.discard_pile.is_empty());
    }

    #[test]
    fn hand_select_copy_matches_dual_wield_selected_card_replacement() {
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
        assert_eq!(
            combat_state
                .zones
                .hand
                .iter()
                .filter(|c| c.uuid == 10)
                .count(),
            0,
            "Java hand select removes the selected original before DualWieldAction creates copies"
        );
        assert_eq!(
            combat_state
                .zones
                .hand
                .iter()
                .filter(|card| card.id == CardId::Strike)
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![101],
            "one replacement copy fits in hand after the selected original is removed"
        );
        assert_eq!(combat_state.zones.discard_pile.len(), 2);
        assert_eq!(
            combat_state
                .zones
                .discard_pile
                .iter()
                .map(|card| (card.id, card.uuid))
                .collect::<Vec<_>>(),
            vec![(CardId::Strike, 102), (CardId::Strike, 103),]
        );
    }

    #[test]
    fn hand_select_upgrade_matches_armaments_screen_order() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect {
                reason: HandSelectReason::Upgrade,
                candidate_uuids: vec![20, 30],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
            });
        let mut combat_state = blank_test_combat();
        let mut already_upgraded = CombatCard::new(CardId::Strike, 10);
        already_upgraded.upgrades = 1;
        combat_state.zones.hand = vec![
            already_upgraded,
            CombatCard::new(CardId::Defend, 20),
            CombatCard::new(CardId::Bash, 30),
            CombatCard::new(CardId::Wound, 40),
        ];

        handle_hand_select(
            &mut engine_state,
            &mut combat_state,
            &[20, 30],
            1,
            true,
            false,
            HandSelectReason::Upgrade,
            ClientInput::SubmitHandSelect(vec![20]),
        )
        .expect("upgrade selection should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(
            combat_state
                .zones
                .hand
                .iter()
                .map(|card| (card.id, card.uuid, card.upgrades))
                .collect::<Vec<_>>(),
            vec![
                (CardId::Bash, 30, 0),
                (CardId::Defend, 20, 1),
                (CardId::Strike, 10, 1),
                (CardId::Wound, 40, 0),
            ],
            "Java ArmamentsAction removes non-upgradeable cards before selection, then addToTop returns the selected card and non-upgradeables"
        );
    }

    #[test]
    fn hand_select_retain_marks_selected_cards_for_one_turn_retain() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect {
                reason: HandSelectReason::Retain,
                candidate_uuids: vec![10, 20],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 20),
        ];

        handle_hand_select(
            &mut engine_state,
            &mut combat_state,
            &[10, 20],
            1,
            true,
            false,
            HandSelectReason::Retain,
            ClientInput::SubmitHandSelect(vec![20]),
        )
        .expect("retain selection should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(
            combat_state
                .zones
                .hand
                .iter()
                .map(|card| (card.id, card.uuid, card.retain_override))
                .collect::<Vec<_>>(),
            vec![
                (CardId::Strike, 10, None),
                (CardId::Defend, 20, Some(true)),
            ],
            "Java RetainCardsAction/MeditateAction marks only the selected cards with one-turn retain"
        );
    }

    #[test]
    fn hand_select_forethought_uses_combat_cost_not_turn_cost_for_free_once() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect {
                reason: HandSelectReason::PutToBottomOfDraw,
                candidate_uuids: vec![10],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
            });
        let mut combat_state = blank_test_combat();
        let mut temporarily_free_defend = CombatCard::new(CardId::Defend, 10);
        temporarily_free_defend.set_cost_for_turn_java(0);
        combat_state.zones.hand = vec![temporarily_free_defend];

        handle_hand_select(
            &mut engine_state,
            &mut combat_state,
            &[10],
            1,
            true,
            false,
            HandSelectReason::PutToBottomOfDraw,
            ClientInput::SubmitHandSelect(vec![10]),
        )
        .expect("Forethought-style selection should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert!(combat_state.zones.hand.is_empty());
        assert_eq!(combat_state.zones.draw_pile.len(), 1);
        let moved = &combat_state.zones.draw_pile[0];
        assert_eq!(moved.uuid, 10);
        assert!(
            moved.free_to_play_once,
            "Java ForethoughtAction checks AbstractCard.cost > 0, so a card made free only for this turn still becomes free once after redraw"
        );
    }

    #[test]
    fn grid_select_rejects_duplicate_candidate_without_mutation() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
                source_pile: PileType::Discard,
                candidate_uuids: vec![10],
                min_cards: 1,
                max_cards: 2,
                can_cancel: false,
                reason: GridSelectReason::MoveToDrawPile,
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 10)];

        let result = handle_grid_select(
            &mut engine_state,
            &mut combat_state,
            &[10],
            PileType::Discard,
            1,
            2,
            false,
            GridSelectReason::MoveToDrawPile,
            ClientInput::SubmitGridSelect(vec![10, 10]),
        );

        assert_eq!(result, Err("Duplicate grid selection"));
        assert_eq!(
            combat_state
                .zones
                .discard_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![10]
        );
        assert!(combat_state.zones.draw_pile.is_empty());
    }

    #[test]
    fn discard_to_hand_selection_leaves_card_when_hand_is_full() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
                source_pile: PileType::Discard,
                candidate_uuids: vec![20],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: GridSelectReason::DiscardToHand,
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.hand = (0..10)
            .map(|idx| CombatCard::new(CardId::Defend, 100 + idx))
            .collect();
        combat_state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 20)];

        handle_grid_select(
            &mut engine_state,
            &mut combat_state,
            &[20],
            PileType::Discard,
            1,
            1,
            false,
            GridSelectReason::DiscardToHand,
            ClientInput::SubmitGridSelect(vec![20]),
        )
        .expect("full hand still resolves like Java BetterDiscardPileToHandAction");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(combat_state.zones.hand.len(), 10);
        assert_eq!(
            combat_state
                .zones
                .discard_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![20],
            "Java leaves selected discard cards in discard when the hand is full"
        );
    }

    #[test]
    fn deck_to_hand_selection_discards_selected_card_when_hand_is_full() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
                source_pile: PileType::Draw,
                candidate_uuids: vec![30],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: GridSelectReason::AttackFromDeckToHand,
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.hand = (0..10)
            .map(|idx| CombatCard::new(CardId::Defend, 200 + idx))
            .collect();
        combat_state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 30)];

        handle_grid_select(
            &mut engine_state,
            &mut combat_state,
            &[30],
            PileType::Draw,
            1,
            1,
            false,
            GridSelectReason::AttackFromDeckToHand,
            ClientInput::SubmitGridSelect(vec![30]),
        )
        .expect("full hand should discard selected deck-to-hand card");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert!(combat_state.zones.draw_pile.is_empty());
        assert_eq!(
            combat_state
                .zones
                .discard_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![30],
            "Java SecretWeapon/SecretTechnique move selected draw-pile card to discard when hand is full"
        );
    }

    #[test]
    fn deck_to_hand_selection_rejects_stale_candidate_without_partial_mutation() {
        let mut engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
                source_pile: PileType::Draw,
                candidate_uuids: vec![40, 50],
                min_cards: 2,
                max_cards: 2,
                can_cancel: false,
                reason: GridSelectReason::SkillFromDeckToHand,
            });
        let mut combat_state = blank_test_combat();
        combat_state.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 40)];

        let result = handle_grid_select(
            &mut engine_state,
            &mut combat_state,
            &[40, 50],
            PileType::Draw,
            2,
            2,
            false,
            GridSelectReason::SkillFromDeckToHand,
            ClientInput::SubmitGridSelect(vec![40, 50]),
        );

        assert_eq!(result, Err("Grid candidate no longer in draw pile"));
        assert_eq!(
            combat_state
                .zones
                .draw_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![40]
        );
        assert!(combat_state.zones.hand.is_empty());
        assert!(combat_state.zones.discard_pile.is_empty());
    }
}
