use sts_simulator::ai::strategy::decision_pipeline::DecisionCandidateKind;
use sts_simulator::ai::strategy::shop_boss_preview::shop_boss_preview_bundles;

use super::branch_path::{
    BranchPathCandidateSnapshot, BranchPathShopBossPreviewBundleSnapshot,
    BranchPathShopBossPreviewSnapshot, BranchPathState, BranchPathStep, ChoiceAnnotationSnapshot,
};
use super::candidate_ir_adapter::shop_tiny_kind;
use super::owner_model::OwnerChoice;
use super::run_deadline::RunDeadline;
use super::{
    decision_delta, runner, shop_boss_preview_bundle_expansion, Args, Branch, BranchStatus,
};

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
            shop_boss_preview_candidates:
                super::branch_path::BranchPathShopBossPreviewSnapshot::from_choices(choices),
            shop_boss_preview_bundles:
                super::branch_path::BranchPathShopBossPreviewBundleSnapshot::from_choices(
                    choices,
                    branch.session.run_state.gold,
                ),
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
    children.extend(expand_shop_boss_preview_bundle_children(
        branch,
        args,
        deadline,
        choices,
        next_branch_id,
    ));
    children
}

fn expand_shop_boss_preview_bundle_children(
    branch: &Branch,
    args: Args,
    deadline: RunDeadline,
    choices: &[OwnerChoice],
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    if args.shop_boss_preview_bundle_limit == 0 {
        return Vec::new();
    }
    let kinds = choices
        .iter()
        .filter_map(|choice| {
            choice
                .annotation
                .candidate()
                .map(|decision| decision.evaluation.candidate.kind)
        })
        .collect::<Vec<_>>();
    let bundles = shop_boss_preview_bundles(
        kinds,
        branch.session.run_state.gold,
        args.shop_boss_preview_bundle_limit + 1,
    );
    let all_bundle_snapshots = BranchPathShopBossPreviewBundleSnapshot::from_choices(
        choices,
        branch.session.run_state.gold,
    );
    let preview_candidates = BranchPathShopBossPreviewSnapshot::from_choices(choices);
    let candidate_pool = BranchPathCandidateSnapshot::from_choices(choices, usize::MAX);
    let mut children = Vec::new();
    for bundle in bundles
        .into_iter()
        .filter(|bundle| !bundle.items.is_empty())
        .take(args.shop_boss_preview_bundle_limit)
    {
        let mut session = branch.session.clone();
        let (advance, delta) =
            match shop_boss_preview_bundle_expansion::apply_shop_boss_preview_bundle(
                &mut session,
                &bundle.items,
            ) {
                Ok(()) => {
                    let delta = decision_delta::decision_delta(
                        &branch.session.run_state,
                        &session.run_state,
                    );
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
            key: None,
            action_debug: format!("ShopBossPreviewBundle({:?})", bundle.items),
            label: format!(
                "Shop preview bundle: {}",
                bundle_label(choices, &bundle.items)
            ),
            annotation: ChoiceAnnotationSnapshot::None,
            state_before: Some(BranchPathState::from_branch(branch)),
            decision_delta: delta,
            candidate_pool: candidate_pool.clone(),
            shop_boss_preview_candidates: preview_candidates.clone(),
            shop_boss_preview_bundles: all_bundle_snapshots.clone(),
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

fn bundle_label(choices: &[OwnerChoice], items: &[DecisionCandidateKind]) -> String {
    items
        .iter()
        .map(|item| {
            choices
                .iter()
                .find_map(|choice| {
                    if shop_tiny_kind(&choice.key) == *item {
                        Some(choice.label.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| format!("{item:?}"))
        })
        .collect::<Vec<_>>()
        .join(" + ")
}
