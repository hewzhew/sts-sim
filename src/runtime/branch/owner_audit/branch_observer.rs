use std::path::PathBuf;

use super::owner_model::OwnerChoice;
use super::run_capsule::RunCapsule;
use super::run_slice_result::ArtifactWriteSummary;
use super::{combat_gap_case, render, run_contract, run_persistence, trace, Args, Branch};

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
                artifacts.combat_case_written = true;
                println!("  combat_gap_case: {}", path.display());
            }
            Ok(Some(_)) => {
                artifacts.combat_case_written = true;
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
    if let Some(capsule) = capsule {
        artifacts.merge(capsule.save_terminal_result(args, generation, branch)?);
    }
    if let Some(reason) = run_contract::satisfied(args.objective, &branch.status) {
        artifacts.merge(run_persistence::finalize_objective_result(
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
