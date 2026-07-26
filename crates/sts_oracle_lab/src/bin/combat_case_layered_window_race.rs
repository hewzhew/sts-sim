use super::*;

#[derive(Debug, Args)]
pub(super) struct CombatCaseLayeredWindowRaceArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    source_window_index: usize,
    #[arg(long, default_value_t = 500_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 20_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 32)]
    beam_width: usize,
    #[arg(long, default_value_t = 6)]
    retained_per_view: usize,
    /// Total generator work available while acquiring the selected source
    /// window. Window publication itself is demand-driven.
    #[arg(long, default_value_t = 8_192)]
    source_generation_work: usize,
    #[arg(long, default_value_t = 8)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 3)]
    continuation_turn_layers: usize,
    #[arg(long, default_value_t = 256)]
    continuation_service_quantum_work: usize,
    /// Resume all parents in the selected source window as one shared
    /// turn-synchronous cohort instead of multiplying a full continuation
    /// beam by every parent.
    #[arg(long)]
    shared_window_continuation: bool,
    /// Locate exact states inside parent-local continuation windows.
    #[arg(long)]
    watch_exact_state_hash: Vec<String>,
    /// Include one compact best-per-view summary for every parent-local
    /// continuation window.
    #[arg(long)]
    lineage_window_summaries: bool,
    /// After every source candidate exposes one exact layer, continue a
    /// bounded union of the strongest parents from each independent guide
    /// view. No scalar consensus winner receives exclusive authority.
    #[arg(long)]
    continue_parent_portfolio: bool,
    #[arg(long, default_value_t = 2)]
    portfolio_parents_per_view: usize,
    #[arg(long, default_value_t = 1)]
    portfolio_windows_per_parent: usize,
    #[arg(long, default_value_t = 2_048)]
    portfolio_service_quantum_work: usize,
    /// Repeat the parent-portfolio split this many additional turn
    /// boundaries before entering the final layered continuation.
    #[arg(long, default_value_t = 0)]
    portfolio_recursive_splits: usize,
    #[arg(long, default_value_t = 10)]
    nested_continuation_turn_layers: usize,
    #[arg(long)]
    solved_suffix_case: Option<PathBuf>,
    #[arg(long)]
    solved_suffix_actions: Option<PathBuf>,
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
}

pub(super) fn run(args: CombatCaseLayeredWindowRaceArgs) -> Result<(), String> {
    let CombatCaseLayeredWindowRaceArgs {
        case,
        source_window_index,
        max_nodes,
        wall_ms,
        max_engine_steps_per_transition,
        beam_width,
        retained_per_view,
        source_generation_work,
        generation_quantum_work,
        continuation_turn_layers,
        continuation_service_quantum_work,
        shared_window_continuation,
        watch_exact_state_hash,
        lineage_window_summaries,
        continue_parent_portfolio,
        portfolio_parents_per_view,
        portfolio_windows_per_parent,
        portfolio_service_quantum_work,
        portfolio_recursive_splits,
        nested_continuation_turn_layers,
        solved_suffix_case,
        solved_suffix_actions,
        export_witness_actions,
    } = args;
    let command_started = Instant::now();
    let loaded = load_combat_case(&case)?;
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let original_position = loaded.position.clone();
    let original_root = CombatDecisionRoot::new(loaded.position.clone())
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let source_root = CombatDecisionRoot::new(loaded.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let deadline = Instant::now() + Duration::from_millis(wall_ms);
    let policy = existing_combat_knowledge_policy_v1();
    let solved_suffixes = load_layered_solved_suffix_index(
        solved_suffix_case.as_ref(),
        solved_suffix_actions.as_ref(),
        max_engine_steps_per_transition,
    )?;
    let base_config = LayeredCombatWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        beam_width,
        retained_per_view,
        generation_quantum_work,
        max_turn_layers: 1,
    };
    let mut source = LayeredCombatWitnessSession::with_policy_and_solved_suffixes(
        source_root,
        base_config,
        policy.clone(),
        solved_suffixes.clone(),
    );
    let source_report = source.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: source_generation_work.max(1),
            additional_engine_steps: source_generation_work
                .max(1)
                .saturating_mul(max_engine_steps_per_transition.max(1)),
            deadline: Some(deadline),
        },
        &EngineCombatStepper,
    );
    if let Some(witness) = source_report.witness.as_ref() {
        if let Some(path) = export_witness_actions.as_ref() {
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
        return print_json(&json!({
            "schema_name": "OracleCombatCaseLayeredWindowRaceV1",
            "schema_version": 1,
            "case": case,
            "runtime": oracle_lab_runtime_identity(),
            "mode": {
                "scheduler": "resumable_candidate_continuation_race",
                "v2_donor_enabled": false,
                "solved_suffix_count": solved_suffixes.len(),
            },
            "elapsed_ms": command_started.elapsed().as_millis(),
            "source": {
                "status": format!("{:?}", source_report.status),
                "generation_work": source_report.counters.generation_work,
                "solved_suffix_matches": source_report.counters.solved_suffix_matches,
                "solved_suffix_replay_engine_steps": source_report.counters.solved_suffix_replay_engine_steps,
            },
            "race": null,
            "lineage_portfolio": null,
            "exported_witness_actions": export_witness_actions,
            "witness": {
                "final_hp": witness.final_position.combat.entities.player.current_hp,
                "hp_loss": initial_hp.saturating_sub(
                    witness.final_position.combat.entities.player.current_hp,
                ),
                "action_count": witness.actions.len(),
                "negative_log_policy": witness.negative_log_policy,
                "replay_engine_steps": witness.replay_engine_steps,
                "discovery_source": format!("{:?}", witness.discovery_source),
            },
        }));
    }
    let window = source
        .deferred_windows()
        .into_iter()
        .find(|window| {
            window.relative_turn_depth == 1 && window.source_window_index == source_window_index
        })
        .ok_or_else(|| {
            format!(
                "deferred window {source_window_index} was not generated; source status={:?}",
                source_report.status
            )
        })?;
    let candidate_count = window.candidates.len();
    let selected_window_discrepancy = window.window_discrepancy;
    let continuation = LayeredCombatWitnessConfig {
        max_turn_layers: if continue_parent_portfolio {
            1
        } else {
            continuation_turn_layers
        },
        ..base_config
    };
    if shared_window_continuation {
        let mut continuation_session =
            LayeredCombatWitnessSession::from_deferred_window_with_solved_suffixes(
                original_root,
                window,
                continuation,
                policy,
                solved_suffixes.clone(),
            );
        let remaining_work = max_nodes.saturating_sub(source_report.counters.generation_work);
        let continuation_report = continuation_session.advance(
            LayeredCombatWitnessQuantum {
                additional_generation_work: remaining_work,
                additional_engine_steps: remaining_work
                    .saturating_mul(max_engine_steps_per_transition.max(1)),
                deadline: Some(deadline),
            },
            &EngineCombatStepper,
        );
        if let (Some(path), Some(witness)) = (
            export_witness_actions.as_ref(),
            continuation_report.witness.as_ref(),
        ) {
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
        let watched_states = watch_exact_state_hash
            .iter()
            .map(|hash| {
                let parent_work = continuation_report
                    .layers
                    .iter()
                    .enumerate()
                    .filter_map(|(layer_index, layer)| {
                        layer
                            .parent_work
                            .iter()
                            .find(|parent| parent.exact_state_hash == *hash)
                            .map(|parent| {
                                json!({
                                    "layer_index": layer_index,
                                    "generation_work": parent.generation_work,
                                    "completed_turn_options": parent.completed_turn_options,
                                    "finished": parent.finished,
                                })
                            })
                    })
                    .collect::<Vec<_>>();
                let retained_layers = continuation_report
                    .layers
                    .iter()
                    .enumerate()
                    .filter_map(|(layer_index, layer)| {
                        layer
                            .retained_exact_state_hashes
                            .iter()
                            .any(|candidate| candidate == hash)
                            .then_some(layer_index)
                    })
                    .collect::<Vec<_>>();
                let frontier = continuation_report
                    .frontier
                    .iter()
                    .any(|candidate| candidate.exact_state_hash == *hash);
                json!({
                    "exact_state_hash": hash,
                    "parent_work": parent_work,
                    "retained_layers": retained_layers,
                    "frontier": frontier,
                })
            })
            .collect::<Vec<_>>();
        return print_json(&json!({
            "schema_name": "OracleCombatCaseLayeredSharedWindowV1",
            "schema_version": 1,
            "case": case,
            "runtime": oracle_lab_runtime_identity(),
            "mode": {
                "scheduler": "shared_turn_synchronous_window",
                "v2_donor_enabled": false,
                "solved_suffix_count": solved_suffixes.len(),
            },
            "elapsed_ms": command_started.elapsed().as_millis(),
            "source": {
                "status": format!("{:?}", source_report.status),
                "generation_work": source_report.counters.generation_work,
                "candidate_count": candidate_count,
                "source_window_index": source_window_index,
                "window_discrepancy": selected_window_discrepancy,
            },
            "continuation": {
                "status": format!("{:?}", continuation_report.status),
                "counters": {
                    "generation_work": continuation_report.counters.generation_work,
                    "engine_steps": continuation_report.counters.engine_steps,
                    "expanded_parents": continuation_report.counters.expanded_parents,
                    "completed_turn_options": continuation_report.counters.completed_turn_options,
                    "unique_next_turn_states": continuation_report.counters.unique_next_turn_states,
                    "duplicate_next_turn_states": continuation_report.counters.duplicate_next_turn_states,
                    "completed_layers": continuation_report.counters.completed_layers,
                    "solved_suffix_matches": continuation_report.counters.solved_suffix_matches,
                },
                "layers": continuation_report.layers.iter().map(|layer| json!({
                    "relative_turn_depth": layer.relative_turn_depth,
                    "player_turn": layer.player_turn,
                    "parent_states": layer.parent_states,
                    "generation_work": layer.generation_work,
                    "completed_turn_options": layer.completed_turn_options,
                    "unique_next_turn_states": layer.unique_next_turn_states,
                    "retained_next_turn_states": layer.retained_next_turn_states,
                    "truncated_parents": layer.truncated_parents,
                    "emitted_windows": layer.emitted_windows,
                })).collect::<Vec<_>>(),
                "watched_states": watched_states,
            },
            "exported_witness_actions": export_witness_actions,
            "witness": continuation_report.witness.as_ref().map(|witness| json!({
                "final_hp": witness.final_position.combat.entities.player.current_hp,
                "hp_loss": initial_hp.saturating_sub(
                    witness.final_position.combat.entities.player.current_hp,
                ),
                "action_count": witness.actions.len(),
                "negative_log_policy": witness.negative_log_policy,
                "replay_engine_steps": witness.replay_engine_steps,
                "discovery_source": format!("{:?}", witness.discovery_source),
            })),
        }));
    }
    let mut race = LayeredCombatCandidateRaceSession::from_window_with_solved_suffixes(
        original_root,
        window,
        LayeredCombatCandidateRaceConfig {
            continuation,
            service_quantum_work: continuation_service_quantum_work,
        },
        policy.clone(),
        solved_suffixes.clone(),
    );
    let remaining_work = max_nodes.saturating_sub(source_report.counters.generation_work);
    let race_report = race.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: remaining_work,
            additional_engine_steps: remaining_work
                .saturating_mul(max_engine_steps_per_transition.max(1)),
            deadline: Some(deadline),
        },
        &EngineCombatStepper,
    );
    let lineage_windows = race.deferred_lineage_windows();
    let lineage_parent_ranks =
        rank_layered_combat_lineage_parents(&lineage_windows, policy.as_ref());
    let mut portfolio_report = None;
    if continue_parent_portfolio && race_report.witness.is_none() {
        let portfolio_root = CombatDecisionRoot::new(original_position.clone())
            .map_err(|error| format!("invalid portfolio combat root: {error:?}"))?;
        let nested_config = LayeredCombatWitnessConfig {
            max_turn_layers: nested_continuation_turn_layers,
            ..base_config
        };
        let mut portfolio =
            LayeredCombatLineagePortfolioSession::from_lineage_windows_with_solved_suffixes(
                portfolio_root,
                lineage_windows.clone(),
                LayeredCombatLineagePortfolioConfig {
                    candidate_race: LayeredCombatCandidateRaceConfig {
                        continuation: nested_config,
                        service_quantum_work: continuation_service_quantum_work,
                    },
                    parents_per_view: portfolio_parents_per_view,
                    windows_per_parent: portfolio_windows_per_parent,
                    service_quantum_work: portfolio_service_quantum_work,
                    recursive_splits: portfolio_recursive_splits,
                },
                policy.clone(),
                solved_suffixes.clone(),
            );
        let remaining_work = max_nodes
            .saturating_sub(source_report.counters.generation_work)
            .saturating_sub(race_report.counters.generation_work);
        portfolio_report = Some(portfolio.advance(
            LayeredCombatWitnessQuantum {
                additional_generation_work: remaining_work,
                additional_engine_steps:
                    remaining_work.saturating_mul(max_engine_steps_per_transition.max(1)),
                deadline: Some(deadline),
            },
            &EngineCombatStepper,
        ));
    }
    let watched_lineage_states = lineage_windows
        .iter()
        .flat_map(|lineage| {
            lineage
                .window
                .candidates
                .iter()
                .enumerate()
                .filter(|(_, candidate)| {
                    watch_exact_state_hash.contains(&candidate.exact_state_hash)
                })
                .map(|(candidate_index, candidate)| {
                    json!({
                        "exact_state_hash": candidate.exact_state_hash,
                        "parent_candidate_index": lineage.parent_candidate_index,
                        "parent_exact_state_hash": lineage.parent_exact_state_hash,
                        "relative_turn_depth": lineage.window.relative_turn_depth,
                        "window_discrepancy": lineage.window.window_discrepancy,
                        "source_window_index": lineage.window.source_window_index,
                        "candidate_index": candidate_index,
                        "action_count": candidate.actions.len(),
                        "negative_log_policy": candidate.negative_log_policy,
                        "guides": existing_combat_guide_diagnostics(&candidate.position),
                    })
                })
        })
        .collect::<Vec<_>>();
    let lineage_window_summaries = lineage_window_summaries.then(|| {
        lineage_windows
            .iter()
            .map(|lineage| {
                let best_policy = lineage
                    .window
                    .candidates
                    .iter()
                    .map(|candidate| candidate.negative_log_policy)
                    .min_by(f64::total_cmp);
                let best_progress = lineage
                    .window
                    .candidates
                    .iter()
                    .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(&candidate.position))
                    .max();
                let best_survival = lineage
                    .window
                    .candidates
                    .iter()
                    .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(&candidate.position))
                    .max();
                let best_horizon = lineage
                    .window
                    .candidates
                    .iter()
                    .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(&candidate.position))
                    .max();
                let best_setup = lineage
                    .window
                    .candidates
                    .iter()
                    .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(&candidate.position))
                    .max();
                json!({
                    "parent_candidate_index": lineage.parent_candidate_index,
                    "parent_exact_state_hash": lineage.parent_exact_state_hash,
                    "source_window_index": lineage.window.source_window_index,
                    "window_discrepancy": lineage.window.window_discrepancy,
                    "candidate_count": lineage.window.candidates.len(),
                    "best_policy_negative_log": best_policy,
                    "best_progress": best_progress,
                    "best_survival": best_survival,
                    "best_horizon": best_horizon,
                    "best_setup": best_setup,
                })
            })
            .collect::<Vec<_>>()
    });
    let final_witness = portfolio_report
        .as_ref()
        .and_then(|report| report.witness.as_ref())
        .or(race_report.witness.as_ref());
    if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), final_witness) {
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
    print_json(&json!({
        "schema_name": "OracleCombatCaseLayeredWindowRaceV1",
        "schema_version": 1,
        "case": case,
        "runtime": oracle_lab_runtime_identity(),
        "mode": {
            "scheduler": "resumable_candidate_continuation_race",
            "v2_donor_enabled": false,
            "solved_suffix_count": solved_suffixes.len(),
        },
        "elapsed_ms": command_started.elapsed().as_millis(),
        "source": {
            "status": format!("{:?}", source_report.status),
            "generation_work": source_report.counters.generation_work,
            "source_window_index": source_window_index,
            "window_discrepancy": selected_window_discrepancy,
            "candidate_count": candidate_count,
        },
        "race": {
            "status": format!("{:?}", race_report.status),
            "generation_work": race_report.counters.generation_work,
            "engine_steps": race_report.counters.engine_steps,
            "services": race_report.counters.services,
            "candidates": race_report.candidates.iter().map(|candidate| json!({
                "candidate_index": candidate.candidate_index,
                "exact_state_hash": candidate.exact_state_hash,
                "generation_work": candidate.generation_work,
                "engine_steps": candidate.engine_steps,
                "completed_layers": candidate.completed_layers,
                "terminal": candidate.terminal,
                "found_witness": candidate.found_witness,
            })).collect::<Vec<_>>(),
        },
        "lineage_window_count": lineage_windows.len(),
        "lineage_parent_ranks": lineage_parent_ranks.iter().map(|parent| json!({
            "parent_candidate_index": parent.parent_candidate_index,
            "parent_exact_state_hash": parent.parent_exact_state_hash,
            "consensus_rank": parent.consensus_rank,
            "rank_sum": parent.rank_sum,
            "anchor_rank": parent.anchor_rank,
            "guide_ranks": parent.guide_ranks.iter().map(|(lane, rank)| json!({
                "lane": lane.value(),
                "rank": rank,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "lineage_window_summaries": lineage_window_summaries,
        "watched_lineage_states": watched_lineage_states,
        "lineage_portfolio": portfolio_report.as_ref().map(|report| json!({
            "status": format!("{:?}", report.status),
            "generation_work": report.counters.generation_work,
            "engine_steps": report.counters.engine_steps,
            "services": report.counters.services,
            "selected_parent_count": report.selected_parent_count,
            "deferred_parent_count": report.deferred_parent_count,
            "deferred_window_count": report.deferred_window_count,
            "entries": lineage_portfolio_entries_json(&report.entries),
        })),
        "exported_witness_actions": final_witness.is_some()
            .then_some(export_witness_actions.as_ref())
            .flatten(),
        "witness": final_witness.map(|witness| json!({
            "final_hp": witness.final_position.combat.entities.player.current_hp,
            "hp_loss": initial_hp.saturating_sub(
                witness.final_position.combat.entities.player.current_hp,
            ),
            "action_count": witness.actions.len(),
            "negative_log_policy": witness.negative_log_policy,
            "replay_engine_steps": witness.replay_engine_steps,
            "discovery_source": format!("{:?}", witness.discovery_source),
        })),
    }))
}
