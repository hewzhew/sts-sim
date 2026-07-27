use super::combat_policy_controls::load_action_imitation_policy;
use super::*;

#[derive(Debug, Args)]
pub(super) struct CombatCaseAtomicLevinArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    #[arg(long, default_value_t = 250_000)]
    max_transitions: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 10_000)]
    uniform_exploration_ppm: u32,
    /// Use robust root-LTS with entry into each new player turn as a
    /// structural clue. The q-th observed boundary receives weight 1/q.
    #[arg(long)]
    reroot_player_turn_boundaries: bool,
    /// Diagnostic-only exact states to observe without changing search.
    #[arg(long)]
    watch_state_hash: Vec<String>,
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
}

pub(super) fn run_atomic_levin(args: CombatCaseAtomicLevinArgs) -> Result<(), String> {
    let CombatCaseAtomicLevinArgs {
        case,
        action_imitation_artifact,
        max_transitions,
        wall_ms,
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        reroot_player_turn_boundaries,
        watch_state_hash,
        export_witness_actions,
    } = args;
    let command_started = Instant::now();
    let case_path = case.clone();
    let case = load_combat_case(&case)?;
    let root = CombatDecisionRoot::new(case.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let initial_hp = root.position().combat.entities.player.current_hp;
    let policy = action_imitation_artifact
        .as_deref()
        .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
        .transpose()?
        .unwrap_or_else(existing_combat_knowledge_policy_v1);
    let mut search = AtomicLevinWitnessSession::with_policy(
        root,
        AtomicLevinWitnessConfig {
            max_engine_steps_per_transition,
            uniform_exploration_ppm,
            rerooting: if reroot_player_turn_boundaries {
                AtomicLevinRerooting::PlayerTurnBoundaries
            } else {
                AtomicLevinRerooting::Disabled
            },
            ..AtomicLevinWitnessConfig::default()
        },
        policy,
    );
    for exact_state_hash in &watch_state_hash {
        search.watch_exact_state_hash(exact_state_hash.clone());
    }
    let started = Instant::now();
    let report = search.advance(
        &EngineCombatStepper,
        AtomicLevinWitnessQuantum {
            additional_applied_transitions: max_transitions,
            additional_engine_steps: max_transitions
                .saturating_mul(max_engine_steps_per_transition),
            deadline: Some(started + Duration::from_millis(wall_ms)),
        },
    );
    let elapsed = started.elapsed();
    if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), report.witness.as_ref())
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let actions = witness
            .actions
            .iter()
            .map(|action| action.input.clone())
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?;
        std::fs::write(path, bytes).map_err(|error| error.to_string())?;
    }
    print_json(&serde_json::json!({
            "schema_name": "OracleCombatCaseAtomicLevinV1",
            "schema_version": 1,
            "case": case_path,
            "runtime": oracle_lab_runtime_identity(),
            "mode": {
                "search": "atomic_levin_policy_tree",
                "state_guides": false,
                "complete_turn_generator": false,
                "v2_donor": false,
                "action_imitation_artifact": action_imitation_artifact,
                "uniform_exploration_ppm": uniform_exploration_ppm,
                "rerooting": if reroot_player_turn_boundaries {
                    "player_turn_boundaries"
                } else {
                    "disabled"
                },
            },
            "status": format!("{:?}", report.status),
            "timing_ms": {
                "setup": started.duration_since(command_started).as_millis(),
                "search": elapsed.as_millis(),
                "total_before_print": command_started.elapsed().as_millis(),
            },
            "budget": {
                "max_transitions": max_transitions,
                "wall_ms": wall_ms,
                "max_engine_steps_per_transition": max_engine_steps_per_transition,
            },
            "work": {
                "work_pops": report.after.work_pops,
                "expanded_exact_states": report.after.expanded_exact_states,
                "applied_action_transitions": report.after.applied_action_transitions,
                "engine_steps": report.after.engine_steps,
                "exact_states": report.after.exact_states,
                "reopened_exact_states": report.after.reopened_exact_states,
                "duplicate_or_dominated_successors": report.after.duplicate_or_dominated_successors,
                "structured_inputs_materialized": report.after.structured_inputs_materialized,
                "reroot_points_assigned": report.after.reroot_points_assigned,
                "rerooted_action_transitions": report.after.rerooted_action_transitions,
            },
            "frontier": {
                "entries": report.frontier_entries,
                "max_atomic_depth": report.max_atomic_depth,
                "max_player_turn": report.max_player_turn,
                "unsupported_stable_boundaries": report.unsupported_stable_boundaries,
                "transition_step_limit_gaps": report.transition_step_limit_gaps,
            },
            "watched_states": watch_state_hash.iter().map(|exact_state_hash| {
                let state = search.watched_state(exact_state_hash);
                json!({
                    "exact_state_hash": exact_state_hash,
                    "state": state.map(|state| json!({
                        "discovered": state.discovered,
                        "accepted": state.accepted,
                        "expanded": state.expanded,
                        "first_discovery_after_transitions": state.first_discovery_after_transitions,
                        "first_expansion_after_work_pops": state.first_expansion_after_work_pops,
                        "best_atomic_depth": state.best_atomic_depth,
                        "best_negative_log_policy": state.best_negative_log_policy,
                        "best_levin_log_priority": state.best_levin_log_priority,
                        "reroot_ordinal": state.reroot_ordinal,
                        "reroot_weight": state.reroot_weight,
                    })),
                })
            }).collect::<Vec<_>>(),
            "exported_witness_actions": report.witness.is_some()
                .then_some(export_witness_actions.as_ref())
                .flatten(),
            "witness": report.witness.as_ref().map(|witness| serde_json::json!({
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
