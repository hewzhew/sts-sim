use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser};
use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_capture::load_combat_capture_v1;
use sts_simulator::eval::combat_search_v2::{
    compare_combat_search_v2_rollout_policies, load_combat_search_v2_benchmark,
    load_combat_search_v2_snapshot, load_combat_search_v2_start, run_combat_search_v2_benchmark,
    run_combat_search_v2_loaded_start, CombatSearchV2RunOptions,
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

    #[arg(long, value_parser = parse_potion_policy)]
    potion_policy: Option<CombatSearchV2PotionPolicy>,

    #[arg(long)]
    max_potions_used: Option<u32>,

    #[arg(long, value_parser = parse_rollout_policy)]
    rollout_policy: Option<CombatSearchV2RolloutPolicy>,

    #[arg(long)]
    compare_rollout: Option<String>,

    #[arg(long)]
    explain_case: Option<String>,

    #[arg(long)]
    rollout_max_evaluations: Option<usize>,

    #[arg(long)]
    rollout_max_actions: Option<usize>,

    #[arg(long, value_parser = parse_turn_plan_policy)]
    turn_plan_policy: Option<CombatSearchV2TurnPlanPolicy>,

    #[arg(long)]
    validate_only: bool,

    #[arg(long)]
    gate_only: bool,

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
    if args.compare_rollout.is_some() && args.benchmark_spec.is_none() {
        return Err("--compare-rollout requires --benchmark-spec".into());
    }
    if args.compare_rollout.is_some() && args.gate_only {
        return Err("--compare-rollout cannot be used with --gate-only".into());
    }
    if args.compare_rollout.is_some() && args.rollout_policy.is_some() {
        return Err("--compare-rollout cannot be combined with --rollout-policy".into());
    }
    if args.explain_case.is_some() && args.benchmark_spec.is_none() {
        return Err("--explain-case requires --benchmark-spec".into());
    }
    if args.explain_case.is_some() && args.compare_rollout.is_some() {
        return Err("--explain-case cannot be combined with --compare-rollout".into());
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

    let options = CombatSearchV2RunOptions {
        max_nodes: args.max_nodes,
        max_actions_per_line: args.max_actions_per_line,
        max_engine_steps_per_action: args.max_engine_steps_per_action,
        wall_ms: args.wall_ms,
        potion_policy: args.potion_policy,
        max_potions_used: args.max_potions_used,
        rollout_policy: args.rollout_policy,
        rollout_max_evaluations: args.rollout_max_evaluations,
        rollout_max_actions: args.rollout_max_actions,
        turn_plan_policy: args.turn_plan_policy,
    };
    let payload = if let Some(path) = args.benchmark_spec.as_ref() {
        let loaded = load_combat_search_v2_benchmark(path)?;
        if let Some(compare) = args.compare_rollout.as_deref() {
            let (left, right) = parse_rollout_policy_pair(compare)?;
            let run = compare_combat_search_v2_rollout_policies(&loaded, options, left, right);
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
                options.to_search_config(case.start.label.clone()),
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
        let run = run_combat_search_v2_loaded_start(&loaded, options);
        serde_json::to_string_pretty(&run.search_report)?
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

fn parse_rollout_policy(value: &str) -> Result<CombatSearchV2RolloutPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "disabled" | "off" | "none" => Ok(CombatSearchV2RolloutPolicy::Disabled),
        "conservative" | "conservative-no-potion" | "conservative_no_potion" | "no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::ConservativeNoPotion)
        }
        "phase-aware" | "phase_aware" | "phase-aware-no-potion" | "phase_aware_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::PhaseAwareNoPotion)
        }
        _ => Err(format!(
            "invalid rollout policy '{value}', expected disabled|conservative_no_potion|phase_aware_no_potion"
        )),
    }
}

fn parse_potion_policy(value: &str) -> Result<CombatSearchV2PotionPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "never" => Ok(CombatSearchV2PotionPolicy::Never),
        "all" | "all_legal_potion_actions" => Ok(CombatSearchV2PotionPolicy::All),
        "semantic"
        | "semantic-budgeted"
        | "semantic_budgeted"
        | "semantic_budgeted_potion_actions" => Ok(CombatSearchV2PotionPolicy::SemanticBudgeted),
        _ => Err(format!(
            "invalid potion policy '{value}', expected never|all|semantic"
        )),
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
        _ => Err(format!(
            "invalid turn plan policy '{value}', expected diagnostic_only|root_frontier_seed"
        )),
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
