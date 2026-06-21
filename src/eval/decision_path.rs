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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionPathStepV1 {
    Command {
        command: String,
    },
    DecisionParentCoordinate {
        raw: String,
        depth: Option<usize>,
        decision_kind: Option<String>,
        coordinate_id: Option<String>,
    },
    RouteDecisionParentCoordinate {
        raw: String,
        ordinal: Option<usize>,
        command_key: Option<String>,
    },
    ReplayAdvanceCoordinate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionPathEnvelopeV1 {
    steps: Vec<DecisionPathStepV1>,
}

impl DecisionPathEnvelopeV1 {
    pub fn from_commands(commands: &[String]) -> Self {
        Self {
            steps: parse_decision_path_steps_v1(commands),
        }
    }

    pub fn from_steps(steps: Vec<DecisionPathStepV1>) -> Self {
        Self { steps }
    }

    pub fn steps(&self) -> &[DecisionPathStepV1] {
        &self.steps
    }

    pub fn to_commands(&self) -> Vec<String> {
        decision_path_steps_to_commands_v1(&self.steps)
    }

    pub fn executable_commands(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter_map(|step| match step {
                DecisionPathStepV1::Command { command } => Some(command.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn contains_decision_parent_coordinate(&self) -> bool {
        self.steps.iter().any(|step| {
            matches!(
                step.kind(),
                DecisionPathCommandKindV1::DecisionParentCoordinate
                    | DecisionPathCommandKindV1::RouteDecisionParentCoordinate
            )
        })
    }

    pub fn contains_route_parent_coordinate(&self) -> bool {
        self.steps.iter().any(|step| {
            matches!(
                step.kind(),
                DecisionPathCommandKindV1::RouteDecisionParentCoordinate
            )
        })
    }

    pub fn journal_parent_depth_against(&self, branch: &DecisionPathEnvelopeV1) -> Option<usize> {
        let commands = self.to_commands();
        let branch_commands = branch.to_commands();
        decision_path_journal_parent_command_depth_v1(&commands, &branch_commands)
    }
}

impl DecisionPathStepV1 {
    pub fn kind(&self) -> DecisionPathCommandKindV1 {
        match self {
            Self::Command { .. } => DecisionPathCommandKindV1::Executable,
            Self::DecisionParentCoordinate { .. } => {
                DecisionPathCommandKindV1::DecisionParentCoordinate
            }
            Self::RouteDecisionParentCoordinate { .. } => {
                DecisionPathCommandKindV1::RouteDecisionParentCoordinate
            }
            Self::ReplayAdvanceCoordinate => DecisionPathCommandKindV1::ReplayAdvanceCoordinate,
        }
    }

    pub fn is_coordinate(&self) -> bool {
        !matches!(self.kind(), DecisionPathCommandKindV1::Executable)
    }
}

pub fn classify_decision_path_command_v1(command: &str) -> DecisionPathCommandKindV1 {
    parse_decision_path_step_v1(command).kind()
}

pub fn parse_decision_path_steps_v1(commands: &[String]) -> Vec<DecisionPathStepV1> {
    commands
        .iter()
        .map(|command| parse_decision_path_step_v1(command))
        .collect()
}

pub fn parse_decision_path_step_v1(command: &str) -> DecisionPathStepV1 {
    if command == DECISION_PATH_REPLAY_ADVANCE_COMMAND_V1 {
        return DecisionPathStepV1::ReplayAdvanceCoordinate;
    }
    if let Some(rest) = command.strip_prefix(DECISION_PATH_PARENT_COMMAND_PREFIX_V1) {
        return parse_decision_parent_coordinate_v1(command, rest);
    }
    if let Some(rest) = command.strip_prefix(DECISION_PATH_ROUTE_PARENT_COMMAND_PREFIX_V1) {
        return parse_route_parent_coordinate_v1(command, rest);
    }
    DecisionPathStepV1::Command {
        command: command.to_string(),
    }
}

pub fn decision_path_steps_to_commands_v1(steps: &[DecisionPathStepV1]) -> Vec<String> {
    steps.iter().map(decision_path_step_to_command_v1).collect()
}

pub fn decision_path_step_to_command_v1(step: &DecisionPathStepV1) -> String {
    match step {
        DecisionPathStepV1::Command { command } => command.clone(),
        DecisionPathStepV1::DecisionParentCoordinate { raw, .. } => raw.clone(),
        DecisionPathStepV1::RouteDecisionParentCoordinate { raw, .. } => raw.clone(),
        DecisionPathStepV1::ReplayAdvanceCoordinate => {
            DECISION_PATH_REPLAY_ADVANCE_COMMAND_V1.to_string()
        }
    }
}

fn parse_decision_parent_coordinate_v1(command: &str, rest: &str) -> DecisionPathStepV1 {
    let mut parts = rest.split(':').collect::<Vec<_>>();
    let depth = parts.first().and_then(|part| part.parse::<usize>().ok());
    if depth.is_some() {
        parts.remove(0);
    }
    let decision_kind = (!parts.is_empty()).then(|| parts.remove(0).to_string());
    let coordinate_id = (!parts.is_empty()).then(|| parts.join(":"));
    DecisionPathStepV1::DecisionParentCoordinate {
        raw: command.to_string(),
        depth,
        decision_kind,
        coordinate_id,
    }
}

fn parse_route_parent_coordinate_v1(command: &str, rest: &str) -> DecisionPathStepV1 {
    let mut parts = rest.split(':').collect::<Vec<_>>();
    let ordinal = parts.first().and_then(|part| part.parse::<usize>().ok());
    if ordinal.is_some() {
        parts.remove(0);
    }
    let command_key = (!parts.is_empty()).then(|| parts.join(":"));
    DecisionPathStepV1::RouteDecisionParentCoordinate {
        raw: command.to_string(),
        ordinal,
        command_key,
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
    DecisionPathEnvelopeV1::from_commands(commands).contains_decision_parent_coordinate()
}

pub fn decision_path_commands_include_route_parent_coordinate_v1(commands: &[String]) -> bool {
    DecisionPathEnvelopeV1::from_commands(commands).contains_route_parent_coordinate()
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

    #[test]
    fn typed_steps_preserve_legacy_command_strings_and_parse_marker_fields() {
        let commands = vec![
            "rp 1".to_string(),
            "__decision_parent:1:reward:abcd".to_string(),
            "__route_decision:0:go_1".to_string(),
            "__branch_experiment_replay_advance".to_string(),
        ];

        let steps = parse_decision_path_steps_v1(&commands);

        assert_eq!(
            steps,
            vec![
                DecisionPathStepV1::Command {
                    command: "rp 1".to_string()
                },
                DecisionPathStepV1::DecisionParentCoordinate {
                    raw: "__decision_parent:1:reward:abcd".to_string(),
                    depth: Some(1),
                    decision_kind: Some("reward".to_string()),
                    coordinate_id: Some("abcd".to_string()),
                },
                DecisionPathStepV1::RouteDecisionParentCoordinate {
                    raw: "__route_decision:0:go_1".to_string(),
                    ordinal: Some(0),
                    command_key: Some("go_1".to_string()),
                },
                DecisionPathStepV1::ReplayAdvanceCoordinate,
            ]
        );
        assert_eq!(decision_path_steps_to_commands_v1(&steps), commands);
    }

    #[test]
    fn typed_decision_parent_parser_accepts_old_non_numeric_marker_shape() {
        let step = parse_decision_path_step_v1("__decision_parent:card_reward:floor=3:index=0");

        assert_eq!(
            step,
            DecisionPathStepV1::DecisionParentCoordinate {
                raw: "__decision_parent:card_reward:floor=3:index=0".to_string(),
                depth: None,
                decision_kind: Some("card_reward".to_string()),
                coordinate_id: Some("floor=3:index=0".to_string()),
            }
        );
    }

    #[test]
    fn path_envelope_exposes_typed_steps_without_losing_legacy_commands() {
        let commands = vec![
            "rp 1".to_string(),
            "__decision_parent:1:reward:abcd".to_string(),
            "event 0".to_string(),
        ];
        let envelope = DecisionPathEnvelopeV1::from_commands(&commands);
        let parent_envelope = DecisionPathEnvelopeV1::from_commands(&[
            "rp 1".to_string(),
            "__decision_parent:1:reward:abcd".to_string(),
        ]);
        let branch =
            DecisionPathEnvelopeV1::from_commands(&["rp 1".to_string(), "event 0".to_string()]);

        assert_eq!(envelope.to_commands(), commands);
        assert_eq!(envelope.executable_commands(), vec!["rp 1", "event 0"]);
        assert!(envelope.contains_decision_parent_coordinate());
        assert_eq!(
            parent_envelope.journal_parent_depth_against(&branch),
            Some(1)
        );
    }
}
