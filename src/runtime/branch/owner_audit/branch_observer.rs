use std::path::PathBuf;

use super::owner_model::OwnerChoice;
use super::run_capsule::{RunCapsule, RunCapsuleSave};
use super::run_slice_result::{ArtifactKind, ArtifactRef, ArtifactWriteSummary};
use super::{branch_status_view, combat_gap_case, render, run_contract, trace, Args, Branch};

pub(super) struct BranchRecordOutcome {
    pub(super) objective_completed: bool,
    pub(super) artifacts: ArtifactWriteSummary,
}

pub(super) fn record_branch_node(
    args: Args,
    generation: usize,
    branch: &Branch,
    choices: &[OwnerChoice],
    expanded_mask: &[bool],
    trace: &mut Option<trace::TraceWriter>,
    combat_gap_case_dir: Option<&PathBuf>,
    human_output: bool,
) -> Result<ArtifactWriteSummary, String> {
    let mut artifacts = ArtifactWriteSummary::default();
    if human_output {
        render::print_branch_timeline(generation, branch, choices, expanded_mask);
    }
    if let Some(trace) = trace.as_mut() {
        trace.record_node(generation, branch, choices, expanded_mask)?;
    }
    if let Some(dir) = combat_gap_case_dir {
        match combat_gap_case::save_combat_gap_case(dir, args, generation, branch) {
            Ok(Some(path)) if human_output => {
                artifacts.record_ref(combat_case_ref(path.clone()));
                println!("  combat_gap_case: {}", path.display());
            }
            Ok(Some(path)) => {
                artifacts.record_ref(combat_case_ref(path));
            }
            Ok(None) => {}
            Err(err) if human_output => {
                println!("  combat_gap_case_error: {}", render::one_line(&err));
            }
            Err(_) => {}
        }
    }
    Ok(artifacts)
}

fn combat_case_ref(path: PathBuf) -> ArtifactRef {
    ArtifactRef::new(
        ArtifactKind::CombatCase,
        path,
        "branch_tiny_combat_gap_case",
        "owner_audit_runtime",
    )
}

pub(super) fn record_stopped_branch(
    args: Args,
    generation: usize,
    branch: &Branch,
    trace: &mut Option<trace::TraceWriter>,
    capsule: Option<&RunCapsule>,
    human_output: bool,
) -> Result<BranchRecordOutcome, String> {
    if let Some(trace) = trace.as_mut() {
        trace.record_branch_snapshot(generation, "stopped", branch)?;
    }
    record_terminal_and_objective(args, generation, branch, capsule, human_output)
}

pub(super) fn record_child_branch(
    args: Args,
    generation: usize,
    branch: &Branch,
    capsule: Option<&RunCapsule>,
    human_output: bool,
) -> Result<BranchRecordOutcome, String> {
    record_terminal_and_objective(args, generation, branch, capsule, human_output)
}

fn record_terminal_and_objective(
    args: Args,
    generation: usize,
    branch: &Branch,
    capsule: Option<&RunCapsule>,
    human_output: bool,
) -> Result<BranchRecordOutcome, String> {
    let mut artifacts = ArtifactWriteSummary::default();
    if !branch.status.is_resumable() {
        if let Some(capsule) = capsule {
            artifacts.merge(capsule.record_stopped_trajectory(generation, branch)?);
        }
    }
    if let Some(capsule) = capsule {
        artifacts.merge(capsule.save_terminal_result(args, generation, branch)?);
    }
    if let Some(reason) = run_contract::satisfied(args.objective, &branch.status) {
        artifacts.merge(finalize_objective_result(
            capsule,
            args,
            generation,
            branch,
            reason.as_str(),
            human_output,
        )?);
        return Ok(BranchRecordOutcome {
            objective_completed: true,
            artifacts,
        });
    }
    Ok(BranchRecordOutcome {
        objective_completed: false,
        artifacts,
    })
}

fn finalize_objective_result(
    capsule: Option<&RunCapsule>,
    args: Args,
    generation: usize,
    branch: &Branch,
    reason: &'static str,
    human_output: bool,
) -> Result<ArtifactWriteSummary, String> {
    let mut artifacts = ArtifactWriteSummary::default();
    if let Some(capsule) = capsule {
        capsule.save_completed_result(args, generation, branch, reason)?;
        artifacts.merge(capsule.artifact_writes(RunCapsuleSave::Result));
        if human_output {
            println!("run_capsule_result: {}", capsule.result_path().display());
        }
    } else if human_output {
        println!(
            "run_objective_completed: reason={} branch={} status={}",
            reason,
            branch.id,
            render::one_line(&branch_status_view::status_boundary_label(&branch.status))
        );
    }
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    use super::*;
    use crate::runtime::branch::owner_audit::branch_model::BranchStatus;
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;

    fn stopped_challenger() -> Branch {
        Branch {
            id: 2,
            parent_id: Some(1),
            path: Vec::new(),
            session: RunControlSession::new(RunControlConfig::default()),
            status: BranchStatus::CombatGap {
                boundary: "boss".to_string(),
                reason: "no win".to_string(),
            },
            policy_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
            combat_portfolio: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    #[test]
    fn stopped_branch_records_trajectory_before_discard() {
        let root = std::env::temp_dir().join("stopped_branch_trajectory_observation");
        let _ = std::fs::remove_dir_all(&root);
        let capsule = RunCapsule::new(root.clone());
        let mut trace = None;

        record_stopped_branch(
            crate::runtime::branch::default_branch_args(7),
            4,
            &stopped_challenger(),
            &mut trace,
            Some(&capsule),
            false,
        )
        .unwrap();

        let state: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("trajectory_state.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(state["observations"][0]["snapshot"]["lane"], "challenger-1");
        let _ = std::fs::remove_dir_all(root);
    }
}
