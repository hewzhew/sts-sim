//! Shared helpers for campaign decision paths.
//!
//! Branch campaign paths still store both executable game commands and a small
//! amount of replay/journal metadata in the same string vector.  This module is
//! the one place that knows how to classify those coordinate markers.  Callers
//! should not parse `__decision_parent:` or `__route_decision:` directly.

pub const DECISION_PATH_REPLAY_ADVANCE_COMMAND_V1: &str = "__branch_experiment_replay_advance";
pub const DECISION_PATH_PARENT_COMMAND_PREFIX_V1: &str = "__decision_parent:";
pub const DECISION_PATH_ROUTE_PARENT_COMMAND_PREFIX_V1: &str = "__route_decision:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionPathCommandKindV1 {
    Executable,
    DecisionParentCoordinate,
    RouteDecisionParentCoordinate,
    ReplayAdvanceCoordinate,
}

pub fn classify_decision_path_command_v1(command: &str) -> DecisionPathCommandKindV1 {
    if command == DECISION_PATH_REPLAY_ADVANCE_COMMAND_V1 {
        DecisionPathCommandKindV1::ReplayAdvanceCoordinate
    } else if command.starts_with(DECISION_PATH_PARENT_COMMAND_PREFIX_V1) {
        DecisionPathCommandKindV1::DecisionParentCoordinate
    } else if command.starts_with(DECISION_PATH_ROUTE_PARENT_COMMAND_PREFIX_V1) {
        DecisionPathCommandKindV1::RouteDecisionParentCoordinate
    } else {
        DecisionPathCommandKindV1::Executable
    }
}

pub fn decision_path_command_is_coordinate_v1(command: &str) -> bool {
    !matches!(
        classify_decision_path_command_v1(command),
        DecisionPathCommandKindV1::Executable
    )
}

pub fn decision_path_command_is_decision_parent_coordinate_v1(command: &str) -> bool {
    matches!(
        classify_decision_path_command_v1(command),
        DecisionPathCommandKindV1::DecisionParentCoordinate
            | DecisionPathCommandKindV1::RouteDecisionParentCoordinate
    )
}

pub fn decision_path_command_is_route_parent_coordinate_v1(command: &str) -> bool {
    matches!(
        classify_decision_path_command_v1(command),
        DecisionPathCommandKindV1::RouteDecisionParentCoordinate
    )
}

pub fn decision_path_commands_include_decision_parent_coordinate_v1(commands: &[String]) -> bool {
    commands
        .iter()
        .any(|command| decision_path_command_is_decision_parent_coordinate_v1(command))
}

pub fn decision_path_commands_include_route_parent_coordinate_v1(commands: &[String]) -> bool {
    commands
        .iter()
        .any(|command| decision_path_command_is_route_parent_coordinate_v1(command))
}

pub fn decision_path_command_prefix_matches_v1(prefix: &[String], commands: &[String]) -> bool {
    prefix.len() <= commands.len()
        && prefix
            .iter()
            .zip(commands.iter())
            .all(|(left, right)| left == right)
}

pub fn decision_path_journal_parent_command_depth_v1(
    journal_parent_commands: &[String],
    branch_commands: &[String],
) -> Option<usize> {
    if journal_parent_commands.last().is_some_and(|command| {
        classify_decision_path_command_v1(command)
            == DecisionPathCommandKindV1::DecisionParentCoordinate
    }) {
        let parent_commands = &journal_parent_commands[..journal_parent_commands.len() - 1];
        return decision_path_command_prefix_matches_v1(parent_commands, branch_commands)
            .then_some(parent_commands.len());
    }
    decision_path_command_prefix_matches_v1(journal_parent_commands, branch_commands)
        .then_some(journal_parent_commands.len())
}

pub fn decision_path_executable_commands_v1(commands: &[String]) -> Vec<&str> {
    commands
        .iter()
        .filter(|command| {
            matches!(
                classify_decision_path_command_v1(command),
                DecisionPathCommandKindV1::Executable
            )
        })
        .map(|command| command.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_classification_distinguishes_metadata_from_executable_commands() {
        assert_eq!(
            classify_decision_path_command_v1("rp 1"),
            DecisionPathCommandKindV1::Executable
        );
        assert_eq!(
            classify_decision_path_command_v1("__decision_parent:1:reward:abcd"),
            DecisionPathCommandKindV1::DecisionParentCoordinate
        );
        assert_eq!(
            classify_decision_path_command_v1("__route_decision:0:go_1"),
            DecisionPathCommandKindV1::RouteDecisionParentCoordinate
        );
        assert_eq!(
            classify_decision_path_command_v1("__branch_experiment_replay_advance"),
            DecisionPathCommandKindV1::ReplayAdvanceCoordinate
        );
    }

    #[test]
    fn journal_parent_depth_ignores_decision_parent_marker_but_not_route_coordinate() {
        let branch_commands = vec![
            "rp 1".to_string(),
            "__route_decision:0:go_1".to_string(),
            "go 1".to_string(),
        ];
        let reward_parent = vec![
            "rp 1".to_string(),
            "__decision_parent:1:reward:abcd".to_string(),
        ];
        let route_parent = vec!["rp 1".to_string(), "__route_decision:0:go_1".to_string()];

        assert_eq!(
            decision_path_journal_parent_command_depth_v1(&reward_parent, &branch_commands),
            Some(1)
        );
        assert_eq!(
            decision_path_journal_parent_command_depth_v1(&route_parent, &branch_commands),
            Some(2)
        );
    }
}
