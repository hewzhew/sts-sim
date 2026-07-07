use std::collections::VecDeque;
use std::path::PathBuf;

use super::owner_model::{DecisionKey, OwnerChoice};
use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::{branch_frontier, branch_generation_step, branch_scheduler, trace, Args, Branch};

type BranchWork = (Branch, bool, Vec<OwnerChoice>);

pub(super) struct PreparedGeneration {
    work: Vec<BranchWork>,
    expanded_masks: Vec<Vec<bool>>,
    pub(super) total_expanded: usize,
}

pub(super) enum GenerationAdvance {
    ObjectiveCompleted(Branch),
    Advanced {
        next: VecDeque<Branch>,
        generation_result: Option<(usize, Branch)>,
    },
}

impl PreparedGeneration {
    pub(super) fn into_frontier(self) -> VecDeque<Branch> {
        self.work.into_iter().map(|(branch, _, _)| branch).collect()
    }
}

pub(super) fn prepare_generation(
    frontier: &mut VecDeque<Branch>,
    args: Args,
    generation: usize,
    deadline: RunDeadline,
    recent_expanded_keys: &mut Vec<DecisionKey>,
) -> PreparedGeneration {
    let mut work = Vec::new();
    while let Some(branch) = frontier.pop_front() {
        work.push(branch_scheduler::prepare_branch_work(
            branch, args, generation, deadline,
        ));
    }
    let expanded_masks =
        branch_frontier::expansion_masks(&work, args.max_branches, recent_expanded_keys);
    let total_expanded = expanded_masks
        .iter()
        .flatten()
        .filter(|expanded| **expanded)
        .count();
    PreparedGeneration {
        work,
        expanded_masks,
        total_expanded,
    }
}

pub(super) fn advance_generation(
    prepared: PreparedGeneration,
    args: Args,
    child_args: Args,
    generation: usize,
    deadline: RunDeadline,
    next_branch_id: &mut usize,
    trace: &mut Option<trace::TraceWriter>,
    combat_gap_case_dir: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
    human_output: bool,
) -> Result<GenerationAdvance, String> {
    let mut next = VecDeque::new();
    let mut deferred = VecDeque::new();
    let mut generation_result = None;
    for ((branch, expandable, choices), expanded_mask) in
        prepared.work.into_iter().zip(prepared.expanded_masks)
    {
        match branch_generation_step::advance_branch_work(
            branch,
            expandable,
            choices,
            expanded_mask,
            args,
            child_args,
            generation,
            deadline,
            next_branch_id,
            trace,
            combat_gap_case_dir,
            capsule,
            human_output,
        )? {
            branch_generation_step::BranchWorkAdvance::ObjectiveCompleted(branch) => {
                return Ok(GenerationAdvance::ObjectiveCompleted(branch));
            }
            branch_generation_step::BranchWorkAdvance::Deferred(branch) => {
                deferred.push_back(branch);
            }
            branch_generation_step::BranchWorkAdvance::GenerationResult(branch) => {
                generation_result = Some((generation, branch));
            }
            branch_generation_step::BranchWorkAdvance::Children(children) => {
                next.extend(children);
            }
        }
    }
    next.append(&mut deferred);
    Ok(GenerationAdvance::Advanced {
        next,
        generation_result,
    })
}
