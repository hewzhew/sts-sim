use super::owner_model::{OwnerChoice, OwnerDecision};
use super::run_deadline::RunDeadline;
use super::{owners, runner, Args, Branch, BranchStatus};

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
