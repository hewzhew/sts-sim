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
    let read_started = Instant::now();
    let bytes = std::fs::read(&args.case)
        .map_err(|error| format!("cannot read combat case '{}': {error}", args.case.display()))?;
    let read_elapsed_ns = elapsed_nanos(read_started);
    let parse_started = Instant::now();
    let loaded: CombatCaseRoot = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cannot parse combat case '{}': {error}",
            args.case.display()
        )
    })?;
    let parse_elapsed_ns = elapsed_nanos(parse_started);
    if loaded.schema != "combat_case" && loaded.schema != "combat_gap_case" {
        return Err(format!(
            "expected combat_case or combat_gap_case, got {}",
            loaded.schema
        ));
    }

    let setup_started = Instant::now();
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
    let setup_elapsed_ns = elapsed_nanos(setup_started);
    let policy_line_started = Instant::now();
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
    let policy_line_elapsed_ns = elapsed_nanos(policy_line_started);
    let search_started = Instant::now();
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
    let search_elapsed_ns = elapsed_nanos(search_started);

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
        "phase_ns": {
            "read_case": read_elapsed_ns,
            "parse_case": parse_elapsed_ns,
            "setup": setup_elapsed_ns,
            "policy_line": policy_line_elapsed_ns,
            "main_search": search_elapsed_ns,
        },
        "search_counters": {
            "selections": report.counters.selections,
            "node_visits": report.counters.node_visits,
            "generation_work": report.counters.generation_work,
            "engine_steps": report.counters.engine_steps,
            "exact_nodes": report.counters.exact_nodes,
            "exact_edges": report.counters.exact_edges,
            "completed_turn_options": report.counters.completed_turn_options,
            "applied_action_transitions": report.counters.applied_action_transitions,
            "unique_successor_states": report.counters.unique_successor_states,
            "duplicate_exact_successors": report.counters.duplicate_exact_successors,
            "duplicate_successor_edges": report.counters.duplicate_successor_edges,
        },
        "performance_ns": {
            "selection": report.performance_timing.selection_elapsed_ns,
            "generation": report.performance_timing.generation_elapsed_ns,
            "admission": report.performance_timing.admission_elapsed_ns,
            "atomic_expand": report.performance_timing.atomic_expand_elapsed_ns,
            "transition_simulation": report.performance_timing.transition_simulation_elapsed_ns,
            "transition_identity": report.performance_timing.transition_identity_elapsed_ns,
            "transition_admission": report.performance_timing.transition_admission_elapsed_ns,
            "transition_trace": report.performance_timing.transition_trace_elapsed_ns,
            "transition_seen": report.performance_timing.transition_seen_elapsed_ns,
            "transition_publish": report.performance_timing.transition_publish_elapsed_ns,
        },
        "plan_suffix": policy_line_report.as_ref().map(|line| json!({
            "proposed_turns": line.proposed_turns,
            "chosen_action_transitions": line.chosen_action_transitions,
            "rejected_preview_transitions": line.rejected_preview_transitions,
            "deferred_actions": line.deferred_actions,
            "policy_line_engine_steps": line.engine_steps,
            "policy_line_performance_ns": {
                "legal_surface": line.legal_surface_elapsed_ns,
                "policy_ranking": line.policy_ranking_elapsed_ns,
                "transition_preview": line.transition_preview_elapsed_ns,
                "action_identity": line.action_identity_elapsed_ns,
                "plan_annotation": line.plan_annotation_elapsed_ns,
                "successor_admission": line.successor_admission_elapsed_ns,
            },
            "attempts": line.suffix_probe_attempts,
            "generation_work": line.suffix_probe_generation_work,
            "engine_steps": line.suffix_probe_engine_steps,
            "completed_turn_options": line.suffix_probe_completed_turn_options,
            "applied_action_transitions": line.suffix_probe_applied_action_transitions,
            "unique_successor_states": line.suffix_probe_unique_successor_states,
            "performance_ns": line.suffix_probe_performance_timing,
            "setup_elapsed_ns": line.suffix_probe_setup_elapsed_ns,
            "advance_elapsed_ns": line.suffix_probe_advance_elapsed_ns,
            "replay_elapsed_ns": line.suffix_probe_replay_elapsed_ns,
        })),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
