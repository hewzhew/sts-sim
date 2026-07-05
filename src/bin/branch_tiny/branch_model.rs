use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlSession,
};
use sts_simulator::state::events::EventId;

use super::branch_path::BranchPathStep;
use super::combat_search_report::CombatSearchPortfolioReport;

#[derive(Clone)]
pub(super) struct Branch {
    pub(super) id: usize,
    pub(super) parent_id: Option<usize>,
    pub(super) path: Vec<BranchPathStep>,
    pub(super) session: RunControlSession,
    pub(super) status: BranchStatus,
    pub(super) combat_portfolio: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}

#[derive(Clone)]
pub(super) enum BranchStatus {
    Running {
        boundary: String,
        owner: Owner,
    },
    AwaitingAuto {
        boundary: String,
        reason: String,
    },
    Terminal(TerminalOutcome),
    AutomationGap {
        boundary: String,
        site: BoundarySite,
    },
    CombatGap {
        boundary: String,
        reason: String,
    },
    OperationBudgetExhausted {
        boundary: String,
        reason: String,
    },
    BudgetGap {
        boundary: String,
        reason: String,
    },
    ApplyFailed(String),
    AdvanceFailed(String),
}

impl BranchStatus {
    pub(super) fn is_resumable(&self) -> bool {
        matches!(
            self,
            BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. }
        )
    }

    pub(super) fn is_expandable_now(&self) -> bool {
        matches!(self, BranchStatus::Running { .. })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum TerminalOutcome {
    Victory,
    Defeat,
}

impl TerminalOutcome {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum Owner {
    NeowStart,
    CardReward,
    BossRelic,
    Event(EventId),
    RewardTiny,
    ShopTiny,
    Campfire,
    RunChoice,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum BoundarySite {
    Event(EventId),
    Reward,
    Shop,
    Route,
    Campfire,
    BossRelic,
    RunChoice,
    Treasure,
    Terminal,
    Unknown,
}
