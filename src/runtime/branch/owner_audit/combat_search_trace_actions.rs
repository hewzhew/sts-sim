use sts_simulator::eval::run_control::{
    CombatAutomationTrajectorySource, RunControlTraceAnnotationV1,
};

pub(super) fn complete_search_action_keys(
    annotations: &[RunControlTraceAnnotationV1],
) -> Vec<String> {
    annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source, actions, ..
            } if is_complete_search_source(*source) => Some(
                actions
                    .iter()
                    .map(|action| action.action_key.clone())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn is_complete_search_source(source: CombatAutomationTrajectorySource) -> bool {
    matches!(
        source,
        CombatAutomationTrajectorySource::SearchCombat
            | CombatAutomationTrajectorySource::V2Donor
            | CombatAutomationTrajectorySource::CompleteLineSolver
            | CombatAutomationTrajectorySource::TurnPlanRescue
            | CombatAutomationTrajectorySource::TurnPoolRescue
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::{
        CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1,
    };
    use sts_simulator::state::core::ClientInput;

    #[test]
    fn complete_search_action_keys_accepts_search_sources() {
        let annotations = vec![
            trajectory(
                CombatAutomationTrajectorySource::SearchCombatTurnSegment,
                "ignore",
            ),
            trajectory(CombatAutomationTrajectorySource::TurnPoolRescue, "keep"),
        ];

        assert_eq!(complete_search_action_keys(&annotations), vec!["keep"]);
    }

    #[test]
    fn complete_search_action_keys_returns_empty_without_complete_source() {
        let annotations = vec![trajectory(
            CombatAutomationTrajectorySource::SearchCombatSmokeBombSurvival,
            "ignore",
        )];

        assert!(complete_search_action_keys(&annotations).is_empty());
    }

    fn trajectory(
        source: CombatAutomationTrajectorySource,
        action_key: &'static str,
    ) -> RunControlTraceAnnotationV1 {
        CombatAutomationTrajectoryRecordV1::new(
            source,
            vec![CombatAutomationActionV1 {
                step_index: 0,
                action_key: action_key.to_string(),
                input: ClientInput::EndTurn,
                opportunity_before: None,
                drawn_cards: Vec::new(),
                combat_after: None,
            }],
        )
        .into_annotation()
    }
}
