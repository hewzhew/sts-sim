use std::collections::VecDeque;
use std::path::PathBuf;

use super::owner_model::{DecisionKey, OwnerChoice};
use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::{branch_frontier, branch_observer, branch_scheduler, trace, Args, Branch};

type BranchWork = (Branch, bool, Vec<OwnerChoice>);

pub(super) struct PreparedGeneration {
    work: Vec<BranchWork>,
    expanded_masks: Vec<Vec<bool>>,
    pub(super) total_expanded: usize,
}

pub(super) enum GenerationAdvance {
    ObjectiveCompleted,
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
) -> Result<GenerationAdvance, String> {
    let mut next = VecDeque::new();
    let mut deferred = VecDeque::new();
    let mut generation_result = None;
    for ((branch, expandable, choices), expanded_mask) in
        prepared.work.into_iter().zip(prepared.expanded_masks)
    {
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
                return Ok(GenerationAdvance::ObjectiveCompleted);
            }
            if branch.status.is_resumable() {
                deferred.push_back(branch);
                continue;
            }
            generation_result = Some((generation, branch.clone()));
            continue;
        }
        if !expanded_mask.iter().any(|expanded| *expanded) {
            deferred.push_back(branch);
            continue;
        }
        for child in branch_scheduler::expand_registered_owner(
            &branch,
            child_args,
            deadline,
            choices
                .into_iter()
                .enumerate()
                .filter(|(index, _)| expanded_mask[*index])
                .map(|(_, choice)| choice),
            next_branch_id,
        ) {
            if branch_observer::record_child_branch(args, generation + 1, &child, capsule)? {
                return Ok(GenerationAdvance::ObjectiveCompleted);
            }
            next.push_back(child);
        }
    }
    next.append(&mut deferred);
    Ok(GenerationAdvance::Advanced {
        next,
        generation_result,
    })
}
