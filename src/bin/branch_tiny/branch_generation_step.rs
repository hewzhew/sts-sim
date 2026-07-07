use std::path::PathBuf;

use super::owner_model::OwnerChoice;
use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::{branch_observer, owner_choice_expander, trace, Args, Branch};

pub(super) enum BranchWorkAdvance {
    ObjectiveCompleted(Branch),
    Deferred(Branch),
    GenerationResult(Branch),
    Children(Vec<Branch>),
}

pub(super) fn advance_branch_work(
    branch: Branch,
    expandable: bool,
    choices: Vec<OwnerChoice>,
    expanded_mask: Vec<bool>,
    args: Args,
    child_args: Args,
    generation: usize,
    deadline: RunDeadline,
    next_branch_id: &mut usize,
    trace: &mut Option<trace::TraceWriter>,
    combat_gap_case_dir: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
) -> Result<BranchWorkAdvance, String> {
    branch_observer::record_branch_node(
        args,
        generation,
        &branch,
        &choices,
        &expanded_mask,
        trace,
        combat_gap_case_dir,
    )?;
    if !expandable {
        if branch_observer::record_stopped_branch(args, generation, &branch, trace, capsule)? {
            return Ok(BranchWorkAdvance::ObjectiveCompleted(branch));
        }
        return if branch.status.is_resumable() {
            Ok(BranchWorkAdvance::Deferred(branch))
        } else {
            Ok(BranchWorkAdvance::GenerationResult(branch))
        };
    }
    if !expanded_mask.iter().any(|expanded| *expanded) {
        return Ok(BranchWorkAdvance::Deferred(branch));
    }

    let mut children = Vec::new();
    for child in owner_choice_expander::expand_registered_owner(
        &branch,
        child_args,
        deadline,
        &choices,
        &expanded_mask,
        next_branch_id,
    ) {
        if branch_observer::record_child_branch(args, generation + 1, &child, capsule)? {
            return Ok(BranchWorkAdvance::ObjectiveCompleted(child));
        }
        children.push(child);
    }
    Ok(BranchWorkAdvance::Children(children))
}
