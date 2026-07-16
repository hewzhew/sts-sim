use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use sts_simulator::ai::strategy::challenger_policy_state::CommitmentStatus;
use sts_simulator::ai::strategy::challenger_signature::DeckBurdenBand;
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::deck_strategic_deficit::StrategicBurdenLevel;
use sts_simulator::ai::strategy::trajectory_comparison::{
    compare_trajectories, TrajectoryComparison, TrajectoryConstruction,
    TrajectoryDeployabilityEvidence, TrajectoryPressureEvidence, TrajectoryProgress,
    TrajectoryResources, TrajectorySnapshot, TrajectoryTerminal,
};

use super::branch_model::{Branch, BranchStatus, TerminalOutcome};
use super::search_comparability::classify_search_comparability;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct FrontierTrajectoryEvaluation {
    pub(super) snapshots: Vec<TrajectorySnapshot>,
    pub(super) comparisons: Vec<TrajectoryComparison>,
}

pub(super) fn trajectory_snapshot(branch: &Branch) -> TrajectorySnapshot {
    let run = &branch.session.run_state;
    let plan = DeckPlanSnapshot::from_run_state(run);
    let burden = match plan.strategic_deficit.deck_burden {
        StrategicBurdenLevel::Clean => DeckBurdenBand::Clean,
        StrategicBurdenLevel::Watch => DeckBurdenBand::Watch,
        StrategicBurdenLevel::Heavy => DeckBurdenBand::Heavy,
    };
    let (completed_commitments, active_commitments, failed_commitments) = branch
        .policy_lane
        .challenger_policy()
        .map(|policy| {
            policy.commitments.iter().fold(
                (0_u16, 0_u16, 0_u16),
                |(completed, active, failed), commitment| match commitment.status {
                    CommitmentStatus::Completed => (completed.saturating_add(1), active, failed),
                    CommitmentStatus::Active => (completed, active.saturating_add(1), failed),
                    CommitmentStatus::Abandoned | CommitmentStatus::Expired => {
                        (completed, active, failed.saturating_add(1))
                    }
                },
            )
        })
        .unwrap_or((0, 0, 0));

    let comparison_search_start = branch
        .comparison_search_start
        .unwrap_or(0)
        .min(branch.combat_search_history.len());
    TrajectorySnapshot {
        lane: branch.policy_lane.label(),
        terminal: terminal_state(&branch.status),
        progress: TrajectoryProgress {
            act: run.act_num,
            floor: run.floor_num,
        },
        pressure: TrajectoryPressureEvidence::Unknown,
        deployability: TrajectoryDeployabilityEvidence::Unknown,
        resources: TrajectoryResources {
            hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            potion_count: run
                .potions
                .iter()
                .filter(|potion| potion.is_some())
                .count()
                .min(u8::MAX as usize) as u8,
        },
        construction: TrajectoryConstruction {
            burden,
            completed_commitments,
            active_commitments,
            failed_commitments,
        },
        search_comparability: classify_search_comparability(
            &branch.combat_search_history[comparison_search_start..],
        ),
        full_search_comparability: classify_search_comparability(&branch.combat_search_history),
    }
}

pub(super) fn frontier_trajectory_evaluation(
    frontier: &VecDeque<Branch>,
) -> FrontierTrajectoryEvaluation {
    trajectory_evaluation_from_snapshots(frontier.iter().map(trajectory_snapshot).collect())
}

pub(super) fn trajectory_evaluation_from_snapshots(
    mut snapshots: Vec<TrajectorySnapshot>,
) -> FrontierTrajectoryEvaluation {
    snapshots.sort_by(|left, right| lane_sort_key(&left.lane).cmp(&lane_sort_key(&right.lane)));
    let baseline = snapshots
        .iter()
        .find(|snapshot| snapshot.lane == "baseline");

    let comparisons = baseline
        .map(|baseline| {
            snapshots
                .iter()
                .filter(|snapshot| snapshot.lane != "baseline")
                .map(|challenger| compare_trajectories(baseline, challenger))
                .collect()
        })
        .unwrap_or_default();

    FrontierTrajectoryEvaluation {
        snapshots,
        comparisons,
    }
}

fn lane_sort_key(lane: &str) -> (u8, u16, &str) {
    if lane == "baseline" {
        return (0, 0, lane);
    }
    let number = lane
        .strip_prefix("challenger-")
        .and_then(|value| value.parse().ok())
        .unwrap_or(u16::MAX);
    (1, number, lane)
}

fn terminal_state(status: &BranchStatus) -> TrajectoryTerminal {
    match status {
        BranchStatus::Terminal(TerminalOutcome::Victory) => TrajectoryTerminal::Victory,
        BranchStatus::Terminal(TerminalOutcome::Defeat) => TrajectoryTerminal::Defeat,
        BranchStatus::CombatGap { .. }
        | BranchStatus::OperationBudgetExhausted { .. }
        | BranchStatus::BudgetGap { .. } => TrajectoryTerminal::CoverageLimited,
        BranchStatus::AutomationGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => TrajectoryTerminal::Gap,
        BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => {
            TrajectoryTerminal::Running
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use sts_simulator::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
    use sts_simulator::ai::strategy::challenger_policy_state::{
        ChallengerPolicyState, CommitmentHorizon, CommitmentStatus, StrategyCommitment,
    };
    use sts_simulator::ai::strategy::trajectory_comparison::{
        TrajectoryDeployabilityEvidence, TrajectoryPressureEvidence,
        TrajectorySearchComparabilityStatus,
    };
    use sts_simulator::eval::run_control::{
        CombatSearchTraceSummary, RunControlConfig, RunControlSession,
    };

    use super::*;
    use crate::runtime::branch::owner_audit::branch_model::{BranchStatus, Owner};
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;

    fn test_branch(policy_lane: BranchPolicyLane) -> Branch {
        Branch {
            id: 1,
            parent_id: Some(0),
            path: Vec::new(),
            session: RunControlSession::new(RunControlConfig::default()),
            status: BranchStatus::Running {
                owner: Owner::CardReward,
                boundary: "test".to_string(),
            },
            policy_lane,
            combat_portfolio: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            trajectory: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    fn commitment(status: CommitmentStatus) -> StrategyCommitment {
        StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status,
            requirements: Vec::new(),
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        }
    }

    #[test]
    fn baseline_snapshot_keeps_uninstrumented_layers_unknown() {
        let branch = test_branch(BranchPolicyLane::default());
        let snapshot = trajectory_snapshot(&branch);

        assert_eq!(snapshot.lane, "baseline");
        assert_eq!(snapshot.pressure, TrajectoryPressureEvidence::Unknown);
        assert_eq!(
            snapshot.deployability,
            TrajectoryDeployabilityEvidence::Unknown
        );
        assert_eq!(snapshot.resources.hp, branch.session.run_state.current_hp);
    }

    #[test]
    fn snapshot_classifies_retained_node_bounded_search_history() {
        let mut branch = test_branch(BranchPolicyLane::default());
        branch.combat_search_history = vec![CombatSearchTraceSummary {
            source: "primary".to_string(),
            coverage_status: "NodeBudgetLimited".to_string(),
            node_budget_hit: true,
            ..CombatSearchTraceSummary::default()
        }];

        let snapshot = trajectory_snapshot(&branch);

        assert_eq!(
            snapshot.search_comparability.status,
            TrajectorySearchComparabilityStatus::Comparable
        );
        assert_eq!(snapshot.search_comparability.node_bounded_attempts, 1);
    }

    #[test]
    fn snapshot_excludes_shared_prefix_from_pair_eligibility_but_keeps_full_audit() {
        let mut branch = test_branch(BranchPolicyLane::default());
        branch.combat_search_history = vec![
            CombatSearchTraceSummary {
                source: "shared-prefix".to_string(),
                coverage_status: "TimeBudgetLimited".to_string(),
                deadline_hit: true,
                ..CombatSearchTraceSummary::default()
            },
            CombatSearchTraceSummary {
                source: "relic-suffix".to_string(),
                coverage_status: "NodeBudgetLimited".to_string(),
                node_budget_hit: true,
                ..CombatSearchTraceSummary::default()
            },
        ];
        branch.comparison_search_start = Some(1);

        let snapshot = trajectory_snapshot(&branch);

        assert_eq!(
            snapshot.search_comparability.status,
            TrajectorySearchComparabilityStatus::Comparable
        );
        assert_eq!(snapshot.search_comparability.total_attempts, 1);
        assert_eq!(
            snapshot.full_search_comparability.status,
            TrajectorySearchComparabilityStatus::WallSafetyLimited
        );
        assert_eq!(snapshot.full_search_comparability.total_attempts, 2);
    }

    #[test]
    fn challenger_snapshot_counts_commitment_outcomes_without_scoring_them() {
        let mut policy = ChallengerPolicyState::new(1);
        policy.commitments = vec![
            commitment(CommitmentStatus::Completed),
            commitment(CommitmentStatus::Active),
            commitment(CommitmentStatus::Expired),
        ];
        let branch = test_branch(BranchPolicyLane::challenger(policy));

        let snapshot = trajectory_snapshot(&branch);

        assert_eq!(snapshot.construction.completed_commitments, 1);
        assert_eq!(snapshot.construction.active_commitments, 1);
        assert_eq!(snapshot.construction.failed_commitments, 1);
    }

    #[test]
    fn frontier_evaluation_pairs_every_challenger_with_baseline() {
        let frontier = VecDeque::from([
            test_branch(BranchPolicyLane::default()),
            test_branch(BranchPolicyLane::challenger(ChallengerPolicyState::new(1))),
            test_branch(BranchPolicyLane::challenger(ChallengerPolicyState::new(2))),
        ]);

        let evaluation = frontier_trajectory_evaluation(&frontier);

        assert_eq!(evaluation.snapshots.len(), 3);
        assert_eq!(evaluation.comparisons.len(), 2);
        assert!(evaluation
            .comparisons
            .iter()
            .all(|item| item.baseline_lane == "baseline"));
    }

    #[test]
    fn snapshot_evaluation_pairs_latest_lanes_without_live_branches() {
        let baseline = trajectory_snapshot(&test_branch(BranchPolicyLane::default()));
        let challenger = trajectory_snapshot(&test_branch(BranchPolicyLane::challenger(
            ChallengerPolicyState::new(1),
        )));

        let evaluation =
            trajectory_evaluation_from_snapshots(vec![challenger.clone(), baseline.clone()]);

        assert_eq!(evaluation.snapshots[0].lane, "baseline");
        assert_eq!(evaluation.snapshots[1].lane, "challenger-1");
        assert_eq!(evaluation.comparisons.len(), 1);
        assert_eq!(evaluation.comparisons[0].baseline_lane, "baseline");
    }
}
