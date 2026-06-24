use crate::content::cards;
use crate::state::core::{PendingChoice, PileType};
use crate::state::selection::SelectionScope;

use super::super::*;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ActionEquivalenceKey {
    pub(super) kind: ActionEquivalenceKind,
    pub(super) signature: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum ActionEquivalenceKind {
    StarterBasicPlayCard,
    SingleCardPendingChoiceSelection,
}

pub(super) fn equivalence_key_for_choice(
    engine: &EngineState,
    combat: &CombatState,
    choice: &CombatActionChoice,
) -> Option<ActionEquivalenceKey> {
    match &choice.input {
        ClientInput::PlayCard { card_index, target } => {
            if !matches!(engine, EngineState::CombatPlayerTurn) {
                return None;
            }
            let card = combat.zones.hand.get(*card_index)?;
            if !cards::is_starter_basic(card.id) {
                return None;
            }
            Some(ActionEquivalenceKey {
                kind: ActionEquivalenceKind::StarterBasicPlayCard,
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

fn pending_single_card_selection_key(
    engine: &EngineState,
    combat: &CombatState,
    uuids: &[u32],
) -> Option<ActionEquivalenceKey> {
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
            combat.zones.hand.as_slice(),
        ),
        _ => return None,
    };
    let card = cards.iter().find(|card| card.uuid == *uuid)?;
    Some(ActionEquivalenceKey {
        kind: ActionEquivalenceKind::SingleCardPendingChoiceSelection,
        signature: format!("{scope}/selected_card:{}", card_runtime_signature(card)),
    })
}

fn pile_cards(combat: &CombatState, pile: PileType) -> &[crate::runtime::combat::CombatCard] {
    match pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &[],
    }
}

fn starter_basic_card_signature(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> String {
    format!(
        "play_card/starter_basic/{}/target:{}",
        card_runtime_signature(card),
        crate::sim::combat_action::target_label(combat, target),
    )
}

fn card_runtime_signature(card: &crate::runtime::combat::CombatCard) -> String {
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
