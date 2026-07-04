use super::owner_model::{OwnerChoice, OwnerDecision};
use super::run_deadline::RunDeadline;
use super::{
    decision_delta, owners, runner, Args, Branch, BranchPathState, BranchPathStep, BranchStatus,
    ChoiceAnnotationSnapshot,
};

pub(super) fn prepare_branch_work(
    mut branch: Branch,
    args: Args,
    generation: usize,
    deadline: RunDeadline,
) -> (Branch, bool, Vec<OwnerChoice>) {
    let mut expandable = generation < args.generations && branch.status.is_expandable_now();
    let mut choices = if expandable {
        branch_owner_choices(&branch)
    } else {
        Vec::new()
    };
    if generation < args.generations
        && (matches!(branch.status, BranchStatus::AwaitingAuto { .. })
            || (expandable && choices.is_empty()))
    {
        let advance = runner::advance_to_owner_or_gap(
            &mut branch.session,
            deadline.cap_args(args, 1),
            deadline,
        );
        branch.status = advance.status;
        branch.combat_portfolio = advance.combat_portfolio;
        branch.auto_steps = advance.auto_steps;
        branch.combat_search = advance.combat_search;
        expandable = generation < args.generations && branch.status.is_expandable_now();
        choices = if expandable {
            branch_owner_choices(&branch)
        } else {
            Vec::new()
        };
    }
    (branch, expandable, choices)
}

pub(super) fn expand_registered_owner(
    branch: &Branch,
    args: Args,
    deadline: RunDeadline,
    candidates: impl IntoIterator<Item = OwnerChoice>,
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    let mut children = Vec::new();
    for choice in candidates {
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

fn branch_owner_choices(branch: &Branch) -> Vec<OwnerChoice> {
    let BranchStatus::Running { owner, .. } = branch.status else {
        return Vec::new();
    };
    let surface = sts_simulator::eval::run_control::build_decision_surface(&branch.session);
    match owners::owner_decision(&branch.session, owner, &surface) {
        OwnerDecision::Candidates(choices) => choices,
        OwnerDecision::Routine(_) | OwnerDecision::Gap(_) => Vec::new(),
    }
}
