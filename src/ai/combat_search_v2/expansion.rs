use super::*;
use std::collections::BTreeMap;

mod reporting;
mod types;

pub(super) use types::{ActionExpansionDiagnosticsCollector, ActionExpansionSummary};
use types::{
    ActionExpansionGroupKey, ActionExpansionGroupObservation, ActionExpansionGroupSummary,
    ActionExpansionKind,
};

impl ActionExpansionDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &ActionExpansionSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.total_atomic_actions = self
            .total_atomic_actions
            .saturating_add(summary.action_count as u64);
        self.total_fanout_groups = self
            .total_fanout_groups
            .saturating_add(summary.group_count as u64);
        self.fanout_groups_max = self.fanout_groups_max.max(summary.group_count);

        for group in &summary.groups {
            self.max_group_size = self.max_group_size.max(group.action_count);
            let count = self.kind_counts.entry(group.key.kind).or_default();
            count.atomic_actions = count
                .atomic_actions
                .saturating_add(group.action_count as u64);
            count.fanout_groups = count.fanout_groups.saturating_add(1);
            count.max_group_size = count.max_group_size.max(group.action_count);
            self.remember_largest_group(ActionExpansionGroupObservation {
                observed_at_state_query: self.states_observed,
                key: group.key.clone(),
                action_count: group.action_count,
            });
        }
    }
}

pub(super) fn summarize_action_expansion(
    engine: &EngineState,
    combat: &CombatState,
    choices: &[CombatActionChoice],
) -> ActionExpansionSummary {
    let mut groups: BTreeMap<ActionExpansionGroupKey, usize> = BTreeMap::new();
    for choice in choices {
        let key = group_key_for_input(engine, combat, &choice.input);
        *groups.entry(key).or_insert(0) += 1;
    }

    let groups = groups
        .into_iter()
        .map(|(key, action_count)| ActionExpansionGroupSummary { key, action_count })
        .collect::<Vec<_>>();

    ActionExpansionSummary {
        action_count: choices.len(),
        group_count: groups.len(),
        groups,
    }
}

fn group_key_for_input(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> ActionExpansionGroupKey {
    match input {
        ClientInput::PlayCard { card_index, .. } => ActionExpansionGroupKey {
            kind: ActionExpansionKind::PlayCard,
            signature: combat
                .zones
                .hand
                .get(*card_index)
                .map(|card| {
                    format!(
                        "play_card/hand:{card_index}/card:{}+{}/uuid:{}/cost:{}",
                        crate::content::cards::java_id(card.id),
                        card.upgrades,
                        card.uuid,
                        card.cost_for_turn_java()
                    )
                })
                .unwrap_or_else(|| format!("play_card/hand:{card_index}/missing_card")),
        },
        ClientInput::EndTurn => ActionExpansionGroupKey {
            kind: ActionExpansionKind::EndTurn,
            signature: "end_turn".to_string(),
        },
        ClientInput::UsePotion { potion_index, .. } => ActionExpansionGroupKey {
            kind: ActionExpansionKind::UsePotion,
            signature: combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(Option::as_ref)
                .map(|potion| {
                    format!(
                        "use_potion/slot:{potion_index}/potion:{:?}/uuid:{}",
                        potion.id, potion.uuid
                    )
                })
                .unwrap_or_else(|| format!("use_potion/slot:{potion_index}/missing_potion")),
        },
        ClientInput::DiscardPotion(slot) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::DiscardPotion,
            signature: format!("discard_potion/slot:{slot}"),
        },
        ClientInput::SubmitDiscoverChoice(_) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::DiscoverChoice,
            signature: format!("discover_choice/{}", pending_choice_label(engine)),
        },
        ClientInput::SubmitHandSelect(uuids) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::HandSelect,
            signature: format!(
                "hand_select/{}/selected:{}",
                pending_choice_label(engine),
                uuids.len()
            ),
        },
        ClientInput::SubmitGridSelect(uuids) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::GridSelect,
            signature: format!(
                "grid_select/{}/selected:{}",
                pending_choice_label(engine),
                uuids.len()
            ),
        },
        ClientInput::SubmitScryDiscard(indices) => ActionExpansionGroupKey {
            kind: ActionExpansionKind::ScryDiscard,
            signature: format!("scry_discard/selected:{}", indices.len()),
        },
        ClientInput::Cancel => ActionExpansionGroupKey {
            kind: ActionExpansionKind::Cancel,
            signature: format!("cancel/{}", pending_choice_label(engine)),
        },
        ClientInput::Proceed => ActionExpansionGroupKey {
            kind: ActionExpansionKind::Proceed,
            signature: format!("proceed/{}", engine_state_label(engine)),
        },
        _ => ActionExpansionGroupKey {
            kind: ActionExpansionKind::Other,
            signature: format!("{input:?}"),
        },
    }
}

fn pending_choice_label(engine: &EngineState) -> &'static str {
    match engine {
        EngineState::PendingChoice(choice) => match choice {
            crate::state::core::PendingChoice::HandSelect { .. } => "pending_hand_select",
            crate::state::core::PendingChoice::GridSelect { .. } => "pending_grid_select",
            crate::state::core::PendingChoice::DiscoverySelect(_) => "pending_discovery_select",
            crate::state::core::PendingChoice::CardRewardSelect { .. } => {
                "pending_card_reward_select"
            }
            crate::state::core::PendingChoice::ForeignInfluenceSelect { .. } => {
                "pending_foreign_influence_select"
            }
            crate::state::core::PendingChoice::ChooseOneSelect { .. } => {
                "pending_choose_one_select"
            }
            crate::state::core::PendingChoice::StanceChoice => "pending_stance_choice",
            _ => "pending_other_choice",
        },
        _ => "not_pending_choice",
    }
}

fn engine_state_label(engine: &EngineState) -> &'static str {
    match engine {
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::CombatStart(_) => "combat_start",
        EngineState::PendingChoice(_) => "pending_choice",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::TreasureRoom(_) => "treasure_room",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map_navigation",
        EngineState::EventRoom => "event_room",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}

#[cfg(test)]
mod tests;
