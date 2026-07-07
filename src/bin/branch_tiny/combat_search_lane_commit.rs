use sts_simulator::eval::run_control::RunControlAutoStopKind;

use super::combat_search_lanes::CombatSearchLaneCommitPolicy;
use super::{BranchStatus, TerminalOutcome};

pub(super) fn lane_commits(
    policy: CombatSearchLaneCommitPolicy,
    status: &BranchStatus,
    stop_kind: Option<RunControlAutoStopKind>,
) -> bool {
    lane_accepted(status)
        || matches!(
            policy,
            CombatSearchLaneCommitPolicy::AcceptedLineOrPrimaryChunk
        ) && primary_operation_budget_exhausted(status, stop_kind)
}

pub(super) fn primary_operation_budget_exhausted(
    status: &BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
) -> bool {
    primary_stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
        || matches!(status, BranchStatus::OperationBudgetExhausted { .. })
}

fn lane_accepted(status: &BranchStatus) -> bool {
    !matches!(
        status,
        BranchStatus::CombatGap { .. }
            | BranchStatus::OperationBudgetExhausted { .. }
            | BranchStatus::BudgetGap { .. }
            | BranchStatus::ApplyFailed(_)
            | BranchStatus::AdvanceFailed(_)
            | BranchStatus::Terminal(TerminalOutcome::Defeat)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_policy_commits_operation_budget_chunk() {
        let status = BranchStatus::OperationBudgetExhausted {
            boundary: "Combat".to_string(),
            reason: "operation budget exhausted".to_string(),
        };

        assert!(lane_commits(
            CombatSearchLaneCommitPolicy::AcceptedLineOrPrimaryChunk,
            &status,
            Some(RunControlAutoStopKind::OperationBudgetExhausted)
        ));
        assert!(primary_operation_budget_exhausted(
            &status,
            Some(RunControlAutoStopKind::OperationBudgetExhausted)
        ));
    }

    #[test]
    fn accepted_line_only_policy_rejects_combat_gap() {
        let status = BranchStatus::CombatGap {
            boundary: "Combat".to_string(),
            reason: "no complete win".to_string(),
        };

        assert!(!lane_commits(
            CombatSearchLaneCommitPolicy::AcceptedLineOnly,
            &status,
            Some(RunControlAutoStopKind::CombatSearchNoCompleteWin)
        ));
    }

    #[test]
    fn accepted_line_only_policy_commits_terminal_victory() {
        let status = BranchStatus::Terminal(TerminalOutcome::Victory);

        assert!(lane_commits(
            CombatSearchLaneCommitPolicy::AcceptedLineOnly,
            &status,
            None
        ));
    }
}
