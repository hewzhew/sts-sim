use super::*;
use crate::content::cards;
use crate::state::core::{PendingChoice, PileType};
use std::collections::BTreeMap;

mod diagnostics;
pub(super) use diagnostics::ActionEquivalenceDiagnosticsCollector;

#[derive(Clone, Debug)]
pub(super) struct ActionEquivalenceResult {
    pub(super) choices: Vec<IndexedActionChoice>,
    pub(super) summary: ActionEquivalenceSummary,
}

#[derive(Clone, Debug)]
pub(super) struct ActionEquivalenceSummary {
    atomic_actions_in: usize,
    representative_actions_out: usize,
    groups: Vec<ActionEquivalenceGroupSummary>,
}

#[derive(Clone, Debug)]
struct ActionEquivalenceGroupSummary {
    key: ActionEquivalenceKey,
    representative_original_action_id: usize,
    removed_original_action_ids: Vec<usize>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ActionEquivalenceKey {
    kind: ActionEquivalenceKind,
    signature: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ActionEquivalenceKind {
    StarterBasicPlayCard,
    SingleCardPendingChoiceSelection,
}

#[derive(Clone, Debug)]
struct PendingEquivalenceGroup {
    representative_original_action_id: usize,
    removed_original_action_ids: Vec<usize>,
}

pub(super) fn compress_equivalent_actions(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<CombatActionChoice>,
) -> ActionEquivalenceResult {
    let atomic_actions_in = choices.len();
    let mut representatives = Vec::with_capacity(choices.len());
    let mut seen: BTreeMap<ActionEquivalenceKey, usize> = BTreeMap::new();
    let mut groups: Vec<(ActionEquivalenceKey, PendingEquivalenceGroup)> = Vec::new();

    for (original_action_id, choice) in choices.into_iter().enumerate() {
        let Some(key) = equivalence_key_for_choice(engine, combat, &choice) else {
            representatives.push(IndexedActionChoice {
                original_action_id,
                choice,
            });
            continue;
        };

        if let Some(group_index) = seen.get(&key).copied() {
            groups[group_index]
                .1
                .removed_original_action_ids
                .push(original_action_id);
        } else {
            representatives.push(IndexedActionChoice {
                original_action_id,
                choice,
            });
            seen.insert(key.clone(), groups.len());
            groups.push((
                key,
                PendingEquivalenceGroup {
                    representative_original_action_id: original_action_id,
                    removed_original_action_ids: Vec::new(),
                },
            ));
        }
    }

    let groups = groups
        .into_iter()
        .filter_map(|(key, group)| {
            if group.removed_original_action_ids.is_empty() {
                None
            } else {
                Some(ActionEquivalenceGroupSummary {
                    key,
                    representative_original_action_id: group.representative_original_action_id,
                    removed_original_action_ids: group.removed_original_action_ids,
                })
            }
        })
        .collect::<Vec<_>>();

    ActionEquivalenceResult {
        summary: ActionEquivalenceSummary {
            atomic_actions_in,
            representative_actions_out: representatives.len(),
            groups,
        },
        choices: representatives,
    }
}

fn equivalence_key_for_choice(
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
        ClientInput::SubmitGridSelect(uuids) => {
            pending_single_card_selection_key(engine, combat, uuids)
        }
        ClientInput::SubmitHandSelect(uuids) => {
            pending_single_card_selection_key(engine, combat, uuids)
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

impl ActionEquivalenceSummary {
    fn actions_removed(&self) -> usize {
        self.atomic_actions_in
            .saturating_sub(self.representative_actions_out)
    }
}

impl ActionEquivalenceGroupSummary {
    fn group_size(&self) -> usize {
        self.removed_original_action_ids.len().saturating_add(1)
    }
}

#[cfg(test)]
mod tests;
