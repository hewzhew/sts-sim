use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use serde_json::json;
use sts_combat_planner::{
    CombatDecisionRoot, LayeredCombatWitnessConfig, LayeredCombatWitnessQuantum,
    LayeredCombatWitnessSession, TurnOptionGeneratorConfig,
};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use sts_oracle_runtime::eval::run_control::existing_combat_knowledge_policy_v1;
use sts_oracle_runtime::sim::combat::EngineCombatStepper;

use super::combat_planning_view::{
    existing_combat_guide_diagnostics, layered_candidate_view_ranks,
};
use super::combat_policy_controls::load_action_imitation_policy;
use super::combat_replay_tools::save_combat_inputs;
use super::{oracle_lab_runtime_identity, print_json};

#[derive(Debug, Args)]
pub(super) struct CombatCaseLayeredArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long, conflicts_with = "guidance_bundle")]
    action_imitation_artifact: Option<PathBuf>,
    /// Optional immutable action-policy plus turn-boundary value package.
    /// This lab control lets the layered search test learned guidance
    /// without changing legality, exact-state ownership, or terminal truth.
    #[arg(long, conflicts_with = "action_imitation_artifact")]
    guidance_bundle: Option<PathBuf>,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 32)]
    beam_width: usize,
    #[arg(long, default_value_t = 6)]
    retained_per_view: usize,
    #[arg(long, default_value_t = 8)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 32)]
    max_turn_layers: usize,
    /// Report where these exact states reside in deferred beam windows
    /// without exporting the complete frontier.
    #[arg(long)]
    watch_exact_state_hash: Vec<String>,
    /// If a replay-verified win is found, save its exact ClientInput list.
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
}

pub(super) fn run(args: CombatCaseLayeredArgs) -> Result<(), String> {
    let CombatCaseLayeredArgs {
        case,
        action_imitation_artifact,
        guidance_bundle,
        max_nodes,
        wall_ms,
        max_engine_steps_per_transition,
        beam_width,
        retained_per_view,
        generation_quantum_work,
        max_turn_layers,
        watch_exact_state_hash,
        export_witness_actions,
    } = args;
    let command_started = Instant::now();
    let loaded = load_combat_case(&case)?;
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let root = CombatDecisionRoot::new(loaded.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let deadline = Instant::now() + Duration::from_millis(wall_ms);
    let config = LayeredCombatWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        beam_width,
        retained_per_view,
        generation_quantum_work,
        max_turn_layers,
    };
    let policy = if let Some(path) = guidance_bundle.as_deref() {
        CombatGuidanceBundleV1::load(path)?.policy(existing_combat_knowledge_policy_v1())?
    } else {
        action_imitation_artifact
            .as_deref()
            .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
            .transpose()?
            .unwrap_or_else(existing_combat_knowledge_policy_v1)
    };
    let diagnostic_policy = policy.clone();
    let mut session = LayeredCombatWitnessSession::with_policy(root, config, policy);
    let report = session.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: max_nodes,
            additional_engine_steps: max_nodes.saturating_mul(max_engine_steps_per_transition),
            deadline: Some(deadline),
        },
        &EngineCombatStepper,
    );
    let mut watched_states = Vec::new();
    for window in session.deferred_windows() {
        for (candidate_index, candidate) in window.candidates.iter().enumerate() {
            if !watch_exact_state_hash.contains(&candidate.exact_state_hash) {
                continue;
            }
            watched_states.push(json!({
                "exact_state_hash": candidate.exact_state_hash,
                "relative_turn_depth": window.relative_turn_depth,
                "window_discrepancy": window.window_discrepancy,
                "source_window_index": window.source_window_index,
                "candidate_index": candidate_index,
                "action_count": candidate.actions.len(),
                "negative_log_policy": candidate.negative_log_policy,
                "view_ranks": layered_candidate_view_ranks(
                    &window.candidates,
                    candidate_index,
                    diagnostic_policy.as_ref(),
                ),
                "guides": existing_combat_guide_diagnostics(&candidate.position),
            }));
        }
    }
    if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), report.witness.as_ref())
    {
        save_combat_inputs(
            path,
            witness.actions.iter().map(|action| action.input.clone()),
        )?;
    }
    let frontier = report
        .frontier
        .iter()
        .map(|state| {
            json!({
                "exact_state_hash": state.exact_state_hash,
                "player_turn": state.position.combat.turn.turn_count,
                "player_hp": state.position.combat.entities.player.current_hp,
                "enemy_hp": state.position.combat.entities.monsters.iter()
                    .map(|monster| monster.current_hp.max(0))
                    .sum::<i32>(),
                "path_action_count": state.actions.len(),
                "negative_log_policy": state.negative_log_policy,
                "guides": existing_combat_guide_diagnostics(&state.position),
            })
        })
        .collect::<Vec<_>>();
    let layers = report
        .layers
        .iter()
        .map(|layer| {
            json!({
                "relative_turn_depth": layer.relative_turn_depth,
                "window_discrepancy": layer.window_discrepancy,
                "source_window_index": layer.source_window_index,
                "player_turn": layer.player_turn,
                "parent_states": layer.parent_states,
                "parent_exact_state_hashes": layer.parent_exact_state_hashes,
                "parent_work": layer.parent_work.iter().map(|parent| json!({
                    "exact_state_hash": parent.exact_state_hash,
                    "generation_work": parent.generation_work,
                    "completed_turn_options": parent.completed_turn_options,
                    "finished": parent.finished,
                })).collect::<Vec<_>>(),
                "expanded_parents": layer.expanded_parents,
                "generation_work": layer.generation_work,
                "completed_turn_options": layer.completed_turn_options,
                "unique_next_turn_states": layer.unique_next_turn_states,
                "duplicate_next_turn_states": layer.duplicate_next_turn_states,
                "retained_next_turn_states": layer.retained_next_turn_states,
                "retained_exact_state_hashes": layer.retained_exact_state_hashes,
                "truncated_parents": layer.truncated_parents,
                "emitted_windows": layer.emitted_windows,
            })
        })
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema_name": "OracleCombatCaseLayeredV1",
        "schema_version": 1,
        "case": case,
        "runtime": oracle_lab_runtime_identity(),
        "mode": {
            "scheduler": "recoverable_turn_synchronous_multi_view_beam",
            "v2_donor_enabled": false,
            "action_imitation_artifact": action_imitation_artifact,
            "guidance_bundle": guidance_bundle,
        },
        "status": format!("{:?}", report.status),
        "elapsed_ms": command_started.elapsed().as_millis(),
        "config": {
            "beam_width": beam_width,
            "retained_per_view": retained_per_view,
            "generation_quantum_work": generation_quantum_work,
            "max_turn_layers": max_turn_layers,
        },
        "budget": {
            "generation_work": max_nodes,
            "wall_ms": wall_ms,
            "max_engine_steps_per_transition": max_engine_steps_per_transition,
        },
        "work": {
            "generation_work": report.counters.generation_work,
            "engine_steps": report.counters.engine_steps,
            "expanded_parents": report.counters.expanded_parents,
            "completed_turn_options": report.counters.completed_turn_options,
            "unique_next_turn_states": report.counters.unique_next_turn_states,
            "duplicate_next_turn_states": report.counters.duplicate_next_turn_states,
            "truncated_parents": report.counters.truncated_parents,
            "completed_layers": report.counters.completed_layers,
            "deferred_windows": report.counters.deferred_windows,
            "recovered_window_expansions": report.counters.recovered_window_expansions,
            "maximum_window_discrepancy": report.counters.maximum_window_discrepancy,
        },
        "layers": layers,
        "frontier": frontier,
        "generation_gap_count": report.generation_gaps.len(),
        "watched_states": watched_states,
        "exported_witness_actions": report.witness.is_some()
            .then_some(export_witness_actions.as_ref())
            .flatten(),
        "witness": report.witness.as_ref().map(|witness| json!({
            "discovery_source": witness.discovery_source,
            "final_hp": witness.final_position.combat.entities.player.current_hp,
            "hp_loss": initial_hp.saturating_sub(
                witness.final_position.combat.entities.player.current_hp,
            ),
            "action_count": witness.actions.len(),
            "negative_log_policy": witness.negative_log_policy,
            "replay_engine_steps": witness.replay_engine_steps,
        })),
    }))
}
