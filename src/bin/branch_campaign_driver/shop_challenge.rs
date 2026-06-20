use std::collections::BTreeMap;

use sts_simulator::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1, ShopCompileModeV1, ShopPlanStepV1,
    ShopPlanV1, ShopPolicyConfigV1,
};
use sts_simulator::eval::branch_experiment::{
    run_branch_experiment_from_session_with_snapshots_v1, BranchExperimentBranchReportV1,
    BranchExperimentBranchStatusV1, BranchExperimentConfigV1,
};
use sts_simulator::eval::run_control::{
    shop_plan_step_input_and_label_v1, AutoCombatCaptureConfig, RunControlCommand,
    RunControlHpLossLimit, RunControlSession,
};
use sts_simulator::state::core::EngineState;

use super::command_inputs::ShopChallengeInput;

pub(super) fn render_checkpoint_shop_plan_challenge_v1(
    seed: u64,
    base_session: &RunControlSession,
    input: &ShopChallengeInput,
) -> Result<String, String> {
    let EngineState::Shop(shop) = &base_session.engine_state else {
        return Err(format!(
            "--challenge-shop-plans requires Shop engine state, got {:?}",
            base_session.engine_state
        ));
    };
    let context = build_shop_decision_context_v1(&base_session.run_state, shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::BranchTopK {
            max_plans: input.challenge_max_plans,
        },
    );
    let plans = projected_shop_plans_v1(&compiled)
        .into_iter()
        .take(input.challenge_max_plans)
        .collect::<Vec<_>>();

    let mut lines = Vec::new();
    lines.push(format!(
        "ShopPlanChallengeV1 seed={seed} act={} floor={} hp={}/{} gold={} plans={} depth={} max_branches={}",
        base_session.run_state.act_num,
        base_session.run_state.floor_num,
        base_session.run_state.current_hp,
        base_session.run_state.max_hp,
        base_session.run_state.gold,
        plans.len(),
        input.challenge_depth,
        input.challenge_max_branches
    ));
    lines.push(format!(
        "context: conversion_pressure={} affordable_purchase_exists={} boss={:?}",
        context.conversion_pressure, context.affordable_purchase_exists, context.need.boss
    ));

    let mut comparison_rows = Vec::new();
    for (idx, plan) in plans.iter().enumerate() {
        lines.push(String::new());
        lines.push(format!(
            "Plan {idx}: {} | source={:?} cost={} overlay=[{}]",
            plan.label,
            plan.source,
            plan.total_gold_spent,
            shop_plan_overlay_tags_v1(plan).join(",")
        ));
        lines.push(format!("  reason: {}", plan.reason));
        let mut session = base_session.clone();
        match apply_shop_plan_then_leave_v1(&mut session, plan) {
            Ok(applied) => {
                lines.push(format!("  applied: {}", applied.join(" -> ")));
                let config = branch_experiment_config_for_shop_challenge_v1(seed, &session, input)?;
                let result = run_branch_experiment_from_session_with_snapshots_v1(session, &config);
                lines.push(format!(
                    "  result: branches={} branch_points={} pruned={} wall_limit={} frontier_limit={}",
                    result.report.branches.len(),
                    result.report.explored_branch_points,
                    result.report.pruned_branch_count,
                    result.report.wall_limit_hit,
                    result.report.frontier_group_limit_hit
                ));
                lines.push(format!(
                    "  statuses: {}",
                    render_status_counts_v1(&result.report.branches)
                ));
                if let Some(best) = best_challenge_branch_v1(&result.report.branches) {
                    lines.push(format!(
                        "  best: {}",
                        render_challenge_branch_summary_v1(best)
                    ));
                    comparison_rows.push(ShopPlanChallengeComparisonRowV1::from_best_branch(
                        idx,
                        plan,
                        &applied,
                        &result.report.branches,
                        best,
                    ));
                } else {
                    comparison_rows.push(ShopPlanChallengeComparisonRowV1::empty_result(
                        idx, plan, &applied,
                    ));
                }
                let stop_reasons = result
                    .report
                    .branches
                    .iter()
                    .map(|branch| branch.stop_reason.clone())
                    .filter(|reason| !reason.is_empty())
                    .collect::<std::collections::BTreeSet<_>>();
                if !stop_reasons.is_empty() {
                    lines.push(format!(
                        "  stop_reasons: {}",
                        stop_reasons.into_iter().collect::<Vec<_>>().join(" | ")
                    ));
                }
            }
            Err(err) => {
                lines.push(format!("  apply_error: {err}"));
                comparison_rows.push(ShopPlanChallengeComparisonRowV1::apply_error(
                    idx, plan, &err,
                ));
            }
        }
    }

    if !comparison_rows.is_empty() {
        comparison_rows.sort_by(|left, right| right.sort_key().cmp(&left.sort_key()));
        lines.push(String::new());
        lines.push("Comparison:".to_string());
        for row in comparison_rows {
            lines.push(format!("  {}", row.render()));
        }
    }

    Ok(lines.join("\n"))
}

#[derive(Clone, Debug)]
struct ShopPlanChallengeComparisonRowV1 {
    plan_index: usize,
    label: String,
    applied: String,
    branches: usize,
    status_counts: String,
    best_act: u8,
    best_floor: i32,
    best_hp: i32,
    best_max_hp: i32,
    best_gold: i32,
    best_deck_count: usize,
    best_frontier: String,
    note: String,
}

impl ShopPlanChallengeComparisonRowV1 {
    fn from_best_branch(
        plan_index: usize,
        plan: &ShopPlanV1,
        applied: &[String],
        branches: &[BranchExperimentBranchReportV1],
        best: &BranchExperimentBranchReportV1,
    ) -> Self {
        Self {
            plan_index,
            label: plan.label.clone(),
            applied: applied.join(" -> "),
            branches: branches.len(),
            status_counts: render_status_counts_v1(branches),
            best_act: best.summary.act,
            best_floor: best.summary.floor,
            best_hp: best.summary.hp,
            best_max_hp: best.summary.max_hp,
            best_gold: best.summary.gold,
            best_deck_count: best.summary.deck_count,
            best_frontier: best.frontier.boundary_title.clone(),
            note: best.stop_reason.clone(),
        }
    }

    fn empty_result(plan_index: usize, plan: &ShopPlanV1, applied: &[String]) -> Self {
        Self {
            plan_index,
            label: plan.label.clone(),
            applied: applied.join(" -> "),
            branches: 0,
            status_counts: "-".to_string(),
            best_act: 0,
            best_floor: 0,
            best_hp: 0,
            best_max_hp: 0,
            best_gold: 0,
            best_deck_count: 0,
            best_frontier: "-".to_string(),
            note: "no rollout branches".to_string(),
        }
    }

    fn apply_error(plan_index: usize, plan: &ShopPlanV1, error: &str) -> Self {
        Self {
            plan_index,
            label: plan.label.clone(),
            applied: "-".to_string(),
            branches: 0,
            status_counts: "ApplyError=1".to_string(),
            best_act: 0,
            best_floor: 0,
            best_hp: 0,
            best_max_hp: 0,
            best_gold: 0,
            best_deck_count: 0,
            best_frontier: "-".to_string(),
            note: error.to_string(),
        }
    }

    fn sort_key(&self) -> (u8, i32, i32, i32, i32) {
        (
            self.best_act,
            self.best_floor,
            self.best_hp,
            self.best_gold,
            -(self.best_deck_count as i32),
        )
    }

    fn render(&self) -> String {
        format!(
            "plan {} | {} | best=A{}F{} HP {}/{} gold {} deck {} {} | branches={} statuses={} | applied={} | note={}",
            self.plan_index,
            self.label,
            self.best_act,
            self.best_floor,
            self.best_hp,
            self.best_max_hp,
            self.best_gold,
            self.best_deck_count,
            self.best_frontier,
            self.branches,
            self.status_counts,
            self.applied,
            self.note
        )
    }
}

fn projected_shop_plans_v1(
    compiled: &sts_simulator::ai::shop_policy_v1::CompiledShopDecisionV1,
) -> Vec<ShopPlanV1> {
    let mut plans = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for projection in &compiled.branch_frontier {
        let Some(candidate) = compiled
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == projection.plan_id)
        else {
            continue;
        };
        if seen.insert(candidate.plan.plan_id.clone()) {
            plans.push(candidate.plan.clone());
        }
    }
    if plans.is_empty() {
        if let Some(projection) = &compiled.rollout_head {
            if let Some(candidate) = compiled
                .candidate_plans
                .iter()
                .find(|candidate| candidate.plan.plan_id == projection.plan_id)
            {
                plans.push(candidate.plan.clone());
            }
        }
    }
    plans
}

fn apply_shop_plan_then_leave_v1(
    session: &mut RunControlSession,
    plan: &ShopPlanV1,
) -> Result<Vec<String>, String> {
    let mut applied = Vec::new();
    for step in &plan.steps {
        let (input, label) = shop_plan_step_input_and_label_v1(step);
        session.apply_command(RunControlCommand::Input(input))?;
        applied.push(label);
    }
    if matches!(session.engine_state, EngineState::Shop(_)) {
        let (input, label) = shop_plan_step_input_and_label_v1(&ShopPlanStepV1::LeaveShop);
        session.apply_command(RunControlCommand::Input(input))?;
        applied.push(label);
    }
    if applied.is_empty() {
        applied.push("no executable shop step".to_string());
    }
    Ok(applied)
}

fn branch_experiment_config_for_shop_challenge_v1(
    seed: u64,
    session: &RunControlSession,
    input: &ShopChallengeInput,
) -> Result<BranchExperimentConfigV1, String> {
    Ok(BranchExperimentConfigV1 {
        seed,
        ascension_level: session.run_state.ascension_level,
        player_class: session.run_state.player_class,
        final_act: input.final_act,
        max_branches: input.challenge_max_branches,
        max_branches_per_frontier_group: Some(input.challenge_max_branches),
        retention_budget_profile: input.retention_budget_profile,
        max_reward_options_per_branch: input.max_reward_options_per_branch,
        max_campfire_options_per_branch: Some(input.max_campfire_options_per_branch),
        max_depth: input.challenge_depth,
        auto_max_operations: input.auto_max_operations,
        experiment_wall_ms: Some(input.experiment_wall_ms),
        search_max_nodes: input.search_max_nodes,
        search_wall_ms: Some(input.search_wall_ms),
        search_max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
        search_options: input.search_options.clone(),
        auto_capture: AutoCombatCaptureConfig::default(),
        include_skip: false,
        include_event_reward_skip: false,
        auto_leave_after_shop_purchase_branch: true,
        defer_branch_settle: true,
        prefix_commands: Vec::new(),
        replay_trace_path: None,
        replay_trace_max_steps: None,
    })
}

fn shop_plan_overlay_tags_v1(plan: &ShopPlanV1) -> Vec<String> {
    let mut tags = Vec::new();
    for step in &plan.steps {
        match step {
            ShopPlanStepV1::RemoveCard { .. } => tags.push("remove_card_plan".to_string()),
            ShopPlanStepV1::BuyRelic { .. } => tags.push("buy_relic_plan".to_string()),
            ShopPlanStepV1::BuyPotion { .. } => tags.push("buy_potion_plan".to_string()),
            ShopPlanStepV1::BuyCard { .. } => tags.push("buy_card_plan".to_string()),
            ShopPlanStepV1::LeaveShop => tags.push("leave_shop_plan".to_string()),
        }
    }
    if tags.is_empty() {
        tags.push("empty_shop_plan".to_string());
    }
    tags.sort();
    tags.dedup();
    tags
}

fn render_status_counts_v1(branches: &[BranchExperimentBranchReportV1]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for branch in branches {
        *counts.entry(format!("{:?}", branch.status)).or_default() += 1;
    }
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .into_iter()
        .map(|(status, count)| format!("{status}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn best_challenge_branch_v1(
    branches: &[BranchExperimentBranchReportV1],
) -> Option<&BranchExperimentBranchReportV1> {
    branches.iter().max_by_key(|branch| {
        (
            branch.summary.act,
            branch.summary.floor,
            status_rank_v1(branch.status),
            branch.summary.hp,
            branch.summary.gold,
        )
    })
}

fn status_rank_v1(status: BranchExperimentBranchStatusV1) -> i32 {
    match status {
        BranchExperimentBranchStatusV1::TerminalVictory => 5,
        BranchExperimentBranchStatusV1::Active => 4,
        BranchExperimentBranchStatusV1::NeedsHumanBoundary => 3,
        BranchExperimentBranchStatusV1::Failed => 2,
        BranchExperimentBranchStatusV1::TerminalDefeat => 1,
        BranchExperimentBranchStatusV1::Pruned => 0,
    }
}

fn render_challenge_branch_summary_v1(branch: &BranchExperimentBranchReportV1) -> String {
    format!(
        "{:?} A{}F{} HP {}/{} gold {} deck {} | {} | stop={}",
        branch.status,
        branch.summary.act,
        branch.summary.floor,
        branch.summary.hp,
        branch.summary.max_hp,
        branch.summary.gold,
        branch.summary.deck_count,
        branch.frontier.boundary_title,
        branch.stop_reason
    )
}
