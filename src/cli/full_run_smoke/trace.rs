use super::*;
use serde_json::json;

pub fn probe_combat_plan_from_trace(
    config: &FullRunTracePlanProbeConfig,
) -> Result<crate::bot::combat::CombatTurnPlanProbeReport, String> {
    let (trace, seed, ascension, final_act, player_class, target_trace_step, ctx) =
        replay_trace_to_combat_frontier(
            &config.trace_file,
            config.step_index,
            config.ascension,
            config.final_act,
            config.player_class.clone(),
            config.max_steps,
        )?;
    let Some(combat) = ctx.combat_state.as_ref() else {
        return Err(format!(
            "trace step {} replayed to non-combat state {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    };

    let mut report =
        crate::bot::combat::probe_turn_plans(&ctx.engine_state, combat, config.probe_config);
    report.source_trace = trace_probe_source(
        &config.trace_file,
        config.step_index,
        seed,
        ascension,
        final_act,
        &player_class,
        &trace,
        &target_trace_step,
    );
    Ok(report)
}

pub fn probe_combat_draw_marginal_from_trace(
    config: &FullRunTraceDrawMarginalProbeConfig,
) -> Result<crate::bot::combat::CombatDrawMarginalProbeReport, String> {
    let (trace, seed, ascension, final_act, player_class, target_trace_step, ctx) =
        replay_trace_to_combat_frontier(
            &config.trace_file,
            config.step_index,
            config.ascension,
            config.final_act,
            config.player_class.clone(),
            config.max_steps,
        )?;
    let Some(combat) = ctx.combat_state.as_ref() else {
        return Err(format!(
            "trace step {} replayed to non-combat state {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    };
    let target = draw_marginal_target_from_trace_config(
        &ctx.engine_state,
        combat,
        config.target_card,
        config.target_hand_index,
        config.target_action_key.clone(),
    )?;

    let mut report = crate::bot::combat::probe_draw_marginal_value_for_target(
        &ctx.engine_state,
        combat,
        target,
        config.probe_config,
    );
    report.source_trace = trace_probe_source(
        &config.trace_file,
        config.step_index,
        seed,
        ascension,
        final_act,
        &player_class,
        &trace,
        &target_trace_step,
    );
    Ok(report)
}

pub fn build_candidate_outcome_pack_from_trace(
    config: &FullRunTraceCandidateOutcomePackConfig,
) -> Result<CombatCandidateOutcomePackReport, String> {
    let (trace, seed, ascension, final_act, player_class, target_trace_step, ctx) =
        replay_trace_to_combat_frontier(
            &config.trace_file,
            config.step_index,
            config.ascension,
            config.final_act,
            config.player_class.clone(),
            config.max_steps,
        )?;
    let Some(combat) = ctx.combat_state.as_ref() else {
        return Err(format!(
            "trace step {} replayed to non-combat state {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    };

    let legal_inputs = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
    let action_candidates = build_action_candidates(&legal_inputs, Some(&ctx));
    let start_outcome = outcome_vector_from_combat(combat, &ctx.engine_state, combat);
    let mut paired = legal_inputs
        .into_iter()
        .zip(action_candidates)
        .filter(|(input, _)| !config.controlled_v0 || controlled_v0_root_input(input))
        .collect::<Vec<_>>();
    if let Some(max_candidates) = config.max_candidates {
        paired.truncate(max_candidates);
    }

    let mut candidates = Vec::with_capacity(paired.len());
    for (candidate_index, (input, candidate)) in paired.into_iter().enumerate() {
        let bounded_objectives = bounded_objective_oracle_for_root(
            &ctx.engine_state,
            combat,
            &input,
            config.max_engine_steps_per_action,
        );
        let solution = crate::bot::combat::exact_turn_solver::solve_exact_turn_with_config(
            &ctx.engine_state,
            combat,
            crate::bot::combat::exact_turn_solver::ExactTurnConfig {
                max_nodes: config.max_exact_nodes_per_candidate,
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
                root_inputs: Some(vec![input]),
            },
        );
        let unique_outcomes = unique_outcome_vectors(
            solution
                .nondominated_end_states
                .iter()
                .map(|state| {
                    outcome_vector_from_combat(
                        combat,
                        &state.frontier_engine,
                        &state.frontier_combat,
                    )
                })
                .collect::<Vec<_>>(),
        );
        let outcome_aggregate = aggregate_candidate_outcomes(
            combat,
            &solution.nondominated_end_states,
            unique_outcomes,
        );
        let oracle_quality = candidate_oracle_quality(&solution);
        candidates.push(CombatRootCandidateOutcome {
            candidate_index,
            candidate,
            exact_turn: CombatExactTurnOutcomeSummary {
                status: if solution.truncated {
                    "truncated"
                } else {
                    "ok"
                }
                .to_string(),
                truncated: solution.truncated,
                explored_nodes: solution.explored_nodes,
                dominance_prunes: solution.dominance_prunes,
                cycle_cuts: solution.cycle_cuts,
                cache_hits: solution.cache_hits,
                cache_misses: solution.cache_misses,
                elapsed_ms: solution.elapsed_ms,
                best_line_debug: solution
                    .best_line
                    .iter()
                    .map(|input| format!("{input:?}"))
                    .collect(),
                nondominated_end_state_count: solution.nondominated_end_states.len(),
                truncation: CombatExactTurnTruncationSummary {
                    max_nodes_hit: solution.truncation.max_nodes_hit,
                    engine_step_limit_hit: solution.truncation.engine_step_limit_hit,
                    deadline_hit: solution.truncation.deadline_hit,
                    cycle_cut: solution.truncation.cycle_cut,
                    step_projection_truncated: solution.truncation.step_projection_truncated,
                },
            },
            oracle_quality,
            bounded_objectives,
            outcome_aggregate,
        });
    }
    let pairwise_labels = build_bounded_pairwise_labels(&candidates);

    let mut truth_warnings = vec![
        "primary targets are engine outcome vectors, not plan-query status".to_string(),
        "PlanScoreBreakdown and card/query labels are intentionally absent as primary labels"
            .to_string(),
        "exact-turn suffixes are same-turn only and do not prove long-horizon optimal play"
            .to_string(),
        "nondominated end states are aggregated to avoid forcing one scalar utility".to_string(),
        "unique outcome vectors are deduplicated before export to avoid duplicate-label weighting"
            .to_string(),
        "pairwise_labels come from bounded objective interval separation, not full-turn exact suffix enumeration"
            .to_string(),
        "train/test splitting must group by split_group_key".to_string(),
    ];
    if config.max_candidates.is_some() {
        truth_warnings.push("candidate list was truncated by --max-candidates".to_string());
    }
    if config.controlled_v0 {
        truth_warnings.push(
            "controlled_v0 filtered root candidates to play_card and end_turn only".to_string(),
        );
    }

    let pack_oracle_quality = pack_oracle_quality(
        &candidates,
        pairwise_labels.len(),
        config.controlled_v0,
        config.min_eligible_candidates,
    );

    Ok(CombatCandidateOutcomePackReport {
        schema_version: COMBAT_CANDIDATE_OUTCOME_PACK_SCHEMA_VERSION.to_string(),
        source_trace: trace_probe_source(
            &config.trace_file,
            config.step_index,
            seed,
            ascension,
            final_act,
            &player_class,
            &trace,
            &target_trace_step,
        ),
        split_group_key: format!(
            "{}::step_{}",
            config.trace_file.display(),
            config.step_index
        ),
        split_group_key_kind: "trace_file_step".to_string(),
        observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
        observation: build_observation(&ctx),
        start_outcome,
        oracle_config: CombatCandidateOutcomeOracleConfig {
            oracle_kind: "forced_root_exact_turn_suffix_outcome_v0".to_string(),
            root_action_policy: "evaluate_each_legal_root_action_in_same_state".to_string(),
            max_exact_nodes_per_candidate: config.max_exact_nodes_per_candidate,
            max_engine_steps_per_action: config.max_engine_steps_per_action,
            primary_label_policy: "engine_outcome_vector_only".to_string(),
            controlled_v0: config.controlled_v0,
        },
        pack_oracle_quality,
        candidate_count: candidates.len(),
        candidates,
        pairwise_labels,
        training_contract: CombatCandidateOutcomeTrainingContract {
            allowed_primary_targets: vec![
                "pairwise_labels.*".to_string(),
                "outcome_aggregate.min_hp_lost".to_string(),
                "outcome_aggregate.max_enemy_hp_reduction".to_string(),
                "outcome_aggregate.any_combat_cleared".to_string(),
                "outcome_aggregate.min_projected_unblocked_damage".to_string(),
                "outcome_aggregate.unique_outcomes.*".to_string(),
            ],
            disallowed_primary_targets: vec![
                "plan_query.status".to_string(),
                "PlanScoreBreakdown.total_score".to_string(),
                "card_id+query_name".to_string(),
                "static_cashout_score".to_string(),
            ],
            required_split_grouping: "split_group_key".to_string(),
            required_ablations: vec![
                "candidate_only".to_string(),
                "card_or_action_only".to_string(),
                "state_only".to_string(),
                "full_state_plus_candidate".to_string(),
            ],
            closed_loop_gate:
                "trained evaluator must improve engine candidate selection over baseline policies"
                    .to_string(),
        },
        truth_warnings,
    })
}

pub fn build_candidate_outcome_pack_batch_from_traces(
    config: &FullRunTraceCandidateOutcomePackBatchConfig,
) -> Result<CombatCandidateOutcomePackBatchReport, String> {
    let mut trace_files = Vec::new();
    for input in &config.trace_inputs {
        collect_trace_files(input, &mut trace_files)?;
    }
    trace_files.sort();
    trace_files.dedup();

    let mut targets = Vec::new();
    let mut errors = Vec::new();
    for trace_file in &trace_files {
        match controlled_v0_trace_steps(trace_file, config.step_start, config.step_end) {
            Ok(mut steps) => targets.append(&mut steps),
            Err(err) => errors.push(err),
        }
    }
    if let Some(limit) = config.step_limit {
        targets.truncate(limit);
    }

    std::fs::create_dir_all(&config.out_dir).map_err(|err| {
        format!(
            "failed to create candidate outcome batch output dir '{}': {err}",
            config.out_dir.display()
        )
    })?;

    let mut budget_summaries = Vec::new();
    for budget in &config.budgets {
        let budget_dir = config.out_dir.join(format!("budget_{budget}"));
        std::fs::create_dir_all(&budget_dir).map_err(|err| {
            format!(
                "failed to create candidate outcome budget dir '{}': {err}",
                budget_dir.display()
            )
        })?;

        let mut packs = Vec::new();
        let mut pack_manifest = Vec::new();
        let mut trainable_manifest = Vec::new();
        for (trace_file, step_index) in &targets {
            let pack_config = FullRunTraceCandidateOutcomePackConfig {
                trace_file: trace_file.clone(),
                step_index: *step_index,
                ascension: config.ascension,
                final_act: config.final_act,
                player_class: config.player_class.clone(),
                max_steps: config.max_steps,
                max_exact_nodes_per_candidate: *budget,
                max_engine_steps_per_action: config.max_engine_steps_per_action,
                max_candidates: None,
                controlled_v0: true,
                min_eligible_candidates: config.min_eligible_candidates,
            };
            match build_candidate_outcome_pack_from_trace(&pack_config) {
                Ok(pack) => {
                    let out_path = budget_dir.join(pack_file_name(trace_file, *step_index));
                    write_pretty_json(&out_path, &pack)?;
                    let out_string = out_path.display().to_string();
                    if pack.pack_oracle_quality.bounded_pairwise_manifest_eligible {
                        trainable_manifest.push(out_string.clone());
                    }
                    pack_manifest.push(out_string);
                    packs.push(pack);
                }
                Err(err) => errors.push(format!(
                    "failed to build candidate outcome pack for '{}' step {} budget {}: {err}",
                    trace_file.display(),
                    step_index,
                    budget
                )),
            }
        }

        let summary = summarize_candidate_outcome_budget(
            *budget,
            &packs,
            pack_manifest,
            trainable_manifest,
            config.min_eligible_candidates,
        );
        write_pretty_json(&budget_dir.join("summary.json"), &summary)?;
        budget_summaries.push(summary);
    }

    let selected_budget = budget_summaries
        .iter()
        .filter(|summary| {
            summary.median_candidate_elapsed_ms <= config.median_runtime_ms_limit
                && summary.trainable_pair_count > 0
                && summary.trainable_pair_count >= config.min_trainable_pairs
                && !summary.trainable_manifest.is_empty()
        })
        .map(|summary| summary.budget)
        .min();
    let oracle_ready = selected_budget.is_some();
    let oracle_ready_reason = selected_budget
        .map(|budget| format!("selected minimum budget {budget} satisfying oracle quality gate"))
        .unwrap_or_else(|| {
            format!(
                "no budget satisfied bounded trainable_pair_count >= {}, median_candidate_elapsed_ms <= {}, and non-empty trainable manifest",
                config.min_trainable_pairs,
                config.median_runtime_ms_limit
            )
        });
    let trainable_manifest = selected_budget
        .and_then(|budget| {
            budget_summaries
                .iter()
                .find(|summary| summary.budget == budget)
                .map(|summary| summary.trainable_manifest.clone())
        })
        .unwrap_or_default();
    let diagnostic_manifest = budget_summaries
        .iter()
        .flat_map(|summary| summary.pack_manifest.iter().cloned())
        .collect::<Vec<_>>();

    let report = CombatCandidateOutcomePackBatchReport {
        schema_version: "combat_candidate_outcome_pack_batch_v0".to_string(),
        generated_pack_schema_version: COMBAT_CANDIDATE_OUTCOME_PACK_SCHEMA_VERSION.to_string(),
        out_dir: config.out_dir.display().to_string(),
        budgets: budget_summaries,
        selected_budget,
        oracle_ready,
        oracle_ready_reason,
        trainable_manifest,
        diagnostic_manifest,
        errors,
    };
    write_pretty_json(&config.out_dir.join("suite_summary.json"), &report)?;
    Ok(report)
}

pub fn run_recursive_rollout_validation_from_traces(
    config: &FullRunTraceRecursiveRolloutValidationConfig,
) -> Result<serde_json::Value, String> {
    let started = Instant::now();
    let mut trace_files = Vec::new();
    for input in &config.trace_inputs {
        collect_trace_files(input, &mut trace_files)?;
    }
    trace_files.sort();
    trace_files.dedup();

    let mut targets = Vec::new();
    let mut errors = Vec::new();
    for trace_file in &trace_files {
        match controlled_v0_trace_steps(trace_file, config.step_start, config.step_end) {
            Ok(mut steps) => targets.append(&mut steps),
            Err(err) => errors.push(err),
        }
    }
    if let Some(limit) = config.step_limit {
        targets.truncate(limit);
    }

    std::fs::create_dir_all(&config.out_dir).map_err(|err| {
        format!(
            "failed to create recursive rollout output dir '{}': {err}",
            config.out_dir.display()
        )
    })?;

    let mut pack_manifest = Vec::new();
    let mut trainable_manifest = Vec::new();
    let mut total_candidates = 0usize;
    let mut total_pairwise_labels = 0usize;
    let mut rollout_elapsed_samples = Vec::new();
    let mut terminal_counts = BTreeMap::<String, usize>::new();

    for (trace_file, step_index) in &targets {
        let pack_started = Instant::now();
        let replay = replay_trace_to_combat_frontier(
            trace_file,
            *step_index,
            config.ascension,
            config.final_act,
            config.player_class.clone(),
            config.max_steps,
        );
        let (trace, seed, ascension, final_act, player_class, target_trace_step, ctx) = match replay
        {
            Ok(value) => value,
            Err(err) => {
                errors.push(format!(
                    "failed to replay '{}' step {} for recursive rollout: {err}",
                    trace_file.display(),
                    step_index
                ));
                continue;
            }
        };
        let Some(combat) = ctx.combat_state.as_ref() else {
            errors.push(format!(
                "trace '{}' step {} replayed to non-combat state",
                trace_file.display(),
                step_index
            ));
            continue;
        };
        let legal_inputs = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let action_candidates = build_action_candidates(&legal_inputs, Some(&ctx));
        let mut paired = legal_inputs
            .into_iter()
            .zip(action_candidates)
            .filter(|(input, _)| !config.controlled_v0 || controlled_v0_root_input(input))
            .collect::<Vec<_>>();
        if let Some(max_candidates) = config.max_candidates {
            paired.truncate(max_candidates);
        }

        let mut candidate_reports = Vec::new();
        let mut utilities = Vec::new();
        for (candidate_index, (input, candidate)) in paired.into_iter().enumerate() {
            let rollout_started = Instant::now();
            let outcome = rollout_root_candidate_with_continuation(
                &ctx,
                input,
                config.continuation_policy,
                config.horizon_decisions,
                config
                    .max_steps
                    .unwrap_or_else(|| step_index.saturating_add(128).max(512)),
            );
            let elapsed_ms = rollout_started.elapsed().as_millis();
            rollout_elapsed_samples.push(elapsed_ms);
            let utility = recursive_rollout_utility(&outcome);
            *terminal_counts
                .entry(
                    outcome
                        .get("terminal_kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                )
                .or_default() += 1;
            utilities.push((candidate_index, candidate.action_key.clone(), utility));
            candidate_reports.push(serde_json::json!({
                "candidate_index": candidate_index,
                "candidate": candidate,
                "rollout": outcome,
                "rollout_elapsed_ms": elapsed_ms,
            }));
        }

        let pairwise_labels = recursive_rollout_pairwise_labels(&utilities);
        total_candidates = total_candidates.saturating_add(candidate_reports.len());
        total_pairwise_labels = total_pairwise_labels.saturating_add(pairwise_labels.len());

        let split_group_key = format!("{}::step_{}", trace_file.display(), step_index);
        let pack = serde_json::json!({
            "schema_version": "recursive_rollout_validation_pack_v0",
            "source_trace": trace_probe_source(
                trace_file,
                *step_index,
                seed,
                ascension,
                final_act,
                &player_class,
                &trace,
                &target_trace_step,
            ),
            "split_group_key": split_group_key,
            "split_group_key_kind": "trace_file_step",
            "observation_schema_version": FULL_RUN_OBSERVATION_SCHEMA_VERSION,
            "action_schema_version": FULL_RUN_ACTION_SCHEMA_VERSION,
            "observation": build_observation(&ctx),
            "start_outcome": outcome_vector_from_combat(combat, &ctx.engine_state, combat),
            "oracle_config": {
                "oracle_kind": "recursive_rollout_validation_v0",
                "horizon_decisions": config.horizon_decisions,
                "continuation_policy": config.continuation_policy.as_str(),
                "controlled_v0": config.controlled_v0,
            },
            "candidate_count": candidate_reports.len(),
            "candidates": candidate_reports,
            "pairwise_labels": pairwise_labels,
            "pack_elapsed_ms": pack_started.elapsed().as_millis(),
        });
        let out_path = config.out_dir.join(pack_file_name(trace_file, *step_index));
        write_pretty_json(&out_path, &pack)?;
        let out_string = out_path.display().to_string();
        if pack
            .get("pairwise_labels")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|labels| !labels.is_empty())
        {
            trainable_manifest.push(out_string.clone());
        }
        pack_manifest.push(out_string);
    }

    rollout_elapsed_samples.sort_unstable();
    let median_rollout_elapsed_ms = if rollout_elapsed_samples.is_empty() {
        0
    } else {
        rollout_elapsed_samples[rollout_elapsed_samples.len().saturating_sub(1) / 2]
    };
    let elapsed_ms = started.elapsed().as_millis();
    let rollouts_per_second = if elapsed_ms == 0 {
        total_candidates as f32
    } else {
        (total_candidates as f32 * 1000.0) / elapsed_ms as f32
    };
    let report = serde_json::json!({
        "schema_version": "recursive_rollout_validation_suite_v0",
        "out_dir": config.out_dir.display().to_string(),
        "config": {
            "trace_inputs": config.trace_inputs.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "step_start": config.step_start,
            "step_end": config.step_end,
            "step_limit": config.step_limit,
            "horizon_decisions": config.horizon_decisions,
            "continuation_policy": config.continuation_policy.as_str(),
            "controlled_v0": config.controlled_v0,
            "max_candidates": config.max_candidates,
            "parallelism": 1,
        },
        "pack_count": pack_manifest.len(),
        "trainable_pack_count": trainable_manifest.len(),
        "candidate_count": total_candidates,
        "pairwise_label_count": total_pairwise_labels,
        "elapsed_ms": elapsed_ms,
        "median_rollout_elapsed_ms": median_rollout_elapsed_ms,
        "rollouts_per_second": rollouts_per_second,
        "terminal_counts": terminal_counts,
        "pack_manifest": pack_manifest,
        "trainable_manifest": trainable_manifest,
        "errors": errors,
    });
    write_pretty_json(&config.out_dir.join("suite_summary.json"), &report)?;
    Ok(report)
}

pub fn run_branch_from_trace(
    config: &FullRunTraceBranchRunConfig,
) -> Result<serde_json::Value, String> {
    let (trace, seed, ascension, final_act, player_class, target_trace_step, mut ctx) =
        replay_trace_to_frontier(
            &config.trace_file,
            config.step_index,
            config.ascension,
            config.final_act,
            config.player_class.clone(),
            config.max_steps,
        )?;
    let max_steps = config.max_steps.unwrap_or_else(|| {
        trace
            .get("config")
            .and_then(|value| value.get("max_steps"))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| config.step_index.saturating_add(512).max(1500))
    });
    let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
    if legal.is_empty() {
        return Err(format!(
            "trace step {} replayed to {} with no legal actions",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    }
    let candidates = build_action_candidates(&legal, Some(&ctx));
    let forced_index = if let Some(index) = config.target_action_index {
        if index >= legal.len() {
            return Err(format!(
                "target action index {} out of range for {} legal actions",
                index,
                legal.len()
            ));
        }
        index
    } else if let Some(key) = config.target_action_key.as_ref() {
        candidates
            .iter()
            .position(|candidate| candidate.action_key == *key)
            .ok_or_else(|| {
                format!(
                    "target action key '{}' not legal at step {}; legal keys: {:?}",
                    key,
                    config.step_index,
                    candidates
                        .iter()
                        .map(|candidate| candidate.action_key.clone())
                        .collect::<Vec<_>>()
                )
            })?
    } else {
        return Err(
            "run-branch-from-trace requires --target-action-key or --target-action-index"
                .to_string(),
        );
    };
    let forced_input = legal
        .get(forced_index)
        .cloned()
        .ok_or_else(|| format!("missing legal action at forced index {forced_index}"))?;
    let forced_key = candidates
        .get(forced_index)
        .map(|candidate| candidate.action_key.clone())
        .unwrap_or_else(|| action_key_for_input(&forced_input, ctx.combat_state.as_ref()));
    let source_chosen = target_trace_step
        .get("chosen_action_key")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    let mut branch_steps = Vec::new();
    let mut branch_action_keys = Vec::new();
    let mut terminal_reason = "step_cap".to_string();
    let mut crash = None::<String>;

    branch_action_keys.push(forced_key.clone());
    if config.include_trace {
        branch_steps.push(json!({
            "branch_step": 0,
            "source_step_index": config.step_index,
            "floor": ctx.run_state.floor_num,
            "act": ctx.run_state.act_num,
            "engine_state": engine_state_label(&ctx.engine_state),
            "decision_type": decision_type(&ctx.engine_state),
            "chosen_action_index": forced_index,
            "chosen_action_key": forced_key,
            "forced": true,
            "hp": ctx.run_state.current_hp,
            "gold": ctx.run_state.gold,
        }));
    }
    if let Err(err) = apply_rollout_action(&mut ctx, forced_input, max_steps) {
        crash = Some(err);
        terminal_reason = "engine_error".to_string();
    }

    let mut branch_decisions = 1usize;
    while crash.is_none() && branch_decisions < max_steps {
        if let Err(err) = prepare_decision_point(&mut ctx, max_steps) {
            crash = Some(err);
            terminal_reason = "engine_error".to_string();
            break;
        }
        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_reason = "game_over".to_string();
            break;
        }
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        if legal.is_empty() {
            crash = Some(format!(
                "no legal actions at {} floor {}",
                engine_state_label(&ctx.engine_state),
                ctx.run_state.floor_num
            ));
            terminal_reason = "no_legal_actions".to_string();
            break;
        }
        let chosen_index = match config.continuation_policy {
            RunPolicyKind::RuleBaselineV0 | RunPolicyKind::RuleBaselineV1Candidate => {
                choose_rule_baseline_action(&ctx, &legal)
            }
            RunPolicyKind::RuleBaselineV0Control => {
                choose_rule_baseline_v0_control_action(&ctx, &legal)
            }
            RunPolicyKind::PlanQueryV0 => choose_plan_query_action(&ctx, &legal)
                .unwrap_or_else(|| choose_rule_baseline_action(&ctx, &legal)),
            RunPolicyKind::RandomMasked => choose_rule_baseline_action(&ctx, &legal),
        };
        let Some(input) = legal.get(chosen_index).cloned() else {
            crash = Some(format!(
                "continuation action index {chosen_index} out of range"
            ));
            terminal_reason = "engine_error".to_string();
            break;
        };
        let action_key = action_key_for_input(&input, ctx.combat_state.as_ref());
        branch_action_keys.push(action_key.clone());
        if config.include_trace {
            branch_steps.push(json!({
                "branch_step": branch_decisions,
                "source_step_index": config.step_index + branch_decisions,
                "floor": ctx.run_state.floor_num,
                "act": ctx.run_state.act_num,
                "engine_state": engine_state_label(&ctx.engine_state),
                "decision_type": decision_type(&ctx.engine_state),
                "chosen_action_index": chosen_index,
                "chosen_action_key": action_key,
                "forced": false,
                "hp": ctx.run_state.current_hp,
                "gold": ctx.run_state.gold,
            }));
        }
        if let Err(err) = apply_rollout_action(&mut ctx, input, max_steps) {
            crash = Some(err);
            terminal_reason = "engine_error".to_string();
            break;
        }
        branch_decisions += 1;
    }

    if crash.is_none() {
        let _ = prepare_decision_point(&mut ctx, max_steps);
        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_reason = "game_over".to_string();
        }
    }
    let result = match &ctx.engine_state {
        EngineState::GameOver(RunResult::Victory) => "victory",
        EngineState::GameOver(RunResult::Defeat) => "defeat",
        _ if crash.is_some() => "crash",
        _ => "step_cap",
    };

    Ok(json!({
        "schema_version": "full_run_branch_from_trace_v0",
        "source_trace": trace_probe_source(
            &config.trace_file,
            config.step_index,
            seed,
            ascension,
            final_act,
            &player_class,
            &trace,
            &target_trace_step,
        ),
        "source_chosen_action_key": source_chosen,
        "forced_action_index": forced_index,
        "forced_action_key": forced_key,
        "continuation_policy": config.continuation_policy.as_str(),
        "legal_actions": candidates,
        "result": result,
        "terminal_reason": terminal_reason,
        "crash": crash,
        "floor": ctx.run_state.floor_num,
        "act": ctx.run_state.act_num,
        "hp": ctx.run_state.current_hp,
        "max_hp": ctx.run_state.max_hp,
        "gold": ctx.run_state.gold,
        "deck_size": ctx.run_state.master_deck.len(),
        "relic_count": ctx.run_state.relics.len(),
        "combat_win_count": ctx.combat_win_count,
        "branch_decisions": branch_decisions,
        "branch_action_keys": branch_action_keys,
        "branch_steps": if config.include_trace { json!(branch_steps) } else { serde_json::Value::Null },
    }))
}

#[derive(Clone, Debug)]
struct CombatPlanSearchNode {
    ctx: EpisodeContext,
    action_keys: Vec<String>,
    depth: usize,
    terminal_kind: Option<String>,
    engine_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
struct CombatPlanSearchScore {
    terminal_bucket: i32,
    alive: i32,
    combat_win_delta: i32,
    primary_monster_hp_reduction: i32,
    monster_hp_reduction: i32,
    incoming_safety: i32,
    hp: i32,
    block: i32,
    depth_neg: i32,
}

pub fn search_combat_plan_from_trace(
    config: &FullRunTraceCombatPlanSearchConfig,
) -> Result<serde_json::Value, String> {
    let (trace, seed, ascension, final_act, player_class, target_trace_step, mut start_ctx) =
        replay_trace_to_frontier(
            &config.trace_file,
            config.step_index,
            config.ascension,
            config.final_act,
            config.player_class.clone(),
            config.max_steps,
        )?;
    let max_steps = config.max_steps.unwrap_or_else(|| {
        trace
            .get("config")
            .and_then(|value| value.get("max_steps"))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| config.step_index.saturating_add(512).max(1500))
    });
    prepare_decision_point(&mut start_ctx, max_steps)?;
    if !matches!(
        start_ctx.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) || start_ctx.combat_state.is_none()
    {
        return Err(format!(
            "trace step {} replayed to non-combat decision {}",
            config.step_index,
            engine_state_label(&start_ctx.engine_state)
        ));
    }

    let start_combat_win_count = start_ctx.combat_win_count;
    let start_monster_hp = total_alive_monster_hp_for_ctx(&start_ctx);
    let start_primary_monster_hp = primary_alive_monster_hp_for_ctx(&start_ctx);
    let start_hp = current_player_hp_for_ctx(&start_ctx);
    let start_legal = legal_actions(
        &start_ctx.engine_state,
        &start_ctx.run_state,
        &start_ctx.combat_state,
    );
    let start_candidates = build_action_candidates(&start_legal, Some(&start_ctx));

    let mut beam = vec![CombatPlanSearchNode {
        ctx: start_ctx.clone(),
        action_keys: Vec::new(),
        depth: 0,
        terminal_kind: None,
        engine_error: None,
    }];
    let mut terminal_nodes: Vec<CombatPlanSearchNode> = Vec::new();
    let mut censored_nodes: Vec<CombatPlanSearchNode> = Vec::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();
    let mut visited_best: std::collections::HashMap<String, CombatPlanSearchScore> =
        std::collections::HashMap::new();
    let mut expanded_nodes = 0usize;
    let mut generated_nodes = 1usize;
    let mut max_depth_reached = 0usize;
    let mut budget_exhausted = false;
    let mut node_budget_censored_count = 0usize;
    let mut max_depth_censored_count = 0usize;
    let mut branching_pruned_count = 0usize;
    let mut beam_pruned_count = 0usize;
    let mut turn_sequence_beam_pruned_count = 0usize;
    let mut state_dedup_pruned_count = 0usize;

    for _depth in 0..config.max_depth_decisions {
        if beam.is_empty() || budget_exhausted {
            break;
        }
        let mut next_nodes = Vec::<CombatPlanSearchNode>::new();
        for node in beam.into_iter() {
            if generated_nodes >= config.max_nodes {
                budget_exhausted = true;
                node_budget_censored_count += 1;
                censored_nodes.push(node);
                break;
            }
            let mut prepared = node.clone();
            if let Err(err) = prepare_decision_point(&mut prepared.ctx, max_steps) {
                let mut failed = prepared;
                failed.terminal_kind = Some("engine_error".to_string());
                failed.engine_error = Some(err.clone());
                errors.push(json!({
                    "depth": failed.depth,
                    "action_keys": failed.action_keys,
                    "error": err,
                }));
                terminal_nodes.push(failed);
                continue;
            }
            if let Some(kind) =
                combat_plan_terminal_kind(&prepared.ctx, start_combat_win_count)
            {
                let mut terminal = prepared;
                terminal.terminal_kind = Some(kind);
                terminal_nodes.push(terminal);
                continue;
            }
            if prepared.depth >= config.max_depth_decisions {
                max_depth_censored_count += 1;
                censored_nodes.push(prepared);
                continue;
            }
            let start_turn_count = prepared
                .ctx
                .combat_state
                .as_ref()
                .map(|combat| combat.turn.turn_count);
            let mut turn_frontier = vec![prepared];
            let mut turn_steps = 0usize;
            while !turn_frontier.is_empty() {
                if budget_exhausted {
                    break;
                }
                if turn_steps >= config.max_turn_sequence_actions {
                    max_depth_censored_count += turn_frontier.len();
                    censored_nodes.extend(turn_frontier);
                    break;
                }
                let mut turn_next = Vec::<CombatPlanSearchNode>::new();
                for mut seq_node in turn_frontier.into_iter() {
                    if generated_nodes >= config.max_nodes {
                        budget_exhausted = true;
                        node_budget_censored_count += 1;
                        censored_nodes.push(seq_node);
                        break;
                    }
                    if let Err(err) = prepare_decision_point(&mut seq_node.ctx, max_steps) {
                        let mut failed = seq_node;
                        failed.terminal_kind = Some("engine_error".to_string());
                        failed.engine_error = Some(err.clone());
                        errors.push(json!({
                            "depth": failed.depth,
                            "action_keys": failed.action_keys,
                            "error": err,
                        }));
                        terminal_nodes.push(failed);
                        continue;
                    }
                    if let Some(kind) =
                        combat_plan_terminal_kind(&seq_node.ctx, start_combat_win_count)
                    {
                        let mut terminal = seq_node;
                        terminal.terminal_kind = Some(kind);
                        terminal_nodes.push(terminal);
                        continue;
                    }
                    if seq_node.depth >= config.max_depth_decisions {
                        max_depth_censored_count += 1;
                        censored_nodes.push(seq_node);
                        continue;
                    }
                    let current_turn_count = seq_node
                        .ctx
                        .combat_state
                        .as_ref()
                        .map(|combat| combat.turn.turn_count);
                    if start_turn_count.is_some()
                        && current_turn_count.is_some()
                        && current_turn_count != start_turn_count
                    {
                        let score = combat_plan_score(
                            &seq_node,
                            start_combat_win_count,
                            start_monster_hp,
                            start_primary_monster_hp,
                        );
                        let key = combat_plan_state_key(&seq_node.ctx, seq_node.depth);
                        if visited_best
                            .get(&key)
                            .is_some_and(|existing| existing >= &score)
                        {
                            state_dedup_pruned_count += 1;
                            continue;
                        }
                        visited_best.insert(key, score);
                        next_nodes.push(seq_node);
                        continue;
                    }

                    let legal = legal_actions(
                        &seq_node.ctx.engine_state,
                        &seq_node.ctx.run_state,
                        &seq_node.ctx.combat_state,
                    );
                    if legal.is_empty() {
                        let mut failed = seq_node;
                        failed.terminal_kind = Some("no_legal_actions".to_string());
                        failed.engine_error = Some(format!(
                            "no legal actions at {} floor {}",
                            engine_state_label(&failed.ctx.engine_state),
                            failed.ctx.run_state.floor_num
                        ));
                        terminal_nodes.push(failed);
                        continue;
                    }
                    expanded_nodes += 1;
                    let candidates = build_action_candidates(&legal, Some(&seq_node.ctx));
                    let mut children = Vec::<CombatPlanSearchNode>::new();
                    for (action_index, input) in legal.into_iter().enumerate() {
                        if generated_nodes >= config.max_nodes {
                            budget_exhausted = true;
                            break;
                        }
                        let child_ctx = seq_node.ctx.clone();
                        let key = candidates
                            .get(action_index)
                            .map(|candidate| candidate.action_key.clone())
                            .unwrap_or_else(|| {
                                action_key_for_input(&input, child_ctx.combat_state.as_ref())
                            });
                        let mut action_keys = seq_node.action_keys.clone();
                        action_keys.push(key);
                        let mut child = CombatPlanSearchNode {
                            ctx: child_ctx,
                            action_keys,
                            depth: seq_node.depth + 1,
                            terminal_kind: None,
                            engine_error: None,
                        };
                        if let Err(err) = apply_rollout_action(&mut child.ctx, input, max_steps) {
                            child.terminal_kind = Some("engine_error".to_string());
                            child.engine_error = Some(err.clone());
                            errors.push(json!({
                                "depth": child.depth,
                                "action_keys": child.action_keys,
                                "error": err,
                            }));
                        } else if let Err(err) = prepare_decision_point(&mut child.ctx, max_steps) {
                            child.terminal_kind = Some("engine_error".to_string());
                            child.engine_error = Some(err.clone());
                            errors.push(json!({
                                "depth": child.depth,
                                "action_keys": child.action_keys,
                                "error": err,
                            }));
                        } else if let Some(kind) =
                            combat_plan_terminal_kind(&child.ctx, start_combat_win_count)
                        {
                            child.terminal_kind = Some(kind);
                        }
                        generated_nodes += 1;
                        max_depth_reached = max_depth_reached.max(child.depth);
                        children.push(child);
                    }

                    children.sort_by(|left, right| {
                        combat_plan_score(
                            right,
                            start_combat_win_count,
                            start_monster_hp,
                            start_primary_monster_hp,
                        )
                        .cmp(&combat_plan_score(
                            left,
                            start_combat_win_count,
                            start_monster_hp,
                            start_primary_monster_hp,
                        ))
                    });
                    if let Some(max_branching) = config.max_branching {
                        if children.len() > max_branching {
                            branching_pruned_count += children.len() - max_branching;
                        }
                        children.truncate(max_branching);
                    }

                    for child in children {
                        if child.terminal_kind.is_some() {
                            terminal_nodes.push(child);
                            continue;
                        }
                        let current_turn_count = child
                            .ctx
                            .combat_state
                            .as_ref()
                            .map(|combat| combat.turn.turn_count);
                        if start_turn_count.is_some()
                            && current_turn_count.is_some()
                            && current_turn_count != start_turn_count
                        {
                            let score = combat_plan_score(
                                &child,
                                start_combat_win_count,
                                start_monster_hp,
                                start_primary_monster_hp,
                            );
                            let key = combat_plan_state_key(&child.ctx, child.depth);
                            if visited_best
                                .get(&key)
                                .is_some_and(|existing| existing >= &score)
                            {
                                state_dedup_pruned_count += 1;
                                continue;
                            }
                            visited_best.insert(key, score);
                            next_nodes.push(child);
                        } else {
                            turn_next.push(child);
                        }
                    }
                }
                turn_next.sort_by(|left, right| {
                    combat_plan_score(right, start_combat_win_count, start_monster_hp, start_primary_monster_hp)
                        .cmp(&combat_plan_score(left, start_combat_win_count, start_monster_hp, start_primary_monster_hp))
                });
                if turn_next.len() > config.turn_sequence_beam_width {
                    turn_sequence_beam_pruned_count +=
                        turn_next.len() - config.turn_sequence_beam_width;
                    turn_next.truncate(config.turn_sequence_beam_width);
                }
                turn_frontier = turn_next;
                turn_steps += 1;
            }
        }
        next_nodes.sort_by(|left, right| {
        combat_plan_score(right, start_combat_win_count, start_monster_hp, start_primary_monster_hp)
            .cmp(&combat_plan_score(left, start_combat_win_count, start_monster_hp, start_primary_monster_hp))
        });
        if next_nodes.len() > config.beam_width {
            beam_pruned_count += next_nodes.len() - config.beam_width;
            next_nodes.truncate(config.beam_width);
        }
        beam = next_nodes;
    }
    let final_frontier_censored_count = beam.len();
    censored_nodes.extend(beam);

    let mut terminal_counts = std::collections::BTreeMap::<String, usize>::new();
    for node in &terminal_nodes {
        *terminal_counts
            .entry(
                node.terminal_kind
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
            )
            .or_insert(0) += 1;
    }

    let best_clear = best_node_by_kind(
        &terminal_nodes,
        "combat_cleared",
        start_combat_win_count,
        start_monster_hp,
        start_primary_monster_hp,
    );
    let best_death = best_node_by_kind(
        &terminal_nodes,
        "death",
        start_combat_win_count,
        start_monster_hp,
        start_primary_monster_hp,
    );
    let best_censored =
        best_node(&censored_nodes, start_combat_win_count, start_monster_hp, start_primary_monster_hp);
    let search_limited_by = {
        let mut reasons = Vec::new();
        if budget_exhausted {
            reasons.push("node_budget");
        }
        if branching_pruned_count > 0 {
            reasons.push("branching_cap");
        }
        if beam_pruned_count > 0 {
            reasons.push("beam_width");
        }
        if turn_sequence_beam_pruned_count > 0 {
            reasons.push("turn_sequence_beam_width");
        }
        if final_frontier_censored_count > 0 || max_depth_censored_count > 0 {
            reasons.push("depth_or_frontier_censor");
        }
        reasons
    };
    let search_exhaustive_under_config = search_limited_by.is_empty() && censored_nodes.is_empty();

    let frontier = if config.include_frontier {
        let mut frontier = censored_nodes.clone();
        frontier.sort_by(|left, right| {
            combat_plan_score(
                right,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
            )
            .cmp(&combat_plan_score(
                left,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
            ))
        });
        json!(
            frontier
                .iter()
                .take(20)
                .map(|node| {
                    combat_plan_node_json(
                        node,
                        start_combat_win_count,
                        start_monster_hp,
                        start_primary_monster_hp,
                        "censored_frontier",
                    )
                })
                .collect::<Vec<_>>()
        )
    } else {
        serde_json::Value::Null
    };

    Ok(json!({
        "schema_version": "combat_plan_search_from_trace_v0",
        "contract": {
            "uses_frontier_eval": false,
            "uses_exact_turn_best_line": false,
            "uses_action_label": false,
            "beam_role": "scheduler_only",
            "expansion_unit": "current_turn_sequence_to_next_turn_boundary",
            "terminal_priority": "combat_cleared > alive_censored > death"
        },
        "source_trace": trace_probe_source(
            &config.trace_file,
            config.step_index,
            seed,
            ascension,
            final_act,
            &player_class,
            &trace,
            &target_trace_step,
        ),
        "config": {
            "max_nodes": config.max_nodes,
            "beam_width": config.beam_width,
            "max_depth_decisions": config.max_depth_decisions,
            "turn_sequence_beam_width": config.turn_sequence_beam_width,
            "max_turn_sequence_actions": config.max_turn_sequence_actions,
            "max_branching": config.max_branching,
            "max_steps": max_steps,
        },
        "start": {
            "step_index": config.step_index,
            "floor": start_ctx.run_state.floor_num,
            "act": start_ctx.run_state.act_num,
            "hp": start_hp,
            "max_hp": start_ctx.run_state.max_hp,
            "gold": start_ctx.run_state.gold,
            "combat_win_count": start_combat_win_count,
            "monster_hp": start_monster_hp,
            "primary_monster_hp": start_primary_monster_hp,
            "engine_state": engine_state_label(&start_ctx.engine_state),
            "legal_action_count": start_candidates.len(),
            "legal_actions": start_candidates,
        },
        "search_summary": {
            "generated_nodes": generated_nodes,
            "expanded_nodes": expanded_nodes,
            "visited_state_count": visited_best.len(),
            "terminal_counts": terminal_counts,
            "censored_count": censored_nodes.len(),
            "budget_exhausted": budget_exhausted,
            "node_budget_censored_count": node_budget_censored_count,
            "max_depth_censored_count": max_depth_censored_count,
            "final_frontier_censored_count": final_frontier_censored_count,
            "branching_pruned_count": branching_pruned_count,
            "beam_pruned_count": beam_pruned_count,
            "turn_sequence_beam_pruned_count": turn_sequence_beam_pruned_count,
            "state_dedup_pruned_count": state_dedup_pruned_count,
            "state_dedup_assumed_equivalent": true,
            "search_exhaustive_under_config": search_exhaustive_under_config,
            "search_limited_by": search_limited_by,
            "max_depth_reached": max_depth_reached,
            "error_count": errors.len(),
        },
        "best_complete_clear": best_clear.map(|node| {
            combat_plan_node_json(
                node,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
                "combat_cleared",
            )
        }),
        "best_alive_censored": best_censored.map(|node| {
            combat_plan_node_json(
                node,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
                "censored_frontier",
            )
        }),
        "best_death": best_death.map(|node| {
            combat_plan_node_json(
                node,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
                "death",
            )
        }),
        "frontier": frontier,
        "errors": errors,
    }))
}

fn best_node_by_kind<'a>(
    nodes: &'a [CombatPlanSearchNode],
    kind: &str,
    start_combat_win_count: usize,
    start_monster_hp: i32,
    start_primary_monster_hp: i32,
) -> Option<&'a CombatPlanSearchNode> {
    nodes
        .iter()
        .filter(|node| node.terminal_kind.as_deref() == Some(kind))
        .max_by(|left, right| {
            combat_plan_score(
                left,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
            )
            .cmp(&combat_plan_score(
                right,
                start_combat_win_count,
                start_monster_hp,
                start_primary_monster_hp,
            ))
        })
}

fn best_node<'a>(
    nodes: &'a [CombatPlanSearchNode],
    start_combat_win_count: usize,
    start_monster_hp: i32,
    start_primary_monster_hp: i32,
) -> Option<&'a CombatPlanSearchNode> {
    nodes.iter().max_by(|left, right| {
        combat_plan_score(
            left,
            start_combat_win_count,
            start_monster_hp,
            start_primary_monster_hp,
        )
        .cmp(&combat_plan_score(
            right,
            start_combat_win_count,
            start_monster_hp,
            start_primary_monster_hp,
        ))
    })
}

fn combat_plan_terminal_kind(
    ctx: &EpisodeContext,
    start_combat_win_count: usize,
) -> Option<String> {
    if matches!(ctx.engine_state, EngineState::GameOver(RunResult::Defeat))
        || current_player_hp_for_ctx(ctx) <= 0
    {
        return Some("death".to_string());
    }
    if ctx.combat_win_count > start_combat_win_count {
        return Some("combat_cleared".to_string());
    }
    if matches!(ctx.engine_state, EngineState::GameOver(RunResult::Victory)) {
        return Some("combat_cleared".to_string());
    }
    if ctx.combat_state.is_none()
        && !matches!(
            ctx.engine_state,
            EngineState::CombatPlayerTurn | EngineState::CombatProcessing | EngineState::PendingChoice(_)
        )
    {
        return Some("combat_cleared".to_string());
    }
    None
}

fn combat_plan_score(
    node: &CombatPlanSearchNode,
    start_combat_win_count: usize,
    start_monster_hp: i32,
    start_primary_monster_hp: i32,
) -> CombatPlanSearchScore {
    let terminal = node.terminal_kind.as_deref();
    let hp = current_player_hp_for_ctx(&node.ctx);
    let current_monster_hp = total_alive_monster_hp_for_ctx(&node.ctx);
    let current_primary_monster_hp = primary_alive_monster_hp_for_ctx(&node.ctx);
    let primary_monster_hp_reduction =
        (start_primary_monster_hp - current_primary_monster_hp).max(0);
    let monster_hp_reduction = (start_monster_hp - current_monster_hp).max(0);
    let incoming = node
        .ctx
        .combat_state
        .as_ref()
        .map(visible_incoming_damage)
        .unwrap_or(0);
    let block = node
        .ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.block)
        .unwrap_or(0);
    let unblocked = (incoming - block).max(0);
    let combat_win_delta = node
        .ctx
        .combat_win_count
        .saturating_sub(start_combat_win_count) as i32;
    let terminal_bucket = match terminal {
        Some("combat_cleared") => 4,
        None => 2,
        Some("death") => 0,
        Some("engine_error") | Some("no_legal_actions") => -1,
        Some(_) => 1,
    };
    CombatPlanSearchScore {
        terminal_bucket,
        alive: if hp > 0 { 1 } else { 0 },
        combat_win_delta,
        primary_monster_hp_reduction,
        monster_hp_reduction,
        incoming_safety: -unblocked,
        hp,
        block,
        depth_neg: -(node.depth as i32),
    }
}

fn combat_plan_node_json(
    node: &CombatPlanSearchNode,
    start_combat_win_count: usize,
    start_monster_hp: i32,
    start_primary_monster_hp: i32,
    label: &str,
) -> serde_json::Value {
    let hp = current_player_hp_for_ctx(&node.ctx);
    let monster_hp = total_alive_monster_hp_for_ctx(&node.ctx);
    let primary_monster_hp = primary_alive_monster_hp_for_ctx(&node.ctx);
    let incoming = node
        .ctx
        .combat_state
        .as_ref()
        .map(visible_incoming_damage)
        .unwrap_or(0);
    let block = node
        .ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.block)
        .unwrap_or(0);
    json!({
        "label": label,
        "terminal_kind": node.terminal_kind,
        "engine_error": node.engine_error,
        "depth": node.depth,
        "score": combat_plan_score(
            node,
            start_combat_win_count,
            start_monster_hp,
            start_primary_monster_hp
        ),
        "action_keys": node.action_keys,
        "first_action_key": node.action_keys.first(),
        "last_action_key": node.action_keys.last(),
        "state": {
            "engine_state": engine_state_label(&node.ctx.engine_state),
            "decision_type": decision_type(&node.ctx.engine_state),
            "floor": node.ctx.run_state.floor_num,
            "act": node.ctx.run_state.act_num,
            "hp": hp,
            "max_hp": node.ctx.run_state.max_hp,
            "block": block,
            "incoming": incoming,
            "unblocked": (incoming - block).max(0),
            "monster_hp": monster_hp,
            "monster_hp_reduction": (start_monster_hp - monster_hp).max(0),
            "primary_monster_hp": primary_monster_hp,
            "primary_monster_hp_reduction": (start_primary_monster_hp - primary_monster_hp).max(0),
            "combat_win_delta": node.ctx.combat_win_count.saturating_sub(start_combat_win_count),
            "hand": node.ctx.combat_state.as_ref().map(|combat| {
                combat.zones.hand.iter().map(|card| format!("{:?}", card.id)).collect::<Vec<_>>()
            }),
        }
    })
}

fn current_player_hp_for_ctx(ctx: &EpisodeContext) -> i32 {
    ctx.combat_state
        .as_ref()
        .map(|combat| combat.entities.player.current_hp)
        .unwrap_or(ctx.run_state.current_hp)
}

fn total_alive_monster_hp_for_ctx(ctx: &EpisodeContext) -> i32 {
    ctx.combat_state
        .as_ref()
        .map(|combat| {
            combat
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.current_hp > 0 && !monster.is_dying && !monster.is_escaped)
                .map(|monster| monster.current_hp + monster.block)
                .sum()
        })
        .unwrap_or(0)
}

fn primary_alive_monster_hp_for_ctx(ctx: &EpisodeContext) -> i32 {
    ctx.combat_state
        .as_ref()
        .and_then(|combat| {
            combat
                .entities
                .monsters
                .iter()
                .find(|monster| {
                    monster.current_hp > 0 && !monster.is_dying && !monster.is_escaped
                })
                .map(|monster| monster.current_hp + monster.block)
        })
        .unwrap_or(0)
}

fn combat_plan_state_key(ctx: &EpisodeContext, depth: usize) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    depth.hash(&mut hasher);
    format!("{:?}", ctx.engine_state).hash(&mut hasher);
    ctx.run_state.floor_num.hash(&mut hasher);
    ctx.run_state.current_hp.hash(&mut hasher);
    ctx.combat_win_count.hash(&mut hasher);
    if let Some(combat) = ctx.combat_state.as_ref() {
        combat.turn.turn_count.hash(&mut hasher);
        combat.turn.energy.hash(&mut hasher);
        combat.entities.player.current_hp.hash(&mut hasher);
        combat.entities.player.block.hash(&mut hasher);
        for card in &combat.zones.hand {
            card.uuid.hash(&mut hasher);
            card.id.hash(&mut hasher);
            card.cost_for_turn.hash(&mut hasher);
        }
        for card in &combat.zones.draw_pile {
            card.uuid.hash(&mut hasher);
            card.id.hash(&mut hasher);
        }
        for card in &combat.zones.discard_pile {
            card.uuid.hash(&mut hasher);
            card.id.hash(&mut hasher);
        }
        for monster in &combat.entities.monsters {
            monster.id.hash(&mut hasher);
            monster.current_hp.hash(&mut hasher);
            monster.block.hash(&mut hasher);
            monster.planned_move_id().hash(&mut hasher);
            monster.is_dying.hash(&mut hasher);
            monster.is_escaped.hash(&mut hasher);
        }
    }
    format!("{:016x}", hasher.finish())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RecursiveRolloutUtility {
    alive: i32,
    combat_cleared: i32,
    combat_win_delta: i32,
    hp_loss_neg: i32,
    monster_hp_reduction: i32,
    final_block: i32,
    decision_steps_neg: i32,
}

fn rollout_root_candidate_with_continuation(
    start: &EpisodeContext,
    root_input: ClientInput,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    max_steps: usize,
) -> serde_json::Value {
    let mut ctx = start.clone();
    let start_hp = rollout_player_hp(&ctx);
    let start_monster_hp = ctx
        .combat_state
        .as_ref()
        .map(total_living_monster_hp_for_pack)
        .unwrap_or(0);
    let start_combat_wins = ctx.combat_win_count;
    let root_action_key = action_key_for_input(&root_input, ctx.combat_state.as_ref());
    let mut decision_steps = 0usize;
    let mut chosen_action_keys = Vec::new();
    let mut engine_errors = Vec::new();
    let mut terminal_kind = "horizon".to_string();

    match apply_rollout_action(&mut ctx, root_input, max_steps) {
        Ok(_) => {
            decision_steps += 1;
            chosen_action_keys.push(root_action_key);
        }
        Err(err) => {
            engine_errors.push(err);
            terminal_kind = "engine_error".to_string();
        }
    }

    while engine_errors.is_empty() && decision_steps < horizon_decisions {
        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_kind = match ctx.engine_state {
                EngineState::GameOver(RunResult::Victory) => "victory".to_string(),
                EngineState::GameOver(RunResult::Defeat) => "defeat".to_string(),
                _ => "game_over".to_string(),
            };
            break;
        }

        if let Err(err) = prepare_decision_point(&mut ctx, max_steps) {
            engine_errors.push(err);
            terminal_kind = "engine_error".to_string();
            break;
        }

        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_kind = match ctx.engine_state {
                EngineState::GameOver(RunResult::Victory) => "victory".to_string(),
                EngineState::GameOver(RunResult::Defeat) => "defeat".to_string(),
                _ => "game_over".to_string(),
            };
            break;
        }

        if ctx.combat_win_count > start_combat_wins {
            terminal_kind = "combat_cleared".to_string();
            break;
        }

        if !matches!(
            ctx.engine_state,
            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
        ) {
            terminal_kind = "noncombat_frontier".to_string();
            break;
        }

        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        if legal.is_empty() {
            terminal_kind = "no_legal_actions".to_string();
            break;
        }

        let chosen_index = match continuation_policy {
            RunPolicyKind::RuleBaselineV0
            | RunPolicyKind::RuleBaselineV1Candidate
            | RunPolicyKind::RandomMasked => choose_rule_baseline_action(&ctx, &legal),
            RunPolicyKind::RuleBaselineV0Control => {
                choose_rule_baseline_v0_control_action(&ctx, &legal)
            }
            RunPolicyKind::PlanQueryV0 => choose_plan_query_action(&ctx, &legal)
                .unwrap_or_else(|| choose_rule_baseline_action(&ctx, &legal)),
        };
        let Some(input) = legal.get(chosen_index).cloned() else {
            engine_errors.push(format!(
                "continuation policy returned out-of-range action index {chosen_index}"
            ));
            terminal_kind = "engine_error".to_string();
            break;
        };
        chosen_action_keys.push(action_key_for_input(&input, ctx.combat_state.as_ref()));
        match apply_rollout_action(&mut ctx, input, max_steps) {
            Ok(_) => decision_steps += 1,
            Err(err) => {
                engine_errors.push(err);
                terminal_kind = "engine_error".to_string();
                break;
            }
        }
    }

    if engine_errors.is_empty() {
        if matches!(ctx.engine_state, EngineState::GameOver(RunResult::Defeat)) {
            terminal_kind = "defeat".to_string();
        } else if matches!(ctx.engine_state, EngineState::GameOver(RunResult::Victory)) {
            terminal_kind = "victory".to_string();
        } else if ctx.combat_win_count > start_combat_wins
            || ctx
                .combat_state
                .as_ref()
                .is_some_and(combat_cleared_for_pack)
        {
            terminal_kind = "combat_cleared".to_string();
        }
    }

    let final_hp = rollout_player_hp(&ctx);
    let final_block = ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.block)
        .unwrap_or(0);
    let final_monster_hp = ctx
        .combat_state
        .as_ref()
        .map(total_living_monster_hp_for_pack)
        .unwrap_or(0);
    let combat_win_delta = ctx.combat_win_count.saturating_sub(start_combat_wins);
    let player_dead =
        matches!(ctx.engine_state, EngineState::GameOver(RunResult::Defeat)) || final_hp <= 0;
    let combat_cleared = combat_win_delta > 0
        || ctx
            .combat_state
            .as_ref()
            .is_some_and(combat_cleared_for_pack);

    serde_json::json!({
        "terminal_kind": terminal_kind,
        "engine_state": engine_state_label(&ctx.engine_state),
        "decision_steps": decision_steps,
        "horizon_decisions": horizon_decisions,
        "continuation_policy": continuation_policy.as_str(),
        "chosen_action_keys": chosen_action_keys,
        "engine_errors": engine_errors,
        "combat_win_delta": combat_win_delta,
        "combat_cleared": combat_cleared,
        "player_dead": player_dead,
        "start_hp": start_hp,
        "final_hp": final_hp,
        "hp_lost": (start_hp - final_hp).max(0),
        "start_monster_hp": start_monster_hp,
        "final_monster_hp": final_monster_hp,
        "monster_hp_reduction": (start_monster_hp - final_monster_hp).max(0),
        "final_block": final_block,
        "final_energy": ctx.combat_state.as_ref().map(|combat| combat.turn.energy as i32),
        "final_hand_count": ctx.combat_state.as_ref().map(|combat| combat.zones.hand.len()),
        "final_draw_count": ctx.combat_state.as_ref().map(|combat| combat.zones.draw_pile.len()),
        "final_discard_count": ctx.combat_state.as_ref().map(|combat| combat.zones.discard_pile.len()),
    })
}

fn apply_rollout_action(
    ctx: &mut EpisodeContext,
    action: ClientInput,
    _max_steps: usize,
) -> Result<(), String> {
    let keep_running = tick_run(
        &mut ctx.engine_state,
        &mut ctx.run_state,
        &mut ctx.combat_state,
        Some(action),
    );
    if let Some(errors) = take_engine_error_diagnostics(ctx) {
        return Err(errors.join("; "));
    }
    finish_combat_if_needed(ctx);
    if !keep_running && !matches!(ctx.engine_state, EngineState::GameOver(_)) {
        return Err(format!(
            "engine stopped at non-terminal state {}",
            engine_state_label(&ctx.engine_state)
        ));
    }
    Ok(())
}

fn rollout_player_hp(ctx: &EpisodeContext) -> i32 {
    ctx.combat_state
        .as_ref()
        .map(|combat| combat.entities.player.current_hp)
        .unwrap_or(ctx.run_state.current_hp)
}

fn recursive_rollout_utility(outcome: &serde_json::Value) -> RecursiveRolloutUtility {
    let player_dead = outcome
        .get("player_dead")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let combat_cleared = outcome
        .get("combat_cleared")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    RecursiveRolloutUtility {
        alive: if player_dead { 0 } else { 1 },
        combat_cleared: if combat_cleared { 1 } else { 0 },
        combat_win_delta: json_i32(outcome, "combat_win_delta"),
        hp_loss_neg: -json_i32(outcome, "hp_lost"),
        monster_hp_reduction: json_i32(outcome, "monster_hp_reduction"),
        final_block: json_i32(outcome, "final_block"),
        decision_steps_neg: -json_i32(outcome, "decision_steps"),
    }
}

fn recursive_rollout_pairwise_labels(
    utilities: &[(usize, String, RecursiveRolloutUtility)],
) -> Vec<CombatCandidatePairwiseLabel> {
    let mut labels = Vec::new();
    for left_index in 0..utilities.len() {
        for right_index in (left_index + 1)..utilities.len() {
            let left = &utilities[left_index];
            let right = &utilities[right_index];
            if left.2 == right.2 {
                continue;
            }
            let (preferred, rejected) = if left.2 > right.2 {
                (left, right)
            } else {
                (right, left)
            };
            labels.push(CombatCandidatePairwiseLabel {
                objective: "recursive_rollout_value".to_string(),
                preferred_candidate_index: preferred.0,
                rejected_candidate_index: rejected.0,
                preferred_action_key: preferred.1.clone(),
                rejected_action_key: rejected.1.clone(),
                confidence: "rollout_horizon".to_string(),
                reason: first_recursive_utility_gap(&preferred.2, &rejected.2),
                interval_gap: 1,
                label_source: "recursive_rollout_validation_v0".to_string(),
            });
        }
    }
    labels
}

fn first_recursive_utility_gap(
    preferred: &RecursiveRolloutUtility,
    rejected: &RecursiveRolloutUtility,
) -> String {
    if preferred.alive != rejected.alive {
        return "preferred survives while rejected dies".to_string();
    }
    if preferred.combat_cleared != rejected.combat_cleared {
        return "preferred clears combat within rollout horizon".to_string();
    }
    if preferred.combat_win_delta != rejected.combat_win_delta {
        return "preferred has higher combat win delta".to_string();
    }
    if preferred.hp_loss_neg != rejected.hp_loss_neg {
        return "preferred loses less hp".to_string();
    }
    if preferred.monster_hp_reduction != rejected.monster_hp_reduction {
        return "preferred reduces more monster hp".to_string();
    }
    if preferred.final_block != rejected.final_block {
        return "preferred ends with more block".to_string();
    }
    "preferred reaches comparable outcome with fewer decisions".to_string()
}

fn json_i32(value: &serde_json::Value, key: &str) -> i32 {
    value
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
        .clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn draw_marginal_target_from_trace_config(
    engine: &EngineState,
    combat: &CombatState,
    target_card: CardId,
    target_hand_index: Option<usize>,
    target_action_key: Option<String>,
) -> Result<crate::bot::combat::CombatDrawMarginalTarget, String> {
    let mut target = if let Some(hand_index) = target_hand_index {
        let card = combat.zones.hand.get(hand_index).ok_or_else(|| {
            format!(
                "target hand index {} out of range for hand size {}",
                hand_index,
                combat.zones.hand.len()
            )
        })?;
        if card.id != target_card {
            return Err(format!(
                "target hand index {} has {:?}, expected {:?}",
                hand_index, card.id, target_card
            ));
        }
        crate::bot::combat::CombatDrawMarginalTarget::hand_instance(
            target_card,
            hand_index,
            card.uuid,
        )
    } else {
        crate::bot::combat::CombatDrawMarginalTarget::card(target_card)
    };
    if let Some(action_key) = target_action_key {
        let legal_keys = crate::bot::combat::legal_moves::get_legal_moves(engine, combat)
            .into_iter()
            .map(|action| action_key_for_input(&action, Some(combat)))
            .collect::<Vec<_>>();
        if !legal_keys.iter().any(|key| key == &action_key) {
            return Err(format!(
                "target action key '{}' is not legal at replayed step; legal keys: {:?}",
                action_key, legal_keys
            ));
        }
        target = target.with_root_action_key(action_key);
    }
    Ok(target)
}

fn controlled_v0_root_input(input: &ClientInput) -> bool {
    matches!(input, ClientInput::PlayCard { .. } | ClientInput::EndTurn)
}

fn bounded_objective_oracle_for_root(
    engine: &EngineState,
    start: &CombatState,
    input: &ClientInput,
    max_engine_steps: usize,
) -> CombatCandidateBoundedObjectives {
    let start_total_hp = total_living_monster_hp_for_pack(start);
    let (root_engine, root_combat, root_steps, root_truncated) =
        simulate_root_to_bounded_frontier(engine, start, input, max_engine_steps);
    let root_total_hp = total_living_monster_hp_for_pack(&root_combat);
    let damage_done_immediate = (start_total_hp - root_total_hp).max(0);
    let hp_loss_immediate =
        (start.entities.player.current_hp - root_combat.entities.player.current_hp).max(0);
    let incoming = visible_incoming_damage_for_pack(&root_combat);
    let (hand_damage_upper, hand_block_upper, setup_upper, access_notes) =
        remaining_hand_upper_bounds(&root_combat);
    let damage_upper_bound = (damage_done_immediate + hand_damage_upper).min(start_total_hp);
    let block_after_root = root_combat.entities.player.block;
    let block_upper_bound = block_after_root + hand_block_upper;
    let hp_loss_lower_bound = hp_loss_immediate + (incoming - block_upper_bound).max(0);
    let hp_loss_upper_bound = hp_loss_immediate + (incoming - block_after_root).max(0);
    let lethal_lower_bound = if combat_cleared_for_pack(&root_combat) {
        1
    } else {
        0
    };
    let lethal_upper_bound = if root_total_hp <= hand_damage_upper {
        1
    } else {
        lethal_lower_bound
    };
    let setup_lower_bound = if root_played_setup(input, start) {
        1
    } else {
        0
    };
    let setup_upper_bound = setup_lower_bound.max(setup_upper);

    let mut uncertainty_flags = access_notes;
    if root_truncated {
        uncertainty_flags.push("root_simulation_truncated".to_string());
    }
    if matches!(root_engine, EngineState::PendingChoice(_)) {
        uncertainty_flags
            .push("root_stopped_at_pending_choice_without_branch_expansion".to_string());
    }
    if hand_has_draw_or_generation(&root_combat) {
        uncertainty_flags.push("remaining_hand_contains_draw_or_generation".to_string());
    }

    let confidence = if uncertainty_flags.is_empty() {
        "bounded_conservative"
    } else {
        "bounded_uncertain"
    };
    let objective_bounds = vec![
        CombatCandidateObjectiveBound {
            objective: "lethal".to_string(),
            lower_bound: lethal_lower_bound,
            upper_bound: lethal_upper_bound,
            higher_is_better: true,
            confidence: confidence.to_string(),
            notes: uncertainty_flags.clone(),
        },
        CombatCandidateObjectiveBound {
            objective: "damage".to_string(),
            lower_bound: damage_done_immediate,
            upper_bound: damage_upper_bound,
            higher_is_better: true,
            confidence: confidence.to_string(),
            notes: uncertainty_flags.clone(),
        },
        CombatCandidateObjectiveBound {
            objective: "hp_loss".to_string(),
            lower_bound: hp_loss_lower_bound,
            upper_bound: hp_loss_upper_bound,
            higher_is_better: false,
            confidence: confidence.to_string(),
            notes: uncertainty_flags.clone(),
        },
        CombatCandidateObjectiveBound {
            objective: "block".to_string(),
            lower_bound: block_after_root,
            upper_bound: block_upper_bound,
            higher_is_better: true,
            confidence: confidence.to_string(),
            notes: uncertainty_flags.clone(),
        },
        CombatCandidateObjectiveBound {
            objective: "setup".to_string(),
            lower_bound: setup_lower_bound,
            upper_bound: setup_upper_bound,
            higher_is_better: true,
            confidence: confidence.to_string(),
            notes: uncertainty_flags.clone(),
        },
    ];

    CombatCandidateBoundedObjectives {
        oracle_kind: "root_action_bounded_objective_v0".to_string(),
        root_simulation_status: if root_truncated { "truncated" } else { "ok" }.to_string(),
        root_engine_state: engine_state_label(&root_engine).to_string(),
        root_engine_steps: root_steps,
        root_simulation_truncated: root_truncated,
        uncertainty_flags,
        damage_done_immediate,
        damage_upper_bound,
        hp_loss_lower_bound,
        hp_loss_upper_bound,
        block_after_root,
        block_upper_bound,
        lethal_lower_bound,
        lethal_upper_bound,
        setup_lower_bound,
        setup_upper_bound,
        objective_bounds,
    }
}

fn simulate_root_to_bounded_frontier(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    max_engine_steps: usize,
) -> (EngineState, CombatState, u32, bool) {
    let mut next_engine = engine.clone();
    let mut next_combat = combat.clone();
    let mut steps = 0usize;
    let budget = max_engine_steps.max(1);
    let alive =
        crate::engine::core::tick_engine(&mut next_engine, &mut next_combat, Some(input.clone()));
    steps += 1;
    if !alive {
        return (next_engine, next_combat, steps as u32, false);
    }
    normalize_bounded_root_processing(&mut next_engine, &next_combat);
    while !bounded_root_frontier_is_stable(&next_engine, &next_combat) {
        if steps >= budget {
            return (next_engine, next_combat, steps as u32, true);
        }
        let alive = crate::engine::core::tick_engine(&mut next_engine, &mut next_combat, None);
        steps += 1;
        if !alive {
            return (next_engine, next_combat, steps as u32, false);
        }
        normalize_bounded_root_processing(&mut next_engine, &next_combat);
    }
    (next_engine, next_combat, steps as u32, false)
}

fn normalize_bounded_root_processing(engine: &mut EngineState, combat: &CombatState) {
    if *engine == EngineState::CombatPlayerTurn
        && (combat.has_pending_actions() || !combat.zones.queued_cards.is_empty())
    {
        *engine = EngineState::CombatProcessing;
    }
}

fn bounded_root_frontier_is_stable(engine: &EngineState, combat: &CombatState) -> bool {
    match engine {
        EngineState::CombatPlayerTurn
        | EngineState::PendingChoice(_)
        | EngineState::GameOver(_) => true,
        EngineState::CombatProcessing => {
            crate::engine::core::is_smoke_escape_stable_boundary(engine, combat)
        }
        EngineState::RewardScreen(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::EventCombat(_)
        | EngineState::BossRelicSelect(_) => true,
    }
}

fn remaining_hand_upper_bounds(combat: &CombatState) -> (i32, i32, i32, Vec<String>) {
    let mut damage = 0;
    let mut block = 0;
    let mut setup = 0;
    let mut notes = Vec::new();
    for card in &combat.zones.hand {
        let def = crate::content::cards::get_card_definition(card.id);
        let upgraded_damage = def.base_damage + def.upgrade_damage * card.upgrades as i32;
        let upgraded_block = def.base_block + def.upgrade_block * card.upgrades as i32;
        let target_count = if def.is_multi_damage {
            living_monster_count_for_pack(combat).max(1) as i32
        } else {
            1
        };
        damage += upgraded_damage.max(0) * target_count;
        block += upgraded_block.max(0);
        if matches!(def.card_type, CardType::Power) || card_is_setup_like(card.id) {
            setup = 1;
        }
        if card_draws_cards(card.id) || card_generates_cards(card.id) {
            notes.push(format!("unexpanded_access_card:{:?}", card.id));
        }
    }
    (damage, block, setup, notes)
}

fn root_played_setup(input: &ClientInput, combat: &CombatState) -> bool {
    match input {
        ClientInput::PlayCard { card_index, .. } => {
            combat.zones.hand.get(*card_index).is_some_and(|card| {
                let def = crate::content::cards::get_card_definition(card.id);
                matches!(def.card_type, CardType::Power) || card_is_setup_like(card.id)
            })
        }
        _ => false,
    }
}

fn hand_has_draw_or_generation(combat: &CombatState) -> bool {
    combat
        .zones
        .hand
        .iter()
        .any(|card| card_draws_cards(card.id) || card_generates_cards(card.id))
}

fn card_is_setup_like(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Inflame
            | CardId::DemonForm
            | CardId::LimitBreak
            | CardId::Metallicize
            | CardId::Barricade
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Corruption
            | CardId::Armaments
            | CardId::Entrench
    )
}

fn card_generates_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::InfernalBlade | CardId::Discovery | CardId::ThinkingAhead | CardId::Transmutation
    )
}

fn build_bounded_pairwise_labels(
    candidates: &[CombatRootCandidateOutcome],
) -> Vec<CombatCandidatePairwiseLabel> {
    let mut labels = Vec::new();
    for left_idx in 0..candidates.len() {
        for right_idx in (left_idx + 1)..candidates.len() {
            let left = &candidates[left_idx];
            let right = &candidates[right_idx];
            for objective in &left.bounded_objectives.objective_bounds {
                let Some(right_objective) = right
                    .bounded_objectives
                    .objective_bounds
                    .iter()
                    .find(|candidate| candidate.objective == objective.objective)
                else {
                    continue;
                };
                if let Some((preferred, rejected, gap)) =
                    separated_interval_preference(left, objective, right, right_objective)
                {
                    labels.push(CombatCandidatePairwiseLabel {
                        objective: objective.objective.clone(),
                        preferred_candidate_index: preferred.candidate_index,
                        rejected_candidate_index: rejected.candidate_index,
                        preferred_action_key: preferred.candidate.action_key.clone(),
                        rejected_action_key: rejected.candidate.action_key.clone(),
                        confidence: "interval_separated".to_string(),
                        reason: format!(
                            "{} interval separates: preferred lower/upper {}..{}, rejected {}..{}",
                            objective.objective,
                            preferred_bound(preferred, &objective.objective).0,
                            preferred_bound(preferred, &objective.objective).1,
                            preferred_bound(rejected, &objective.objective).0,
                            preferred_bound(rejected, &objective.objective).1
                        ),
                        interval_gap: gap,
                        label_source: "bounded_objective_interval_separation_v0".to_string(),
                    });
                }
            }
        }
    }
    labels
}

fn separated_interval_preference<'a>(
    left: &'a CombatRootCandidateOutcome,
    left_bound: &CombatCandidateObjectiveBound,
    right: &'a CombatRootCandidateOutcome,
    right_bound: &CombatCandidateObjectiveBound,
) -> Option<(
    &'a CombatRootCandidateOutcome,
    &'a CombatRootCandidateOutcome,
    i32,
)> {
    if left_bound.higher_is_better {
        if left_bound.lower_bound > right_bound.upper_bound {
            return Some((
                left,
                right,
                left_bound.lower_bound - right_bound.upper_bound,
            ));
        }
        if right_bound.lower_bound > left_bound.upper_bound {
            return Some((
                right,
                left,
                right_bound.lower_bound - left_bound.upper_bound,
            ));
        }
    } else {
        if left_bound.upper_bound < right_bound.lower_bound {
            return Some((
                left,
                right,
                right_bound.lower_bound - left_bound.upper_bound,
            ));
        }
        if right_bound.upper_bound < left_bound.lower_bound {
            return Some((
                right,
                left,
                left_bound.lower_bound - right_bound.upper_bound,
            ));
        }
    }
    None
}

fn preferred_bound(candidate: &CombatRootCandidateOutcome, objective: &str) -> (i32, i32) {
    candidate
        .bounded_objectives
        .objective_bounds
        .iter()
        .find(|bound| bound.objective == objective)
        .map(|bound| (bound.lower_bound, bound.upper_bound))
        .unwrap_or((0, 0))
}

fn candidate_oracle_quality(
    solution: &crate::bot::combat::exact_turn_solver::ExactTurnSolution,
) -> CombatCandidateOracleQuality {
    let mut ineligibility_reasons = Vec::new();
    if solution.truncated {
        ineligibility_reasons.push("exact_turn_truncated".to_string());
    }
    if solution.truncation.max_nodes_hit {
        ineligibility_reasons.push("max_nodes_hit".to_string());
    }
    if solution.truncation.engine_step_limit_hit {
        ineligibility_reasons.push("engine_step_limit_hit".to_string());
    }
    if solution.truncation.deadline_hit {
        ineligibility_reasons.push("deadline_hit".to_string());
    }
    if solution.truncation.cycle_cut {
        ineligibility_reasons.push("cycle_cut".to_string());
    }
    if solution.truncation.step_projection_truncated {
        ineligibility_reasons.push("step_projection_truncated".to_string());
    }
    if solution.nondominated_end_states.is_empty() {
        ineligibility_reasons.push("no_nondominated_end_states".to_string());
    }

    CombatCandidateOracleQuality {
        eligible_for_training: ineligibility_reasons.is_empty(),
        ineligibility_reasons,
    }
}

fn pack_oracle_quality(
    candidates: &[CombatRootCandidateOutcome],
    bounded_pairwise_label_count: usize,
    controlled_v0: bool,
    min_eligible_candidates: usize,
) -> CombatCandidateOutcomePackOracleQuality {
    let trainable_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.oracle_quality.eligible_for_training)
        .count();
    let ineligible_candidate_count = candidates.len().saturating_sub(trainable_candidate_count);
    let truncated_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.exact_turn.truncated)
        .count();
    let mut truncation_reasons = BTreeMap::new();
    for candidate in candidates {
        if candidate.exact_turn.truncation.max_nodes_hit {
            increment_reason(&mut truncation_reasons, "max_nodes_hit");
        }
        if candidate.exact_turn.truncation.engine_step_limit_hit {
            increment_reason(&mut truncation_reasons, "engine_step_limit_hit");
        }
        if candidate.exact_turn.truncation.deadline_hit {
            increment_reason(&mut truncation_reasons, "deadline_hit");
        }
        if candidate.exact_turn.truncation.cycle_cut {
            increment_reason(&mut truncation_reasons, "cycle_cut");
        }
        if candidate.exact_turn.truncation.step_projection_truncated {
            increment_reason(&mut truncation_reasons, "step_projection_truncated");
        }
    }
    let trainable_pair_count =
        trainable_candidate_count.saturating_mul(trainable_candidate_count.saturating_sub(1)) / 2;

    CombatCandidateOutcomePackOracleQuality {
        trainable_candidate_count,
        ineligible_candidate_count,
        trainable_pair_count,
        truncated_candidate_count,
        truncation_reasons,
        controlled_v0,
        trainable_manifest_eligible: trainable_candidate_count >= min_eligible_candidates,
        bounded_pairwise_label_count,
        bounded_pairwise_manifest_eligible: bounded_pairwise_label_count > 0,
    }
}

fn increment_reason(reasons: &mut BTreeMap<String, usize>, reason: &str) {
    *reasons.entry(reason.to_string()).or_default() += 1;
}

fn collect_trace_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Err(format!(
            "candidate outcome batch input '{}' is not a file or directory",
            path.display()
        ));
    }
    for entry in std::fs::read_dir(path)
        .map_err(|err| format!("failed to read trace input dir '{}': {err}", path.display()))?
    {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read entry under trace input dir '{}': {err}",
                path.display()
            )
        })?;
        collect_trace_files(&entry.path(), out)?;
    }
    Ok(())
}

fn controlled_v0_trace_steps(
    trace_file: &Path,
    step_start: usize,
    step_end: Option<usize>,
) -> Result<Vec<(PathBuf, usize)>, String> {
    let raw = std::fs::read_to_string(trace_file).map_err(|err| {
        format!(
            "failed to read trace file '{}' for candidate outcome batch: {err}",
            trace_file.display()
        )
    })?;
    let trace: serde_json::Value = serde_json::from_str(&raw).map_err(|err| {
        format!(
            "failed to parse trace JSON '{}' for candidate outcome batch: {err}",
            trace_file.display()
        )
    })?;
    let steps = trace
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("trace '{}' missing steps[]", trace_file.display()))?;
    let end = step_end.unwrap_or(steps.len()).min(steps.len());
    let mut targets = Vec::new();
    for step_index in step_start.min(end)..end {
        let step = &steps[step_index];
        let decision_type = step
            .get("decision_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let engine_state = step
            .get("engine_state")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if decision_type == "combat" && engine_state == "combat_player_turn" {
            targets.push((trace_file.to_path_buf(), step_index));
        }
    }
    Ok(targets)
}

fn summarize_candidate_outcome_budget(
    budget: usize,
    packs: &[CombatCandidateOutcomePackReport],
    pack_manifest: Vec<String>,
    trainable_manifest: Vec<String>,
    _min_eligible_candidates: usize,
) -> CombatCandidateOutcomeBudgetSummary {
    let pack_count = packs.len();
    let mut candidate_count = 0usize;
    let mut eligible_candidate_count = 0usize;
    let mut truncated_candidate_count = 0usize;
    let mut trainable_pair_count = 0usize;
    let mut elapsed_samples = Vec::new();
    let mut truncation_reasons = BTreeMap::new();

    for pack in packs {
        trainable_pair_count = trainable_pair_count.saturating_add(pack.pairwise_labels.len());
        for (reason, count) in &pack.pack_oracle_quality.truncation_reasons {
            *truncation_reasons.entry(reason.clone()).or_default() += *count;
        }
        for candidate in &pack.candidates {
            candidate_count = candidate_count.saturating_add(1);
            if candidate.oracle_quality.eligible_for_training {
                eligible_candidate_count = eligible_candidate_count.saturating_add(1);
            }
            if candidate.exact_turn.truncated {
                truncated_candidate_count = truncated_candidate_count.saturating_add(1);
            }
            elapsed_samples.push(candidate.exact_turn.elapsed_ms);
        }
    }
    elapsed_samples.sort_unstable();
    let median_candidate_elapsed_ms = if elapsed_samples.is_empty() {
        0
    } else {
        elapsed_samples[elapsed_samples.len().saturating_sub(1) / 2]
    };
    let eligible_candidate_ratio = if candidate_count == 0 {
        0.0
    } else {
        eligible_candidate_count as f32 / candidate_count as f32
    };

    CombatCandidateOutcomeBudgetSummary {
        budget,
        pack_count,
        trainable_pack_count: trainable_manifest.len(),
        candidate_count,
        eligible_candidate_count,
        truncated_candidate_count,
        eligible_candidate_ratio,
        trainable_pair_count,
        median_candidate_elapsed_ms,
        truncation_reasons,
        pack_manifest,
        trainable_manifest,
    }
}

fn pack_file_name(trace_file: &Path, step_index: usize) -> String {
    format!(
        "{}_{}_step_{}.json",
        sanitize_file_component(
            trace_file
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("trace")
        ),
        stable_path_hash(trace_file),
        step_index
    )
}

fn sanitize_file_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn stable_path_hash(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.display().to_string().hash(&mut hasher);
    hasher.finish()
}

fn write_pretty_json<T: serde::Serialize>(path: &Path, payload: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create candidate outcome output parent '{}': {err}",
                parent.display()
            )
        })?;
    }
    std::fs::write(
        path,
        serde_json::to_string_pretty(payload)
            .map_err(|err| format!("failed to serialize candidate outcome JSON: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write candidate outcome JSON '{}': {err}",
            path.display()
        )
    })
}

fn aggregate_candidate_outcomes(
    start: &CombatState,
    end_states: &[crate::bot::combat::exact_turn_solver::TurnEndState],
    unique_outcomes: Vec<CombatCandidateOutcomeVector>,
) -> CombatCandidateOutcomeAggregate {
    let start_total_hp = total_living_monster_hp_for_pack(start);
    let mut min_projected_unblocked_damage = i32::MAX;
    let mut max_projected_unblocked_damage = i32::MIN;
    let mut min_total_monster_hp = i32::MAX;
    let mut max_total_monster_hp = i32::MIN;
    let mut max_enemy_hp_reduction = i32::MIN;
    let mut min_hp_lost = i32::MAX;
    let mut max_hp_lost = i32::MIN;
    let mut max_final_hp = i32::MIN;
    let mut min_final_hp = i32::MAX;
    let mut max_final_block = i32::MIN;
    let mut min_spent_potions = u8::MAX;
    let mut min_exhausted_cards = u16::MAX;
    let mut any_combat_cleared = false;
    let mut any_player_dead = false;

    for state in end_states {
        let outcome =
            outcome_vector_from_combat(start, &state.frontier_engine, &state.frontier_combat);
        min_projected_unblocked_damage =
            min_projected_unblocked_damage.min(outcome.projected_unblocked_damage);
        max_projected_unblocked_damage =
            max_projected_unblocked_damage.max(outcome.projected_unblocked_damage);
        min_total_monster_hp = min_total_monster_hp.min(outcome.total_monster_hp);
        max_total_monster_hp = max_total_monster_hp.max(outcome.total_monster_hp);
        max_enemy_hp_reduction =
            max_enemy_hp_reduction.max((start_total_hp - outcome.total_monster_hp).max(0));
        min_hp_lost = min_hp_lost.min(state.resources.hp_lost);
        max_hp_lost = max_hp_lost.max(state.resources.hp_lost);
        max_final_hp = max_final_hp.max(state.resources.final_hp);
        min_final_hp = min_final_hp.min(state.resources.final_hp);
        max_final_block = max_final_block.max(state.resources.final_block);
        min_spent_potions = min_spent_potions.min(state.resources.spent_potions);
        min_exhausted_cards = min_exhausted_cards.min(state.resources.exhausted_cards);
        any_combat_cleared |= outcome.combat_cleared;
        any_player_dead |= outcome.player_dead;
    }

    if end_states.is_empty() {
        let outcome = outcome_vector_from_combat(start, &EngineState::CombatPlayerTurn, start);
        min_projected_unblocked_damage = outcome.projected_unblocked_damage;
        max_projected_unblocked_damage = outcome.projected_unblocked_damage;
        min_total_monster_hp = outcome.total_monster_hp;
        max_total_monster_hp = outcome.total_monster_hp;
        max_enemy_hp_reduction = 0;
        min_hp_lost = 0;
        max_hp_lost = 0;
        max_final_hp = outcome.player_hp;
        min_final_hp = outcome.player_hp;
        max_final_block = outcome.player_block;
        min_spent_potions = 0;
        min_exhausted_cards = 0;
        any_combat_cleared = outcome.combat_cleared;
        any_player_dead = outcome.player_dead;
    }

    CombatCandidateOutcomeAggregate {
        nondominated_count: end_states.len(),
        unique_outcome_count: unique_outcomes.len(),
        any_combat_cleared,
        any_player_dead,
        any_no_hp_loss: min_hp_lost == 0,
        min_projected_unblocked_damage,
        max_projected_unblocked_damage,
        min_total_monster_hp,
        max_total_monster_hp,
        max_enemy_hp_reduction,
        min_hp_lost,
        max_hp_lost,
        max_final_hp,
        min_final_hp,
        max_final_block,
        min_spent_potions,
        min_exhausted_cards,
        representative_outcome: unique_outcomes.first().cloned(),
        unique_outcomes,
    }
}

fn unique_outcome_vectors(
    outcomes: Vec<CombatCandidateOutcomeVector>,
) -> Vec<CombatCandidateOutcomeVector> {
    let mut unique = Vec::new();
    for outcome in outcomes {
        if !unique.contains(&outcome) {
            unique.push(outcome);
        }
    }
    unique
}

fn outcome_vector_from_combat(
    start: &CombatState,
    engine: &EngineState,
    combat: &CombatState,
) -> CombatCandidateOutcomeVector {
    let total_monster_hp = total_living_monster_hp_for_pack(combat);
    let living_monster_count = living_monster_count_for_pack(combat);
    let start_living = living_monster_count_for_pack(start);
    let visible_incoming_damage = visible_incoming_damage_for_pack(combat);
    CombatCandidateOutcomeVector {
        engine_state: engine_state_label(engine).to_string(),
        terminal_kind: terminal_kind_label(engine, combat).to_string(),
        combat_cleared: combat_cleared_for_pack(combat),
        player_dead: player_dead_for_pack(engine, combat),
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy as i32,
        visible_incoming_damage,
        projected_unblocked_damage: (visible_incoming_damage - combat.entities.player.block).max(0),
        total_monster_hp,
        living_monster_count,
        monster_hp_reduction_from_start: (total_living_monster_hp_for_pack(start)
            - total_monster_hp)
            .max(0),
        monster_deaths_from_start: start_living.saturating_sub(living_monster_count),
        hp_lost_from_start: (start.entities.player.current_hp - combat.entities.player.current_hp)
            .max(0),
        hand_count: combat.zones.hand.len(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        player_powers: power_snapshots_for_owner(combat, 0, "player".to_string()),
        monsters: monster_snapshots_for_pack(combat),
    }
}

fn monster_snapshots_for_pack(combat: &CombatState) -> Vec<CombatMonsterSnapshot> {
    combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            let visible_incoming_damage =
                crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster);
            CombatMonsterSnapshot {
                slot: monster.slot,
                entity_id: monster.id,
                monster_id: format!("{:?}", monster.monster_type),
                hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                alive: !monster.is_dying
                    && !monster.is_escaped
                    && !monster.half_dead
                    && monster.current_hp > 0,
                dying: monster.is_dying,
                escaped: monster.is_escaped,
                half_dead: monster.half_dead,
                planned_move_id: monster.planned_move_id(),
                visible_incoming_damage,
                powers: power_snapshots_for_owner(
                    combat,
                    monster.id,
                    format!("monster_slot:{}", monster.slot),
                ),
            }
        })
        .collect()
}

fn power_snapshots_for_owner(
    combat: &CombatState,
    owner: crate::core::EntityId,
    owner_label: String,
) -> Vec<CombatPowerSnapshot> {
    combat
        .entities
        .power_db
        .get(&owner)
        .map(|powers| {
            powers
                .iter()
                .map(|power| CombatPowerSnapshot {
                    owner: owner_label.clone(),
                    power_id: format!("{:?}", power.power_type),
                    amount: power.amount,
                    extra_data: power.extra_data,
                    just_applied: power.just_applied,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn visible_incoming_damage_for_pack(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .map(|monster| {
            crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster)
        })
        .sum()
}

fn total_living_monster_hp_for_pack(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn living_monster_count_for_pack(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .count()
}

fn combat_cleared_for_pack(combat: &CombatState) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
}

fn player_dead_for_pack(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(engine, EngineState::GameOver(RunResult::Defeat))
        || combat.entities.player.current_hp <= 0
}

fn terminal_kind_label(engine: &EngineState, combat: &CombatState) -> &'static str {
    if player_dead_for_pack(engine, combat) {
        "defeat"
    } else if matches!(engine, EngineState::GameOver(RunResult::Victory)) {
        "victory"
    } else if combat_cleared_for_pack(combat) {
        "combat_cleared"
    } else {
        "ongoing"
    }
}

type ReplayedTraceFrontier = (
    serde_json::Value,
    u64,
    u8,
    bool,
    String,
    serde_json::Value,
    EpisodeContext,
);

fn replay_trace_to_frontier(
    trace_file: &Path,
    step_index: usize,
    ascension_override: Option<u8>,
    final_act_override: Option<bool>,
    player_class_override: Option<String>,
    max_steps_override: Option<usize>,
) -> Result<ReplayedTraceFrontier, String> {
    let raw = std::fs::read_to_string(trace_file).map_err(|err| {
        format!(
            "failed to read trace file '{}': {err}",
            trace_file.display()
        )
    })?;
    let trace: serde_json::Value =
        serde_json::from_str(&raw).map_err(|err| format!("failed to parse trace JSON: {err}"))?;
    let summary = trace
        .get("summary")
        .ok_or_else(|| "trace missing summary".to_string())?;
    let seed = summary
        .get("seed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "trace summary missing seed".to_string())?;
    let trace_config = trace.get("config");
    let ascension = ascension_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("ascension"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u8
    });
    let final_act = final_act_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("final_act"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });
    let player_class = player_class_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("player_class"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Ironclad")
            .to_string()
    });
    let max_steps = max_steps_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("max_steps"))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| step_index.saturating_add(128).max(512))
    });
    let steps = trace
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "trace missing steps[]".to_string())?;
    if step_index >= steps.len() {
        return Err(format!(
            "step-index {} out of range for trace with {} step(s)",
            step_index,
            steps.len()
        ));
    }

    let mut ctx = EpisodeContext {
        engine_state: EngineState::EventRoom,
        run_state: RunState::new(
            seed,
            ascension,
            final_act,
            normalize_player_class(&player_class),
        ),
        combat_state: None,
        stashed_event_combat: None,
        forced_engine_ticks: 0,
        combat_win_count: 0,
    };

    for (step_idx, step) in steps.iter().take(step_index).enumerate() {
        prepare_decision_point(&mut ctx, max_steps)?;
        let action = trace_step_action(step)
            .map_err(|err| format!("failed to decode action at trace step {step_idx}: {err}"))?;
        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            Some(action),
        );
        if let Some(errors) = take_engine_error_diagnostics(&mut ctx) {
            return Err(format!(
                "replay to step {} rejected trace action: {}",
                step_idx,
                errors.join("; ")
            ));
        }
        finish_combat_if_needed(&mut ctx);
        if !keep_running {
            return Err(format!(
                "engine stopped while replaying trace before requested step {}",
                step_index
            ));
        }
    }

    prepare_decision_point(&mut ctx, max_steps)?;
    let target_trace_step = steps[step_index].clone();
    Ok((
        trace,
        seed,
        ascension,
        final_act,
        player_class,
        target_trace_step,
        ctx,
    ))
}

fn replay_trace_to_combat_frontier(
    trace_file: &Path,
    step_index: usize,
    ascension_override: Option<u8>,
    final_act_override: Option<bool>,
    player_class_override: Option<String>,
    max_steps_override: Option<usize>,
) -> Result<ReplayedTraceFrontier, String> {
    let raw = std::fs::read_to_string(trace_file).map_err(|err| {
        format!(
            "failed to read trace file '{}': {err}",
            trace_file.display()
        )
    })?;
    let trace: serde_json::Value =
        serde_json::from_str(&raw).map_err(|err| format!("failed to parse trace JSON: {err}"))?;
    let summary = trace
        .get("summary")
        .ok_or_else(|| "trace missing summary".to_string())?;
    let seed = summary
        .get("seed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "trace summary missing seed".to_string())?;
    let trace_config = trace.get("config");
    let ascension = ascension_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("ascension"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u8
    });
    let final_act = final_act_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("final_act"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });
    let player_class = player_class_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("player_class"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Ironclad")
            .to_string()
    });
    let max_steps = max_steps_override.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("max_steps"))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| step_index.saturating_add(128).max(512))
    });
    let steps = trace
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "trace missing steps[]".to_string())?;
    if step_index >= steps.len() {
        return Err(format!(
            "step-index {} out of range for trace with {} step(s)",
            step_index,
            steps.len()
        ));
    }

    let mut ctx = EpisodeContext {
        engine_state: EngineState::EventRoom,
        run_state: RunState::new(
            seed,
            ascension,
            final_act,
            normalize_player_class(&player_class),
        ),
        combat_state: None,
        stashed_event_combat: None,
        forced_engine_ticks: 0,
        combat_win_count: 0,
    };

    for (step_idx, step) in steps.iter().take(step_index).enumerate() {
        prepare_decision_point(&mut ctx, max_steps)?;
        let action = trace_step_action(step)
            .map_err(|err| format!("failed to decode action at trace step {step_idx}: {err}"))?;
        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            Some(action),
        );
        if let Some(errors) = take_engine_error_diagnostics(&mut ctx) {
            return Err(format!(
                "replay to step {} rejected trace action: {}",
                step_idx,
                errors.join("; ")
            ));
        }
        finish_combat_if_needed(&mut ctx);
        if !keep_running {
            return Err(format!(
                "engine stopped while replaying trace before requested step {}",
                step_index
            ));
        }
    }

    prepare_decision_point(&mut ctx, max_steps)?;
    if !matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return Err(format!(
            "trace step {} is not a combat turn frontier: {}",
            step_index,
            engine_state_label(&ctx.engine_state)
        ));
    }

    let target_trace_step = steps[step_index].clone();
    Ok((
        trace,
        seed,
        ascension,
        final_act,
        player_class,
        target_trace_step,
        ctx,
    ))
}

fn trace_probe_source(
    trace_file: &Path,
    step_index: usize,
    seed: u64,
    ascension: u8,
    final_act: bool,
    player_class: &str,
    trace: &serde_json::Value,
    target_trace_step: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "trace_file": trace_file.display().to_string(),
        "step_index": step_index,
        "seed": seed,
        "ascension": ascension,
        "final_act": final_act,
        "player_class": player_class,
        "trace_observation_schema_version": trace.get("observation_schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "trace_action_schema_version": trace.get("action_schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_decision_type": target_trace_step.get("decision_type").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_engine_state": target_trace_step.get("engine_state").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_chosen_action_key": target_trace_step.get("chosen_action_key").cloned().unwrap_or(serde_json::Value::Null),
    })
}

pub fn trace_step_action(step: &serde_json::Value) -> Result<ClientInput, String> {
    let value = step
        .get("chosen_action")
        .ok_or_else(|| "missing chosen_action".to_string())?
        .clone();
    let trace_input: TraceClientInput = serde_json::from_value(value)
        .map_err(|err| format!("chosen_action shape mismatch: {err}"))?;
    Ok(client_input_from_trace_input(trace_input))
}

pub fn client_input_from_trace_input(input: TraceClientInput) -> ClientInput {
    match input {
        TraceClientInput::PlayCard { card_index, target } => {
            ClientInput::PlayCard { card_index, target }
        }
        TraceClientInput::UsePotion {
            potion_index,
            target,
        } => ClientInput::UsePotion {
            potion_index,
            target,
        },
        TraceClientInput::DiscardPotion { potion_index } => {
            ClientInput::DiscardPotion(potion_index)
        }
        TraceClientInput::EndTurn => ClientInput::EndTurn,
        TraceClientInput::SubmitCardChoice { indices } => ClientInput::SubmitCardChoice(indices),
        TraceClientInput::SubmitDiscoverChoice { index } => {
            ClientInput::SubmitDiscoverChoice(index)
        }
        TraceClientInput::SelectMapNode { x } => ClientInput::SelectMapNode(x),
        TraceClientInput::FlyToNode { x, y } => ClientInput::FlyToNode(x, y),
        TraceClientInput::SelectEventOption { index } => ClientInput::SelectEventOption(index),
        TraceClientInput::CampfireOption { choice } => {
            ClientInput::CampfireOption(campfire_choice_from_trace(choice))
        }
        TraceClientInput::EventChoice { index } => ClientInput::EventChoice(index),
        TraceClientInput::SubmitScryDiscard { indices } => ClientInput::SubmitScryDiscard(indices),
        TraceClientInput::SubmitSelection {
            scope,
            selected_card_uuids,
        } => ClientInput::SubmitSelection(SelectionResolution {
            scope: selection_scope_from_trace(scope),
            selected: selected_card_uuids
                .into_iter()
                .map(SelectionTargetRef::CardUuid)
                .collect(),
        }),
        TraceClientInput::SubmitHandSelect { card_uuids } => {
            ClientInput::SubmitHandSelect(card_uuids)
        }
        TraceClientInput::SubmitGridSelect { card_uuids } => {
            ClientInput::SubmitGridSelect(card_uuids)
        }
        TraceClientInput::SubmitDeckSelect { indices } => ClientInput::SubmitDeckSelect(indices),
        TraceClientInput::ClaimReward { index } => ClientInput::ClaimReward(index),
        TraceClientInput::SelectCard { index } => ClientInput::SelectCard(index),
        TraceClientInput::BuyCard { index } => ClientInput::BuyCard(index),
        TraceClientInput::BuyRelic { index } => ClientInput::BuyRelic(index),
        TraceClientInput::BuyPotion { index } => ClientInput::BuyPotion(index),
        TraceClientInput::PurgeCard { index } => ClientInput::PurgeCard(index),
        TraceClientInput::SubmitRelicChoice { index } => ClientInput::SubmitRelicChoice(index),
        TraceClientInput::Proceed => ClientInput::Proceed,
        TraceClientInput::Cancel => ClientInput::Cancel,
    }
}

pub fn campfire_choice_from_trace(choice: TraceCampfireChoice) -> CampfireChoice {
    match choice {
        TraceCampfireChoice::Rest => CampfireChoice::Rest,
        TraceCampfireChoice::Smith { deck_index } => CampfireChoice::Smith(deck_index),
        TraceCampfireChoice::Dig => CampfireChoice::Dig,
        TraceCampfireChoice::Lift => CampfireChoice::Lift,
        TraceCampfireChoice::Toke { deck_index } => CampfireChoice::Toke(deck_index),
        TraceCampfireChoice::Recall => CampfireChoice::Recall,
    }
}

pub fn selection_scope_from_trace(scope: TraceSelectionScope) -> SelectionScope {
    match scope {
        TraceSelectionScope::Hand => SelectionScope::Hand,
        TraceSelectionScope::Deck => SelectionScope::Deck,
        TraceSelectionScope::Grid => SelectionScope::Grid,
    }
}

pub fn trace_input_from_client_input(input: &ClientInput) -> TraceClientInput {
    match input {
        ClientInput::PlayCard { card_index, target } => TraceClientInput::PlayCard {
            card_index: *card_index,
            target: *target,
        },
        ClientInput::UsePotion {
            potion_index,
            target,
        } => TraceClientInput::UsePotion {
            potion_index: *potion_index,
            target: *target,
        },
        ClientInput::DiscardPotion(index) => TraceClientInput::DiscardPotion {
            potion_index: *index,
        },
        ClientInput::EndTurn => TraceClientInput::EndTurn,
        ClientInput::SubmitCardChoice(indices) => TraceClientInput::SubmitCardChoice {
            indices: indices.clone(),
        },
        ClientInput::SubmitDiscoverChoice(index) => {
            TraceClientInput::SubmitDiscoverChoice { index: *index }
        }
        ClientInput::SelectMapNode(x) => TraceClientInput::SelectMapNode { x: *x },
        ClientInput::FlyToNode(x, y) => TraceClientInput::FlyToNode { x: *x, y: *y },
        ClientInput::SelectEventOption(index) => {
            TraceClientInput::SelectEventOption { index: *index }
        }
        ClientInput::CampfireOption(choice) => TraceClientInput::CampfireOption {
            choice: trace_campfire_choice(*choice),
        },
        ClientInput::EventChoice(index) => TraceClientInput::EventChoice { index: *index },
        ClientInput::SubmitScryDiscard(indices) => TraceClientInput::SubmitScryDiscard {
            indices: indices.clone(),
        },
        ClientInput::SubmitSelection(selection) => TraceClientInput::SubmitSelection {
            scope: trace_selection_scope(selection.scope),
            selected_card_uuids: selection
                .selected
                .iter()
                .map(|target| match target {
                    SelectionTargetRef::CardUuid(uuid) => *uuid,
                })
                .collect(),
        },
        ClientInput::SubmitHandSelect(card_uuids) => TraceClientInput::SubmitHandSelect {
            card_uuids: card_uuids.clone(),
        },
        ClientInput::SubmitGridSelect(card_uuids) => TraceClientInput::SubmitGridSelect {
            card_uuids: card_uuids.clone(),
        },
        ClientInput::SubmitDeckSelect(indices) => TraceClientInput::SubmitDeckSelect {
            indices: indices.clone(),
        },
        ClientInput::ClaimReward(index) => TraceClientInput::ClaimReward { index: *index },
        ClientInput::SelectCard(index) => TraceClientInput::SelectCard { index: *index },
        ClientInput::BuyCard(index) => TraceClientInput::BuyCard { index: *index },
        ClientInput::BuyRelic(index) => TraceClientInput::BuyRelic { index: *index },
        ClientInput::BuyPotion(index) => TraceClientInput::BuyPotion { index: *index },
        ClientInput::PurgeCard(index) => TraceClientInput::PurgeCard { index: *index },
        ClientInput::SubmitRelicChoice(index) => {
            TraceClientInput::SubmitRelicChoice { index: *index }
        }
        ClientInput::Proceed => TraceClientInput::Proceed,
        ClientInput::Cancel => TraceClientInput::Cancel,
    }
}

pub fn trace_campfire_choice(choice: CampfireChoice) -> TraceCampfireChoice {
    match choice {
        CampfireChoice::Rest => TraceCampfireChoice::Rest,
        CampfireChoice::Smith(deck_index) => TraceCampfireChoice::Smith { deck_index },
        CampfireChoice::Dig => TraceCampfireChoice::Dig,
        CampfireChoice::Lift => TraceCampfireChoice::Lift,
        CampfireChoice::Toke(deck_index) => TraceCampfireChoice::Toke { deck_index },
        CampfireChoice::Recall => TraceCampfireChoice::Recall,
    }
}

pub fn trace_selection_scope(scope: SelectionScope) -> TraceSelectionScope {
    match scope {
        SelectionScope::Hand => TraceSelectionScope::Hand,
        SelectionScope::Deck => TraceSelectionScope::Deck,
        SelectionScope::Grid => TraceSelectionScope::Grid,
    }
}

pub fn engine_state_label(engine_state: &EngineState) -> &'static str {
    match engine_state {
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map_navigation",
        EngineState::EventRoom => "event_room",
        EngineState::PendingChoice(_) => "pending_choice",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::EventCombat(_) => "event_combat",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}

pub fn decision_type(engine_state: &EngineState) -> &'static str {
    match engine_state {
        EngineState::CombatPlayerTurn => "combat",
        EngineState::PendingChoice(PendingChoice::HandSelect { .. }) => "combat_hand_select",
        EngineState::PendingChoice(PendingChoice::GridSelect { .. }) => "combat_grid_select",
        EngineState::PendingChoice(PendingChoice::DiscoverySelect(_)) => "combat_discovery",
        EngineState::PendingChoice(PendingChoice::ScrySelect { .. }) => "combat_scry",
        EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. }) => "combat_card_reward",
        EngineState::PendingChoice(PendingChoice::StanceChoice) => "combat_stance",
        EngineState::RewardScreen(reward_state) if reward_state.pending_card_choice.is_some() => {
            "reward_card_choice"
        }
        EngineState::RewardScreen(_) => "reward",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map",
        EngineState::EventRoom => "event",
        EngineState::RunPendingChoice(_) => "run_deck_selection",
        EngineState::BossRelicSelect(_) => "boss_relic",
        EngineState::CombatProcessing | EngineState::EventCombat(_) | EngineState::GameOver(_) => {
            "none"
        }
    }
}

pub fn deterministic_replay_error(
    primary: &RunEpisodeSummary,
    replay: &RunEpisodeSummary,
) -> Option<String> {
    let mismatches = [
        ("result", primary.result.clone(), replay.result.clone()),
        (
            "terminal_reason",
            primary.terminal_reason.clone(),
            replay.terminal_reason.clone(),
        ),
        ("floor", primary.floor.to_string(), replay.floor.to_string()),
        ("act", primary.act.to_string(), replay.act.to_string()),
        ("steps", primary.steps.to_string(), replay.steps.to_string()),
        ("hp", primary.hp.to_string(), replay.hp.to_string()),
        (
            "deck_size",
            primary.deck_size.to_string(),
            replay.deck_size.to_string(),
        ),
    ]
    .into_iter()
    .filter_map(|(field, left, right)| {
        if left == right {
            None
        } else {
            Some(format!("{field}: primary={left} replay={right}"))
        }
    })
    .collect::<Vec<_>>();

    if replay.crash.is_some() && primary.crash != replay.crash {
        return Some(format!(
            "replay crashed differently: primary={:?} replay={:?}",
            primary.crash, replay.crash
        ));
    }

    if mismatches.is_empty() {
        None
    } else {
        Some(mismatches.join("; "))
    }
}

pub fn write_trace_file(
    path: &Path,
    config: &RunBatchConfig,
    summary: &RunEpisodeSummary,
    steps: &[RunStepTrace],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create trace parent '{}': {err}",
                parent.display()
            )
        })?;
    }
    let trace = RunEpisodeTraceFile {
        observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
        config: RunTraceConfigV0 {
            seed: summary.seed,
            ascension: config.ascension,
            final_act: config.final_act,
            player_class: config.player_class.to_string(),
            max_steps: config.max_steps,
            policy: config.policy.as_str().to_string(),
            reward_shaping_profile: config.reward_shaping_profile.as_str().to_string(),
        },
        summary: summary.clone(),
        steps: steps.to_vec(),
    };
    std::fs::write(
        path,
        serde_json::to_string_pretty(&trace)
            .map_err(|err| format!("failed to serialize trace: {err}"))?,
    )
    .map_err(|err| format!("failed to write trace '{}': {err}", path.display()))
}

pub fn median_i32(values: &[i32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) as f32 / 2.0
    } else {
        values[mid] as f32
    }
}

pub fn count_by<I>(values: I) -> std::collections::BTreeMap<String, usize>
where
    I: IntoIterator<Item = String>,
{
    let mut counts = std::collections::BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
}
