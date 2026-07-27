use super::*;

#[derive(Debug, Args)]
pub(super) struct DepthBeamTurnAuditArgs {
    #[arg(long)]
    case: PathBuf,
    /// Lab-only typed semantic action-order artifact. The artifact may
    /// reorder legal actions but cannot remove them or claim an outcome.
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    #[arg(long, default_value_t = 20_000)]
    max_applied_transitions: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 32)]
    partial_beam_width: usize,
    #[arg(long, default_value_t = 6)]
    retained_per_view: usize,
    #[arg(long, default_value_t = 32)]
    max_atomic_depth: usize,
    #[arg(long, default_value_t = 256)]
    max_structured_members_per_family: usize,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long)]
    watch_exact_state_hash: Vec<String>,
    #[arg(long, default_value_t = 64)]
    limit: usize,
}

pub(super) fn run_turn(args: DepthBeamTurnAuditArgs) -> Result<(), String> {
    let DepthBeamTurnAuditArgs {
        case,
        action_imitation_artifact,
        max_applied_transitions,
        wall_ms,
        partial_beam_width,
        retained_per_view,
        max_atomic_depth,
        max_structured_members_per_family,
        max_engine_steps_per_transition,
        watch_exact_state_hash,
        limit,
    } = args;
    let case = load_combat_case(&case)?;
    let root = CombatDecisionRoot::new(case.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let policy = action_imitation_artifact
        .as_deref()
        .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
        .transpose()?
        .unwrap_or_else(existing_combat_knowledge_policy_v1);
    let report = generate_depth_beam_turn_options(
        root,
        DepthBeamTurnConfig {
            generator: TurnOptionGeneratorConfig {
                max_engine_steps_per_transition,
                ..TurnOptionGeneratorConfig::default()
            },
            partial_beam_width,
            retained_per_view,
            max_atomic_depth,
            max_structured_members_per_family,
        },
        DepthBeamTurnBudget {
            max_applied_transitions,
            max_engine_steps: max_applied_transitions
                .saturating_mul(max_engine_steps_per_transition.max(1)),
            deadline: Some(Instant::now() + Duration::from_millis(wall_ms)),
        },
        policy.clone(),
        &EngineCombatStepper,
    );
    let option_json = |option: &sts_combat_planner::CompleteTurnOption| {
        json!({
            "exact_successor_hash": option.exact_successor_hash(),
            "boundary": format!("{:?}", option.boundary()),
            "action_count": option.actions().len(),
            "negative_log_policy": option.negative_log_policy(),
            "final_hp": option.exact_successor().combat.entities.player.current_hp,
            "state_guides": policy.state_guides(option.exact_successor()).into_iter().map(|guide| json!({
                "lane": guide.lane.value(),
                "components": guide.rank.components(),
            })).collect::<Vec<_>>(),
            "actions": option.actions().iter().map(|action| json!({
                "input": action.input,
                "expected_successor_hash": action.expected_successor_hash,
            })).collect::<Vec<_>>(),
        })
    };
    let watched = report
        .options
        .iter()
        .filter(|option| {
            watch_exact_state_hash
                .iter()
                .any(|hash| hash == option.exact_successor_hash())
        })
        .map(option_json)
        .collect::<Vec<_>>();
    let options = report
        .options
        .iter()
        .take(limit)
        .map(option_json)
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema_name": "OracleDepthBeamTurnAuditV1",
        "schema_version": 1,
        "behavioral_scope": "read_only_no_search_seeding",
        "status": format!("{:?}", report.status),
        "config": {
            "max_applied_transitions": max_applied_transitions,
            "wall_ms": wall_ms,
            "partial_beam_width": partial_beam_width,
            "retained_per_view": retained_per_view,
            "max_atomic_depth": max_atomic_depth,
            "max_structured_members_per_family": max_structured_members_per_family,
            "max_engine_steps_per_transition": max_engine_steps_per_transition,
            "action_imitation_artifact": action_imitation_artifact,
        },
        "counters": {
            "expanded_partial_states": report.counters.expanded_partial_states,
            "applied_transitions": report.counters.applied_transitions,
            "engine_steps": report.counters.engine_steps,
            "unique_partial_states": report.counters.unique_partial_states,
            "duplicate_exact_successors": report.counters.duplicate_exact_successors,
            "completed_turn_options": report.counters.completed_turn_options,
            "retained_partial_states": report.counters.retained_partial_states,
            "pruned_partial_states": report.counters.pruned_partial_states,
            "maximum_atomic_depth": report.counters.maximum_atomic_depth,
            "truncated_structured_families": report.counters.truncated_structured_families,
        },
        "gap_count": report.gaps.len(),
        "watched": watched,
        "layers": report.layers.iter().map(|layer| json!({
            "atomic_depth": layer.atomic_depth,
            "expanded_partial_states": layer.expanded_partial_states,
            "generated_unique_partial_states": layer.generated_unique_partial_states,
            "retained_partial_states": layer.retained_partial_states,
            "retained_exact_state_hashes": layer.retained_exact_state_hashes,
            "new_completed_turn_options": layer.new_completed_turn_options,
        })).collect::<Vec<_>>(),
        "options": options,
    }))
}

#[derive(Debug, Args)]
pub(super) struct DepthBeamAgendaAuditArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    /// Lab control: apply the state-conditioned learned action order at
    /// every simulated player turn instead of only the search root turn.
    #[arg(long, requires = "action_imitation_artifact")]
    action_imitation_all_turns: bool,
    #[arg(long)]
    value_prototype_artifact: Option<PathBuf>,
    #[arg(long, default_value_t = 500_000)]
    max_applied_transitions: usize,
    #[arg(long, default_value_t = 60_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 128)]
    partial_beam_width: usize,
    #[arg(long, default_value_t = 8)]
    partial_retained_per_view: usize,
    #[arg(long, default_value_t = 32)]
    max_atomic_depth: usize,
    #[arg(long, default_value_t = 4_096)]
    max_applied_transitions_per_parent: usize,
    #[arg(long, default_value_t = 256)]
    max_structured_members_per_family: usize,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long)]
    watch_exact_state_hash: Vec<String>,
    /// Exact terminal witness segments used only to label known boundary
    /// membership in the report. They never affect generation or ranking.
    #[arg(long)]
    diagnostic_corridor_actions: Vec<PathBuf>,
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
}

pub(super) fn run_agenda(args: DepthBeamAgendaAuditArgs) -> Result<(), String> {
    let DepthBeamAgendaAuditArgs {
        case,
        action_imitation_artifact,
        action_imitation_all_turns,
        value_prototype_artifact,
        max_applied_transitions,
        wall_ms,
        partial_beam_width,
        partial_retained_per_view,
        max_atomic_depth,
        max_applied_transitions_per_parent,
        max_structured_members_per_family,
        max_engine_steps_per_transition,
        watch_exact_state_hash,
        diagnostic_corridor_actions,
        export_witness_actions,
    } = args;
    let loaded = load_combat_case(&case)?;
    let diagnostic_corridor = if diagnostic_corridor_actions.is_empty() {
        None
    } else {
        Some(load_exact_turn_corridor(
            &case,
            &diagnostic_corridor_actions,
            max_engine_steps_per_transition,
        )?)
    };
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let root_player_turn = loaded.position.combat.turn.turn_count;
    let root = CombatDecisionRoot::new(loaded.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let base_policy = existing_combat_knowledge_policy_v1();
    let policy = if let Some(path) = action_imitation_artifact.as_deref() {
        let learned = load_action_imitation_policy(path, base_policy.clone())?;
        if action_imitation_all_turns {
            learned
        } else {
            root_player_turn_action_policy_v1(root_player_turn, learned, base_policy)
        }
    } else {
        base_policy
    };
    let (policy, value_report, boundary_guide_lane) =
        if let Some(path) = value_prototype_artifact.as_deref() {
            let artifact = load_value_prototype(path)?;
            let report = artifact.report();
            (
                combat_value_prototype_policy_v1(policy, &artifact),
                Some(report),
                Some(GUIDE_LEARNED_BOUNDARY_VALUE),
            )
        } else {
            (policy, None, None)
        };
    let started = Instant::now();
    let report = search_depth_beam_agenda_witness(
        root,
        DepthBeamAgendaConfig {
            turn: DepthBeamTurnConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                partial_beam_width,
                retained_per_view: partial_retained_per_view,
                max_atomic_depth,
                max_structured_members_per_family,
            },
            boundary_guide_lane,
            max_applied_transitions_per_parent,
        },
        DepthBeamAgendaBudget {
            max_applied_transitions,
            max_engine_steps: max_applied_transitions
                .saturating_mul(max_engine_steps_per_transition.max(1)),
            deadline: Some(Instant::now() + Duration::from_millis(wall_ms)),
        },
        policy,
        &EngineCombatStepper,
    );
    if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), report.witness.as_ref())
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let inputs = witness
            .actions
            .iter()
            .map(|action| action.input.clone())
            .collect::<Vec<_>>();
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    let watched_frontier = report
        .frontier_exact_state_hashes
        .iter()
        .filter(|hash| watch_exact_state_hash.contains(hash))
        .cloned()
        .collect::<Vec<_>>();
    let expanded_hashes = report
        .expanded_parent_exact_state_hashes
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let frontier_hashes = report
        .frontier_exact_state_hashes
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let diagnostic_corridor_membership = diagnostic_corridor.as_ref().map(|corridor| {
        corridor
            .positions_by_rank
            .iter()
            .enumerate()
            .map(|(rank, position)| {
                let hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v2(
                    &position.engine,
                    &position.combat,
                );
                json!({
                    "rank": rank,
                    "player_turn": position.combat.turn.turn_count,
                    "exact_state_hash": hash,
                    "membership": if expanded_hashes.contains(hash.as_str()) {
                        "expanded"
                    } else if frontier_hashes.contains(hash.as_str()) {
                        "frontier"
                    } else {
                        "missing"
                    },
                })
            })
            .collect::<Vec<_>>()
    });
    print_json(&json!({
        "schema_name": "OracleDepthBeamAgendaAuditV1",
        "schema_version": 1,
        "behavioral_scope": "lab_only_no_v2_donor",
        "case": case,
        "runtime": oracle_lab_runtime_identity(),
        "elapsed_ms": started.elapsed().as_millis(),
        "status": format!("{:?}", report.status),
        "config": {
            "action_imitation_artifact": action_imitation_artifact,
            "action_imitation_scope": action_imitation_artifact.as_ref().map(|_| {
                if action_imitation_all_turns {
                    "all_simulated_player_turns"
                } else {
                    "root_player_turn_only"
                }
            }),
            "value_prototype_artifact": value_prototype_artifact,
            "value_prototype": value_report,
            "boundary_guide_lane": boundary_guide_lane.map(CombatGuideLaneId::value),
            "partial_beam_width": partial_beam_width,
            "partial_retained_per_view": partial_retained_per_view,
            "max_atomic_depth": max_atomic_depth,
            "max_applied_transitions_per_parent": max_applied_transitions_per_parent,
            "max_structured_members_per_family": max_structured_members_per_family,
            "diagnostic_corridor_actions": diagnostic_corridor_actions,
        },
        "budget": {
            "max_applied_transitions": max_applied_transitions,
            "wall_ms": wall_ms,
            "max_engine_steps_per_transition": max_engine_steps_per_transition,
        },
        "counters": {
            "applied_transitions": report.counters.applied_transitions,
            "engine_steps": report.counters.engine_steps,
            "expanded_parents": report.counters.expanded_parents,
            "partially_generated_parents": report.counters.partially_generated_parents,
            "generated_complete_turn_options": report.counters.generated_complete_turn_options,
            "unique_boundary_states": report.counters.unique_boundary_states,
            "duplicate_boundary_states": report.counters.duplicate_boundary_states,
            "peak_agenda_states": report.counters.peak_agenda_states,
        },
        "frontier_states": report.frontier_exact_state_hashes.len(),
        "expanded_parent_states": report.expanded_parent_exact_state_hashes.len(),
        "watched_frontier": watched_frontier,
        "diagnostic_corridor_membership": diagnostic_corridor_membership,
        "exported_witness_actions": report.witness.is_some()
            .then_some(export_witness_actions.as_ref())
            .flatten(),
        "witness": report.witness.as_ref().map(|witness| json!({
            "final_hp": witness.final_position.combat.entities.player.current_hp,
            "hp_loss": initial_hp.saturating_sub(
                witness.final_position.combat.entities.player.current_hp,
            ),
            "action_count": witness.actions.len(),
            "negative_log_policy": witness.negative_log_policy,
        })),
    }))
}
