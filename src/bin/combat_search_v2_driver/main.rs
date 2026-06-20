use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser};
use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, CombatSearchV2FrontierPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_capture::load_combat_capture_v1;
use sts_simulator::eval::combat_search_v2::{
    compare_combat_search_v2_frontier_policies, compare_combat_search_v2_rollout_policies,
    compare_combat_search_v2_turn_plan_policies, load_combat_search_v2_benchmark,
    load_combat_search_v2_snapshot, load_combat_search_v2_start,
    run_combat_search_guidance_lab_benchmark_v1, run_combat_search_guidance_lab_v1,
    run_combat_search_v2_benchmark, run_combat_search_v2_loaded_start,
    run_combat_turn_plan_guidance_lab_benchmark_v1, run_combat_turn_plan_guidance_lab_v1,
    CombatSearchV2RunOptions,
};
use sts_simulator::eval::fingerprint::StateFingerprintV1;

#[derive(Parser, Debug)]
#[command(
    about = "Combat Search V2 whole-combat runner over exact combat inputs",
    group(
        ArgGroup::new("input")
            .required(true)
            .multiple(false)
            .args(["start_spec", "combat_snapshot", "benchmark_spec"])
    )
)]
struct Args {
    #[arg(long)]
    start_spec: Option<PathBuf>,

    #[arg(long)]
    combat_snapshot: Option<PathBuf>,

    #[arg(long)]
    benchmark_spec: Option<PathBuf>,

    #[arg(long)]
    max_nodes: Option<usize>,

    #[arg(long)]
    max_actions_per_line: Option<usize>,

    #[arg(long)]
    max_engine_steps_per_action: Option<usize>,

    #[arg(long)]
    wall_ms: Option<u64>,

    #[arg(long)]
    max_hp_loss: Option<String>,

    #[arg(long, value_parser = parse_driver_potion_policy)]
    potion_policy: Option<DriverPotionPolicy>,

    #[arg(long)]
    max_potions_used: Option<u32>,

    #[arg(long, value_parser = parse_rollout_policy)]
    rollout_policy: Option<CombatSearchV2RolloutPolicy>,

    #[arg(long)]
    compare_rollout: Option<String>,

    #[arg(long)]
    compare_turn_plan: Option<String>,

    #[arg(long)]
    compare_frontier: Option<String>,

    #[arg(long)]
    explain_case: Option<String>,

    #[arg(long)]
    rollout_max_evaluations: Option<usize>,

    #[arg(long)]
    rollout_max_actions: Option<usize>,

    #[arg(long)]
    rollout_beam_width: Option<usize>,

    #[arg(long, value_parser = parse_turn_plan_policy)]
    turn_plan_policy: Option<CombatSearchV2TurnPlanPolicy>,

    #[arg(long, value_parser = parse_frontier_policy)]
    frontier_policy: Option<CombatSearchV2FrontierPolicy>,

    #[arg(long)]
    validate_only: bool,

    #[arg(long)]
    gate_only: bool,

    #[arg(long)]
    guidance_lab: bool,

    #[arg(long)]
    turn_plan_guidance_lab: bool,

    #[arg(long)]
    guidance_lab_max_cases: Option<usize>,

    #[arg(long)]
    probe_max_nodes: Option<usize>,

    #[arg(long)]
    probe_wall_ms: Option<u64>,

    #[arg(long)]
    turn_plan_probe_max_inner_nodes: Option<usize>,

    #[arg(long)]
    turn_plan_probe_max_end_states: Option<usize>,

    #[arg(long)]
    turn_plan_probe_per_bucket_limit: Option<usize>,

    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.gate_only && args.benchmark_spec.is_none() {
        return Err("--gate-only requires --benchmark-spec".into());
    }
    if args.gate_only && args.validate_only {
        return Err("--gate-only cannot be used with --validate-only".into());
    }
    if args.guidance_lab && args.turn_plan_guidance_lab {
        return Err("--guidance-lab cannot be combined with --turn-plan-guidance-lab".into());
    }
    if (args.guidance_lab || args.turn_plan_guidance_lab) && args.validate_only {
        return Err("--guidance-lab modes cannot be used with --validate-only".into());
    }
    if (args.guidance_lab || args.turn_plan_guidance_lab) && args.gate_only {
        return Err("--guidance-lab modes cannot be used with --gate-only".into());
    }
    if !args.guidance_lab
        && !args.turn_plan_guidance_lab
        && (args.guidance_lab_max_cases.is_some()
            || args.probe_max_nodes.is_some()
            || args.probe_wall_ms.is_some()
            || args.turn_plan_probe_max_inner_nodes.is_some()
            || args.turn_plan_probe_max_end_states.is_some()
            || args.turn_plan_probe_per_bucket_limit.is_some())
    {
        return Err(
            "--guidance-lab-max-cases, --probe-*, and --turn-plan-probe-* require a guidance lab mode"
                .into(),
        );
    }
    if (args.guidance_lab || args.turn_plan_guidance_lab)
        && (args.compare_rollout.is_some()
            || args.compare_turn_plan.is_some()
            || args.compare_frontier.is_some()
            || args.explain_case.is_some())
    {
        return Err("--guidance-lab modes cannot be combined with compare/explain modes".into());
    }
    if args.compare_rollout.is_some() && args.benchmark_spec.is_none() {
        return Err("--compare-rollout requires --benchmark-spec".into());
    }
    if args.compare_rollout.is_some() && args.gate_only {
        return Err("--compare-rollout cannot be used with --gate-only".into());
    }
    if args.compare_rollout.is_some() && args.rollout_policy.is_some() {
        return Err("--compare-rollout cannot be combined with --rollout-policy".into());
    }
    if args.compare_turn_plan.is_some() && args.benchmark_spec.is_none() {
        return Err("--compare-turn-plan requires --benchmark-spec".into());
    }
    if args.compare_turn_plan.is_some() && args.gate_only {
        return Err("--compare-turn-plan cannot be used with --gate-only".into());
    }
    if args.compare_turn_plan.is_some() && args.turn_plan_policy.is_some() {
        return Err("--compare-turn-plan cannot be combined with --turn-plan-policy".into());
    }
    if args.compare_turn_plan.is_some() && args.compare_rollout.is_some() {
        return Err("--compare-turn-plan cannot be combined with --compare-rollout".into());
    }
    if args.compare_frontier.is_some() && args.benchmark_spec.is_none() {
        return Err("--compare-frontier requires --benchmark-spec".into());
    }
    if args.compare_frontier.is_some() && args.gate_only {
        return Err("--compare-frontier cannot be used with --gate-only".into());
    }
    if args.compare_frontier.is_some() && args.frontier_policy.is_some() {
        return Err("--compare-frontier cannot be combined with --frontier-policy".into());
    }
    if args.compare_frontier.is_some()
        && (args.compare_rollout.is_some() || args.compare_turn_plan.is_some())
    {
        return Err("--compare-frontier cannot be combined with other compare options".into());
    }
    if args.explain_case.is_some() && args.benchmark_spec.is_none() {
        return Err("--explain-case requires --benchmark-spec".into());
    }
    if args.explain_case.is_some() && args.compare_rollout.is_some() {
        return Err("--explain-case cannot be combined with --compare-rollout".into());
    }
    if args.explain_case.is_some() && args.compare_turn_plan.is_some() {
        return Err("--explain-case cannot be combined with --compare-turn-plan".into());
    }
    if args.explain_case.is_some() && args.compare_frontier.is_some() {
        return Err("--explain-case cannot be combined with --compare-frontier".into());
    }
    if args.explain_case.is_some() && args.gate_only {
        return Err("--explain-case cannot be combined with --gate-only".into());
    }
    if args.explain_case.is_some() && args.validate_only {
        return Err("--explain-case cannot be combined with --validate-only".into());
    }
    if args.validate_only {
        let payload = validate_input_payload(&args)?;
        write_or_print(args.output.as_ref(), &payload)?;
        return Ok(());
    }
    let stop_on_win_hp_loss_at_most = match args.max_hp_loss.as_deref() {
        Some(value) => parse_max_hp_loss(value)?,
        None => None,
    };

    let (potion_policy, high_stakes_semantic_potions) = match args.potion_policy {
        Some(DriverPotionPolicy::Search(policy)) => (Some(policy), false),
        Some(DriverPotionPolicy::HighStakesAuto) => (None, true),
        None => (None, false),
    };

    let options = CombatSearchV2RunOptions {
        max_nodes: args.max_nodes,
        max_actions_per_line: args.max_actions_per_line,
        max_engine_steps_per_action: args.max_engine_steps_per_action,
        wall_ms: args.wall_ms,
        stop_on_win_hp_loss_at_most,
        potion_policy,
        max_potions_used: args.max_potions_used,
        high_stakes_semantic_potions,
        rollout_policy: args.rollout_policy,
        rollout_max_evaluations: args.rollout_max_evaluations,
        rollout_max_actions: args.rollout_max_actions,
        rollout_beam_width: args.rollout_beam_width,
        turn_plan_policy: args.turn_plan_policy,
        frontier_policy: args.frontier_policy,
        turn_plan_probe_max_inner_nodes: args.turn_plan_probe_max_inner_nodes,
        turn_plan_probe_max_end_states: args.turn_plan_probe_max_end_states,
        turn_plan_probe_per_bucket_limit: args.turn_plan_probe_per_bucket_limit,
    };
    let payload = if let Some(path) = args.benchmark_spec.as_ref() {
        let loaded = load_combat_search_v2_benchmark(path)?;
        if args.guidance_lab || args.turn_plan_guidance_lab {
            let mut child_options = options.clone();
            if args.probe_max_nodes.is_some() {
                child_options.max_nodes = args.probe_max_nodes;
            }
            if args.probe_wall_ms.is_some() {
                child_options.wall_ms = args.probe_wall_ms;
            }
            if args.turn_plan_guidance_lab {
                let report = run_combat_turn_plan_guidance_lab_benchmark_v1(
                    &loaded,
                    options,
                    child_options,
                    args.guidance_lab_max_cases,
                );
                serde_json::to_string_pretty(&report)?
            } else {
                let report = run_combat_search_guidance_lab_benchmark_v1(
                    &loaded,
                    options,
                    child_options,
                    args.guidance_lab_max_cases,
                );
                serde_json::to_string_pretty(&report)?
            }
        } else if let Some(compare) = args.compare_rollout.as_deref() {
            let (left, right) = parse_rollout_policy_pair(compare)?;
            let run = compare_combat_search_v2_rollout_policies(&loaded, options, left, right);
            serde_json::to_string_pretty(&run)?
        } else if let Some(compare) = args.compare_turn_plan.as_deref() {
            let (left, right) = parse_turn_plan_policy_pair(compare)?;
            let run = compare_combat_search_v2_turn_plan_policies(&loaded, options, left, right);
            serde_json::to_string_pretty(&run)?
        } else if let Some(compare) = args.compare_frontier.as_deref() {
            let (left, right) = parse_frontier_policy_pair(compare)?;
            let run = compare_combat_search_v2_frontier_policies(&loaded, options, left, right);
            serde_json::to_string_pretty(&run)?
        } else if let Some(case_id) = args.explain_case.as_deref() {
            let case = loaded
                .cases
                .iter()
                .find(|case| case.id == case_id)
                .ok_or_else(|| format!("benchmark case '{case_id}' not found"))?;
            let decision = explain_combat_search_v2_initial_decision(
                &case.start.position.engine,
                &case.start.position.combat,
                options
                    .to_search_config_for_position(case.start.label.clone(), &case.start.position),
            );
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_name": "CombatSearchV2BenchmarkDecisionMicroscopeReport",
                "schema_version": 1,
                "benchmark_name": loaded.name.clone(),
                "case_id": case.id.clone(),
                "input_kind": case.input.kind,
                "input_path": case.input.path.display().to_string(),
                "baseline": case.baseline.clone(),
                "baseline_path": case.baseline_path.as_ref().map(|path| path.display().to_string()),
                "decision": decision,
                "notes": [
                    "explain-case is diagnostic-only and does not write artifacts",
                    "use it to inspect the first selected action before changing search policy"
                ],
            }))?
        } else {
            let run = run_combat_search_v2_benchmark(&loaded, options);
            if args.gate_only {
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_name": "CombatSearchV2BenchmarkGateOnlyReport",
                    "schema_version": 1,
                    "benchmark_name": run.benchmark_name,
                    "case_count": run.case_count,
                    "summary": run.summary,
                    "gate": run.gate,
                }))?
            } else {
                serde_json::to_string_pretty(&run)?
            }
        }
    } else {
        let loaded = if let Some(path) = args.combat_snapshot.as_ref() {
            load_combat_search_v2_snapshot(path)?
        } else {
            let path = args
                .start_spec
                .as_ref()
                .expect("clap requires exactly one input");
            load_combat_search_v2_start(path)?
        };
        if args.guidance_lab || args.turn_plan_guidance_lab {
            let mut child_options = options.clone();
            if args.probe_max_nodes.is_some() {
                child_options.max_nodes = args.probe_max_nodes;
            }
            if args.probe_wall_ms.is_some() {
                child_options.wall_ms = args.probe_wall_ms;
            }
            if args.turn_plan_guidance_lab {
                let report = run_combat_turn_plan_guidance_lab_v1(&loaded, options, child_options);
                serde_json::to_string_pretty(&report)?
            } else {
                let report = run_combat_search_guidance_lab_v1(&loaded, options, child_options);
                serde_json::to_string_pretty(&report)?
            }
        } else {
            let run = run_combat_search_v2_loaded_start(&loaded, options);
            serde_json::to_string_pretty(&run.search_report)?
        }
    };
    write_or_print(args.output.as_ref(), &payload)?;
    Ok(())
}

fn parse_rollout_policy_pair(
    value: &str,
) -> Result<(CombatSearchV2RolloutPolicy, CombatSearchV2RolloutPolicy), String> {
    let mut parts = value.split(',').map(str::trim);
    let left = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "compare-rollout requires LEFT,RIGHT".to_string())
        .and_then(parse_rollout_policy)?;
    let right = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "compare-rollout requires LEFT,RIGHT".to_string())
        .and_then(parse_rollout_policy)?;
    if parts.next().is_some() {
        return Err("compare-rollout requires exactly two comma-separated policies".to_string());
    }
    Ok((left, right))
}

fn parse_turn_plan_policy_pair(
    value: &str,
) -> Result<(CombatSearchV2TurnPlanPolicy, CombatSearchV2TurnPlanPolicy), String> {
    let mut parts = value.split(',').map(str::trim);
    let left = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "compare-turn-plan requires LEFT,RIGHT".to_string())
        .and_then(parse_turn_plan_policy)?;
    let right = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "compare-turn-plan requires LEFT,RIGHT".to_string())
        .and_then(parse_turn_plan_policy)?;
    if parts.next().is_some() {
        return Err("compare-turn-plan requires exactly two comma-separated policies".to_string());
    }
    Ok((left, right))
}

fn parse_frontier_policy_pair(
    value: &str,
) -> Result<(CombatSearchV2FrontierPolicy, CombatSearchV2FrontierPolicy), String> {
    let mut parts = value.split(',').map(str::trim);
    let left = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "compare-frontier requires LEFT,RIGHT".to_string())
        .and_then(parse_frontier_policy)?;
    let right = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| "compare-frontier requires LEFT,RIGHT".to_string())
        .and_then(parse_frontier_policy)?;
    if parts.next().is_some() {
        return Err("compare-frontier requires exactly two comma-separated policies".to_string());
    }
    Ok((left, right))
}

fn parse_rollout_policy(value: &str) -> Result<CombatSearchV2RolloutPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "disabled" | "off" | "none" => Ok(CombatSearchV2RolloutPolicy::Disabled),
        "adaptive"
        | "adaptive-no-potion"
        | "adaptive_no_potion"
        | "enemy-mechanics-adaptive-no-potion"
        | "enemy_mechanics_adaptive_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)
        }
        "conservative" | "conservative-no-potion" | "conservative_no_potion" | "no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::ConservativeNoPotion)
        }
        "phase-aware" | "phase_aware" | "phase-aware-no-potion" | "phase_aware_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::PhaseAwareNoPotion)
        }
        "turn-beam" | "turn_beam" | "turn-beam-no-potion" | "turn_beam_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::TurnBeamNoPotion)
        }
        _ => Err(format!(
            "invalid rollout policy '{value}', expected disabled|enemy_mechanics_adaptive_no_potion|conservative_no_potion|phase_aware_no_potion|turn_beam_no_potion"
        )),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverPotionPolicy {
    Search(CombatSearchV2PotionPolicy),
    HighStakesAuto,
}

fn parse_driver_potion_policy(value: &str) -> Result<DriverPotionPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "never" => Ok(DriverPotionPolicy::Search(
            CombatSearchV2PotionPolicy::Never,
        )),
        "all" | "all_legal_potion_actions" => {
            Ok(DriverPotionPolicy::Search(CombatSearchV2PotionPolicy::All))
        }
        "semantic"
        | "semantic-budgeted"
        | "semantic_budgeted"
        | "semantic_budgeted_potion_actions" => Ok(DriverPotionPolicy::Search(
            CombatSearchV2PotionPolicy::SemanticBudgeted,
        )),
        "auto" | "high_stakes" | "high-stakes" | "high_stakes_semantic" => {
            Ok(DriverPotionPolicy::HighStakesAuto)
        }
        _ => Err(format!(
            "invalid potion policy '{value}', expected never|all|semantic|auto"
        )),
    }
}

fn parse_max_hp_loss(value: &str) -> Result<Option<u32>, String> {
    match value.to_ascii_lowercase().as_str() {
        "off" | "none" | "disabled" => Ok(None),
        _ => value.parse::<u32>().map(Some).map_err(|_| {
            format!("invalid max hp loss '{value}', expected a non-negative integer or off")
        }),
    }
}

fn parse_turn_plan_policy(value: &str) -> Result<CombatSearchV2TurnPlanPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "diagnostic" | "diagnostic-only" | "diagnostic_only" | "off" => {
            Ok(CombatSearchV2TurnPlanPolicy::DiagnosticOnly)
        }
        "root-seed" | "root_seed" | "root-frontier-seed" | "root_frontier_seed" | "seed" => {
            Ok(CombatSearchV2TurnPlanPolicy::RootFrontierSeed)
        }
        "turn-boundary-frontier-seed"
        | "turn_boundary_frontier_seed"
        | "turn-boundary-seed"
        | "turn_boundary_seed" => Ok(CombatSearchV2TurnPlanPolicy::TurnBoundaryFrontierSeed),
        "tactical-enemy-turn-boundary-frontier-seed"
        | "tactical_enemy_turn_boundary_frontier_seed"
        | "tactical-turn-boundary-seed"
        | "tactical_turn_boundary_seed"
        | "tactical-seed"
        | "tactical_seed"
        | "support-enemy-turn-boundary-frontier-seed"
        | "support_enemy_turn_boundary_frontier_seed"
        | "support-turn-boundary-seed"
        | "support_turn_boundary_seed"
        | "support-seed"
        | "support_seed" => {
            Ok(CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed)
        }
        _ => Err(format!(
            "invalid turn plan policy '{value}', expected diagnostic_only|root_frontier_seed|turn_boundary_frontier_seed|tactical_enemy_turn_boundary_frontier_seed"
        )),
    }
}

fn parse_frontier_policy(value: &str) -> Result<CombatSearchV2FrontierPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "single" | "single_queue" | "single-queue" => Ok(CombatSearchV2FrontierPolicy::SingleQueue),
        "round_robin"
        | "round-robin"
        | "round_robin_eval_buckets"
        | "round-robin-eval-buckets"
        | "eval_buckets"
        | "eval-buckets" => Ok(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets),
        _ => Err(format!(
            "invalid frontier policy '{value}', expected single_queue|round_robin_eval_buckets"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_turn_plan_policy_pair_accepts_diagnostic_and_seed() {
        assert_eq!(
            parse_turn_plan_policy_pair("diagnostic_only,root_frontier_seed")
                .expect("pair should parse"),
            (
                CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                CombatSearchV2TurnPlanPolicy::RootFrontierSeed
            )
        );
    }

    #[test]
    fn parse_turn_plan_policy_accepts_turn_boundary_seed() {
        assert_eq!(
            parse_turn_plan_policy("turn_boundary_frontier_seed").expect("policy should parse"),
            CombatSearchV2TurnPlanPolicy::TurnBoundaryFrontierSeed
        );
    }

    #[test]
    fn parse_turn_plan_policy_accepts_tactical_enemy_seed() {
        assert_eq!(
            parse_turn_plan_policy("tactical_enemy_turn_boundary_frontier_seed")
                .expect("policy should parse"),
            CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed
        );
    }

    #[test]
    fn parse_turn_plan_policy_keeps_support_enemy_alias() {
        assert_eq!(
            parse_turn_plan_policy("support_enemy_turn_boundary_frontier_seed")
                .expect("policy should parse"),
            CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed
        );
    }

    #[test]
    fn parse_rollout_policy_accepts_turn_beam_no_potion() {
        assert_eq!(
            parse_rollout_policy("turn_beam_no_potion").expect("policy should parse"),
            CombatSearchV2RolloutPolicy::TurnBeamNoPotion
        );
    }

    #[test]
    fn parse_rollout_policy_accepts_adaptive_no_potion() {
        assert_eq!(
            parse_rollout_policy("enemy_mechanics_adaptive_no_potion")
                .expect("policy should parse"),
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
        );
    }

    #[test]
    fn parse_driver_potion_policy_accepts_auto_high_stakes() {
        assert_eq!(
            parse_driver_potion_policy("auto").expect("policy should parse"),
            DriverPotionPolicy::HighStakesAuto
        );
    }

    #[test]
    fn parse_driver_potion_policy_keeps_explicit_semantic_policy() {
        assert_eq!(
            parse_driver_potion_policy("semantic").expect("policy should parse"),
            DriverPotionPolicy::Search(CombatSearchV2PotionPolicy::SemanticBudgeted)
        );
    }

    #[test]
    fn parse_max_hp_loss_accepts_integer_and_off() {
        assert_eq!(
            parse_max_hp_loss("8").expect("hp loss should parse"),
            Some(8)
        );
        assert_eq!(parse_max_hp_loss("off").expect("off should parse"), None);
    }

    #[test]
    fn parse_frontier_policy_pair_accepts_single_and_eval_buckets() {
        assert_eq!(
            parse_frontier_policy_pair("single_queue,round_robin_eval_buckets")
                .expect("pair should parse"),
            (
                CombatSearchV2FrontierPolicy::SingleQueue,
                CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets
            )
        );
    }
}

fn validate_input_payload(args: &Args) -> Result<String, Box<dyn std::error::Error>> {
    let payload = if let Some(path) = args.combat_snapshot.as_ref() {
        let capture = load_combat_capture_v1(path)?;
        serde_json::json!({
            "schema_name": "CombatSearchV2InputValidationReport",
            "schema_version": 1,
            "status": "valid",
            "input_kind": "combat_snapshot",
            "input_path": path.display().to_string(),
            "trust_level": capture.trust_level,
            "provenance": capture.provenance,
            "fingerprints": capture.fingerprints.as_ref().map(compact_fingerprint_report),
            "legal_action_count": capture.legal_actions.as_ref().map(|actions| actions.count),
            "summary": capture.summary,
        })
    } else if let Some(path) = args.benchmark_spec.as_ref() {
        let benchmark = load_combat_search_v2_benchmark(path)?;
        let cases = benchmark
            .cases
            .iter()
            .map(|case| {
                serde_json::json!({
                    "id": case.id,
                    "input_kind": case.input.kind,
                    "input_path": case.input.path.display().to_string(),
                    "trust_level": case.start.artifact_trust_level,
                    "fingerprints": case.start.fingerprints.as_ref().map(compact_fingerprint_report),
                    "expected_fingerprints": case.expected_fingerprints.clone(),
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "schema_name": "CombatSearchV2InputValidationReport",
            "schema_version": 1,
            "status": "valid",
            "input_kind": "benchmark_spec",
            "input_path": path.display().to_string(),
            "benchmark_name": benchmark.name,
            "min_trust_level": benchmark.min_trust_level,
            "case_count": cases.len(),
            "cases": cases,
        })
    } else {
        let path = args
            .start_spec
            .as_ref()
            .expect("clap requires exactly one input");
        let start = load_combat_search_v2_start(path)?;
        serde_json::json!({
            "schema_name": "CombatSearchV2InputValidationReport",
            "schema_version": 1,
            "status": "valid",
            "input_kind": "start_spec",
            "input_path": path.display().to_string(),
            "label": start.label,
            "artifact_trust_level": start.artifact_trust_level,
            "fingerprints": start.fingerprints.as_ref().map(compact_fingerprint_report),
        })
    };
    Ok(serde_json::to_string_pretty(&payload)?)
}

fn compact_fingerprint_report(fingerprints: &StateFingerprintV1) -> serde_json::Value {
    serde_json::json!({
        "boundary": fingerprints.boundary,
        "public_observation_hash": fingerprints.public_observation_hash,
        "legal_candidate_set_hash": fingerprints.legal_candidate_set_hash,
        "legal_candidate_order_hash": fingerprints.legal_candidate_order_hash,
        "exact_state_hash": fingerprints.exact_state_hash,
        "stable_outcome_hash": fingerprints.stable_outcome_hash,
        "rng_boundary": {
            "status": fingerprints.rng_boundary.status,
            "stream_count": fingerprints.rng_boundary.stream_count,
            "digest": fingerprints.rng_boundary.digest,
        }
    })
}

fn write_or_print(path: Option<&PathBuf>, payload: &str) -> Result<(), std::io::Error> {
    if let Some(path) = path {
        ensure_parent_dir(path)?;
        fs::write(path, payload)
    } else {
        println!("{payload}");
        Ok(())
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
