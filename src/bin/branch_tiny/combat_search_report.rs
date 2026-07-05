use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;

use super::branch_status_view;
use super::{Args, BranchStatus, TerminalOutcome};

#[derive(Clone)]
pub(super) struct CombatSearchPortfolioReport {
    pub(super) status: CombatSearchPortfolioStatus,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) action_keys: Vec<String>,
    pub(super) attempts: Vec<CombatSearchLaneReport>,
}

#[derive(Clone)]
pub(super) struct CombatSearchLaneReport {
    pub(super) label: &'static str,
    pub(super) status: CombatSearchPortfolioStatus,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) potion_policy: &'static str,
    pub(super) max_potions_used: Option<u32>,
    pub(super) action_keys: Vec<String>,
}

#[derive(Clone)]
pub(super) enum CombatSearchPortfolioStatus {
    Failed(String),
    Advanced(String),
    Terminal(TerminalOutcome),
}

pub(super) struct CombatSearchLaneReportInput {
    pub(super) label: &'static str,
    pub(super) status: BranchStatus,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub(super) max_potions_used: Option<u32>,
    pub(super) action_keys: Vec<String>,
}

pub(super) fn combat_portfolio_report(
    args: Args,
    status: BranchStatus,
    attempts: Vec<CombatSearchLaneReport>,
) -> CombatSearchPortfolioReport {
    let action_keys = attempts
        .last()
        .map(|attempt| attempt.action_keys.clone())
        .unwrap_or_default();
    let status = combat_portfolio_status(&status);
    CombatSearchPortfolioReport {
        status,
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        action_keys,
        attempts,
    }
}

pub(super) fn combat_portfolio_attempt_report(
    input: CombatSearchLaneReportInput,
) -> CombatSearchLaneReport {
    CombatSearchLaneReport {
        label: input.label,
        status: combat_portfolio_status(&input.status),
        max_nodes: input.max_nodes,
        wall_ms: input.wall_ms,
        potion_policy: potion_policy_label(input.potion_policy),
        max_potions_used: input.max_potions_used,
        action_keys: input.action_keys,
    }
}

fn combat_portfolio_status(status: &BranchStatus) -> CombatSearchPortfolioStatus {
    match status {
        BranchStatus::CombatGap { reason, .. } => {
            CombatSearchPortfolioStatus::Failed(reason.clone())
        }
        BranchStatus::ApplyFailed(err)
        | BranchStatus::AdvanceFailed(err)
        | BranchStatus::OperationBudgetExhausted { reason: err, .. }
        | BranchStatus::BudgetGap { reason: err, .. } => {
            CombatSearchPortfolioStatus::Failed(err.clone())
        }
        BranchStatus::Terminal(TerminalOutcome::Defeat) => {
            CombatSearchPortfolioStatus::Failed("combat portfolio ended in defeat".to_string())
        }
        BranchStatus::Terminal(result) => CombatSearchPortfolioStatus::Terminal(*result),
        _ => CombatSearchPortfolioStatus::Advanced(
            branch_status_view::status_boundary(status).to_string(),
        ),
    }
}

fn potion_policy_label(policy: Option<CombatSearchV2PotionPolicy>) -> &'static str {
    match policy {
        Some(CombatSearchV2PotionPolicy::Never) => "never",
        Some(CombatSearchV2PotionPolicy::All) => "all",
        Some(CombatSearchV2PotionPolicy::SemanticBudgeted) => "semantic",
        None => "default",
    }
}
