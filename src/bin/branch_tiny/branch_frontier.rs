use std::collections::VecDeque;

use super::owner_model::{DecisionKey, OwnerChoice};
use super::{Branch, BranchStatus, TerminalOutcome};

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
