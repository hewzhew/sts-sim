use std::collections::VecDeque;

use super::owner_model::{DecisionKey, OwnerChoice, OwnerDecision};
use super::run_deadline::RunDeadline;
use super::{
    decision_delta, owners, runner, Args, Branch, BranchPathState, BranchPathStep, BranchStatus,
    ChoiceAnnotationSnapshot, TerminalOutcome,
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

pub(super) fn expansion_masks(
    work: &[(Branch, bool, Vec<OwnerChoice>)],
    max_branches: usize,
    recent_expanded_keys: &mut Vec<DecisionKey>,
) -> Vec<Vec<bool>> {
    let mut expanded = work
        .iter()
        .map(|(_, _, choices)| vec![false; choices.len()])
        .collect::<Vec<_>>();
    let mut remaining = max_branches;
    let mut prefer_unused_keys = false;
    while remaining > 0 {
        let mut progressed = false;
        for (branch_index, (_, expandable, choices)) in work.iter().enumerate() {
            if !*expandable {
                continue;
            }
            let Some(choice_index) = next_expansion_choice(
                choices,
                &expanded[branch_index],
                recent_expanded_keys,
                prefer_unused_keys,
            ) else {
                continue;
            };
            expanded[branch_index][choice_index] = true;
            if let Some(key) = choices[choice_index].key.clone() {
                recent_expanded_keys.push(key);
            }
            remaining -= 1;
            progressed = true;
            if remaining == 0 {
                break;
            }
        }
        if !progressed {
            break;
        }
        prefer_unused_keys = true;
    }
    trim_recent_expanded_keys(recent_expanded_keys);
    expanded
}

fn trim_recent_expanded_keys(keys: &mut Vec<DecisionKey>) {
    const RECENT_KEY_LIMIT: usize = 64;
    if keys.len() > RECENT_KEY_LIMIT {
        keys.drain(0..keys.len() - RECENT_KEY_LIMIT);
    }
}

fn next_expansion_choice(
    choices: &[OwnerChoice],
    expanded: &[bool],
    used_keys: &[DecisionKey],
    prefer_unused_keys: bool,
) -> Option<usize> {
    let candidates = choices
        .iter()
        .enumerate()
        .filter(|(index, choice)| choice.auto_expand_allowed() && !expanded[*index]);
    if prefer_unused_keys {
        if let Some((index, _)) = candidates.clone().find(|(_, choice)| {
            choice
                .key
                .as_ref()
                .is_some_and(|key| !used_keys.contains(key))
        }) {
            return Some(index);
        }
    }
    candidates.map(|(index, _)| index).next()
}

pub(super) fn retain_frontier(frontier: &mut VecDeque<Branch>, limit: usize) {
    if frontier.len() <= limit {
        return;
    }
    let mut branches = frontier.drain(..).collect::<Vec<_>>();
    branches.sort_by(|a, b| {
        frontier_retention_key(b)
            .cmp(&frontier_retention_key(a))
            .then_with(|| a.id.cmp(&b.id))
    });
    branches.truncate(limit);
    *frontier = branches.into();
}

fn frontier_retention_key(branch: &Branch) -> (u8, u8, i32, u32, i32) {
    let status = match branch.status {
        BranchStatus::Terminal(TerminalOutcome::Victory) => 4,
        BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => 3,
        BranchStatus::CombatGap { .. }
        | BranchStatus::OperationBudgetExhausted { .. }
        | BranchStatus::BudgetGap { .. } => 1,
        BranchStatus::Terminal(TerminalOutcome::Defeat)
        | BranchStatus::AutomationGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => 0,
    };
    let hp = branch.session.run_state.current_hp;
    let max_hp = branch.session.run_state.max_hp.max(1);
    let hp_ratio = (hp.max(0) as u32).saturating_mul(1000) / max_hp as u32;
    (
        status,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        hp_ratio,
        hp,
    )
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
