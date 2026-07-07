use serde::{Deserialize, Serialize};

use super::run_contract::RunContract;
use super::{Args, BoundarySite, Branch, BranchStatus, Owner, TerminalOutcome};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RunSliceResult {
    pub(super) contract: RunContract,
    pub(super) request_kind: RunSliceRequestKind,
    pub(super) generation_start: usize,
    pub(super) generation_end: usize,
    pub(super) next_branch_id: usize,
    pub(super) stop: RunStop,
    pub(super) frontier: FrontierSummary,
    pub(super) selected_branch: Option<BranchSummary>,
    pub(super) budget: SliceBudgetSummary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum RunSliceRequestKind {
    Start,
    ResumeFrontier,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum RunStop {
    Real(RealStop),
    SoftPause(SoftPause),
    FrontierExhausted(FrontierExhausted),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum RealStop {
    Terminal {
        generation: usize,
        branch_id: usize,
        outcome: TerminalOutcome,
    },
    ObjectiveSatisfied {
        generation: usize,
        reason: String,
    },
    AutomationGap {
        generation: usize,
        branch_id: usize,
        boundary: String,
        site: BoundarySite,
    },
    CombatGap {
        generation: usize,
        branch_id: usize,
        boundary: String,
        reason: String,
    },
    OperationBudgetExhausted {
        generation: usize,
        branch_id: usize,
        boundary: String,
        reason: String,
    },
    BudgetGap {
        generation: usize,
        branch_id: usize,
        boundary: String,
        reason: String,
    },
    ApplyFailed {
        generation: usize,
        branch_id: usize,
        reason: String,
    },
    AdvanceFailed {
        generation: usize,
        branch_id: usize,
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum SoftPause {
    GenerationLimit {
        generation: usize,
        frontier_running_count: usize,
    },
    SliceDeadline {
        generation: usize,
        frontier_running_count: usize,
    },
    AwaitingAutoBoundary {
        generation: usize,
        frontier_running_count: usize,
    },
    SearchBudgetCappedBeforeGeneration {
        generation: usize,
        frontier_running_count: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum FrontierExhausted {
    NoRunningBranches { generation: usize },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FrontierSummary {
    pub(super) total_count: usize,
    pub(super) running_count: usize,
    pub(super) expandable_count: usize,
    pub(super) terminal_count: usize,
    pub(super) gap_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct BranchSummary {
    pub(super) branch_id: usize,
    pub(super) parent_id: Option<usize>,
    pub(super) status_kind: BranchStatusKind,
    pub(super) boundary: Option<String>,
    pub(super) owner: Option<String>,
    pub(super) act: u8,
    pub(super) floor: i32,
    pub(super) hp: i32,
    pub(super) max_hp: i32,
    pub(super) gold: i32,
    pub(super) deck_size: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum BranchStatusKind {
    Running,
    AwaitingAuto,
    Terminal,
    AutomationGap,
    CombatGap,
    OperationBudgetExhausted,
    BudgetGap,
    ApplyFailed,
    AdvanceFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct SliceBudgetSummary {
    pub(super) slice_ms: Option<u64>,
    pub(super) remaining_ms: Option<u64>,
    pub(super) elapsed_ms: u64,
    pub(super) search_budget_was_capped: bool,
    pub(super) boss_budget_was_capped: bool,
}

impl RunSliceResult {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        args: Args,
        request_kind: RunSliceRequestKind,
        generation_start: usize,
        generation_end: usize,
        next_branch_id: usize,
        stop: RunStop,
        frontier: FrontierSummary,
        remaining_ms: Option<u64>,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            contract: RunContract::from_args(args),
            request_kind,
            generation_start,
            generation_end,
            next_branch_id,
            stop,
            frontier,
            selected_branch: None,
            budget: SliceBudgetSummary {
                slice_ms: args.wall_ms,
                remaining_ms,
                elapsed_ms,
                search_budget_was_capped: args.wall_capped_search_budget,
                boss_budget_was_capped: args.wall_capped_boss_budget,
            },
        }
    }

    pub(super) fn with_selected_branch(mut self, branch: &Branch) -> Self {
        self.selected_branch = Some(BranchSummary::from_branch(branch));
        self
    }
}

impl BranchSummary {
    pub(super) fn from_branch(branch: &Branch) -> Self {
        let run = &branch.session.run_state;
        Self {
            branch_id: branch.id,
            parent_id: branch.parent_id,
            status_kind: BranchStatusKind::from_status(&branch.status),
            boundary: status_boundary(&branch.status),
            owner: status_owner(&branch.status),
            act: run.act_num,
            floor: run.floor_num,
            hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            deck_size: run.master_deck.len(),
        }
    }
}

impl BranchStatusKind {
    fn from_status(status: &BranchStatus) -> Self {
        match status {
            BranchStatus::Running { .. } => Self::Running,
            BranchStatus::AwaitingAuto { .. } => Self::AwaitingAuto,
            BranchStatus::Terminal(_) => Self::Terminal,
            BranchStatus::AutomationGap { .. } => Self::AutomationGap,
            BranchStatus::CombatGap { .. } => Self::CombatGap,
            BranchStatus::OperationBudgetExhausted { .. } => Self::OperationBudgetExhausted,
            BranchStatus::BudgetGap { .. } => Self::BudgetGap,
            BranchStatus::ApplyFailed(_) => Self::ApplyFailed,
            BranchStatus::AdvanceFailed(_) => Self::AdvanceFailed,
        }
    }
}

fn status_boundary(status: &BranchStatus) -> Option<String> {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::OperationBudgetExhausted { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => Some(boundary.clone()),
        BranchStatus::Terminal(_)
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => None,
    }
}

fn status_owner(status: &BranchStatus) -> Option<String> {
    match status {
        BranchStatus::Running { owner, .. } => Some(owner_label(*owner).to_string()),
        _ => None,
    }
}

fn owner_label(owner: Owner) -> &'static str {
    match owner {
        Owner::NeowStart => "neow_start",
        Owner::CardReward => "card_reward",
        Owner::BossRelic => "boss_relic",
        Owner::Event(_) => "event",
        Owner::RewardTiny => "reward_tiny",
        Owner::ShopTiny => "shop_tiny",
        Owner::Campfire => "campfire",
        Owner::RunChoice => "run_choice",
    }
}

impl RunStop {
    pub(super) fn from_stopped_branch_status(
        generation: usize,
        branch_id: usize,
        status: &BranchStatus,
    ) -> Option<Self> {
        Some(match status {
            BranchStatus::Terminal(outcome) => Self::Real(RealStop::Terminal {
                generation,
                branch_id,
                outcome: *outcome,
            }),
            BranchStatus::AutomationGap { boundary, site } => Self::Real(RealStop::AutomationGap {
                generation,
                branch_id,
                boundary: boundary.clone(),
                site: *site,
            }),
            BranchStatus::CombatGap { boundary, reason } => Self::Real(RealStop::CombatGap {
                generation,
                branch_id,
                boundary: boundary.clone(),
                reason: reason.clone(),
            }),
            BranchStatus::OperationBudgetExhausted { boundary, reason } => {
                Self::Real(RealStop::OperationBudgetExhausted {
                    generation,
                    branch_id,
                    boundary: boundary.clone(),
                    reason: reason.clone(),
                })
            }
            BranchStatus::BudgetGap { boundary, reason } => Self::Real(RealStop::BudgetGap {
                generation,
                branch_id,
                boundary: boundary.clone(),
                reason: reason.clone(),
            }),
            BranchStatus::ApplyFailed(reason) => Self::Real(RealStop::ApplyFailed {
                generation,
                branch_id,
                reason: reason.clone(),
            }),
            BranchStatus::AdvanceFailed(reason) => Self::Real(RealStop::AdvanceFailed {
                generation,
                branch_id,
                reason: reason.clone(),
            }),
            BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => {
                return None;
            }
        })
    }
}

impl FrontierSummary {
    pub(super) fn from_statuses<'a>(statuses: impl IntoIterator<Item = &'a BranchStatus>) -> Self {
        let mut summary = Self::default();
        for status in statuses {
            summary.total_count += 1;
            if status.is_resumable() {
                summary.running_count += 1;
            }
            if status.is_expandable_now() {
                summary.expandable_count += 1;
            }
            if matches!(status, BranchStatus::Terminal(_)) {
                summary.terminal_count += 1;
            }
            if matches!(
                status,
                BranchStatus::AutomationGap { .. }
                    | BranchStatus::CombatGap { .. }
                    | BranchStatus::OperationBudgetExhausted { .. }
                    | BranchStatus::BudgetGap { .. }
                    | BranchStatus::ApplyFailed(_)
                    | BranchStatus::AdvanceFailed(_)
            ) {
                summary.gap_count += 1;
            }
        }
        summary
    }

    pub(super) fn from_branches<'a>(branches: impl IntoIterator<Item = &'a Branch>) -> Self {
        Self::from_statuses(branches.into_iter().map(|branch| &branch.status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BranchStatus;

    #[test]
    fn run_stop_classifies_combat_gap_status() {
        let status = BranchStatus::CombatGap {
            boundary: "A2F32 Combat".to_string(),
            reason: "combat search did not find win".to_string(),
        };

        let stop = RunStop::from_stopped_branch_status(7, 42, &status).unwrap();

        assert_eq!(
            stop,
            RunStop::Real(RealStop::CombatGap {
                generation: 7,
                branch_id: 42,
                boundary: "A2F32 Combat".to_string(),
                reason: "combat search did not find win".to_string(),
            })
        );
    }

    #[test]
    fn run_stop_does_not_classify_resumable_status_as_real_stop() {
        let status = BranchStatus::Running {
            boundary: "Reward".to_string(),
            owner: crate::Owner::CardReward,
        };

        assert!(RunStop::from_stopped_branch_status(7, 42, &status).is_none());
    }

    #[test]
    fn frontier_summary_counts_current_branch_shapes() {
        let statuses = [
            BranchStatus::Running {
                boundary: "Reward".to_string(),
                owner: crate::Owner::CardReward,
            },
            BranchStatus::AwaitingAuto {
                boundary: "Combat".to_string(),
                reason: "auto capture".to_string(),
            },
            BranchStatus::Terminal(crate::TerminalOutcome::Defeat),
            BranchStatus::CombatGap {
                boundary: "Combat".to_string(),
                reason: "no win".to_string(),
            },
        ];

        let summary = FrontierSummary::from_statuses(statuses.iter());

        assert_eq!(summary.total_count, 4);
        assert_eq!(summary.running_count, 2);
        assert_eq!(summary.expandable_count, 1);
        assert_eq!(summary.terminal_count, 1);
        assert_eq!(summary.gap_count, 1);
    }

    #[test]
    fn run_slice_result_projects_args_into_contract_and_budget_summary() {
        let args = crate::Args {
            seed: 12,
            ascension: 3,
            objective: crate::run_contract::RunObjective::FirstVictory,
            generations: 4,
            max_branches: 5,
            auto_ops: 6,
            search_nodes: 7,
            search_ms: 8,
            rescue_search_nodes: 9,
            rescue_search_ms: 10,
            boss_search_nodes: 11,
            boss_search_ms: 12,
            wall_ms: Some(13),
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: true,
            wall_capped_boss_budget: true,
        };

        let result = RunSliceResult::new(
            args,
            RunSliceRequestKind::Start,
            1,
            2,
            99,
            RunStop::SoftPause(SoftPause::SliceDeadline {
                generation: 2,
                frontier_running_count: 1,
            }),
            FrontierSummary {
                total_count: 2,
                running_count: 1,
                expandable_count: 1,
                terminal_count: 0,
                gap_count: 1,
            },
            Some(3),
            10,
        );

        assert_eq!(result.contract.game.seed, 12);
        assert_eq!(result.request_kind, RunSliceRequestKind::Start);
        assert_eq!(result.generation_start, 1);
        assert_eq!(result.generation_end, 2);
        assert_eq!(result.next_branch_id, 99);
        assert_eq!(result.frontier.running_count, 1);
        assert_eq!(result.budget.slice_ms, Some(13));
        assert_eq!(result.budget.remaining_ms, Some(3));
        assert_eq!(result.budget.elapsed_ms, 10);
        assert!(result.budget.search_budget_was_capped);
        assert!(result.budget.boss_budget_was_capped);
    }
}
