use sts_simulator::ai::strategy::decision_pipeline::DecisionCandidateKind;
use sts_simulator::ai::strategy::shop_boss_preview::shop_boss_preview_bundles;

use super::accepted_high_loss_diagnostic::extend_unique_diagnostics;
use super::branch_path::{
    BranchPathCandidateSnapshot, BranchPathShopBossPreviewBundleSnapshot,
    BranchPathShopBossPreviewSnapshot, BranchPathState, BranchPathStep, ChoiceAnnotationSnapshot,
};
use super::candidate_ir_adapter::shop_tiny_kind;
use super::owner_model::OwnerChoice;
use super::policy_expansion_plan::PolicyExpansion;
use super::run_deadline::RunDeadline;
use super::{
    decision_delta, runner, shop_boss_preview_bundle_expansion, Args, Branch, BranchStatus,
};

pub(super) fn expand_registered_owner(
    branch: &Branch,
    args: Args,
    deadline: RunDeadline,
    choices: &[OwnerChoice],
    policy_expansions: &[PolicyExpansion],
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    let mut children = Vec::new();
    for expansion in policy_expansions.iter().cloned() {
        let choice_index = expansion.choice_index;
        let Some(choice) = choices.get(choice_index).cloned() else {
            continue;
        };
        let policy_lane_label = expansion.child_lane.label();
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
                    accepted_high_loss_diagnostics: Vec::new(),
                },
                None,
            ),
        };
        let combat_search = advance.combat_search;
        let mut combat_search_history = branch.combat_search_history.clone();
        combat_search_history.extend(combat_search.clone());
        let mut accepted_high_loss_diagnostics = branch.accepted_high_loss_diagnostics.clone();
        extend_unique_diagnostics(
            &mut accepted_high_loss_diagnostics,
            advance.accepted_high_loss_diagnostics,
        );
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            policy_lane: policy_lane_label,
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
            policy_lane: expansion.child_lane,
            combat_portfolio: advance.combat_portfolio,
            auto_steps: advance.auto_steps,
            combat_search,
            combat_search_history,
            accepted_high_loss_diagnostics,
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
    if !matches!(
        branch.policy_lane,
        super::branch_policy_lane::BranchPolicyLane::Baseline { .. }
    ) {
        return Vec::new();
    }
    if args.shop_boss_preview_bundle_limit == 0 {
        return Vec::new();
    }
    if let Some(target_floor) = args.shop_boss_preview_target_floor {
        if branch.session.run_state.floor_num != target_floor {
            return Vec::new();
        }
    }
    let kinds = shop_boss_preview_bundle_kinds(choices);
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
                        accepted_high_loss_diagnostics: Vec::new(),
                    },
                    None,
                ),
            };
        let combat_search = advance.combat_search;
        let mut combat_search_history = branch.combat_search_history.clone();
        combat_search_history.extend(combat_search.clone());
        let mut accepted_high_loss_diagnostics = branch.accepted_high_loss_diagnostics.clone();
        extend_unique_diagnostics(
            &mut accepted_high_loss_diagnostics,
            advance.accepted_high_loss_diagnostics,
        );
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            policy_lane: branch.policy_lane.label(),
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
            policy_lane: branch.policy_lane.clone(),
            combat_portfolio: advance.combat_portfolio,
            auto_steps: advance.auto_steps,
            combat_search,
            combat_search_history,
            accepted_high_loss_diagnostics,
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

fn shop_boss_preview_bundle_kinds(choices: &[OwnerChoice]) -> Vec<DecisionCandidateKind> {
    choices
        .iter()
        .filter_map(|choice| {
            choice
                .annotation
                .candidate()
                .map(|decision| decision.evaluation.candidate.kind)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, DecisionCandidateIr,
        ExpansionPlan,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{DecisionCandidateKey, RunControlCommand};

    use super::super::branch_policy_lane::BranchPolicyLane;
    use super::super::policy_expansion_plan::PolicyExpansion;
    use super::super::run_contract::RunObjective;
    use super::super::{BranchStatus, Owner};

    fn candidate_choice(
        kind: DecisionCandidateKind,
        key: DecisionCandidateKey,
        expansion: super::super::owner_model::OwnerChoiceExpansion,
    ) -> OwnerChoice {
        OwnerChoice {
            key: Some(key),
            action: RunControlCommand::Noop,
            label: format!("{kind:?}"),
            annotation: super::super::owner_model::ChoiceAnnotation::Candidate(
                super::super::owner_model::OwnerCandidateDecision {
                    evaluation: CandidateEvaluation {
                        candidate: DecisionCandidateIr { kind },
                        lane: CandidateLane::Mainline,
                        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                        expansion: ExpansionPlan::Auto,
                        scores: Vec::new(),
                    },
                    admission: None,
                },
            ),
            expansion,
        }
    }

    #[test]
    fn shop_preview_bundle_kinds_include_inspect_only_choices_for_review() {
        let leave = candidate_choice(
            DecisionCandidateKind::ShopLeave,
            DecisionCandidateKey::ShopLeave,
            super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
        );
        let blocked_fiend_fire = candidate_choice(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::FiendFire,
                upgrades: 0,
                price: 152,
            },
            DecisionCandidateKey::ShopBuyCard {
                shop_slot: 0,
                card: CardId::FiendFire,
                upgrades: 0,
                price: 152,
            },
            super::super::owner_model::OwnerChoiceExpansion::InspectOnly("blocked"),
        );

        let kinds = shop_boss_preview_bundle_kinds(&[leave, blocked_fiend_fire]);

        assert_eq!(
            kinds,
            vec![
                DecisionCandidateKind::ShopLeave,
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                }
            ]
        );
    }

    fn sample_args() -> Args {
        Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 2,
            max_branches: 3,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn planned_children_keep_distinct_policy_lane_identity() {
        let parent = Branch {
            id: 0,
            parent_id: None,
            path: Vec::new(),
            session: sts_simulator::eval::run_control::RunControlSession::new(
                sts_simulator::eval::run_control::RunControlConfig::default(),
            ),
            status: BranchStatus::Running {
                owner: Owner::CardReward,
                boundary: "test".to_string(),
            },
            policy_lane: BranchPolicyLane::default(),
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            accepted_high_loss_diagnostics: Vec::new(),
        };
        let choices = vec![
            candidate_choice(
                DecisionCandidateKind::CardRewardSkip,
                DecisionCandidateKey::CardRewardSkip {
                    reward_item_index: 0,
                },
                super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
            ),
            candidate_choice(
                DecisionCandidateKind::CardRewardSkip,
                DecisionCandidateKey::CardRewardSkip {
                    reward_item_index: 1,
                },
                super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
            ),
        ];
        let plans = vec![
            PolicyExpansion {
                choice_index: 0,
                child_lane: BranchPolicyLane::default(),
            },
            PolicyExpansion {
                choice_index: 1,
                child_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
            },
        ];
        let mut next_branch_id = 1;

        let children = expand_registered_owner(
            &parent,
            sample_args(),
            RunDeadline::new(Instant::now(), None),
            &choices,
            &plans,
            &mut next_branch_id,
        );

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].policy_lane.label(), "baseline");
        assert_eq!(children[1].policy_lane.label(), "challenger-1");
        assert_eq!(children[1].path[0].policy_lane, "challenger-1");
    }

    #[test]
    fn challenger_lane_does_not_spawn_shop_preview_bundle_children() {
        let mut session = sts_simulator::eval::run_control::RunControlSession::new(
            sts_simulator::eval::run_control::RunControlConfig::default(),
        );
        session.run_state.gold = 200;
        let parent = Branch {
            id: 0,
            parent_id: None,
            path: Vec::new(),
            session,
            status: BranchStatus::Running {
                owner: Owner::ShopTiny,
                boundary: "test".to_string(),
            },
            policy_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            accepted_high_loss_diagnostics: Vec::new(),
        };
        let choices = vec![
            candidate_choice(
                DecisionCandidateKind::ShopLeave,
                DecisionCandidateKey::ShopLeave,
                super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
            ),
            candidate_choice(
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                },
                DecisionCandidateKey::ShopBuyCard {
                    shop_slot: 0,
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                },
                super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
            ),
        ];
        let mut args = sample_args();
        args.shop_boss_preview_bundle_limit = 1;
        let mut next_branch_id = 1;

        let children = expand_shop_boss_preview_bundle_children(
            &parent,
            args,
            RunDeadline::new(Instant::now(), None),
            &choices,
            &mut next_branch_id,
        );

        assert!(children.is_empty());
    }
}
