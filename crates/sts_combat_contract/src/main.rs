//! Lightweight exact combat regression runner.
//!
//! This package deliberately excludes run exploration, shops, routes,
//! continuations, and resident-workspace state. It is the fast compilation
//! boundary for replay-verified tactical contracts.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sts_combat_knowledge::existing_combat_knowledge_policy_v1;
use sts_combat_planner::{
    combat_plan_state_guide_policy_v1, CombatDecisionRoot, LocalTurnGraphWitnessConfig,
    LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession, OracleCombatWitnessSatisfaction,
    TurnOptionGeneratorConfig,
};
use sts_core::sim::combat::{CombatPosition, EngineCombatStepper};

#[derive(Debug, Parser)]
#[command(
    name = "combat_contract",
    about = "Run one replay-verified combat contract without compiling the full oracle runtime"
)]
struct Cli {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    typed_plan_guide: bool,
    #[arg(long)]
    plan_compatible_policy_line: bool,
    #[arg(long, default_value_t = 0, requires = "plan_compatible_policy_line")]
    plan_compatible_suffix_work: usize,
    #[arg(long)]
    expect_witness: bool,
    #[arg(long, requires = "expect_witness")]
    expect_min_final_hp: Option<i32>,
    #[arg(long, requires = "plan_compatible_policy_line")]
    expect_max_plan_suffix_work: Option<usize>,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 1_000_000)]
    max_selections: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 4)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 32)]
    max_turn_depth: usize,
}

#[derive(Deserialize)]
struct CombatCaseRoot {
    schema: String,
    position: CombatPosition,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Cli) -> Result<(), String> {
    let started = Instant::now();
    let bytes = std::fs::read(&args.case)
        .map_err(|error| format!("cannot read combat case '{}': {error}", args.case.display()))?;
    let loaded: CombatCaseRoot = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cannot parse combat case '{}': {error}",
            args.case.display()
        )
    })?;
    if loaded.schema != "combat_case" && loaded.schema != "combat_gap_case" {
        return Err(format!(
            "expected combat_case or combat_gap_case, got {}",
            loaded.schema
        ));
    }

    let root = CombatDecisionRoot::new(loaded.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let config = LocalTurnGraphWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: args.max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        generation_quantum_work: args.generation_quantum_work,
        backed_generation_quantum_work: 256,
        initial_expansion_work: 64,
        root_initial_expansion_work: 2_048,
        lookahead_max_evaluations: args.max_nodes.saturating_div(24).max(1),
        lookahead_work_per_evaluation: 24,
        max_turn_depth: args.max_turn_depth,
        satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
    };
    let policy = existing_combat_knowledge_policy_v1();
    let policy = if args.typed_plan_guide {
        combat_plan_state_guide_policy_v1(policy)
    } else {
        policy
    };
    let mut session = LocalTurnGraphWitnessSession::with_policy(root, config, policy);
    let policy_line_report = args
        .plan_compatible_policy_line
        .then(|| {
            session.offer_plan_compatible_policy_line_with_suffix_probes(
                args.max_turn_depth,
                256,
                args.plan_compatible_suffix_work,
                &EngineCombatStepper,
            )
        })
        .transpose()?;
    let report = session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: args.max_selections,
            additional_generation_work: args.max_nodes,
            additional_engine_steps: args
                .max_nodes
                .saturating_mul(args.max_engine_steps_per_transition),
            deadline: Some(Instant::now() + Duration::from_millis(args.wall_ms)),
        },
        &EngineCombatStepper,
    );

    if args.expect_witness && report.witness.is_none() {
        return Err("combat contract failed: no replay-verified witness".to_owned());
    }
    if let Some(minimum) = args.expect_min_final_hp {
        let actual = report
            .witness
            .as_ref()
            .map(|witness| witness.final_position.combat.entities.player.current_hp)
            .ok_or_else(|| "combat contract failed: final HP requires a witness".to_owned())?;
        if actual < minimum {
            return Err(format!(
                "combat contract failed: final HP {actual} is below {minimum}"
            ));
        }
    }
    if let Some(maximum) = args.expect_max_plan_suffix_work {
        let actual = policy_line_report
            .as_ref()
            .map(|line| line.suffix_probe_generation_work)
            .unwrap_or_default();
        if actual > maximum {
            return Err(format!(
                "combat contract failed: plan suffix work {actual} exceeds {maximum}"
            ));
        }
    }

    let witness = report.witness.as_ref();
    let output = json!({
        "schema_name": "CombatCaseContractResultV1",
        "schema_version": 1,
        "status": if args.expect_witness { "passed" } else { "completed" },
        "runner": "lightweight-combat-contract",
        "case": args.case,
        "elapsed_ms": started.elapsed().as_millis(),
        "final_hp": witness.map(|witness| {
            witness.final_position.combat.entities.player.current_hp
        }),
        "witness_actions": witness.map(|witness| witness.actions.len()),
        "plan_suffix": policy_line_report.as_ref().map(|line| json!({
            "attempts": line.suffix_probe_attempts,
            "generation_work": line.suffix_probe_generation_work,
            "engine_steps": line.suffix_probe_engine_steps,
        })),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}
