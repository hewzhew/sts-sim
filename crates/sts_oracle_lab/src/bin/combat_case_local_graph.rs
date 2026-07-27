use super::combat_planning_view::combat_plan_transition_portfolio_v1;
use super::combat_trace_view::{
    combat_action_label, compact_combat_trace, compact_local_corridor_report,
};
use super::*;

#[derive(Debug, Args)]
pub(super) struct CombatCaseLocalGraphArgs {
    #[arg(long)]
    case: PathBuf,
    /// Diagnostic control: preserve action-policy weights while removing
    /// every boundary and mid-turn state guide.
    #[arg(long, conflicts_with = "root_turn_anchor_only")]
    anchor_only: bool,
    /// Diagnostic control: use only action-policy anchor service during
    /// the root player turn, then restore all guides at later turns.
    #[arg(long, conflicts_with = "anchor_only")]
    root_turn_anchor_only: bool,
    /// Opt-in capability migration: lazily evaluate selected exact states
    /// with bounded rollout evidence. Rollout actions are never injected.
    #[arg(
        long,
        conflicts_with = "anchor_only",
        conflicts_with = "root_turn_anchor_only"
    )]
    rollout_lookahead: bool,
    /// Optional typed action-order policy distilled from exact witnesses.
    /// It changes guidance only; legality and terminal truth stay exact.
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    /// Optional lab-only turn-boundary value prototypes distilled from an
    /// exact witness. This is a teacher upper-bound control, not production.
    #[arg(long)]
    value_prototype_artifact: Option<PathBuf>,
    /// One immutable, compatibility-checked package containing both the
    /// typed action residual and cross-turn value prototypes.
    #[arg(
        long,
        conflicts_with = "action_imitation_artifact",
        conflicts_with = "value_prototype_artifact"
    )]
    guidance_bundle: Option<PathBuf>,
    /// Replay one verified witness and observe each exact player-turn
    /// boundary without changing policy, guides, or search order.
    #[arg(long)]
    watch_corridor_actions: Vec<PathBuf>,
    /// Attach encounter-owned, typed plan facts to newly materialized
    /// exact turn-boundary edges. Diagnostic only: annotations are not
    /// read by policy, scheduling, pruning, or witness authority.
    #[arg(long)]
    plan_transition_annotations: bool,
    /// Opt-in lab control: add the encounter-owned typed combat-plan
    /// state view as one independent guide lane. Action weights,
    /// legality, exact-state identity and terminal truth remain unchanged.
    #[arg(long, conflicts_with = "anchor_only")]
    typed_plan_guide: bool,
    /// Lab-only control: materialize one exact base-policy mainline at
    /// player-turn boundaries. A typed encounter plan may defer a
    /// prematurely resource-consuming action or prefer a precisely timed
    /// action; all rejected alternatives remain searchable.
    #[arg(long)]
    plan_compatible_policy_line: bool,
    /// Deterministic exact-search work granted immediately before the
    /// plan-compatible line would cross a typed combat-plan milestone.
    /// Zero disables suffix probes.
    #[arg(long, default_value_t = 0, requires = "plan_compatible_policy_line")]
    plan_compatible_suffix_work: usize,
    /// Contract assertion: return a non-zero exit status unless an exact,
    /// replay-verified combat witness is found.
    #[arg(long)]
    expect_witness: bool,
    /// Contract assertion: require the verified witness to finish with at
    /// least this much HP.
    #[arg(long, requires = "expect_witness")]
    expect_min_final_hp: Option<i32>,
    /// Contract assertion: fail if all plan-compatible suffix probes
    /// together consume more exact generation work than this allowance.
    #[arg(long, requires = "plan_compatible_policy_line")]
    expect_max_plan_suffix_work: Option<usize>,
    /// Print only the compact contract result after all requested
    /// assertions pass. This keeps repeat regression checks readable.
    #[arg(long, requires = "expect_witness")]
    contract_only: bool,
    /// Print only a hierarchical performance profile. Parent timings remain
    /// separate from nested generator and transition timings, and rates are
    /// normalized by exact work rather than inferred from wall time alone.
    #[arg(
        long,
        conflicts_with = "contract_only",
        conflicts_with = "readable",
        conflicts_with = "trace"
    )]
    performance_only: bool,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 1_000_000)]
    max_selections: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    /// Diagnostic-only quality mode: retain the first verified witness
    /// and keep searching until the explicit work/deadline allowance.
    #[arg(long)]
    improve_incumbent: bool,
    /// Stop at the first replay-verified witness whose HP loss is at most
    /// this non-negative bound. This exposes the planner's existing
    /// satisfaction contract without collapsing every combat to either
    /// first-win or best-HP search.
    #[arg(long, conflicts_with = "improve_incumbent")]
    max_hp_loss: Option<u32>,
    /// Require the exact search to expend at most this many potion resources.
    /// Every finite limit is enforced during generation, not only when a
    /// terminal witness is accepted.
    #[arg(long)]
    max_potions_used: Option<u32>,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 4)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 32)]
    max_turn_depth: usize,
    /// Diagnostic counterfactual: keep the exact combat state, RNG,
    /// deck, relics and potions, but restore current HP to max HP before
    /// search. This classifies arrival debt; it is never a legal witness
    /// for the original run.
    #[arg(long)]
    full_health: bool,
    /// Include readable, exact replay traces for the deepest survival,
    /// deepest progress, and terminal witness paths.
    #[arg(long)]
    readable: bool,
    /// Print only compact per-turn traces for the deepest states and
    /// witness. Omits raw action hashes and full frontier diagnostics.
    #[arg(long, conflicts_with = "readable")]
    trace: bool,
    /// Report exact graph membership and local service for selected states.
    #[arg(long)]
    watch_exact_state_hash: Vec<String>,
    /// If a replay-verified win is found, save its exact ClientInput list.
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
    /// Save the exact deepest-survival state as a standalone diagnostic
    /// combat case. Inspect `deepest.survival_node.exhausted` before using
    /// it as a segmented-search continuation.
    #[arg(
        long,
        visible_alias = "export-deepest-case",
        conflicts_with = "export_deepest_progress_case"
    )]
    export_deepest_survival_case: Option<PathBuf>,
    /// Save the exact deepest-progress state as a new standalone combat
    /// case instead of the survival envelope.
    #[arg(long, conflicts_with = "export_deepest_survival_case")]
    export_deepest_progress_case: Option<PathBuf>,
}

pub(super) fn run(args: CombatCaseLocalGraphArgs) -> Result<(), String> {
    let CombatCaseLocalGraphArgs {
        case,
        anchor_only,
        root_turn_anchor_only,
        rollout_lookahead,
        action_imitation_artifact,
        value_prototype_artifact,
        guidance_bundle,
        watch_corridor_actions,
        plan_transition_annotations,
        typed_plan_guide,
        plan_compatible_policy_line,
        plan_compatible_suffix_work,
        expect_witness,
        expect_min_final_hp,
        expect_max_plan_suffix_work,
        contract_only,
        performance_only,
        max_nodes,
        max_selections,
        wall_ms,
        improve_incumbent,
        max_hp_loss,
        max_potions_used,
        max_engine_steps_per_transition,
        generation_quantum_work,
        max_turn_depth,
        full_health,
        readable,
        trace,
        watch_exact_state_hash,
        export_witness_actions,
        export_deepest_survival_case,
        export_deepest_progress_case,
    } = args;
    let command_started = Instant::now();
    let mut loaded = load_combat_case(&case)?;
    let original_hp = loaded.position.combat.entities.player.current_hp;
    if full_health {
        loaded.position.combat.entities.player.current_hp =
            loaded.position.combat.entities.player.max_hp;
    }
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let root_player_turn = loaded.position.combat.turn.turn_count;
    let search_root_position = loaded.position.clone();
    let watched_corridor = if watch_corridor_actions.is_empty() {
        None
    } else {
        Some(load_exact_turn_corridor(
            &case,
            &watch_corridor_actions,
            max_engine_steps_per_transition,
        )?)
    };
    let root = CombatDecisionRoot::new(loaded.position.clone())
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let satisfaction = if improve_incumbent {
        OracleCombatWitnessSatisfaction::BudgetOrExhaustion
    } else if let Some(limit) = max_hp_loss {
        OracleCombatWitnessSatisfaction::HpLossAtMost(limit)
    } else {
        OracleCombatWitnessSatisfaction::FirstWitness
    };
    let config = LocalTurnGraphWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition,
            allow_potion_expenditure: max_potions_used != Some(0),
            ..TurnOptionGeneratorConfig::default()
        },
        generation_quantum_work,
        backed_generation_quantum_work: 256,
        initial_expansion_work: 64,
        root_initial_expansion_work: 2_048,
        // Backed search charges every rollout to the same deterministic
        // work allowance as exact generation. The count guard merely
        // prevents more evaluations than that allowance can finance.
        lookahead_max_evaluations: max_nodes.saturating_div(24).max(1),
        lookahead_work_per_evaluation: 24,
        max_turn_depth,
        satisfaction,
        max_potions_used,
    };
    let policy = if let Some(path) = guidance_bundle.as_deref() {
        CombatGuidanceBundleV1::load(path)?.policy(existing_combat_knowledge_policy_v1())?
    } else {
        let policy = action_imitation_artifact
            .as_deref()
            .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
            .transpose()?
            .unwrap_or_else(existing_combat_knowledge_policy_v1);
        if let Some(path) = value_prototype_artifact.as_deref() {
            let artifact = load_value_prototype(path)?;
            combat_value_prototype_policy_v1(policy, &artifact)
        } else {
            policy
        }
    };
    let policy = if anchor_only {
        anchor_only_policy(policy)
    } else if root_turn_anchor_only {
        root_turn_anchor_only_policy(root_player_turn, policy)
    } else {
        policy
    };
    let policy = if typed_plan_guide {
        combat_plan_state_guide_policy_v1(policy)
    } else {
        policy
    };
    let mut session = if rollout_lookahead {
        LocalTurnGraphWitnessSession::with_policy_and_lookahead(
            root,
            config,
            policy,
            existing_combat_rollout_lookahead_v1(),
        )
    } else {
        LocalTurnGraphWitnessSession::with_policy(root, config, policy)
    };
    if plan_transition_annotations {
        session
            .enable_plan_transition_annotations()
            .map_err(|error| {
                format!(
                    "cannot enable plan transition annotations after graph construction: \
                             {error:?}"
                )
            })?;
    }
    let policy_line_report = plan_compatible_policy_line
        .then(|| {
            session.offer_plan_compatible_policy_line_with_suffix_probes(
                max_turn_depth,
                256,
                plan_compatible_suffix_work,
                &EngineCombatStepper,
            )
        })
        .transpose()?;
    let search_started = Instant::now();
    let report = session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: max_selections,
            additional_generation_work: max_nodes,
            additional_engine_steps: max_nodes.saturating_mul(max_engine_steps_per_transition),
            deadline: Some(Instant::now() + Duration::from_millis(wall_ms)),
        },
        &EngineCombatStepper,
    );
    let search_elapsed = search_started.elapsed();
    let search_elapsed_ms = search_elapsed.as_millis();
    if expect_witness && report.witness.is_none() {
        return Err("combat-case contract failed: no replay-verified witness".to_owned());
    }
    if let Some(expected_minimum) = expect_min_final_hp {
        let actual = report
            .witness
            .as_ref()
            .map(|witness| witness.final_position.combat.entities.player.current_hp)
            .ok_or_else(|| {
                "combat-case contract failed: final HP requires a verified witness".to_owned()
            })?;
        if actual < expected_minimum {
            return Err(format!(
                "combat-case contract failed: final HP {actual} is below {expected_minimum}"
            ));
        }
    }
    if let Some(expected_maximum) = expect_max_plan_suffix_work {
        let actual = policy_line_report
            .as_ref()
            .map(|policy_line| policy_line.suffix_probe_generation_work)
            .unwrap_or_default();
        if actual > expected_maximum {
            return Err(format!(
                "combat-case contract failed: plan suffix work {actual} exceeds \
                         {expected_maximum}"
            ));
        }
    }
    if contract_only {
        let witness = report
            .witness
            .as_ref()
            .expect("clap requires --expect-witness");
        return print_json(&json!({
            "schema_name": "CombatCaseContractResultV1",
            "schema_version": 1,
            "status": "passed",
            "case": case,
            "elapsed_ms": command_started.elapsed().as_millis(),
            "final_hp": witness.final_position.combat.entities.player.current_hp,
            "witness_actions": witness.actions.len(),
            "plan_suffix": policy_line_report.as_ref().map(|policy_line| json!({
                "attempts": policy_line.suffix_probe_attempts,
                "generation_work": policy_line.suffix_probe_generation_work,
                "engine_steps": policy_line.suffix_probe_engine_steps,
            })),
        }));
    }
    let mut performance_profile =
        combat_case_performance::local_graph_performance_profile(search_elapsed, &report);
    let performance_profile_object = performance_profile
        .as_object_mut()
        .expect("performance profile must be a JSON object");
    performance_profile_object.insert("case".to_owned(), json!(&case));
    performance_profile_object.insert("status".to_owned(), json!(format!("{:?}", report.status)));
    performance_profile_object.insert(
        "witness".to_owned(),
        report
            .witness
            .as_ref()
            .map(|witness| {
                json!({
                    "final_hp": witness.final_position.combat.entities.player.current_hp,
                    "actions": witness.actions.len(),
                })
            })
            .unwrap_or(Value::Null),
    );
    if performance_only {
        return print_json(&performance_profile);
    }
    let performance_timing = json!({
        "selection_elapsed_ns": report.performance_timing.selection_elapsed_ns,
        "generation_elapsed_ns": report.performance_timing.generation_elapsed_ns,
        "admission_elapsed_ns": report.performance_timing.admission_elapsed_ns,
        "atomic_expand_elapsed_ns": report.performance_timing.atomic_expand_elapsed_ns,
        "transition_simulation_elapsed_ns":
            report.performance_timing.transition_simulation_elapsed_ns,
        "transition_identity_elapsed_ns":
            report.performance_timing.transition_identity_elapsed_ns,
        "transition_key_build_elapsed_ns":
            report.performance_timing.transition_key_build_elapsed_ns,
        "transition_key_index_elapsed_ns":
            report.performance_timing.transition_key_index_elapsed_ns,
        "transition_admission_elapsed_ns":
            report.performance_timing.transition_admission_elapsed_ns,
        "transition_trace_elapsed_ns":
            report.performance_timing.transition_trace_elapsed_ns,
        "transition_seen_elapsed_ns":
            report.performance_timing.transition_seen_elapsed_ns,
        "transition_publish_elapsed_ns":
            report.performance_timing.transition_publish_elapsed_ns,
        "transition_publish_trace_node_elapsed_ns":
            report.performance_timing.transition_publish_trace_node_elapsed_ns,
        "transition_publish_boundary_elapsed_ns":
            report.performance_timing.transition_publish_boundary_elapsed_ns,
        "transition_publish_complete_elapsed_ns":
            report.performance_timing.transition_publish_complete_elapsed_ns,
        "transition_publish_push_elapsed_ns":
            report.performance_timing.transition_publish_push_elapsed_ns,
        "transition_publish_guide_elapsed_ns":
            report.performance_timing.transition_publish_guide_elapsed_ns,
        "transition_publish_retain_elapsed_ns":
            report.performance_timing.transition_publish_retain_elapsed_ns,
        "transition_publish_agenda_elapsed_ns":
            report.performance_timing.transition_publish_agenda_elapsed_ns,
        "admission_root_option_elapsed_ns":
            report.performance_timing.admission_root_option_elapsed_ns,
        "admission_witness_filter_elapsed_ns":
            report.performance_timing.admission_witness_filter_elapsed_ns,
        "admission_witness_replay_elapsed_ns":
            report.performance_timing.admission_witness_replay_elapsed_ns,
        "successor_identity_elapsed_ns":
            report.performance_timing.successor_identity_elapsed_ns,
        "successor_lookup_elapsed_ns":
            report.performance_timing.successor_lookup_elapsed_ns,
        "successor_node_build_elapsed_ns":
            report.performance_timing.successor_node_build_elapsed_ns,
        "successor_edge_elapsed_ns":
            report.performance_timing.successor_edge_elapsed_ns,
        "successor_backup_elapsed_ns":
            report.performance_timing.successor_backup_elapsed_ns,
        "admission_refresh_elapsed_ns":
            report.performance_timing.admission_refresh_elapsed_ns,
    });
    let progress = session.progress_snapshot();
    let root_action_families = session
        .root_action_families()
        .into_iter()
        .map(|family| {
            json!({
                "action": combat_action_label(
                    &search_root_position,
                    &family.first_action,
                ),
                "best_root_negative_log_policy":
                    family.best_root_negative_log_policy,
                "completed_root_turn_options":
                    family.completed_root_turn_options,
                "terminal_wins": family.terminal_wins,
                "terminal_losses": family.terminal_losses,
                "escapes": family.escapes,
                "unique_next_turn_successors":
                    family.unique_next_turn_successors,
                "retained_next_turn_successors":
                    family.retained_next_turn_successors,
                "reachable_exact_states": family.reachable_exact_states,
                "reachable_retained_states":
                    family.reachable_retained_states,
                "reachable_generation_work":
                    family.reachable_generation_work,
                "reachable_completed_turn_options":
                    family.reachable_completed_turn_options,
                "max_player_turn": family.max_player_turn,
                "best_hp_at_max_turn": family.best_hp_at_max_turn,
                "lowest_enemy_hp_at_max_turn":
                    family.lowest_enemy_hp_at_max_turn,
            })
        })
        .collect::<Vec<_>>();
    let include_trace = readable || trace;
    let deepest_survival_trace = include_trace
        .then(|| {
            replay_combat_path(
                search_root_position.clone(),
                &progress.deepest_survival_actions,
                max_engine_steps_per_transition,
            )
        })
        .transpose()?;
    let deepest_progress_trace = include_trace
        .then(|| {
            replay_combat_path(
                search_root_position.clone(),
                &progress.deepest_progress_actions,
                max_engine_steps_per_transition,
            )
        })
        .transpose()?;
    let deepest_survival_node = local_graph_state_snapshot_for_path(
        &session,
        search_root_position.clone(),
        &progress.deepest_survival_actions,
        max_engine_steps_per_transition,
    )?;
    let deepest_progress_node = local_graph_state_snapshot_for_path(
        &session,
        search_root_position.clone(),
        &progress.deepest_progress_actions,
        max_engine_steps_per_transition,
    )?;
    let witness_trace = if include_trace {
        report
            .witness
            .as_ref()
            .map(|witness| {
                replay_combat_path(
                    search_root_position.clone(),
                    &witness.actions,
                    max_engine_steps_per_transition,
                )
            })
            .transpose()?
    } else {
        None
    };
    let watched_states = watch_exact_state_hash
        .iter()
        .map(|hash| {
            json!({
                "exact_state_hash": hash,
                "state": session.state_snapshot_by_exact_hash(hash),
                "incoming_from_root": session.edge_snapshot_by_exact_hashes(
                    &sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                        &search_root_position.engine,
                        &search_root_position.combat,
                    ),
                    hash,
                ),
            })
        })
        .collect::<Vec<_>>();
    let watched_corridor = watched_corridor.as_ref().map(|corridor| {
        let mut ranked_hashes = corridor
            .rank_by_exact_hash
            .iter()
            .map(|(hash, rank)| (*rank, hash))
            .collect::<Vec<_>>();
        ranked_hashes.sort_by_key(|(rank, _)| *rank);
        let states = ranked_hashes
            .iter()
            .enumerate()
            .map(|(index, (rank, hash))| {
                let outgoing_to_next = ranked_hashes.get(index + 1).and_then(|(_, next_hash)| {
                    session.edge_snapshot_by_exact_hashes(hash, next_hash)
                });
                json!({
                    "corridor_rank": rank,
                    "exact_state_hash": hash,
                    "state": session.state_snapshot_by_exact_hash(hash),
                    "outgoing_to_next": outgoing_to_next,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "authority": "diagnostic_only",
            "changes_search_order": false,
            "action_count": corridor.action_count,
            "exact_turn_states": states.len(),
            "terminal_final_hp": corridor.terminal_final_hp,
            "states": states,
        })
    });
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
    let exported_deepest_survival_actions =
        if let Some(path) = export_deepest_survival_case.as_ref() {
            Some(export_descendant_combat_case(
                &loaded,
                &progress.deepest_survival_actions,
                path,
                max_engine_steps_per_transition,
                "local_turn_graph_deepest_survival",
            )?)
        } else {
            None
        };
    let exported_deepest_progress_actions =
        if let Some(path) = export_deepest_progress_case.as_ref() {
            Some(export_descendant_combat_case(
                &loaded,
                &progress.deepest_progress_actions,
                path,
                max_engine_steps_per_transition,
                "local_turn_graph_deepest_progress",
            )?)
        } else {
            None
        };
    let watched_corridor_output = if readable {
        watched_corridor.clone().unwrap_or(Value::Null)
    } else {
        compact_local_corridor_report(watched_corridor.as_ref())
    };
    if trace {
        let compact_survival_trace =
            if progress.deepest_survival_actions == progress.deepest_progress_actions {
                json!({"same_as": "deepest_progress_trace"})
            } else {
                compact_combat_trace(deepest_survival_trace.as_ref())
            };
        let plan_transition_portfolio = plan_transition_annotations
            .then(|| combat_plan_transition_portfolio_v1(&session))
            .unwrap_or(Value::Null);
        return print_json(&json!({
            "schema_name": "LocalTurnGraphCombatTraceV1",
            "schema_version": 1,
            "case": case,
            "status": format!("{:?}", report.status),
            "satisfaction": format!("{satisfaction:?}"),
            "elapsed_ms": command_started.elapsed().as_millis(),
            "counterfactual": {
                "full_health": full_health,
                "original_hp": original_hp,
                "search_hp": initial_hp,
            },
            "work": {
                "generation_work": report.counters.generation_work,
                "exact_nodes": report.counters.exact_nodes,
                "completed_turn_options": report.counters.completed_turn_options,
                "applied_action_transitions": report.counters.applied_action_transitions,
            },
            "root_action_families": root_action_families,
            "plan_compatible_policy_line": policy_line_report,
            "plan_transition_annotations": plan_transition_annotations,
            "plan_transition_portfolio": plan_transition_portfolio,
            "deepest": {
                "progress_state": progress.deepest_progress_state,
                "progress_node": deepest_progress_node,
                "progress_trace": compact_combat_trace(deepest_progress_trace.as_ref()),
                "survival_state": progress.deepest_survival_state,
                "survival_node": deepest_survival_node,
                "survival_trace": compact_survival_trace,
            },
            "witness": report.witness.as_ref().map(|witness| json!({
                "final_hp": witness.final_position.combat.entities.player.current_hp,
                "action_count": witness.actions.len(),
                "trace": compact_combat_trace(witness_trace.as_ref()),
            })),
            "exported_witness_actions": report.witness.is_some()
                .then_some(export_witness_actions.as_ref())
                .flatten(),
            "exported_deepest_survival_case": export_deepest_survival_case,
            "exported_deepest_survival_actions": exported_deepest_survival_actions,
            "exported_deepest_progress_case": export_deepest_progress_case,
            "exported_deepest_progress_actions": exported_deepest_progress_actions,
        }));
    }
    let mut output = json!({
        "schema_name": "LocalTurnGraphCombatSearchReportV1",
        "schema_version": 1,
        "case": case,
        "counterfactual": {
            "full_health": full_health,
            "original_hp": original_hp,
            "search_hp": initial_hp,
        },
        "action_imitation_artifact": action_imitation_artifact,
        "value_prototype_artifact": value_prototype_artifact,
        "guidance_bundle": guidance_bundle,
        "watch_corridor_actions": watch_corridor_actions,
        "satisfaction": format!("{satisfaction:?}"),
        "scheduler": if anchor_only {
            "anchor_only"
        } else if root_turn_anchor_only {
            "root_turn_anchor_then_guides"
        } else if rollout_lookahead {
            "anchor_guides_and_lazy_rollout_lookahead"
        } else {
            "anchor_and_guides"
        },
        "status": format!("{:?}", report.status),
        "elapsed_ms": command_started.elapsed().as_millis(),
        "initial_hp": initial_hp,
        "final_hp": report.witness.as_ref().map(|witness| {
            witness.final_position.combat.entities.player.current_hp
        }),
        "witness_actions": report.witness.as_ref().map(|witness| witness.actions.len()),
        "root": {
            "visits": report.root_visits,
            "generated_options": report.root_generated_options,
            "children": report.root_children,
        },
        "root_action_families": root_action_families,
        "plan_compatible_policy_line": policy_line_report,
        "counters": {
            "selections": report.counters.selections,
            "node_visits": report.counters.node_visits,
            "generation_work": report.counters.generation_work,
            "lookahead_evaluations": report.counters.lookahead_evaluations,
            "lookahead_work": report.counters.lookahead_work,
            "atomic_lookahead_evaluations": report.counters.atomic_lookahead_evaluations,
            "atomic_lookahead_work": report.counters.atomic_lookahead_work,
            "boundary_lookahead_evaluations": report.counters.boundary_lookahead_evaluations,
            "boundary_lookahead_work": report.counters.boundary_lookahead_work,
            "engine_steps": report.counters.engine_steps,
            "exact_nodes": report.counters.exact_nodes,
            "exact_edges": report.counters.exact_edges,
            "completed_turn_options": report.counters.completed_turn_options,
            "applied_action_transitions": report.counters.applied_action_transitions,
            "unique_successor_states": report.counters.unique_successor_states,
            "duplicate_exact_successors": report.counters.duplicate_exact_successors,
            "duplicate_successor_edges": report.counters.duplicate_successor_edges,
            "terminal_losses": report.counters.terminal_losses,
            "depth_limited_successors": report.counters.depth_limited_successors,
            "exhausted_nodes": report.counters.exhausted_nodes,
            "maximum_turn_depth": report.counters.maximum_turn_depth,
        },
        "progress": {
            "retained_states": progress.retained_states,
            "retained_state_work": session.retained_state_work(),
            "max_player_turn": progress.max_player_turn,
            "max_path_atomic_depth": progress.max_path_atomic_depth,
            "deepest_survival_state": progress.deepest_survival_state,
            "deepest_survival_node": deepest_survival_node,
            "deepest_survival_actions": readable.then_some(&progress.deepest_survival_actions),
            "deepest_survival_trace": deepest_survival_trace,
            "deepest_progress_state": progress.deepest_progress_state,
            "deepest_progress_node": deepest_progress_node,
            "deepest_progress_actions": readable.then_some(&progress.deepest_progress_actions),
            "deepest_progress_trace": deepest_progress_trace,
            "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
        },
        "witness_trace": witness_trace,
        "generation_gap_count": report.generation_gaps.len(),
        "watched_states": watched_states,
        "watched_corridor": watched_corridor_output,
        "exported_witness_actions": report.witness.is_some()
            .then_some(export_witness_actions.as_ref())
            .flatten(),
        "exported_deepest_survival_case": export_deepest_survival_case,
        "exported_deepest_survival_actions": exported_deepest_survival_actions,
        "exported_deepest_progress_case": export_deepest_progress_case,
        "exported_deepest_progress_actions": exported_deepest_progress_actions,
    });
    let plan_transition_portfolio = plan_transition_annotations
        .then(|| combat_plan_transition_portfolio_v1(&session))
        .unwrap_or(Value::Null);
    output["counters"]["annotated_exact_edges"] = json!(report.counters.annotated_exact_edges);
    output["counters"]["terminal_win_options"] = json!(report.counters.terminal_win_options);
    output["counters"]["witness_replay_attempts"] = json!(report.counters.witness_replay_attempts);
    output["counters"]["witness_replay_improvements"] =
        json!(report.counters.witness_replay_improvements);
    output["counters"]["witness_replay_dominated_skips"] =
        json!(report.counters.witness_replay_dominated_skips);
    let output_object = output
        .as_object_mut()
        .expect("combat-case report must be a JSON object");
    output_object.insert(
        "plan_transition_annotations".to_string(),
        json!(plan_transition_annotations),
    );
    output_object.insert(
        "plan_transition_portfolio".to_string(),
        plan_transition_portfolio,
    );
    output_object.insert("search_elapsed_ms".to_string(), json!(search_elapsed_ms));
    output_object.insert("performance_timing".to_string(), performance_timing);
    output_object.insert("performance_profile".to_string(), performance_profile);
    print_json(&output)
}
