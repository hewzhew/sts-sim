//! Conservative equivalence classes over exact legal combat inputs.
//!
//! The simulator keeps the complete legal action surface.  This module only
//! identifies actions whose differing runtime handles are proven irrelevant
//! for the supported case, so downstream search or learning owners may choose
//! one canonical representative without changing engine legality.

use std::collections::BTreeMap;

use crate::content::cards;
use crate::runtime::combat::{CardPileView, CombatCard, CombatState};
use crate::state::core::{ClientInput, EngineState, PendingChoice, PileType};
use crate::state::selection::SelectionScope;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CombatActionEquivalenceKeyV1 {
    pub kind: CombatActionEquivalenceKindV1,
    pub signature: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CombatActionEquivalenceKindV1 {
    StarterBasicPlayCard,
    SingleCardPendingChoiceSelection,
}

/// Returns a conservative exact-state equivalence key for one legal input.
///
/// `None` means the action remains its own class.  In particular, duplicate
/// non-starter cards are intentionally not generalized yet: UUID-sensitive
/// combat effects need a broader proof before that surface can be compressed.
pub fn combat_action_equivalence_key_v1(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> Option<CombatActionEquivalenceKeyV1> {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            if !matches!(engine, EngineState::CombatPlayerTurn) {
                return None;
            }
            let card = combat.zones.hand.get(*card_index)?;
            if !cards::is_starter_basic(card.id) {
                return None;
            }
            Some(CombatActionEquivalenceKeyV1 {
                kind: CombatActionEquivalenceKindV1::StarterBasicPlayCard,
                signature: starter_basic_card_signature(combat, card, *target),
            })
        }
        ClientInput::SubmitSelection(resolution)
            if matches!(
                resolution.scope,
                SelectionScope::Hand | SelectionScope::Grid
            ) =>
        {
            pending_single_card_selection_key(engine, combat, &resolution.selected_card_uuids())
        }
        _ => None,
    }
}

/// Maps every input ordinal to the first exact-state-equivalent input ordinal.
///
/// The returned vector is aligned one-to-one with `inputs`; an unclassified
/// action maps to itself.  The first member of a class is therefore the stable
/// canonical representative while the complete legal input list remains
/// untouched.
pub fn canonical_combat_action_representatives_v1(
    engine: &EngineState,
    combat: &CombatState,
    inputs: &[ClientInput],
) -> Vec<usize> {
    let mut seen = BTreeMap::new();
    inputs
        .iter()
        .enumerate()
        .map(|(ordinal, input)| {
            let Some(key) = combat_action_equivalence_key_v1(engine, combat, input) else {
                return ordinal;
            };
            *seen.entry(key).or_insert(ordinal)
        })
        .collect()
}

fn pending_single_card_selection_key(
    engine: &EngineState,
    combat: &CombatState,
    uuids: &[u32],
) -> Option<CombatActionEquivalenceKeyV1> {
    let [uuid] = uuids else {
        return None;
    };
    let EngineState::PendingChoice(choice) = engine else {
        return None;
    };

    let (scope, cards) = match choice {
        PendingChoice::GridSelect {
            source_pile,
            reason,
            candidate_uuids,
            ..
        } if candidate_uuids.contains(uuid) => (
            format!("grid_select/source:{source_pile:?}/reason:{reason:?}"),
            pile_cards(combat, *source_pile),
        ),
        PendingChoice::HandSelect {
            reason,
            candidate_uuids,
            ..
        } if candidate_uuids.contains(uuid) => (
            format!("hand_select/reason:{reason:?}"),
            CardPileView::Contiguous(&combat.zones.hand),
        ),
        _ => return None,
    };
    let card = cards.iter().find(|card| card.uuid == *uuid)?;
    Some(CombatActionEquivalenceKeyV1 {
        kind: CombatActionEquivalenceKindV1::SingleCardPendingChoiceSelection,
        signature: format!("{scope}/selected_card:{}", card_runtime_signature(card)),
    })
}

fn pile_cards(combat: &CombatState, pile: PileType) -> CardPileView<'_> {
    match pile {
        PileType::Draw => CardPileView::Contiguous(combat.zones.draw_pile.as_ref()),
        PileType::Discard => CardPileView::Discard(&combat.zones.discard_pile),
        PileType::Exhaust => CardPileView::Contiguous(combat.zones.exhaust_pile.as_slice()),
        PileType::Hand => CardPileView::Contiguous(&combat.zones.hand),
        PileType::Limbo => CardPileView::Contiguous(&combat.zones.limbo),
        PileType::MasterDeck => CardPileView::Contiguous(&[]),
    }
}

fn starter_basic_card_signature(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> String {
    format!(
        "play_card/starter_basic/{}/target:{}",
        card_runtime_signature(card),
        crate::sim::combat_action::target_label(combat, target),
    )
}

fn card_runtime_signature(card: &CombatCard) -> String {
    format!(
        "card:{}+{}/misc:{}/damage_override:{:?}/block_override:{:?}/cost_modifier:{}/cost_for_turn:{:?}/base_damage_mut:{}/base_block_mut:{}/base_magic_num_mut:{}/multi_damage:{:?}/exhaust_override:{:?}/retain_override:{:?}/free_to_play_once:{}/energy_on_use:{}",
        cards::java_id(card.id),
        card.upgrades,
        card.misc_value,
        card.base_damage_override,
        card.base_block_override,
        card.cost_modifier,
        card.cost_for_turn,
        card.base_damage_mut,
        card.base_block_mut,
        card.base_magic_num_mut,
        card.multi_damage,
        card.exhaust_override,
        card.retain_override,
        card.free_to_play_once,
        card.energy_on_use
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;

    #[test]
    fn canonical_representatives_merge_only_runtime_identical_starter_basics() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 1;
        combat.entities.monsters = vec![monster];
        let mut free = CombatCard::new(CardId::Defend, 12);
        free.free_to_play_once = true;
        combat.zones.hand = vec![
            CombatCard::new(CardId::Defend, 10),
            CombatCard::new(CardId::Defend, 11),
            free,
        ];
        let inputs = vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
            ClientInput::EndTurn,
        ];

        assert_eq!(
            canonical_combat_action_representatives_v1(
                &EngineState::CombatPlayerTurn,
                &combat,
                &inputs,
            ),
            vec![0, 0, 2, 3]
        );
    }
}
