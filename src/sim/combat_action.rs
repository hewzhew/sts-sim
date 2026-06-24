use crate::content::cards;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::ClientInput;
use crate::state::selection::SelectionScope;

#[derive(Clone, Debug, PartialEq)]
pub struct CombatActionChoice {
    pub input: ClientInput,
    pub action_key: String,
    pub action_debug: String,
}

impl CombatActionChoice {
    pub fn from_input(combat: &CombatState, input: ClientInput) -> Self {
        let action_key = combat_action_key(combat, &input);
        let action_debug = format!("{input:?}");
        Self {
            input,
            action_key,
            action_debug,
        }
    }
}

pub fn combat_action_key(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                format!(
                    "combat/play_card/hand:{card_index}/card:{}#{}/target:{}",
                    card_label(card),
                    card.uuid,
                    target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| format!("{input:?}")),
        ClientInput::UsePotion {
            potion_index,
            target,
        } => combat
            .entities
            .potions
            .get(*potion_index)
            .and_then(|potion| potion.as_ref())
            .map(|potion| {
                format!(
                    "combat/use_potion/slot:{potion_index}/potion:{:?}#{}/target:{}",
                    potion.id,
                    potion.uuid,
                    target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "combat/use_potion/slot:{potion_index}/target:{}",
                    target_label(combat, *target)
                )
            }),
        ClientInput::DiscardPotion(slot) => format!("combat/discard_potion/slot:{slot}"),
        ClientInput::EndTurn => "combat/end_turn".to_string(),
        ClientInput::SubmitDiscoverChoice(index) => format!("combat/submit_choice/{index}"),
        ClientInput::SubmitSelection(resolution) => {
            let prefix = match resolution.scope {
                SelectionScope::Hand => "combat/hand_select",
                SelectionScope::Grid => "combat/grid_select",
                SelectionScope::Deck => "combat/deck_select",
            };
            format!("{prefix}/{}", uuid_list(&resolution.selected_card_uuids()))
        }
        ClientInput::SubmitScryDiscard(indices) => {
            format!(
                "combat/scry_discard/{}",
                indices
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        ClientInput::Cancel => "combat/cancel".to_string(),
        ClientInput::Proceed => "combat/proceed".to_string(),
        _ => format!("{input:?}"),
    }
}

fn card_label(card: &CombatCard) -> String {
    format!("{}+{}", cards::java_id(card.id), card.upgrades)
}

pub fn target_label(combat: &CombatState, target: Option<usize>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

fn uuid_list(uuids: &[u32]) -> String {
    uuids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
