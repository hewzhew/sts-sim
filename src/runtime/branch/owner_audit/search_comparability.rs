use sts_simulator::ai::strategy::trajectory_comparison::{
    TrajectorySearchComparability, TrajectorySearchComparabilityStatus,
};
use sts_simulator::eval::run_control::{CombatLineAdjudicationV1, CombatSearchTraceSummary};

#[derive(Clone, Copy)]
enum AttemptComparability {
    ExactAccepted,
    NodeBounded,
    Exhaustive,
    WallLimited,
    Insufficient,
}

pub(super) fn classify_search_comparability(
    attempts: &[CombatSearchTraceSummary],
) -> TrajectorySearchComparability {
    let mut result = TrajectorySearchComparability::comparable_without_attempts();
    for attempt in attempts {
        result.total_attempts = result.total_attempts.saturating_add(1);
        match classify_attempt(attempt) {
            AttemptComparability::ExactAccepted => {
                result.exact_accepted_attempts = result.exact_accepted_attempts.saturating_add(1);
            }
            AttemptComparability::NodeBounded => {
                result.node_bounded_attempts = result.node_bounded_attempts.saturating_add(1);
            }
            AttemptComparability::Exhaustive => {
                result.exhaustive_attempts = result.exhaustive_attempts.saturating_add(1);
            }
            AttemptComparability::WallLimited => {
                result.wall_limited_attempts = result.wall_limited_attempts.saturating_add(1);
            }
            AttemptComparability::Insufficient => {
                result.insufficient_attempts = result.insufficient_attempts.saturating_add(1);
            }
        }
    }
    result.status = if result.wall_limited_attempts > 0 {
        TrajectorySearchComparabilityStatus::WallSafetyLimited
    } else if result.insufficient_attempts > 0 {
        TrajectorySearchComparabilityStatus::InsufficientEvidence
    } else {
        TrajectorySearchComparabilityStatus::Comparable
    };
    result
}

fn classify_attempt(attempt: &CombatSearchTraceSummary) -> AttemptComparability {
    if matches!(
        attempt.execution_adjudication.as_ref(),
        Some(CombatLineAdjudicationV1::Accepted { .. })
    ) {
        return AttemptComparability::ExactAccepted;
    }
    if attempt.deadline_hit || coverage_is(&attempt.coverage_status, "timebudgetlimited") {
        return AttemptComparability::WallLimited;
    }
    if matches!(
        attempt.execution_adjudication.as_ref(),
        Some(CombatLineAdjudicationV1::ReplayFailed { .. })
    ) {
        return AttemptComparability::Insufficient;
    }
    let unadjudicated_candidate = attempt.execution_adjudication.is_none()
        && (attempt.complete_win_found
            || attempt.best_win.is_some()
            || coverage_is(&attempt.coverage_status, "acceptedcompletecandidate"));
    if unadjudicated_candidate {
        return AttemptComparability::Insufficient;
    }
    if attempt.node_budget_hit || coverage_is(&attempt.coverage_status, "nodebudgetlimited") {
        return AttemptComparability::NodeBounded;
    }
    if coverage_is(&attempt.coverage_status, "exhaustive") {
        return AttemptComparability::Exhaustive;
    }
    AttemptComparability::Insufficient
}

fn coverage_is(actual: &str, expected: &str) -> bool {
    actual
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .eq(expected.chars())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId;
    use sts_simulator::ai::strategy::trajectory_comparison::{
        TrajectorySearchComparability, TrajectorySearchComparabilityStatus,
    };
    use sts_simulator::eval::run_control::{
        CombatLineAdjudicationV1, CombatLineCleanlinessV1, CombatLineObservedOutcomeV1,
        CombatLineRejectionReasonV1, CombatSearchTraceSummary,
    };
    use sts_simulator::sim::combat::CombatTerminal;

    fn attempt(coverage: &str) -> CombatSearchTraceSummary {
        CombatSearchTraceSummary {
            source: "test".to_string(),
            coverage_status: coverage.to_string(),
            ..CombatSearchTraceSummary::default()
        }
    }

    fn observed_outcome() -> CombatLineObservedOutcomeV1 {
        CombatLineObservedOutcomeV1 {
            terminal: CombatTerminal::Win,
            final_hp: 40,
            hp_loss: 10,
            potions_used: 0,
            action_count: 8,
            gold_delta: 0,
            ritual_dagger_growth: 0,
            gained_curses: Vec::new(),
        }
    }

    fn accepted() -> CombatLineAdjudicationV1 {
        CombatLineAdjudicationV1::Accepted {
            policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            cleanliness: CombatLineCleanlinessV1::Clean,
            observed_outcome: observed_outcome(),
        }
    }

    fn rejected() -> CombatLineAdjudicationV1 {
        CombatLineAdjudicationV1::Rejected {
            policy: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            reason: CombatLineRejectionReasonV1::NewCurse { cards: Vec::new() },
            observed_outcome: observed_outcome(),
        }
    }

    #[test]
    fn exact_accepted_attempt_remains_comparable_after_safety_deadline() {
        let mut item = attempt("TimeBudgetLimited");
        item.deadline_hit = true;
        item.execution_adjudication = Some(accepted());

        let result = classify_search_comparability(&[item]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::Comparable
        );
        assert_eq!(result.exact_accepted_attempts, 1);
        assert_eq!(result.wall_limited_attempts, 0);
    }

    #[test]
    fn wall_limited_primary_is_not_erased_by_accepted_rescue() {
        let mut primary = attempt("TimeBudgetLimited");
        primary.deadline_hit = true;
        let mut rescue = attempt("AcceptedCompleteCandidate");
        rescue.execution_adjudication = Some(accepted());

        let result = classify_search_comparability(&[primary, rescue]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::WallSafetyLimited
        );
        assert_eq!(result.wall_limited_attempts, 1);
        assert_eq!(result.exact_accepted_attempts, 1);
    }

    #[test]
    fn node_bounded_exact_rejection_is_comparable() {
        let mut item = attempt("NodeBudgetLimited");
        item.node_budget_hit = true;
        item.execution_adjudication = Some(rejected());

        let result = classify_search_comparability(&[item]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::Comparable
        );
        assert_eq!(result.node_bounded_attempts, 1);
    }

    #[test]
    fn exhaustive_exact_rejection_is_comparable() {
        let mut item = attempt("exhaustive");
        item.execution_adjudication = Some(rejected());

        let result = classify_search_comparability(&[item]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::Comparable
        );
        assert_eq!(result.exhaustive_attempts, 1);
    }

    #[test]
    fn unadjudicated_winning_candidate_is_insufficient() {
        let mut item = attempt("AcceptedCompleteCandidate");
        item.complete_win_found = true;

        let result = classify_search_comparability(&[item]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::InsufficientEvidence
        );
        assert_eq!(result.insufficient_attempts, 1);
    }

    #[test]
    fn replay_failure_and_unknown_coverage_are_insufficient() {
        let mut replay_failed = attempt("AcceptedCompleteCandidate");
        replay_failed.execution_adjudication = Some(CombatLineAdjudicationV1::ReplayFailed {
            policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            error: "replay drift".to_string(),
        });

        let result =
            classify_search_comparability(&[replay_failed, attempt("FutureCoverageVocabulary")]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::InsufficientEvidence
        );
        assert_eq!(result.insufficient_attempts, 2);
    }

    #[test]
    fn no_search_attempts_is_comparable() {
        assert_eq!(
            classify_search_comparability(&[]),
            TrajectorySearchComparability::comparable_without_attempts()
        );
    }
}
