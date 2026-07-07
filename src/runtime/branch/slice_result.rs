use serde::{Deserialize, Serialize};

use super::{Args, BoundarySite, BranchStatus, Owner, RunContract, TerminalOutcome};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunSliceResult {
    pub contract: RunContract,
    pub request_kind: RunSliceRequestKind,
    pub generation_start: usize,
    pub generation_end: usize,
    pub next_branch_id: usize,
    pub stop: RunStop,
    pub frontier: FrontierSummary,
    pub selected_branch: Option<BranchSummary>,
    pub budget: SliceBudgetSummary,
    pub artifacts: ArtifactWriteSummary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RunSliceRequestKind {
    Start,
    ResumeFrontier,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RunStop {
    Real(RealStop),
    SoftPause(SoftPause),
    FrontierExhausted(FrontierExhausted),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RealStop {
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
pub enum SoftPause {
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
pub enum FrontierExhausted {
    NoRunningBranches { generation: usize },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierSummary {
    pub total_count: usize,
    pub running_count: usize,
    pub expandable_count: usize,
    pub terminal_count: usize,
    pub gap_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BranchSummary {
    pub branch_id: usize,
    pub parent_id: Option<usize>,
    pub status_kind: BranchStatusKind,
    pub boundary: Option<String>,
    pub owner: Option<String>,
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BranchStatusKind {
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
pub struct SliceBudgetSummary {
    pub slice_ms: Option<u64>,
    pub remaining_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub search_budget_was_capped: bool,
    pub boss_budget_was_capped: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactWriteSummary {
    pub manifest_written: bool,
    pub frontier_written: bool,
    pub result_written: bool,
    pub path_written: bool,
    pub summary_written: bool,
    pub terminal_written: bool,
    pub combat_case_written: bool,
}

impl ArtifactWriteSummary {
    pub fn merge(&mut self, other: Self) {
        self.manifest_written |= other.manifest_written;
        self.frontier_written |= other.frontier_written;
        self.result_written |= other.result_written;
        self.path_written |= other.path_written;
        self.summary_written |= other.summary_written;
        self.terminal_written |= other.terminal_written;
        self.combat_case_written |= other.combat_case_written;
    }

    pub fn manifest() -> Self {
        Self {
            manifest_written: true,
            ..Self::default()
        }
    }

    pub fn frontier_checkpoint() -> Self {
        Self {
            frontier_written: true,
            ..Self::default()
        }
    }
}

impl RunSliceResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
            artifacts: ArtifactWriteSummary::default(),
        }
    }

    pub fn with_selected_branch_summary(mut self, branch: BranchSummary) -> Self {
        self.selected_branch = Some(branch);
        self
    }

    pub fn with_artifacts(mut self, artifacts: ArtifactWriteSummary) -> Self {
        self.artifacts = artifacts;
        self
    }
}

impl BranchSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        branch_id: usize,
        parent_id: Option<usize>,
        status: &BranchStatus,
        act: u8,
        floor: i32,
        hp: i32,
        max_hp: i32,
        gold: i32,
        deck_size: usize,
    ) -> Self {
        Self {
            branch_id,
            parent_id,
            status_kind: BranchStatusKind::from_status(status),
            boundary: status_boundary(status),
            owner: status_owner(status),
            act,
            floor,
            hp,
            max_hp,
            gold,
            deck_size,
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
    pub fn from_stopped_branch_status(
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
    pub fn from_statuses<'a>(statuses: impl IntoIterator<Item = &'a BranchStatus>) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::{default_branch_args, BoundarySite, BranchStatus, Owner};

    #[test]
    fn run_stop_classifies_public_branch_status() {
        let status = BranchStatus::CombatGap {
            boundary: "A2F32 Combat".to_string(),
            reason: "no win".to_string(),
        };

        let stop = RunStop::from_stopped_branch_status(7, 42, &status).unwrap();

        assert_eq!(
            stop,
            RunStop::Real(RealStop::CombatGap {
                generation: 7,
                branch_id: 42,
                boundary: "A2F32 Combat".to_string(),
                reason: "no win".to_string(),
            })
        );
    }

    #[test]
    fn frontier_summary_counts_public_branch_statuses() {
        let statuses = [
            BranchStatus::Running {
                boundary: "Reward".to_string(),
                owner: Owner::CardReward,
            },
            BranchStatus::AutomationGap {
                boundary: "Event".to_string(),
                site: BoundarySite::Unknown,
            },
        ];

        let summary = FrontierSummary::from_statuses(statuses.iter());

        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.running_count, 1);
        assert_eq!(summary.expandable_count, 1);
        assert_eq!(summary.gap_count, 1);
    }

    #[test]
    fn run_slice_result_is_structured_runtime_output() {
        let mut args = default_branch_args(12);
        args.wall_ms = Some(13);
        args.wall_capped_search_budget = true;

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
        assert_eq!(result.budget.slice_ms, Some(13));
        assert!(result.budget.search_budget_was_capped);
    }
}
