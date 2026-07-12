use std::path::PathBuf;

use super::owner_model::OwnerChoice;
use super::policy_expansion_plan::PolicyExpansion;
use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::run_slice_result::ArtifactWriteSummary;
use super::{branch_observer, owner_choice_expander, trace, Args, Branch};

pub(super) enum BranchWorkAdvance {
    ObjectiveCompleted {
        branch: Branch,
        artifacts: ArtifactWriteSummary,
    },
    Deferred {
        branch: Branch,
        artifacts: ArtifactWriteSummary,
    },
    GenerationResult {
        branch: Branch,
        artifacts: ArtifactWriteSummary,
    },
    Children {
        children: Vec<Branch>,
        artifacts: ArtifactWriteSummary,
    },
}

pub(super) fn advance_branch_work(
    branch: Branch,
    expandable: bool,
    choices: Vec<OwnerChoice>,
    expanded_mask: Vec<bool>,
    policy_expansions: Vec<PolicyExpansion>,
    args: Args,
    child_args: Args,
    generation: usize,
    deadline: RunDeadline,
    next_branch_id: &mut usize,
    trace: &mut Option<trace::TraceWriter>,
    combat_gap_case_dir: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
    human_output: bool,
) -> Result<BranchWorkAdvance, String> {
    let mut artifacts = branch_observer::record_branch_node(
        args,
        generation,
        &branch,
        &choices,
        &expanded_mask,
        trace,
        combat_gap_case_dir,
        human_output,
    )?;
    if !expandable {
        let outcome = branch_observer::record_stopped_branch(
            args,
            generation,
            &branch,
            trace,
            capsule,
            human_output,
        )?;
        artifacts.merge(outcome.artifacts);
        if outcome.objective_completed {
            return Ok(BranchWorkAdvance::ObjectiveCompleted { branch, artifacts });
        }
        return if branch.status.is_resumable() {
            Ok(BranchWorkAdvance::Deferred { branch, artifacts })
        } else {
            Ok(BranchWorkAdvance::GenerationResult { branch, artifacts })
        };
    }
    if !expanded_mask.iter().any(|expanded| *expanded) {
        return Ok(BranchWorkAdvance::Deferred { branch, artifacts });
    }

    let mut children = Vec::new();
    for child in owner_choice_expander::expand_registered_owner(
        &branch,
        child_args,
        deadline,
        &choices,
        &policy_expansions,
        next_branch_id,
    ) {
        let outcome = branch_observer::record_child_branch(
            args,
            generation + 1,
            &child,
            capsule,
            human_output,
        )?;
        artifacts.merge(outcome.artifacts);
        if outcome.objective_completed {
            return Ok(BranchWorkAdvance::ObjectiveCompleted {
                branch: child,
                artifacts,
            });
        }
        children.push(child);
    }
    Ok(BranchWorkAdvance::Children {
        children,
        artifacts,
    })
}
