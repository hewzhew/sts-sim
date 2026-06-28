use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, run_combat_search_v2,
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2Config, CombatSearchV2PotionPolicy,
    CombatSearchV2Report, CombatSearchV2RolloutPolicy, CombatSearchV2TrajectoryReport,
    CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::sim::combat::CombatPosition;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    case: PathBuf,
    #[arg(long, default_value_t = 20_000)]
    nodes: usize,
    #[arg(long, default_value_t = 300)]
    ms: u64,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    search_only: bool,
    #[arg(long)]
    accept_any_win: bool,
    #[arg(long)]
    accept_win_hp_loss: Option<u32>,
    #[arg(long, value_parser = parse_child_rollout_policy)]
    child_rollout: Option<CombatSearchV2ChildRolloutPolicy>,
    #[arg(long, value_parser = parse_rollout_policy)]
    rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    #[arg(long, value_parser = parse_potion_policy)]
    potion_policy: Option<CombatSearchV2PotionPolicy>,
    #[arg(long)]
    max_potions_used: Option<u32>,
    #[arg(long)]
    rollout_actions: Option<usize>,
    #[arg(long)]
    rollout_evaluations: Option<usize>,
    #[arg(long, value_parser = parse_turn_plan_policy)]
    turn_plan_policy: Option<CombatSearchV2TurnPlanPolicy>,
}

#[derive(Deserialize)]
struct CombatGapCase {
    schema: String,
    source: serde_json::Value,
    gap: serde_json::Value,
    run: serde_json::Value,
    combat: serde_json::Value,
    position: CombatPosition,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let probe_started = Instant::now();
    let args = Args::parse();
    let case = load_case(&args.case)?;
    let config = CombatSearchV2Config {
        max_nodes: args.nodes,
        wall_time: Some(Duration::from_millis(args.ms)),
        stop_on_win_hp_loss_at_most: accept_win_limit(&args),
        child_rollout_policy: args
            .child_rollout
            .unwrap_or_else(|| CombatSearchV2Config::default().child_rollout_policy),
        rollout_policy: args
            .rollout_policy
            .unwrap_or_else(|| CombatSearchV2Config::default().rollout_policy),
        rollout_max_actions: args
            .rollout_actions
            .unwrap_or_else(|| CombatSearchV2Config::default().rollout_max_actions),
        rollout_max_evaluations: args
            .rollout_evaluations
            .unwrap_or_else(|| CombatSearchV2Config::default().rollout_max_evaluations),
        potion_policy: args
            .potion_policy
            .unwrap_or_else(|| CombatSearchV2Config::default().potion_policy),
        max_potions_used: args.max_potions_used,
        turn_plan_policy: args
            .turn_plan_policy
            .unwrap_or_else(|| CombatSearchV2Config::default().turn_plan_policy),
        input_label: Some(format!("combat_gap_case:{}", args.case.display())),
        ..CombatSearchV2Config::default()
    };
    let search_started = Instant::now();
    let report = run_combat_search_v2(&case.position.engine, &case.position.combat, config.clone());
    let search_wall_ms = search_started.elapsed().as_millis();
    let microscope_started = Instant::now();
    let microscope = if args.search_only {
        None
    } else {
        Some(explain_combat_search_v2_initial_decision(
            &case.position.engine,
            &case.position.combat,
            config,
        ))
    };
    let microscope_wall_ms = microscope
        .as_ref()
        .map(|_| microscope_started.elapsed().as_millis());
    let post_search_diagnostics_us = report
        .performance
        .shadow_audit_elapsed_us
        .saturating_add(report.performance.root_turn_plan_diagnostics_elapsed_us);
    let budgeted_search_core_ms = report
        .performance
        .total_elapsed_us
        .saturating_sub(post_search_diagnostics_us)
        / 1000;
    let probe_timing = json!({
        "budget_wall_time_ms": args.ms,
        "budgeted_search_core_ms": budgeted_search_core_ms,
        "post_search_diagnostics_ms": post_search_diagnostics_us / 1000,
        "search_report_elapsed_ms": report.stats.elapsed_ms,
        "search_wall_ms": search_wall_ms,
        "microscope_wall_ms": microscope_wall_ms,
        "total_wall_ms": probe_started.elapsed().as_millis(),
    });
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": "combat_gap_probe",
                "case": case_header(&case),
                "probe_timing": probe_timing,
                "search": compact_search_report(&report),
                "initial_decision": microscope,
            }))?
        );
    } else {
        print_human(&case, &report, microscope.as_ref(), &probe_timing);
    }
    Ok(())
}

fn accept_win_limit(args: &Args) -> Option<u32> {
    if args.accept_any_win {
        Some(u32::MAX)
    } else {
        args.accept_win_hp_loss
    }
}

fn parse_child_rollout_policy(value: &str) -> Result<CombatSearchV2ChildRolloutPolicy, String> {
    match value {
        "immediate" => Ok(CombatSearchV2ChildRolloutPolicy::Immediate),
        "lazy" | "lazy_on_pop" => Ok(CombatSearchV2ChildRolloutPolicy::LazyOnPop),
        _ => Err(format!("unknown child rollout policy: {value}")),
    }
}

fn parse_rollout_policy(value: &str) -> Result<CombatSearchV2RolloutPolicy, String> {
    match value {
        "disabled" => Ok(CombatSearchV2RolloutPolicy::Disabled),
        "adaptive" | "enemy_mechanics_adaptive_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)
        }
        "conservative" | "conservative_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::ConservativeNoPotion)
        }
        "phase" | "phase_aware_no_potion" => Ok(CombatSearchV2RolloutPolicy::PhaseAwareNoPotion),
        "turn_beam" | "turn_beam_no_potion" => Ok(CombatSearchV2RolloutPolicy::TurnBeamNoPotion),
        _ => Err(format!("unknown rollout policy: {value}")),
    }
}

fn parse_potion_policy(value: &str) -> Result<CombatSearchV2PotionPolicy, String> {
    match value {
        "never" | "none" => Ok(CombatSearchV2PotionPolicy::Never),
        "all" | "all_legal_potion_actions" => Ok(CombatSearchV2PotionPolicy::All),
        "semantic" | "semantic_budgeted" | "semantic_budgeted_potion_actions" => {
            Ok(CombatSearchV2PotionPolicy::SemanticBudgeted)
        }
        _ => Err(format!("unknown potion policy: {value}")),
    }
}

fn parse_turn_plan_policy(value: &str) -> Result<CombatSearchV2TurnPlanPolicy, String> {
    match value {
        "diagnostic_only" | "none" => Ok(CombatSearchV2TurnPlanPolicy::DiagnosticOnly),
        "root_frontier_seed" => Ok(CombatSearchV2TurnPlanPolicy::RootFrontierSeed),
        "turn_boundary_frontier_seed" => Ok(CombatSearchV2TurnPlanPolicy::TurnBoundaryFrontierSeed),
        "tactical_enemy_turn_boundary_frontier_seed" | "tactical" => {
            Ok(CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed)
        }
        _ => Err(format!("unknown turn plan policy: {value}")),
    }
}

fn load_case(path: &PathBuf) -> Result<CombatGapCase, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let case: CombatGapCase = serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    if case.schema != "combat_gap_case" {
        return Err(format!("expected combat_gap_case, got {}", case.schema));
    }
    Ok(case)
}

fn case_header(case: &CombatGapCase) -> serde_json::Value {
    json!({
        "source": case.source,
        "gap": case.gap,
        "run": case.run,
        "combat": case.combat,
    })
}

fn compact_search_report(report: &CombatSearchV2Report) -> serde_json::Value {
    json!({
        "outcome": report.outcome,
        "budget": report.budget,
        "stats": report.stats,
        "best_complete": report.best_complete_trajectory.as_ref().map(trajectory_summary),
        "best_frontier": report.best_frontier_trajectory.as_ref().map(trajectory_summary),
        "rollout": report.rollout,
        "diagnostics": {
            "branching": report.diagnostics.branching,
            "expansion": report.diagnostics.expansion,
            "turn_plan": report.diagnostics.turn_plan,
            "pruning": report.diagnostics.pruning,
            "frontier": report.diagnostics.frontier,
        },
        "performance": report.performance,
    })
}

fn print_human(
    case: &CombatGapCase,
    report: &CombatSearchV2Report,
    microscope: Option<
        &sts_simulator::ai::combat_search_v2::CombatSearchV2DecisionMicroscopeReport,
    >,
    probe_timing: &serde_json::Value,
) {
    println!("combat_gap_probe");
    println!("  source: {}", one_line(&case.source));
    println!("  original_gap: {}", one_line(&case.gap));
    println!("  run: {}", one_line(&case.run));
    println!("  combat: {}", one_line(&case.combat));
    println!(
        "  outcome: found={} status={:?} reason={}",
        report.outcome.complete_trajectory_found,
        report.outcome.coverage_status,
        report.outcome.coverage_reason
    );
    println!(
        "  budget: nodes={} ms={:?} potion={} max_potions={:?}",
        report.budget.max_nodes,
        report.budget.wall_time_ms,
        report.search_policy.potion_policy,
        report.budget.max_potions_used
    );
    println!(
        "  stats: expanded={} generated={} wins={} losses={} deadline={} node_budget={} elapsed={}ms",
        report.stats.nodes_expanded,
        report.stats.nodes_generated,
        report.stats.terminal_wins,
        report.stats.terminal_losses,
        report.stats.deadline_hit,
        report.stats.node_budget_hit,
        report.stats.elapsed_ms
    );
    println!("  timing: {}", one_line(probe_timing));
    if let Some(best) = report.best_complete_trajectory.as_ref() {
        print_trajectory("best_complete", best);
    } else if let Some(best) = report.best_frontier_trajectory.as_ref() {
        print_trajectory("best_frontier", best);
    }
    println!(
        "  branching: legal_avg={:.2} legal_max={} generated_per_expanded={:.2}",
        report.diagnostics.branching.legal_actions_avg,
        report.diagnostics.branching.legal_actions_max,
        report.diagnostics.branching.nodes_generated_per_expanded
    );
    println!(
        "  turn_plan: root_states={} plans={} max_plans={} inner_nodes={}",
        report.diagnostics.turn_plan.root_states_observed,
        report.diagnostics.turn_plan.total_plans,
        report.diagnostics.turn_plan.max_plans_in_state,
        report.diagnostics.turn_plan.total_inner_nodes_expanded
    );
    if let Some(microscope) = microscope {
        print_initial_decision(microscope);
    }
}

fn print_trajectory(label: &str, trajectory: &CombatSearchV2TrajectoryReport) {
    let actions = trajectory
        .actions
        .iter()
        .take(12)
        .map(|action| action.action_key.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");
    println!(
        "  {label}: terminal={:?} final_hp={} hp_loss={} turns={} actions={}",
        trajectory.terminal,
        trajectory.final_hp,
        trajectory.hp_loss,
        trajectory.turns,
        trajectory.actions.len()
    );
    if !actions.is_empty() {
        println!("    path: {actions}");
    }
    if trajectory.actions.len() > 12 {
        println!("    ... {} more actions", trajectory.actions.len() - 12);
    }
}

fn print_initial_decision(
    microscope: &sts_simulator::ai::combat_search_v2::CombatSearchV2DecisionMicroscopeReport,
) {
    println!(
        "  initial_decision: candidates={} selected={}",
        microscope.candidate_count,
        microscope
            .selected_first_action
            .as_ref()
            .map(|action| action.action_key.as_str())
            .unwrap_or("-")
    );
    for candidate in microscope.candidates.iter().take(8) {
        println!(
            "    - {} role={} selected={} one_step={}",
            candidate.action_key,
            candidate.action_role,
            candidate.selected_by_best_complete,
            one_line(&json!(candidate.one_step))
        );
    }
}

fn trajectory_summary(trajectory: &CombatSearchV2TrajectoryReport) -> serde_json::Value {
    json!({
        "terminal": trajectory.terminal,
        "final_hp": trajectory.final_hp,
        "hp_loss": trajectory.hp_loss,
        "turns": trajectory.turns,
        "actions": trajectory.actions.iter().map(|action| &action.action_key).take(16).collect::<Vec<_>>(),
        "action_count": trajectory.actions.len(),
    })
}

fn one_line(value: &serde_json::Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "<json>".to_string())
        .chars()
        .take(240)
        .collect()
}
