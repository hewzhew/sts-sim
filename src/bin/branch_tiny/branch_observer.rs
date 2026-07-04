use std::path::PathBuf;

use super::owner_model::OwnerChoice;
use super::run_capsule::RunCapsule;
use super::{combat_gap_case, render, run_contract, run_persistence, trace, Args, Branch};

pub(super) fn record_branch_node(
    args: Args,
    generation: usize,
    branch: &Branch,
    choices: &[OwnerChoice],
    expanded_mask: &[bool],
    trace: &mut Option<trace::TraceWriter>,
    combat_gap_case_dir: Option<&PathBuf>,
) -> Result<(), String> {
    render::print_branch_timeline(generation, branch, choices, expanded_mask);
    if let Some(trace) = trace.as_mut() {
        trace.record_node(generation, branch, choices, expanded_mask)?;
    }
    if let Some(dir) = combat_gap_case_dir {
        match combat_gap_case::save_combat_gap_case(dir, args, generation, branch) {
            Ok(Some(path)) => println!("  combat_gap_case: {}", path.display()),
            Ok(None) => {}
            Err(err) => println!("  combat_gap_case_error: {}", render::one_line(&err)),
        }
    }
    Ok(())
}

pub(super) fn record_stopped_branch(
    args: Args,
    generation: usize,
    branch: &Branch,
    trace: &mut Option<trace::TraceWriter>,
    capsule: Option<&RunCapsule>,
) -> Result<bool, String> {
    if let Some(trace) = trace.as_mut() {
        trace.record_branch_snapshot(generation, "stopped", branch)?;
    }
    record_terminal_and_objective(args, generation, branch, capsule)
}

pub(super) fn record_child_branch(
    args: Args,
    generation: usize,
    branch: &Branch,
    capsule: Option<&RunCapsule>,
) -> Result<bool, String> {
    record_terminal_and_objective(args, generation, branch, capsule)
}

fn record_terminal_and_objective(
    args: Args,
    generation: usize,
    branch: &Branch,
    capsule: Option<&RunCapsule>,
) -> Result<bool, String> {
    if let Some(capsule) = capsule {
        capsule.save_terminal_result(args, generation, branch)?;
    }
    if let Some(reason) = run_contract::satisfied(args.objective, &branch.status) {
        run_persistence::finalize_objective_result(
            capsule,
            args,
            generation,
            branch,
            reason.as_str(),
        )?;
        return Ok(true);
    }
    Ok(false)
}
