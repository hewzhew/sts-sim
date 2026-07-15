use sts_simulator::eval::run_control::{
    build_decision_surface, render_auto_applied_step_compact_v1, RunControlAutoAppliedStepV1,
    RunControlSession,
};

use super::branch_status_view;
use super::combat_search_report::{CombatSearchPortfolioReport, CombatSearchPortfolioStatus};
use super::owner_model::OwnerChoice;
pub(super) use super::render_choice::{render_candidate_decision_compact, render_timeline_choice};
use super::{render_choice, BoundarySite, Branch, BranchStatus, Owner};

pub(super) fn print_branch_timeline(
    generation: usize,
    branch: &Branch,
    choices: &[OwnerChoice],
    expanded: &[bool],
) {
    println!(
        "\n[{generation:02}] b{:04} A{}F{} {} owner={} hp={}/{} deck={} status={}",
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        branch_status_view::status_boundary(&branch.status),
        status_owner(&branch.status),
        branch.session.run_state.current_hp,
        branch.session.run_state.max_hp,
        branch.session.run_state.master_deck.len(),
        status_label(&branch.status),
    );
    if let Some(previous) = branch.path.last() {
        println!(
            "  arrived: {}",
            render_choice::render_timeline_step(previous)
        );
    }
    print_auto_steps(&branch.auto_steps);
    if let Some(report) = branch.combat_portfolio.as_ref() {
        print_combat_portfolio(report);
    }
    print_reward_gap_detail(&branch.session, &branch.status);
    if choices.is_empty() {
        return;
    }
    println!("  choices:");
    for (rank, choice) in choices.iter().enumerate() {
        let marker = if expanded.get(rank).copied().unwrap_or(false) {
            ">"
        } else {
            " "
        };
        println!(
            "  {marker} {:>2}. {}",
            rank + 1,
            render_choice::render_timeline_choice(choice)
        );
    }
    let expanded_count = expanded.iter().filter(|expanded| **expanded).count();
    if expanded_count == 0 && choices.iter().all(|choice| !choice.auto_expand_allowed()) {
        let reason = choices
            .iter()
            .find_map(|choice| choice.inspect_only_reason())
            .unwrap_or("inspect-only owner");
        println!("  expansion: inspect-only ({reason})");
    } else if expanded_count < choices.len() {
        println!(
            "  expansion: expanded {} hidden {}",
            expanded_count,
            choices.len() - expanded_count
        );
    }
}

pub(super) fn one_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .take(160)
        .collect()
}

fn print_auto_steps(steps: &[RunControlAutoAppliedStepV1]) {
    if steps.is_empty() {
        return;
    }
    let shown = steps.iter().take(12).collect::<Vec<_>>();
    println!("  auto:");
    for step in shown {
        println!("    - {}", render_auto_applied_step_compact_v1(step));
    }
    if steps.len() > 12 {
        println!("    ... {} more auto steps", steps.len() - 12);
    }
}

fn print_combat_portfolio(report: &CombatSearchPortfolioReport) {
    println!(
        "  combat_portfolio: {} budget={}nodes/{}ms",
        combat_portfolio_status_label(&report.status),
        report.max_nodes,
        report.wall_ms
    );
    for attempt in &report.attempts {
        println!(
            "    attempt {}: {} selected={} tier={:?} decision={} potion={} max_potions={:?} budget={}nodes/{}ms",
            attempt.label,
            combat_portfolio_status_label(&attempt.status),
            attempt.selected,
            attempt.candidate_tier,
            attempt.incumbent_reason,
            attempt.potion_policy,
            attempt.max_potions_used,
            attempt.max_nodes,
            attempt.wall_ms
        );
        print_action_path("      applied_path", &attempt.action_keys);
    }
}

fn print_action_path(prefix: &str, action_keys: &[String]) {
    if action_keys.is_empty() {
        return;
    }
    let shown = action_keys.iter().take(12).cloned().collect::<Vec<_>>();
    println!("{prefix}: {}", shown.join(" -> "));
    if action_keys.len() > shown.len() {
        println!("      ... {} more actions", action_keys.len() - shown.len());
    }
}

fn combat_portfolio_status_label(status: &CombatSearchPortfolioStatus) -> String {
    match status {
        CombatSearchPortfolioStatus::Failed(reason) => format!("failed ({})", one_line(reason)),
        CombatSearchPortfolioStatus::Advanced(boundary) => format!("combat-win -> {boundary}"),
        CombatSearchPortfolioStatus::Terminal(result) => format!("terminal:{}", result.as_str()),
    }
}

fn print_reward_gap_detail(session: &RunControlSession, status: &BranchStatus) {
    if !matches!(
        status,
        BranchStatus::AutomationGap {
            site: BoundarySite::Reward,
            ..
        }
    ) {
        return;
    }
    let surface = build_decision_surface(session);
    let candidates = super::owner_commands::executable_choices(&surface)
        .into_iter()
        .map(|choice| render_choice::render_timeline_choice(&choice))
        .collect::<Vec<_>>();
    if !candidates.is_empty() {
        println!("    reward_gap_candidates: {}", candidates.join(" | "));
    }
}

fn status_label(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { .. } => "running".to_string(),
        BranchStatus::AwaitingAuto { reason, .. } => {
            format!("awaiting_auto:{}", one_line(reason))
        }
        BranchStatus::Terminal(result) => format!("terminal:{}", result.as_str()),
        BranchStatus::AutomationGap { .. } => "automation_gap".to_string(),
        BranchStatus::CombatGap { reason, .. } => format!("combat_gap:{}", one_line(reason)),
        BranchStatus::OperationBudgetExhausted { reason, .. } => {
            format!("operation_budget:{}", one_line(reason))
        }
        BranchStatus::BudgetGap { reason, .. } => format!("budget_gap:{}", one_line(reason)),
        BranchStatus::ApplyFailed(err) => format!("apply_failed:{}", one_line(err)),
        BranchStatus::AdvanceFailed(err) => format!("advance_failed:{}", one_line(err)),
    }
}

fn status_owner(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { owner, .. } => owner_label(*owner),
        BranchStatus::AwaitingAuto { .. } => "AutoRun".to_string(),
        BranchStatus::AutomationGap { site, .. } => site_label(*site),
        BranchStatus::CombatGap { .. } => "combat_search".to_string(),
        BranchStatus::OperationBudgetExhausted { .. } => "automation_budget".to_string(),
        BranchStatus::BudgetGap { .. } => "automation_budget".to_string(),
        BranchStatus::Terminal(_) => "terminal".to_string(),
        BranchStatus::ApplyFailed(_) => "candidate_apply".to_string(),
        BranchStatus::AdvanceFailed(_) => "automation".to_string(),
    }
}

fn owner_label(owner: Owner) -> String {
    match owner {
        Owner::NeowStart => "NeowStart".to_string(),
        Owner::CardReward => "CardReward".to_string(),
        Owner::BossRelic => "BossRelic".to_string(),
        Owner::Event(event_id) => format!("Event({event_id:?})"),
        Owner::RewardTiny => "RewardTiny".to_string(),
        Owner::ShopTiny => "ShopTiny".to_string(),
        Owner::Campfire => "Campfire".to_string(),
        Owner::RunChoice => "RunChoice".to_string(),
    }
}

fn site_label(site: BoundarySite) -> String {
    match site {
        BoundarySite::Event(event_id) => format!("Event({event_id:?})"),
        BoundarySite::Reward => "Reward".to_string(),
        BoundarySite::Shop => "Shop".to_string(),
        BoundarySite::Route => "Route".to_string(),
        BoundarySite::Campfire => "Campfire".to_string(),
        BoundarySite::BossRelic => "BossRelic".to_string(),
        BoundarySite::RunChoice => "RunChoice".to_string(),
        BoundarySite::Treasure => "Treasure".to_string(),
        BoundarySite::Terminal => "Terminal".to_string(),
        BoundarySite::Unknown => "Unknown".to_string(),
    }
}
