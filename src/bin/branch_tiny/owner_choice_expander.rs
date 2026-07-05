use super::branch_path::{
    BranchPathCandidateSnapshot, BranchPathState, BranchPathStep, ChoiceAnnotationSnapshot,
};
use super::owner_model::OwnerChoice;
use super::run_deadline::RunDeadline;
use super::{decision_delta, runner, Args, Branch, BranchStatus};

pub(super) fn expand_registered_owner(
    branch: &Branch,
    args: Args,
    deadline: RunDeadline,
    choices: &[OwnerChoice],
    expanded_mask: &[bool],
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    let mut children = Vec::new();
    for (choice_index, choice) in choices.iter().cloned().enumerate() {
        if !expanded_mask.get(choice_index).copied().unwrap_or(false) {
            continue;
        }
        let mut session = branch.session.clone();
        let (advance, decision_delta) = match session.apply_command(choice.action.clone()) {
            Ok(_) => {
                let delta =
                    decision_delta::decision_delta(&branch.session.run_state, &session.run_state);
                (
                    runner::advance_to_owner_or_gap(&mut session, args, deadline),
                    delta,
                )
            }
            Err(err) => (
                runner::AdvanceResult {
                    status: BranchStatus::ApplyFailed(err),
                    combat_portfolio: None,
                    auto_steps: Vec::new(),
                    combat_search: Vec::new(),
                },
                None,
            ),
        };
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            key: choice.key,
            action_debug: format!("{:?}", choice.action),
            label: choice.label,
            annotation: ChoiceAnnotationSnapshot::from_annotation(&choice.annotation),
            state_before: Some(BranchPathState::from_branch(branch)),
            decision_delta,
            candidate_pool: BranchPathCandidateSnapshot::from_choices(choices, choice_index),
        });
        let id = *next_branch_id;
        *next_branch_id += 1;
        children.push(Branch {
            id,
            parent_id: Some(branch.id),
            path,
            session,
            status: advance.status,
            combat_portfolio: advance.combat_portfolio,
            auto_steps: advance.auto_steps,
            combat_search: advance.combat_search,
        });
    }
    children
}
