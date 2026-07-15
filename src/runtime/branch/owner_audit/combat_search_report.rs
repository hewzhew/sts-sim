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
    pub(super) engine_fingerprint: String,
    pub(super) candidate_tier: Option<String>,
    pub(super) selected: bool,
    pub(super) incumbent_reason: String,
    pub(super) combat_final_hp: Option<i32>,
    pub(super) run_hp: Option<i32>,
    pub(super) potions_used: Option<u32>,
    pub(super) turns: Option<u32>,
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
    pub(super) engine_fingerprint: String,
    pub(super) candidate_tier: Option<String>,
    pub(super) selected: bool,
    pub(super) incumbent_reason: String,
    pub(super) combat_final_hp: Option<i32>,
    pub(super) run_hp: Option<i32>,
    pub(super) potions_used: Option<u32>,
    pub(super) turns: Option<u32>,
}

pub(super) fn combat_portfolio_report(
    args: Args,
    status: BranchStatus,
    attempts: Vec<CombatSearchLaneReport>,
) -> CombatSearchPortfolioReport {
    let action_keys = attempts
        .iter()
        .find(|attempt| attempt.selected)
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
        engine_fingerprint: input.engine_fingerprint,
        candidate_tier: input.candidate_tier,
        selected: input.selected,
        incumbent_reason: input.incumbent_reason,
        combat_final_hp: input.combat_final_hp,
        run_hp: input.run_hp,
        potions_used: input.potions_used,
        turns: input.turns,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(
        label: &'static str,
        action: &'static str,
        selected: bool,
    ) -> CombatSearchLaneReport {
        CombatSearchLaneReport {
            label,
            status: CombatSearchPortfolioStatus::Advanced("PostCombat".to_string()),
            max_nodes: 10,
            wall_ms: 20,
            potion_policy: "semantic",
            max_potions_used: Some(2),
            action_keys: vec![action.to_string()],
            engine_fingerprint: format!("engine-{label}"),
            candidate_tier: Some("reserve_compliant_complete_win".to_string()),
            selected,
            incumbent_reason: if selected {
                "strict_resource_dominance".to_string()
            } else {
                "replaced_by_better_candidate".to_string()
            },
            combat_final_hp: Some(if selected { 48 } else { 38 }),
            run_hp: Some(if selected { 48 } else { 38 }),
            potions_used: Some(2),
            turns: Some(5),
        }
    }

    #[test]
    fn portfolio_actions_come_from_selected_attempt_not_last_attempt() {
        let mut args = sts_simulator::runtime::branch::default_branch_args(1);
        args.boss_search_nodes = 10;
        args.boss_search_ms = 20;
        let report = combat_portfolio_report(
            args,
            BranchStatus::AwaitingAuto {
                boundary: "PostCombat".to_string(),
                reason: "accepted".to_string(),
            },
            vec![
                attempt("first", "first-action", false),
                attempt("selected", "selected-action", true),
                attempt("last", "last-action", false),
            ],
        );

        assert_eq!(report.action_keys, vec!["selected-action"]);
    }
}
