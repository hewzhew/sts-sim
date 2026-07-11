use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    CombatAutomationTrajectoryRecordV1, CombatSearchTerminalLineSummary,
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedCombatAttritionV1 {
    pub(super) start_hp: i32,
    pub(super) lowest_observed_hp: i32,
    pub(super) observed_combat_drawdown: i32,
    pub(super) terminal_hp: i32,
    pub(super) terminal_rebound_from_observed_low: i32,
    pub(super) persistent_net_hp_loss: i32,
    pub(super) observation_complete: bool,
}

pub(super) fn accepted_combat_attrition_v1(
    start_hp: i32,
    selected: &CombatSearchTerminalLineSummary,
    trajectory: &CombatAutomationTrajectoryRecordV1,
) -> AcceptedCombatAttritionV1 {
    let lowest_observed_hp = trajectory
        .actions
        .iter()
        .filter_map(|action| action.combat_after.as_ref().map(|state| state.player_hp))
        .fold(start_hp, i32::min);
    AcceptedCombatAttritionV1 {
        start_hp,
        lowest_observed_hp,
        observed_combat_drawdown: start_hp.saturating_sub(lowest_observed_hp).max(0),
        terminal_hp: selected.final_hp,
        terminal_rebound_from_observed_low: selected
            .final_hp
            .saturating_sub(lowest_observed_hp)
            .max(0),
        persistent_net_hp_loss: start_hp.saturating_sub(selected.final_hp).max(0),
        observation_complete: trajectory
            .actions
            .iter()
            .all(|action| action.combat_after.is_some()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
    use sts_simulator::eval::run_control::{
        CombatAutomationActionV1, CombatAutomationStepStateV1,
        CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource,
        CombatSearchTerminalLineSummary,
    };
    use sts_simulator::state::core::ClientInput;

    fn terminal_win(final_hp: i32, hp_loss: i32) -> CombatSearchTerminalLineSummary {
        CombatSearchTerminalLineSummary {
            terminal: SearchTerminalLabel::Win,
            final_hp,
            hp_loss,
            turns: 3,
            cards_played: 6,
            potions_used: 0,
            potions_discarded: 0,
            action_count: 4,
        }
    }

    fn trajectory(observed_hp: &[Option<i32>]) -> CombatAutomationTrajectoryRecordV1 {
        CombatAutomationTrajectoryRecordV1::new(
            CombatAutomationTrajectorySource::SearchCombat,
            observed_hp
                .iter()
                .enumerate()
                .map(|(step_index, hp)| CombatAutomationActionV1 {
                    step_index,
                    action_key: format!("combat/test/{step_index}"),
                    input: ClientInput::EndTurn,
                    drawn_cards: Vec::new(),
                    combat_after: hp.map(|player_hp| CombatAutomationStepStateV1 {
                        player_hp,
                        player_max_hp: 74,
                        player_block: 0,
                        energy: 0,
                        cards_played_this_turn: 0,
                        early_end_turn_pending: false,
                        monsters: Vec::new(),
                    }),
                })
                .collect(),
        )
    }

    #[test]
    fn attrition_separates_observed_drawdown_rebound_and_persistent_loss() {
        let selected = terminal_win(20, 24);
        let trajectory = trajectory(&[Some(44), Some(23), Some(8), None]);

        assert_eq!(
            accepted_combat_attrition_v1(44, &selected, &trajectory),
            AcceptedCombatAttritionV1 {
                start_hp: 44,
                lowest_observed_hp: 8,
                observed_combat_drawdown: 36,
                terminal_hp: 20,
                terminal_rebound_from_observed_low: 12,
                persistent_net_hp_loss: 24,
                observation_complete: false,
            }
        );
    }

    #[test]
    fn attrition_without_healing_has_equal_observed_and_net_loss() {
        let selected = terminal_win(61, 13);
        let trajectory = trajectory(&[Some(70), Some(61)]);
        let attrition = accepted_combat_attrition_v1(74, &selected, &trajectory);

        assert_eq!(attrition.observed_combat_drawdown, 13);
        assert_eq!(attrition.terminal_rebound_from_observed_low, 0);
        assert_eq!(attrition.persistent_net_hp_loss, 13);
        assert!(attrition.observation_complete);
    }
}
