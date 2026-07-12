use std::collections::VecDeque;

use serde::Serialize;
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

#[derive(Clone, Debug, Serialize)]
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
    }
}

pub(super) fn frontier_trajectory_evaluation(
    frontier: &VecDeque<Branch>,
) -> FrontierTrajectoryEvaluation {
    let baseline = frontier
        .iter()
        .find(|branch| branch.policy_lane.challenger_policy().is_none())
        .map(trajectory_snapshot);
    let challengers = frontier
        .iter()
        .filter(|branch| branch.policy_lane.challenger_policy().is_some())
        .map(trajectory_snapshot)
        .collect::<Vec<_>>();

    let mut snapshots = Vec::with_capacity(frontier.len());
    if let Some(baseline) = baseline.as_ref() {
        snapshots.push(baseline.clone());
    }
    snapshots.extend(challengers.iter().cloned());
    let comparisons = baseline
        .as_ref()
        .map(|baseline| {
            challengers
                .iter()
                .map(|challenger| compare_trajectories(baseline, challenger))
                .collect()
        })
        .unwrap_or_default();

    FrontierTrajectoryEvaluation {
        snapshots,
        comparisons,
    }
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
    };
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

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
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
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
}
