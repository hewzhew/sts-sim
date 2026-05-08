// Mechanical split from main.rs. Keep this file behavior-only; protocol types remain in main.rs.

fn run_verified_adv_override_batch(
    episodes: usize,
    seed_start: u64,
    seed_step: u64,
    base_config: FullRunEnvConfig,
    options: VerifiedAdvOverrideOptions,
    summary_only: bool,
) -> Result<VerifiedAdvOverrideBatchPayload, String> {
    let mut rows = Vec::with_capacity(episodes);
    for episode_index in 0..episodes {
        let seed = seed_start + episode_index as u64 * seed_step;
        let config = FullRunEnvConfig {
            seed,
            ..base_config.clone()
        };
        rows.push(run_verified_adv_override_episode(
            seed,
            config,
            options.clone(),
        )?);
    }
    let policy_name = format!(
        "verified_adv_override_agent_v0_H{}_{}_{}",
        options.horizon_decisions,
        options.horizon_mode.as_str(),
        options.strategy.as_str()
    );
    let summary = summarize_verified_episodes(&rows);
    let mut policy_summary = BTreeMap::new();
    policy_summary.insert(policy_name, summary);
    Ok(VerifiedAdvOverrideBatchPayload {
        schema_version: "verified_adv_override_rust_batch_v0".to_string(),
        config: VerifiedAdvOverrideRunConfigPayload {
            episodes,
            seed_start,
            seed_step,
            ascension: base_config.ascension,
            final_act: base_config.final_act,
            class: base_config.player_class.to_string(),
            max_steps: base_config.max_steps,
            reward_shaping_profile: base_config.reward_shaping_profile.as_str().to_string(),
            candidate_scope: options.candidate_scope.as_str().to_string(),
            continuation_policy: options.continuation_policy.as_str().to_string(),
            horizon_decisions: options.horizon_decisions,
            horizon_mode: options.horizon_mode.as_str().to_string(),
            oracle_margin: options.oracle_margin,
            verifier_strategy: options.strategy.as_str().to_string(),
            prefilter_horizon_decisions: (options.strategy
                == VerifiedStrategy::TwoStagePrefilterV1)
                .then_some(options.prefilter_horizon_decisions),
            prefilter_horizon_mode: (options.strategy == VerifiedStrategy::TwoStagePrefilterV1)
                .then_some(options.prefilter_horizon_mode.as_str().to_string()),
            prefilter_margin: (options.strategy == VerifiedStrategy::TwoStagePrefilterV1)
                .then_some(options.prefilter_margin),
            prefilter_top_k: (options.strategy == VerifiedStrategy::TwoStagePrefilterV1)
                .then_some(options.prefilter_top_k),
            proposer_model_path: options
                .proposer
                .as_ref()
                .map(|proposer| proposer.model_path.clone()),
            proposer_top_k: options.proposer.as_ref().map(|proposer| proposer.top_k),
            proposer_threshold: options.proposer.as_ref().map(|proposer| proposer.threshold),
            gamma: options.gamma,
            evidence_gate: options.evidence_gate.as_str().to_string(),
            low_evidence_margin: options.low_evidence_margin,
            confirm_low_evidence_horizon_decisions: options
                .confirm_low_evidence
                .map(|confirm| confirm.horizon_decisions),
            confirm_low_evidence_horizon_mode: options
                .confirm_low_evidence
                .map(|confirm| confirm.horizon_mode.as_str().to_string()),
            confirm_low_evidence_margin: options.confirm_low_evidence.map(|confirm| confirm.margin),
            evaluation_mode: options.runtime.mode.as_str().to_string(),
            value_cache_scope: options.runtime.cache_scope.as_str().to_string(),
            value_cache_max_entries: options.runtime.cache_max_entries,
            parallelism: options.runtime.parallelism,
            exact_root_dedup: options.runtime.exact_root_dedup,
        },
        policy_summary,
        episodes: if summary_only { Vec::new() } else { rows },
    })
}

fn run_verified_adv_override_episode(
    seed: u64,
    config: FullRunEnvConfig,
    options: VerifiedAdvOverrideOptions,
) -> Result<VerifiedAdvOverrideEpisodeSummary, String> {
    let mut env = FullRunEnv::new(config.clone())?;
    let mut done = env.info().result != "ongoing";
    let mut total_reward = 0.0f32;
    let mut steps = 0usize;
    let mut learned_decisions = 0usize;
    let mut last_info = env.info();
    let mut stats = VerifiedAdvOverrideStats::default();
    let mut episode_cache = ValueCache::default();
    let mut crash = None;

    while !done && steps < config.max_steps {
        let state = match env.state() {
            Ok(state) => state,
            Err(err) => {
                crash = Some(err);
                break;
            }
        };
        let decision_type = state.observation.decision_type.clone();
        let learned_combat_decision = is_combat_decision_type(&decision_type);
        let action_index = if learned_combat_decision {
            learned_decisions += 1;
            choose_verified_adv_override_action(
                &mut env,
                &state,
                &decision_type,
                steps,
                options.clone(),
                &mut episode_cache,
                &mut stats,
            )
        } else {
            preview_policy_index_from_env(&env, RunPolicyKind::RuleBaselineV0).unwrap_or(0)
        };

        match env.step(action_index) {
            Ok(step) => {
                total_reward += step.reward;
                last_info = step.info;
                done = step.done;
                steps += 1;
            }
            Err(err) => {
                crash = Some(err);
                break;
            }
        }
    }

    let result = if crash.is_some() {
        "crash".to_string()
    } else {
        last_info.result.clone()
    };
    let terminal_reason = if crash.is_some() {
        "script_error".to_string()
    } else {
        last_info.terminal_reason.clone()
    };
    Ok(VerifiedAdvOverrideEpisodeSummary {
        policy: format!(
            "verified_adv_override_agent_v0_H{}_{}_{}",
            options.horizon_decisions,
            options.horizon_mode.as_str(),
            options.strategy.as_str()
        ),
        seed,
        steps,
        done,
        crash,
        result,
        terminal_reason,
        final_floor: last_info.floor,
        final_act: last_info.act,
        final_hp: last_info.hp,
        final_max_hp: last_info.max_hp,
        final_deck_size: last_info.deck_size,
        final_relic_count: last_info.relic_count,
        combat_win_count: last_info.combat_win_count,
        total_reward,
        learned_decisions,
        stats: stats.as_payload(),
    })
}

fn choose_verified_adv_override_action(
    env: &mut FullRunEnv,
    state: &FullRunEnvState,
    decision_type: &str,
    step_index: usize,
    options: VerifiedAdvOverrideOptions,
    episode_cache: &mut ValueCache,
    stats: &mut VerifiedAdvOverrideStats,
) -> usize {
    match options.strategy {
        VerifiedStrategy::SingleStage => choose_verified_adv_override_action_single_stage(
            env,
            state,
            decision_type,
            step_index,
            options.clone(),
            episode_cache,
            stats,
        ),
        VerifiedStrategy::TwoStagePrefilterV1 => choose_verified_adv_override_action_two_stage(
            env,
            state,
            decision_type,
            step_index,
            options,
            episode_cache,
            stats,
        ),
        VerifiedStrategy::ModelProposerV1 => choose_verified_adv_override_action_model_proposer(
            env,
            state,
            decision_type,
            step_index,
            options,
            episode_cache,
            stats,
        ),
    }
}

fn choose_verified_adv_override_action_single_stage(
    env: &mut FullRunEnv,
    state: &FullRunEnvState,
    decision_type: &str,
    step_index: usize,
    options: VerifiedAdvOverrideOptions,
    episode_cache: &mut ValueCache,
    stats: &mut VerifiedAdvOverrideStats,
) -> usize {
    let scoped = scoped_legal_indices_for_state(state, options.candidate_scope);
    if scoped.is_empty() {
        stats.record_missing(decision_type, "no_scoped_candidates");
        return 0;
    }
    let context_keys = decision_context_keys(state);
    stats.record_decision(decision_type, scoped.len(), &context_keys);

    let rule_index = preview_policy_index_from_env(env, RunPolicyKind::RuleBaselineV0);
    let all_legal = legal_indices_for_state(state);
    let Some(rule_index) = rule_index.filter(|index| all_legal.contains(index)) else {
        stats.record_missing(decision_type, "missing_rule_action");
        return scoped[0];
    };
    if !scoped.iter().any(|index| *index != rule_index) {
        stats.record_reject();
        return rule_index;
    }

    let mut eval_indices = scoped.clone();
    if !eval_indices.contains(&rule_index) {
        eval_indices.push(rule_index);
    }
    eval_indices.sort_unstable();
    eval_indices.dedup();

    let payload = match evaluate_candidates(
        env,
        episode_cache,
        eval_indices,
        options.continuation_policy,
        options.horizon_decisions,
        options.horizon_mode,
        options.gamma,
        options.runtime,
        EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        },
    ) {
        Ok(payload) => payload,
        Err(err) => {
            stats.record_missing(decision_type, &format!("evaluation_error:{err}"));
            return rule_index;
        }
    };
    stats.record_final_payload(&payload);

    let rule_evaluation = payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == rule_index);
    let Some(rule_return) = rule_evaluation.map(|evaluation| evaluation.discounted_return) else {
        stats.record_missing(decision_type, "missing_rule_evaluation");
        return rule_index;
    };

    let mut best_index = scoped[0];
    let mut best_return = f32::NEG_INFINITY;
    for index in &scoped {
        if let Some(value) = payload
            .evaluations
            .iter()
            .find(|evaluation| evaluation.ok && evaluation.action_index == *index)
            .map(|evaluation| evaluation.discounted_return)
        {
            if value > best_return {
                best_return = value;
                best_index = *index;
            }
        }
    }
    if !best_return.is_finite() {
        stats.record_missing(decision_type, "missing_scoped_evaluations");
        return rule_index;
    }
    let adv = best_return - rule_return;
    stats.record_best_adv(adv, options.oracle_margin);
    let best_evaluation = payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == best_index);
    let mut selected_evidence = OverrideSelectionEvidence {
        rule_return,
        selected_return: best_return,
        adv_vs_rule: adv,
        horizon_decisions: options.horizon_decisions,
        horizon_mode: options.horizon_mode,
        horizon_stop_reason: best_evaluation
            .map(|evaluation| evaluation.horizon_stop_reason.clone()),
        payoff_reasons: best_evaluation
            .map(|evaluation| evaluation.payoff_reasons.clone())
            .unwrap_or_default(),
        confirmation_kind: None,
        artifact_reasons: Vec::new(),
        evaluated_candidate_count: count_successful_evaluations(&payload),
        policy_step_eval_count: payload.policy_step_eval_count,
    };
    let mut confirmation_kind = None;
    if best_index != rule_index && adv > options.oracle_margin {
        match maybe_confirm_suspect_override(
            env,
            episode_cache,
            state,
            rule_index,
            best_index,
            &options,
            rule_evaluation,
            best_evaluation,
            stats,
        ) {
            Ok(ConfirmationOutcome::Confirmed(confirm_evidence)) => {
                confirmation_kind = confirm_evidence.confirmation_kind.clone();
                selected_evidence = confirm_evidence;
            }
            Ok(ConfirmationOutcome::NotNeeded) => {}
            Ok(ConfirmationOutcome::Rejected) => return rule_index,
            Err(err) => {
                stats.record_missing(decision_type, &format!("confirm_evaluation_error:{err}"));
                stats.record_reject();
                return rule_index;
            }
        }
    }
    let required_margin = if confirmation_kind.is_some() {
        options
            .confirm_low_evidence
            .map(|confirm| confirm.margin)
            .unwrap_or(options.oracle_margin)
    } else {
        margin_for_selected_evidence(&options, best_evaluation)
    };
    if best_index != rule_index && selected_evidence.adv_vs_rule > required_margin {
        match confirmation_kind.as_deref() {
            Some("horizon_artifact_boundary") => stats.record_artifact_confirm_accept(),
            Some("low_evidence") => stats.record_confirm_accept(),
            _ => {}
        }
        stats.record_override_payoff_reasons(&selected_evidence.payoff_reasons);
        stats.record_override(decision_type, &context_keys, selected_evidence.adv_vs_rule);
        stats.record_override_event(VerifiedOverrideEvent::from_selected_evidence(
            step_index,
            state,
            decision_type,
            &context_keys,
            rule_index,
            best_index,
            options.oracle_margin,
            scoped.len(),
            &selected_evidence,
        ));
        best_index
    } else {
        if let Some(kind) = confirmation_kind.as_deref() {
            if kind == "horizon_artifact_boundary" {
                stats.record_artifact_confirm_reject();
                stats.record_reject();
            } else {
                stats.record_confirm_reject();
                stats.record_low_evidence_reject();
            }
        } else if best_index != rule_index
            && adv > options.oracle_margin
            && low_evidence_margin_applies(&options, best_evaluation)
        {
            stats.record_low_evidence_reject();
        } else {
            stats.record_reject();
        }
        rule_index
    }
}

fn choose_verified_adv_override_action_two_stage(
    env: &mut FullRunEnv,
    state: &FullRunEnvState,
    decision_type: &str,
    step_index: usize,
    options: VerifiedAdvOverrideOptions,
    episode_cache: &mut ValueCache,
    stats: &mut VerifiedAdvOverrideStats,
) -> usize {
    let scoped = scoped_legal_indices_for_state(state, options.candidate_scope);
    if scoped.is_empty() {
        stats.record_missing(decision_type, "no_scoped_candidates");
        return 0;
    }
    let context_keys = decision_context_keys(state);
    stats.record_decision(decision_type, scoped.len(), &context_keys);

    let rule_index = preview_policy_index_from_env(env, RunPolicyKind::RuleBaselineV0);
    let all_legal = legal_indices_for_state(state);
    let Some(rule_index) = rule_index.filter(|index| all_legal.contains(index)) else {
        stats.record_missing(decision_type, "missing_rule_action");
        return scoped[0];
    };
    if !scoped.iter().any(|index| *index != rule_index) {
        stats.record_reject();
        return rule_index;
    }

    let mut prefilter_indices = scoped.clone();
    if !prefilter_indices.contains(&rule_index) {
        prefilter_indices.push(rule_index);
    }
    prefilter_indices.sort_unstable();
    prefilter_indices.dedup();

    let prefilter_payload = match evaluate_candidates(
        env,
        episode_cache,
        prefilter_indices,
        options.continuation_policy,
        options.prefilter_horizon_decisions,
        options.prefilter_horizon_mode,
        options.gamma,
        options.runtime,
        EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        },
    ) {
        Ok(payload) => payload,
        Err(err) => {
            stats.record_missing(decision_type, &format!("prefilter_error:{err}"));
            return rule_index;
        }
    };
    stats.record_prefilter_payload(&prefilter_payload);

    let Some(prefilter_rule_return) = prefilter_payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == rule_index)
        .map(|evaluation| evaluation.discounted_return)
    else {
        stats.record_missing(decision_type, "missing_prefilter_rule_evaluation");
        return rule_index;
    };

    let mut candidates_by_prefilter = prefilter_payload
        .evaluations
        .iter()
        .filter(|evaluation| {
            evaluation.ok
                && evaluation.action_index != rule_index
                && scoped.contains(&evaluation.action_index)
        })
        .map(|evaluation| {
            (
                evaluation.action_index,
                evaluation.discounted_return,
                evaluation.discounted_return - prefilter_rule_return,
            )
        })
        .collect::<Vec<_>>();
    candidates_by_prefilter.sort_by(|lhs, rhs| {
        rhs.1
            .partial_cmp(&lhs.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| lhs.0.cmp(&rhs.0))
    });

    let mut final_indices = vec![rule_index];
    for (index, _return_value, adv) in &candidates_by_prefilter {
        if *adv > options.prefilter_margin {
            final_indices.push(*index);
        }
    }
    for (index, _return_value, _adv) in candidates_by_prefilter.iter().take(options.prefilter_top_k)
    {
        final_indices.push(*index);
    }
    final_indices.sort_unstable();
    final_indices.dedup();
    let kept_non_rule_count = final_indices
        .iter()
        .filter(|index| **index != rule_index)
        .count();
    stats.record_prefilter_keep(kept_non_rule_count);

    if kept_non_rule_count == 0 {
        stats.record_reject();
        return rule_index;
    }

    let final_payload = match evaluate_candidates(
        env,
        episode_cache,
        final_indices,
        options.continuation_policy,
        options.horizon_decisions,
        options.horizon_mode,
        options.gamma,
        options.runtime,
        EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        },
    ) {
        Ok(payload) => payload,
        Err(err) => {
            stats.record_missing(decision_type, &format!("final_evaluation_error:{err}"));
            return rule_index;
        }
    };
    stats.record_final_payload(&final_payload);

    let rule_evaluation = final_payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == rule_index);
    let Some(rule_return) = rule_evaluation.map(|evaluation| evaluation.discounted_return) else {
        stats.record_missing(decision_type, "missing_final_rule_evaluation");
        return rule_index;
    };

    let mut best_index = rule_index;
    let mut best_return = rule_return;
    for evaluation in final_payload.evaluations.iter().filter(|evaluation| {
        evaluation.ok
            && evaluation.action_index != rule_index
            && scoped.contains(&evaluation.action_index)
    }) {
        if evaluation.discounted_return > best_return {
            best_return = evaluation.discounted_return;
            best_index = evaluation.action_index;
        }
    }

    let adv = best_return - rule_return;
    stats.record_best_adv(adv, options.oracle_margin);
    let best_evaluation = final_payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == best_index);
    let mut selected_evidence = OverrideSelectionEvidence {
        rule_return,
        selected_return: best_return,
        adv_vs_rule: adv,
        horizon_decisions: options.horizon_decisions,
        horizon_mode: options.horizon_mode,
        horizon_stop_reason: best_evaluation
            .map(|evaluation| evaluation.horizon_stop_reason.clone()),
        payoff_reasons: best_evaluation
            .map(|evaluation| evaluation.payoff_reasons.clone())
            .unwrap_or_default(),
        confirmation_kind: None,
        artifact_reasons: Vec::new(),
        evaluated_candidate_count: count_successful_evaluations(&final_payload),
        policy_step_eval_count: final_payload.policy_step_eval_count,
    };
    let mut confirmation_kind = None;
    if best_index != rule_index && adv > options.oracle_margin {
        match maybe_confirm_suspect_override(
            env,
            episode_cache,
            state,
            rule_index,
            best_index,
            &options,
            rule_evaluation,
            best_evaluation,
            stats,
        ) {
            Ok(ConfirmationOutcome::Confirmed(confirm_evidence)) => {
                confirmation_kind = confirm_evidence.confirmation_kind.clone();
                selected_evidence = confirm_evidence;
            }
            Ok(ConfirmationOutcome::NotNeeded) => {}
            Ok(ConfirmationOutcome::Rejected) => return rule_index,
            Err(err) => {
                stats.record_missing(decision_type, &format!("confirm_evaluation_error:{err}"));
                stats.record_reject();
                return rule_index;
            }
        }
    }
    let required_margin = if confirmation_kind.is_some() {
        options
            .confirm_low_evidence
            .map(|confirm| confirm.margin)
            .unwrap_or(options.oracle_margin)
    } else {
        margin_for_selected_evidence(&options, best_evaluation)
    };
    if best_index != rule_index && selected_evidence.adv_vs_rule > required_margin {
        match confirmation_kind.as_deref() {
            Some("horizon_artifact_boundary") => stats.record_artifact_confirm_accept(),
            Some("low_evidence") => stats.record_confirm_accept(),
            _ => {}
        }
        stats.record_override_payoff_reasons(&selected_evidence.payoff_reasons);
        stats.record_override(decision_type, &context_keys, selected_evidence.adv_vs_rule);
        stats.record_override_event(VerifiedOverrideEvent::from_selected_evidence(
            step_index,
            state,
            decision_type,
            &context_keys,
            rule_index,
            best_index,
            options.oracle_margin,
            scoped.len(),
            &selected_evidence,
        ));
        best_index
    } else {
        if let Some(kind) = confirmation_kind.as_deref() {
            if kind == "horizon_artifact_boundary" {
                stats.record_artifact_confirm_reject();
                stats.record_reject();
            } else {
                stats.record_confirm_reject();
                stats.record_low_evidence_reject();
            }
        } else if best_index != rule_index
            && adv > options.oracle_margin
            && low_evidence_margin_applies(&options, best_evaluation)
        {
            stats.record_low_evidence_reject();
        } else {
            stats.record_reject();
        }
        rule_index
    }
}

fn choose_verified_adv_override_action_model_proposer(
    env: &mut FullRunEnv,
    state: &FullRunEnvState,
    decision_type: &str,
    step_index: usize,
    options: VerifiedAdvOverrideOptions,
    episode_cache: &mut ValueCache,
    stats: &mut VerifiedAdvOverrideStats,
) -> usize {
    let scoped = scoped_legal_indices_for_state(state, options.candidate_scope);
    if scoped.is_empty() {
        stats.record_missing(decision_type, "no_scoped_candidates");
        return 0;
    }
    let context_keys = decision_context_keys(state);
    stats.record_decision(decision_type, scoped.len(), &context_keys);

    let rule_index = preview_policy_index_from_env(env, RunPolicyKind::RuleBaselineV0);
    let all_legal = legal_indices_for_state(state);
    let Some(rule_index) = rule_index.filter(|index| all_legal.contains(index)) else {
        stats.record_missing(decision_type, "missing_rule_action");
        return scoped[0];
    };
    let non_rule = scoped
        .iter()
        .copied()
        .filter(|index| *index != rule_index)
        .collect::<Vec<_>>();
    if non_rule.is_empty() {
        stats.record_reject();
        return rule_index;
    }
    let Some(proposer) = options.proposer.as_ref() else {
        stats.record_missing(decision_type, "missing_proposer_model");
        return rule_index;
    };
    if rule_index >= state.action_candidates.len() {
        stats.record_missing(decision_type, "rule_candidate_out_of_range");
        return rule_index;
    }
    let rule_candidate = &state.action_candidates[rule_index];
    let mut scored = non_rule
        .iter()
        .filter_map(|index| {
            state.action_candidates.get(*index).map(|candidate| {
                (
                    proposer
                        .model
                        .predict_candidate_only(candidate, rule_candidate),
                    *index,
                )
            })
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.1.cmp(&left.1))
    });

    let mut final_indices = vec![rule_index];
    if proposer.threshold >= 0.0 {
        for (score, index) in &scored {
            if *score >= proposer.threshold {
                final_indices.push(*index);
            }
        }
    }
    if proposer.top_k > 0 {
        for (_score, index) in scored.iter().take(proposer.top_k) {
            final_indices.push(*index);
        }
    }
    if proposer.threshold < 0.0 && proposer.top_k == 0 {
        final_indices.extend(non_rule.iter().copied());
    }
    final_indices.sort_unstable();
    final_indices.dedup();
    let kept_non_rule_count = final_indices
        .iter()
        .filter(|index| **index != rule_index)
        .count();
    stats.record_model_proposer_keep(non_rule.len(), kept_non_rule_count);
    if kept_non_rule_count == 0 {
        stats.record_reject();
        return rule_index;
    }

    let final_payload = match evaluate_candidates(
        env,
        episode_cache,
        final_indices,
        options.continuation_policy,
        options.horizon_decisions,
        options.horizon_mode,
        options.gamma,
        options.runtime,
        EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        },
    ) {
        Ok(payload) => payload,
        Err(err) => {
            stats.record_missing(decision_type, &format!("final_evaluation_error:{err}"));
            return rule_index;
        }
    };
    stats.record_final_payload(&final_payload);

    let rule_evaluation = final_payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == rule_index);
    let Some(rule_return) = rule_evaluation.map(|evaluation| evaluation.discounted_return) else {
        stats.record_missing(decision_type, "missing_final_rule_evaluation");
        return rule_index;
    };

    let mut best_index = rule_index;
    let mut best_return = rule_return;
    for evaluation in final_payload.evaluations.iter().filter(|evaluation| {
        evaluation.ok
            && evaluation.action_index != rule_index
            && scoped.contains(&evaluation.action_index)
    }) {
        if evaluation.discounted_return > best_return {
            best_return = evaluation.discounted_return;
            best_index = evaluation.action_index;
        }
    }

    let adv = best_return - rule_return;
    stats.record_best_adv(adv, options.oracle_margin);
    let best_evaluation = final_payload
        .evaluations
        .iter()
        .find(|evaluation| evaluation.ok && evaluation.action_index == best_index);
    let mut selected_evidence = OverrideSelectionEvidence {
        rule_return,
        selected_return: best_return,
        adv_vs_rule: adv,
        horizon_decisions: options.horizon_decisions,
        horizon_mode: options.horizon_mode,
        horizon_stop_reason: best_evaluation
            .map(|evaluation| evaluation.horizon_stop_reason.clone()),
        payoff_reasons: best_evaluation
            .map(|evaluation| evaluation.payoff_reasons.clone())
            .unwrap_or_default(),
        confirmation_kind: None,
        artifact_reasons: Vec::new(),
        evaluated_candidate_count: count_successful_evaluations(&final_payload),
        policy_step_eval_count: final_payload.policy_step_eval_count,
    };
    let mut confirmation_kind = None;
    if best_index != rule_index && adv > options.oracle_margin {
        match maybe_confirm_suspect_override(
            env,
            episode_cache,
            state,
            rule_index,
            best_index,
            &options,
            rule_evaluation,
            best_evaluation,
            stats,
        ) {
            Ok(ConfirmationOutcome::Confirmed(confirm_evidence)) => {
                confirmation_kind = confirm_evidence.confirmation_kind.clone();
                selected_evidence = confirm_evidence;
            }
            Ok(ConfirmationOutcome::NotNeeded) => {}
            Ok(ConfirmationOutcome::Rejected) => return rule_index,
            Err(err) => {
                stats.record_missing(decision_type, &format!("confirm_evaluation_error:{err}"));
                stats.record_reject();
                return rule_index;
            }
        }
    }
    let required_margin = if confirmation_kind.is_some() {
        options
            .confirm_low_evidence
            .map(|confirm| confirm.margin)
            .unwrap_or(options.oracle_margin)
    } else {
        margin_for_selected_evidence(&options, best_evaluation)
    };
    if best_index != rule_index && selected_evidence.adv_vs_rule > required_margin {
        match confirmation_kind.as_deref() {
            Some("horizon_artifact_boundary") => stats.record_artifact_confirm_accept(),
            Some("low_evidence") => stats.record_confirm_accept(),
            _ => {}
        }
        stats.record_override_payoff_reasons(&selected_evidence.payoff_reasons);
        stats.record_override(decision_type, &context_keys, selected_evidence.adv_vs_rule);
        stats.record_override_event(VerifiedOverrideEvent::from_selected_evidence(
            step_index,
            state,
            decision_type,
            &context_keys,
            rule_index,
            best_index,
            options.oracle_margin,
            scoped.len(),
            &selected_evidence,
        ));
        best_index
    } else {
        if let Some(kind) = confirmation_kind.as_deref() {
            if kind == "horizon_artifact_boundary" {
                stats.record_artifact_confirm_reject();
                stats.record_reject();
            } else {
                stats.record_confirm_reject();
                stats.record_low_evidence_reject();
            }
        } else if best_index != rule_index
            && adv > options.oracle_margin
            && low_evidence_margin_applies(&options, best_evaluation)
        {
            stats.record_low_evidence_reject();
        } else {
            stats.record_reject();
        }
        rule_index
    }
}

fn inspect_counterfactual_pending_groups(
    env: &mut FullRunEnv,
    candidate_scope: CandidateScope,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    oracle_margin: f32,
    gamma: f32,
    max_roots: usize,
    max_groups: usize,
    parallelism: usize,
    include_observation: bool,
) -> Result<Value, String> {
    if !oracle_margin.is_finite() {
        return Err("oracle_margin must be finite".to_string());
    }
    if !gamma.is_finite() {
        return Err("gamma must be finite".to_string());
    }
    let state = env.state()?;
    let source_decision_type = state.observation.decision_type.clone();
    let root_scoped = scoped_legal_indices_for_state(&state, candidate_scope);
    let mut roots_considered = 0usize;
    let mut pending_exact_dedup_count = 0usize;
    let mut evaluation_error_count = 0usize;
    let mut unique_pending_envs = Vec::<FullRunEnv>::new();
    let mut groups = Vec::new();

    for root_action_index in root_scoped.iter().copied().take(max_roots) {
        if groups.len() >= max_groups {
            break;
        }
        roots_considered += 1;
        let root = match evaluate_root_candidate(
            env,
            &state,
            root_action_index,
            EvaluationOutputOptions {
                include_state: false,
                include_next_state: true,
                include_continuation_trace: false,
                check_live_env_unchanged: false,
            },
        ) {
            Ok(root) => root,
            Err(_err) => {
                evaluation_error_count += 1;
                continue;
            }
        };
        if root.done || root.final_info.result != "ongoing" {
            continue;
        }
        let mut pending_env = root.env_after.clone();
        let pending_state = pending_env.state()?;
        let pending_decision_type = pending_state.observation.decision_type.clone();
        if pending_decision_type == "combat"
            || !pending_decision_type.starts_with("combat_")
            || pending_decision_type == "combat_card_reward"
        {
            continue;
        }
        if unique_pending_envs
            .iter()
            .any(|existing| existing == &pending_env)
        {
            pending_exact_dedup_count += 1;
            continue;
        }
        unique_pending_envs.push(pending_env.clone());

        let pending_scoped = scoped_legal_indices_for_state(&pending_state, candidate_scope);
        if pending_scoped.is_empty() {
            continue;
        }
        let legal_all = legal_indices_for_state(&pending_state);
        let Some(rule_index) =
            preview_policy_index_from_env(&pending_env, RunPolicyKind::RuleBaselineV0)
                .filter(|index| legal_all.contains(index))
        else {
            continue;
        };
        let mut eval_indices = pending_scoped.clone();
        if !eval_indices.contains(&rule_index) {
            eval_indices.push(rule_index);
        }
        eval_indices.sort_unstable();
        eval_indices.dedup();

        let mut request_cache = ValueCache::default();
        let payload = evaluate_candidates(
            &mut pending_env,
            &mut request_cache,
            eval_indices.clone(),
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            EvaluationRuntimeOptions {
                mode: EvaluationMode::Independent,
                cache_scope: ValueCacheScope::Request,
                cache_max_entries: 0,
                parallelism,
                exact_root_dedup: false,
            },
            EvaluationOutputOptions {
                include_state: false,
                include_next_state: false,
                include_continuation_trace: false,
                check_live_env_unchanged: false,
            },
        )?;
        let Some(rule_return) = payload
            .evaluations
            .iter()
            .find(|evaluation| evaluation.ok && evaluation.action_index == rule_index)
            .map(|evaluation| evaluation.discounted_return)
        else {
            continue;
        };
        let mut best_index = rule_index;
        let mut best_return = rule_return;
        for index in &pending_scoped {
            if let Some(value) = payload
                .evaluations
                .iter()
                .find(|evaluation| evaluation.ok && evaluation.action_index == *index)
                .map(|evaluation| evaluation.discounted_return)
            {
                if value > best_return {
                    best_return = value;
                    best_index = *index;
                }
            }
        }
        let best_adv = best_return - rule_return;
        let selected_index = if best_index != rule_index && best_adv > oracle_margin {
            best_index
        } else {
            rule_index
        };
        let candidate_rows = payload
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.ok)
            .map(|evaluation| {
                let adv = evaluation.discounted_return - rule_return;
                let candidate = pending_state.action_candidates.get(evaluation.action_index);
                json!({
                    "action_index": evaluation.action_index,
                    "action_key": candidate.map(|candidate| candidate.action_key.clone()),
                    "is_rule_choice": evaluation.action_index == rule_index,
                    "is_selected_choice": evaluation.action_index == selected_index,
                    "q_candidate_mean": evaluation.discounted_return,
                    "adv_vs_rule_mean": adv,
                    "passes_margin": evaluation.action_index != rule_index && adv > oracle_margin,
                    "candidate": candidate,
                })
            })
            .collect::<Vec<_>>();
        let pending_choice = pending_state
            .observation
            .combat
            .as_ref()
            .and_then(|combat| combat.pending_choice.as_ref());
        let pending_choice_kind = pending_state
            .observation
            .combat
            .as_ref()
            .and_then(|combat| combat.pending_choice_kind.clone());
        let mut group = json!({
            "schema_version": "verified_teacher_counterfactual_pending_group_v0",
            "source": "counterfactual_root_force",
            "source_decision_type": source_decision_type,
            "parent_action_index": root_action_index,
            "parent_action_key": root.candidate.as_ref().map(|candidate| candidate.action_key.clone()),
            "parent_candidate": root.candidate,
            "decision_type": pending_decision_type,
            "pending_choice_kind": pending_choice_kind,
            "pending_choice": pending_choice,
            "scoped_candidate_count": pending_scoped.len(),
            "candidate_count": eval_indices.len(),
            "rule_index": rule_index,
            "selected_action_index": selected_index,
            "q_rule_mean": rule_return,
            "q_best_mean": best_return,
            "best_adv_vs_rule_mean": best_adv,
            "oracle_margin": oracle_margin,
            "candidate_evaluation_count": payload.evaluations.iter().filter(|evaluation| evaluation.ok).count(),
            "policy_step_eval_count": payload.policy_step_eval_count,
            "candidates": candidate_rows,
        });
        if include_observation {
            group["observation"] = serde_json::to_value(&pending_state.observation)
                .map_err(|err| format!("pending observation serialize failed: {err}"))?;
        }
        groups.push(group);
    }

    Ok(json!({
        "schema_version": "verified_teacher_counterfactual_pending_inspect_v0",
        "source_decision_type": source_decision_type,
        "candidate_scope": candidate_scope.as_str(),
        "continuation_policy": continuation_policy.as_str(),
        "horizon_decisions": horizon_decisions,
        "horizon_mode": horizon_mode.as_str(),
        "oracle_margin": oracle_margin,
        "gamma": gamma,
        "root_scoped_candidate_count": root_scoped.len(),
        "roots_considered": roots_considered,
        "pending_group_count": groups.len(),
        "pending_exact_dedup_count": pending_exact_dedup_count,
        "evaluation_error_count": evaluation_error_count,
        "groups": groups,
    }))
}

fn is_combat_decision_type(decision_type: &str) -> bool {
    decision_type.starts_with("combat")
}

fn legal_indices_for_state(state: &FullRunEnvState) -> Vec<usize> {
    state
        .action_mask
        .iter()
        .enumerate()
        .filter_map(|(index, legal)| {
            if *legal && index < state.action_candidates.len() {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn scoped_legal_indices_for_state(
    state: &FullRunEnvState,
    candidate_scope: CandidateScope,
) -> Vec<usize> {
    state
        .action_mask
        .iter()
        .enumerate()
        .filter_map(|(index, legal)| {
            if *legal
                && index < state.action_candidates.len()
                && candidate_allowed_for_scope(
                    &state.action_candidates[index],
                    candidate_scope,
                    &state.observation.decision_type,
                )
            {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn candidate_allowed_for_scope(
    candidate: &RunActionCandidate,
    scope: CandidateScope,
    decision_type: &str,
) -> bool {
    let key = candidate.action_key.as_str();
    match scope {
        CandidateScope::All => true,
        CandidateScope::ControlledV0 => {
            key.starts_with("combat/play_card") || key.starts_with("combat/end_turn")
        }
        CandidateScope::ControlledV1 => {
            if decision_type == "combat" {
                return key.starts_with("combat/play_card") || key.starts_with("combat/end_turn");
            }
            if !decision_type.starts_with("combat_") || decision_type == "combat_card_reward" {
                return false;
            }
            if key.starts_with("combat/use_potion") || key.starts_with("combat/discard_potion") {
                return false;
            }
            key.starts_with("choice/")
                || key.starts_with("combat/hand_select")
                || key.starts_with("combat/grid_select")
                || key.starts_with("combat/scry_discard")
                || key.starts_with("combat/card_choice")
                || key.starts_with("selection/")
                || key == "proceed"
                || key == "cancel"
        }
    }
}

fn decision_context_keys(state: &FullRunEnvState) -> Vec<String> {
    let observation = &state.observation;
    let mut keys = vec![
        format!("decision_type:{}", observation.decision_type),
        format!("act:{}", observation.act),
    ];
    if let Some(room) = &observation.current_room {
        keys.push(format!("room:{room}"));
    }
    if let Some(combat) = &observation.combat {
        if let Some(kind) = &combat.pending_choice_kind {
            keys.push(format!("pending_choice:{kind}"));
        } else if observation.decision_type == "combat" {
            keys.push("pending_choice:none".to_string());
        }
        if combat.alive_monster_count > 1 {
            keys.push("combat_shape:multi_enemy".to_string());
        } else if combat.alive_monster_count == 1 {
            keys.push("combat_shape:single_enemy".to_string());
        }
        if combat.visible_incoming_damage > combat.player_block {
            keys.push("pressure:incoming_leaks_current_block".to_string());
        } else if combat.visible_incoming_damage > 0 {
            keys.push("pressure:incoming_covered_by_current_block".to_string());
        } else {
            keys.push("pressure:no_visible_incoming".to_string());
        }
        if observation.hp_ratio_milli <= 250 {
            keys.push("hp_band:low".to_string());
        } else if observation.hp_ratio_milli <= 500 {
            keys.push("hp_band:mid".to_string());
        } else {
            keys.push("hp_band:high".to_string());
        }
        if combat.energy == 0 {
            keys.push("resource:zero_energy".to_string());
        }
    }
    keys
}

fn best_adv_bucket(adv: f32, margin: f32) -> &'static str {
    if adv <= 0.0 {
        "adv_le_0"
    } else if adv <= margin {
        "adv_0_to_margin"
    } else if adv <= margin * 2.0 {
        "adv_margin_to_2x_margin"
    } else {
        "adv_gt_2x_margin"
    }
}

impl VerifiedOverrideEvent {
    fn from_selected_evidence(
        step: usize,
        state: &FullRunEnvState,
        decision_type: &str,
        context_keys: &[String],
        rule_index: usize,
        selected_index: usize,
        oracle_margin: f32,
        scoped_candidate_count: usize,
        evidence: &OverrideSelectionEvidence,
    ) -> Self {
        Self {
            step,
            decision_type: decision_type.to_string(),
            act: state.observation.act,
            floor: state.observation.floor,
            hp: state.observation.current_hp,
            max_hp: state.observation.max_hp,
            context_keys: context_keys.to_vec(),
            rule_index,
            selected_index,
            rule_action_key: candidate_action_key(state, rule_index),
            selected_action_key: candidate_action_key(state, selected_index),
            rule_return: evidence.rule_return,
            selected_return: evidence.selected_return,
            adv_vs_rule: evidence.adv_vs_rule,
            oracle_margin,
            horizon_decisions: evidence.horizon_decisions,
            horizon_mode: evidence.horizon_mode.as_str().to_string(),
            horizon_stop_reason: evidence.horizon_stop_reason.clone(),
            payoff_reasons: evidence.payoff_reasons.clone(),
            confirmation_kind: evidence.confirmation_kind.clone(),
            artifact_reasons: evidence.artifact_reasons.clone(),
            scoped_candidate_count,
            evaluated_candidate_count: evidence.evaluated_candidate_count,
            policy_step_eval_count: evidence.policy_step_eval_count,
        }
    }
}

fn candidate_action_key(state: &FullRunEnvState, index: usize) -> Option<String> {
    state
        .action_candidates
        .get(index)
        .map(|candidate| candidate.action_key.clone())
}

#[derive(Clone, Debug)]
struct OverrideSelectionEvidence {
    rule_return: f32,
    selected_return: f32,
    adv_vs_rule: f32,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    horizon_stop_reason: Option<String>,
    payoff_reasons: Vec<String>,
    confirmation_kind: Option<String>,
    artifact_reasons: Vec<String>,
    evaluated_candidate_count: usize,
    policy_step_eval_count: usize,
}

impl OverrideSelectionEvidence {
    fn from_payload(
        payload: &CandidateEvaluationPayload,
        rule_index: usize,
        selected_index: usize,
        horizon_decisions: usize,
        horizon_mode: HorizonMode,
    ) -> Option<Self> {
        let rule_return = payload
            .evaluations
            .iter()
            .find(|evaluation| evaluation.ok && evaluation.action_index == rule_index)
            .map(|evaluation| evaluation.discounted_return)?;
        let selected_evaluation = payload
            .evaluations
            .iter()
            .find(|evaluation| evaluation.ok && evaluation.action_index == selected_index)?;
        let selected_return = selected_evaluation.discounted_return;
        Some(Self {
            rule_return,
            selected_return,
            adv_vs_rule: selected_return - rule_return,
            horizon_decisions,
            horizon_mode,
            horizon_stop_reason: Some(selected_evaluation.horizon_stop_reason.clone()),
            payoff_reasons: selected_evaluation.payoff_reasons.clone(),
            confirmation_kind: None,
            artifact_reasons: Vec::new(),
            evaluated_candidate_count: count_successful_evaluations(payload),
            policy_step_eval_count: payload.policy_step_eval_count,
        })
    }
}

fn low_evidence_margin_applies(
    options: &VerifiedAdvOverrideOptions,
    selected_evaluation: Option<&CandidateEvaluation>,
) -> bool {
    if options.low_evidence_margin.is_none() || options.evidence_gate == EvidenceGate::None {
        return false;
    }
    selected_evaluation.is_some_and(|evaluation| match options.evidence_gate {
        EvidenceGate::None => false,
        EvidenceGate::HorizonCapNoPayoffV1 => {
            evaluation.horizon_stop_reason == "horizon_decision_cap"
                && evaluation.payoff_reasons.is_empty()
        }
        EvidenceGate::HorizonCapAnyV1 => evaluation.horizon_stop_reason == "horizon_decision_cap",
    })
}

enum ConfirmationOutcome {
    NotNeeded,
    Confirmed(OverrideSelectionEvidence),
    Rejected,
}

fn maybe_confirm_suspect_override(
    env: &mut FullRunEnv,
    episode_cache: &mut ValueCache,
    state: &FullRunEnvState,
    rule_index: usize,
    best_index: usize,
    options: &VerifiedAdvOverrideOptions,
    rule_evaluation: Option<&CandidateEvaluation>,
    selected_evaluation: Option<&CandidateEvaluation>,
    stats: &mut VerifiedAdvOverrideStats,
) -> Result<ConfirmationOutcome, String> {
    if best_index == rule_index {
        return Ok(ConfirmationOutcome::NotNeeded);
    }

    let artifact_reasons =
        horizon_artifact_reasons(state, rule_index, best_index, rule_evaluation, selected_evaluation);
    let low_evidence = low_evidence_margin_applies(options, selected_evaluation);
    if artifact_reasons.is_empty() && !low_evidence {
        return Ok(ConfirmationOutcome::NotNeeded);
    }
    let Some(confirm) = options.confirm_low_evidence else {
        if !artifact_reasons.is_empty() {
            stats.record_artifact_confirm_decision(&artifact_reasons);
            stats.record_artifact_confirm_reject();
            stats.record_reject();
            return Ok(ConfirmationOutcome::Rejected);
        }
        return Ok(ConfirmationOutcome::NotNeeded);
    };

    let (confirmation_kind, horizon_mode, horizon_decisions) = if artifact_reasons.is_empty() {
        ("low_evidence".to_string(), confirm.horizon_mode, confirm.horizon_decisions)
    } else {
        stats.record_artifact_confirm_decision(&artifact_reasons);
        (
            "horizon_artifact_boundary".to_string(),
            confirm.horizon_mode,
            confirm.horizon_decisions,
        )
    };
    if artifact_reasons.is_empty() {
        stats.record_confirm_decision();
    }

    let mut confirm_indices = vec![rule_index, best_index];
    confirm_indices.sort_unstable();
    confirm_indices.dedup();
    let runtime = confirmation_runtime_options(options.runtime, horizon_mode);
    let payload = evaluate_candidates(
        env,
        episode_cache,
        confirm_indices,
        options.continuation_policy,
        horizon_decisions,
        horizon_mode,
        options.gamma,
        runtime,
        EvaluationOutputOptions {
            include_state: false,
            include_next_state: false,
            include_continuation_trace: false,
            check_live_env_unchanged: false,
        },
    )?;
    if artifact_reasons.is_empty() {
        stats.record_confirm_payload(&payload);
    } else {
        stats.record_artifact_confirm_payload(&payload);
    }
    let mut evidence = OverrideSelectionEvidence::from_payload(
        &payload,
        rule_index,
        best_index,
        horizon_decisions,
        horizon_mode,
    )
    .ok_or_else(|| "missing_confirm_evaluation".to_string())?;
    evidence.confirmation_kind = Some(confirmation_kind);
    evidence.artifact_reasons = artifact_reasons;
    Ok(ConfirmationOutcome::Confirmed(evidence))
}

fn confirmation_runtime_options(
    mut runtime: EvaluationRuntimeOptions,
    horizon_mode: HorizonMode,
) -> EvaluationRuntimeOptions {
    if horizon_mode != HorizonMode::FixedDecisions && runtime.mode == EvaluationMode::BellmanCachedV1
    {
        runtime.mode = EvaluationMode::Independent;
        runtime.cache_scope = ValueCacheScope::Request;
    }
    runtime
}

fn horizon_artifact_reasons(
    state: &FullRunEnvState,
    rule_index: usize,
    selected_index: usize,
    rule_evaluation: Option<&CandidateEvaluation>,
    selected_evaluation: Option<&CandidateEvaluation>,
) -> Vec<String> {
    let Some(selected_evaluation) = selected_evaluation else {
        return Vec::new();
    };
    if selected_evaluation.horizon_stop_reason != "horizon_decision_cap" {
        return Vec::new();
    }
    let adv = rule_evaluation
        .map(|rule| selected_evaluation.discounted_return - rule.discounted_return)
        .unwrap_or(0.0);
    let rule_key = candidate_action_key(state, rule_index);
    let selected_key = candidate_action_key(state, selected_index);
    let mut reasons = Vec::new();
    let near_combat_win_boundary = (1.75..2.75).contains(&adv);
    if near_combat_win_boundary && selected_evaluation.payoff_reasons.is_empty() {
        reasons.push("cap_adv_near_combat_win_no_payoff".to_string());
    }
    if near_combat_win_boundary
        && same_card_target_swap(rule_key.as_deref(), selected_key.as_deref())
    {
        reasons.push("target_swap_combat_win_boundary".to_string());
    }
    if near_combat_win_boundary && is_end_turn_action(selected_key.as_deref()) {
        reasons.push("end_turn_time_boundary".to_string());
    }
    if (1.75..3.25).contains(&adv)
        && incoming_leaks_current_block(state)
        && candidate_has_block(state, rule_index)
        && !candidate_has_block(state, selected_index)
    {
        reasons.push("skip_block_under_incoming_boundary".to_string());
    }
    if terminal_defeat(rule_evaluation)
        && selected_evaluation.horizon_stop_reason == "horizon_decision_cap"
    {
        reasons.push("terminal_cliff_rule_defeat_selected_truncated".to_string());
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn terminal_defeat(evaluation: Option<&CandidateEvaluation>) -> bool {
    evaluation.is_some_and(|evaluation| {
        evaluation.rollout_done
            && evaluation
                .final_info
                .as_ref()
                .is_some_and(|info| info.result == "defeat")
    })
}

fn same_card_target_swap(rule_key: Option<&str>, selected_key: Option<&str>) -> bool {
    let Some(rule_key) = rule_key else {
        return false;
    };
    let Some(selected_key) = selected_key else {
        return false;
    };
    if !rule_key.contains("target:monster_slot") || !selected_key.contains("target:monster_slot") {
        return false;
    }
    action_card_name(rule_key) == action_card_name(selected_key)
        && action_target(rule_key) != action_target(selected_key)
}

fn action_card_name(action_key: &str) -> Option<&str> {
    action_key
        .split_once("card:")
        .and_then(|(_, rest)| rest.split('/').next())
}

fn action_target(action_key: &str) -> Option<&str> {
    action_key
        .split_once("target:")
        .map(|(_, rest)| rest.split('/').next().unwrap_or(rest))
}

fn is_end_turn_action(action_key: Option<&str>) -> bool {
    action_key == Some("combat/end_turn")
}

fn candidate_has_block(state: &FullRunEnvState, index: usize) -> bool {
    state
        .action_candidates
        .get(index)
        .and_then(|candidate| candidate.card.as_ref())
        .is_some_and(|card| card.base_block > 0 || card.upgraded_block > 0)
}

fn incoming_leaks_current_block(state: &FullRunEnvState) -> bool {
    state.observation.combat.as_ref().is_some_and(|combat| {
        combat.visible_incoming_damage > combat.player_block
    })
}

fn margin_for_selected_evidence(
    options: &VerifiedAdvOverrideOptions,
    selected_evaluation: Option<&CandidateEvaluation>,
) -> f32 {
    if low_evidence_margin_applies(options, selected_evaluation) {
        options
            .low_evidence_margin
            .unwrap_or(options.oracle_margin)
            .max(options.oracle_margin)
    } else {
        options.oracle_margin
    }
}

impl VerifiedAdvOverrideStats {
    fn record_decision(
        &mut self,
        decision_type: &str,
        scoped_candidate_count: usize,
        context_keys: &[String],
    ) {
        self.decision_count += 1;
        self.scoped_candidate_count_sum += scoped_candidate_count;
        *self
            .decision_type_counts
            .entry(decision_type.to_string())
            .or_insert(0) += 1;
        for key in context_keys {
            *self.decision_context_counts.entry(key.clone()).or_insert(0) += 1;
        }
    }

    fn record_override(&mut self, decision_type: &str, context_keys: &[String], adv: f32) {
        self.override_count += 1;
        self.verified_adv_sum += adv;
        if adv < 0.0 {
            self.harmful_override_count += 1;
        }
        *self
            .override_decision_type_counts
            .entry(decision_type.to_string())
            .or_insert(0) += 1;
        for key in context_keys {
            *self.override_context_counts.entry(key.clone()).or_insert(0) += 1;
        }
        self.max_verified_adv = Some(self.max_verified_adv.map_or(adv, |value| value.max(adv)));
        self.min_verified_adv = Some(self.min_verified_adv.map_or(adv, |value| value.min(adv)));
    }

    fn record_best_adv(&mut self, adv: f32, margin: f32) {
        let bucket = best_adv_bucket(adv, margin);
        *self
            .best_adv_bucket_counts
            .entry(bucket.to_string())
            .or_insert(0) += 1;
    }

    fn record_override_payoff_reasons(&mut self, reasons: &[String]) {
        if reasons.is_empty() {
            *self
                .override_payoff_reason_counts
                .entry("none".to_string())
                .or_insert(0) += 1;
            return;
        }
        for reason in reasons {
            *self
                .override_payoff_reason_counts
                .entry(reason.clone())
                .or_insert(0) += 1;
        }
    }

    fn record_override_event(&mut self, event: VerifiedOverrideEvent) {
        self.override_events.push(event);
    }

    fn record_reject(&mut self) {
        self.reject_count += 1;
    }

    fn record_low_evidence_reject(&mut self) {
        self.low_evidence_reject_count += 1;
        self.record_reject();
    }

    fn record_confirm_payload(&mut self, payload: &CandidateEvaluationPayload) {
        let successful = count_successful_evaluations(payload);
        self.confirm_candidate_evaluation_count += successful;
        self.confirm_policy_step_eval_count += payload.policy_step_eval_count;
        self.evaluated_candidate_count += successful;
        self.record_evaluation_payload(payload);
    }

    fn record_artifact_confirm_payload(&mut self, payload: &CandidateEvaluationPayload) {
        let successful = count_successful_evaluations(payload);
        self.artifact_confirm_candidate_evaluation_count += successful;
        self.artifact_confirm_policy_step_eval_count += payload.policy_step_eval_count;
        self.evaluated_candidate_count += successful;
        self.record_evaluation_payload(payload);
    }

    fn record_confirm_decision(&mut self) {
        self.confirm_decision_count += 1;
    }

    fn record_confirm_accept(&mut self) {
        self.confirm_accept_count += 1;
    }

    fn record_confirm_reject(&mut self) {
        self.confirm_reject_count += 1;
    }

    fn record_artifact_confirm_decision(&mut self, reasons: &[String]) {
        self.artifact_confirm_decision_count += 1;
        for reason in reasons {
            *self
                .horizon_artifact_reason_counts
                .entry(reason.clone())
                .or_insert(0) += 1;
        }
    }

    fn record_artifact_confirm_accept(&mut self) {
        self.artifact_confirm_accept_count += 1;
    }

    fn record_artifact_confirm_reject(&mut self) {
        self.artifact_confirm_reject_count += 1;
    }

    fn record_missing(&mut self, decision_type: &str, reason: &str) {
        *self
            .missing_counts
            .entry(format!("{decision_type}:{reason}"))
            .or_insert(0) += 1;
    }

    fn record_prefilter_payload(&mut self, payload: &CandidateEvaluationPayload) {
        let successful = count_successful_evaluations(payload);
        self.prefilter_candidate_evaluation_count += successful;
        self.prefilter_policy_step_eval_count += payload.policy_step_eval_count;
        self.evaluated_candidate_count += successful;
        self.record_evaluation_payload(payload);
    }

    fn record_final_payload(&mut self, payload: &CandidateEvaluationPayload) {
        let successful = count_successful_evaluations(payload);
        self.final_candidate_evaluation_count += successful;
        self.final_policy_step_eval_count += payload.policy_step_eval_count;
        self.evaluated_candidate_count += successful;
        self.record_evaluation_payload(payload);
    }

    fn record_prefilter_keep(&mut self, kept_non_rule_count: usize) {
        self.prefilter_decision_count += 1;
        if kept_non_rule_count > 0 {
            self.prefilter_kept_decision_count += 1;
        }
        self.prefilter_kept_candidate_count += kept_non_rule_count;
    }

    fn record_model_proposer_keep(&mut self, non_rule_count: usize, kept_non_rule_count: usize) {
        self.proposer_decision_count += 1;
        self.proposer_non_rule_candidate_count += non_rule_count;
        self.proposer_kept_candidate_count += kept_non_rule_count;
        self.record_prefilter_keep(kept_non_rule_count);
    }

    fn record_evaluation_payload(&mut self, payload: &CandidateEvaluationPayload) {
        self.cached_root_candidate_count += payload.root_candidate_count;
        self.cached_root_exact_dedup_count += payload.root_exact_dedup_count;
        self.root_rule_equivalent_prune_count += payload.root_rule_equivalent_prune_count;
        self.cached_value_hit_count += payload.value_cache_hit_count;
        self.cached_value_miss_count += payload.value_cache_miss_count;
        self.cached_policy_step_eval_count += payload.policy_step_eval_count;
        self.cached_cache_entry_count_max = self
            .cached_cache_entry_count_max
            .max(payload.cache_entry_count);
        self.parallelism_used_max = self.parallelism_used_max.max(payload.parallelism_used);
        self.candidate_eval_wall_ms += payload.candidate_eval_wall_ms;
        for evaluation in &payload.evaluations {
            if !evaluation.ok {
                continue;
            }
            *self
                .horizon_stop_reason_counts
                .entry(evaluation.horizon_stop_reason.clone())
                .or_insert(0) += 1;
            if evaluation.payoff_reasons.is_empty() {
                *self
                    .payoff_reason_counts
                    .entry("none".to_string())
                    .or_insert(0) += 1;
            } else {
                for reason in &evaluation.payoff_reasons {
                    *self.payoff_reason_counts.entry(reason.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    fn as_payload(&self) -> VerifiedAdvOverrideStatsPayload {
        VerifiedAdvOverrideStatsPayload {
            verified_decision_count: self.decision_count,
            verified_override_count: self.override_count,
            verified_reject_count: self.reject_count,
            verified_override_rate: ratio(self.override_count, self.decision_count).unwrap_or(0.0),
            verified_candidate_evaluation_count: self.evaluated_candidate_count,
            verified_prefilter_candidate_evaluation_count: self
                .prefilter_candidate_evaluation_count,
            verified_final_candidate_evaluation_count: self.final_candidate_evaluation_count,
            verified_prefilter_policy_step_eval_count: self.prefilter_policy_step_eval_count,
            verified_final_policy_step_eval_count: self.final_policy_step_eval_count,
            verified_prefilter_decision_count: self.prefilter_decision_count,
            verified_prefilter_kept_decision_count: self.prefilter_kept_decision_count,
            verified_prefilter_kept_candidate_count: self.prefilter_kept_candidate_count,
            verified_prefilter_kept_rate: ratio(
                self.prefilter_kept_decision_count,
                self.prefilter_decision_count,
            ),
            verified_prefilter_average_kept_candidate_count: ratio(
                self.prefilter_kept_candidate_count,
                self.prefilter_decision_count,
            ),
            verified_proposer_decision_count: self.proposer_decision_count,
            verified_proposer_non_rule_candidate_count: self.proposer_non_rule_candidate_count,
            verified_proposer_kept_candidate_count: self.proposer_kept_candidate_count,
            verified_proposer_keep_rate: ratio(
                self.proposer_kept_candidate_count,
                self.proposer_non_rule_candidate_count,
            ),
            verified_average_scoped_candidate_count: ratio(
                self.scoped_candidate_count_sum,
                self.decision_count,
            )
            .unwrap_or(0.0),
            verified_adv_mean_on_overrides: ratio_f32(self.verified_adv_sum, self.override_count),
            verified_harmful_override_count: self.harmful_override_count,
            verified_harmful_override_rate: ratio(self.harmful_override_count, self.override_count),
            verified_low_evidence_reject_count: self.low_evidence_reject_count,
            verified_confirm_decision_count: self.confirm_decision_count,
            verified_confirm_accept_count: self.confirm_accept_count,
            verified_confirm_reject_count: self.confirm_reject_count,
            verified_confirm_candidate_evaluation_count: self.confirm_candidate_evaluation_count,
            verified_confirm_policy_step_eval_count: self.confirm_policy_step_eval_count,
            verified_artifact_confirm_decision_count: self.artifact_confirm_decision_count,
            verified_artifact_confirm_accept_count: self.artifact_confirm_accept_count,
            verified_artifact_confirm_reject_count: self.artifact_confirm_reject_count,
            verified_artifact_confirm_candidate_evaluation_count: self
                .artifact_confirm_candidate_evaluation_count,
            verified_artifact_confirm_policy_step_eval_count: self
                .artifact_confirm_policy_step_eval_count,
            verified_min_adv_on_overrides: self.min_verified_adv,
            verified_max_adv_on_overrides: self.max_verified_adv,
            verified_decision_type_counts: self.decision_type_counts.clone(),
            verified_override_decision_type_counts: self.override_decision_type_counts.clone(),
            verified_decision_context_counts: self.decision_context_counts.clone(),
            verified_override_context_counts: self.override_context_counts.clone(),
            verified_best_adv_bucket_counts: self.best_adv_bucket_counts.clone(),
            verified_horizon_stop_reason_counts: self.horizon_stop_reason_counts.clone(),
            verified_payoff_reason_counts: self.payoff_reason_counts.clone(),
            verified_override_payoff_reason_counts: self.override_payoff_reason_counts.clone(),
            verified_horizon_artifact_reason_counts: self.horizon_artifact_reason_counts.clone(),
            verified_missing_counts: self.missing_counts.clone(),
            verified_cached_root_candidate_count: self.cached_root_candidate_count,
            verified_cached_root_exact_dedup_count: self.cached_root_exact_dedup_count,
            verified_root_rule_equivalent_prune_count: self.root_rule_equivalent_prune_count,
            verified_cached_value_hit_count: self.cached_value_hit_count,
            verified_cached_value_miss_count: self.cached_value_miss_count,
            verified_cached_policy_step_eval_count: self.cached_policy_step_eval_count,
            verified_cached_cache_entry_count_max: self.cached_cache_entry_count_max,
            verified_parallelism_used_max: self.parallelism_used_max,
            verified_candidate_eval_wall_ms: self.candidate_eval_wall_ms,
            verified_override_events: self.override_events.clone(),
        }
    }
}

fn count_successful_evaluations(payload: &CandidateEvaluationPayload) -> usize {
    payload
        .evaluations
        .iter()
        .filter(|evaluation| evaluation.ok)
        .count()
}

fn summarize_verified_episodes(
    rows: &[VerifiedAdvOverrideEpisodeSummary],
) -> VerifiedAdvOverridePolicySummary {
    let rewards = rows.iter().map(|row| row.total_reward).collect::<Vec<_>>();
    let final_floors = rows
        .iter()
        .map(|row| row.final_floor as f32)
        .collect::<Vec<_>>();
    let final_hps = rows
        .iter()
        .map(|row| row.final_hp as f32)
        .collect::<Vec<_>>();
    let mut result_counts = BTreeMap::new();
    let mut terminal_reason_counts = BTreeMap::new();
    let mut death_floor_counts = BTreeMap::new();
    let mut decision_type_counts = BTreeMap::new();
    let mut override_decision_type_counts = BTreeMap::new();
    let mut decision_context_counts = BTreeMap::new();
    let mut override_context_counts = BTreeMap::new();
    let mut best_adv_bucket_counts = BTreeMap::new();
    let mut horizon_stop_reason_counts = BTreeMap::new();
    let mut payoff_reason_counts = BTreeMap::new();
    let mut override_payoff_reason_counts = BTreeMap::new();
    let mut horizon_artifact_reason_counts = BTreeMap::new();
    let mut missing_counts = BTreeMap::new();
    let mut verified_adv_weighted_sum = 0.0f32;
    let mut verified_overrides = 0usize;
    let mut verified_harmful = 0usize;
    let mut verified_decisions = 0usize;
    let mut verified_evaluations = 0usize;
    let mut prefilter_evaluations = 0usize;
    let mut final_evaluations = 0usize;
    let mut prefilter_policy_steps = 0usize;
    let mut final_policy_steps = 0usize;
    let mut prefilter_decisions = 0usize;
    let mut prefilter_kept_decisions = 0usize;
    let mut prefilter_kept = 0usize;
    let mut proposer_decisions = 0usize;
    let mut proposer_non_rule = 0usize;
    let mut proposer_kept = 0usize;
    let mut confirm_decisions = 0usize;
    let mut confirm_accepts = 0usize;
    let mut confirm_rejects = 0usize;
    let mut confirm_evaluations = 0usize;
    let mut confirm_policy_steps = 0usize;
    let mut artifact_confirm_decisions = 0usize;
    let mut artifact_confirm_accepts = 0usize;
    let mut artifact_confirm_rejects = 0usize;
    let mut artifact_confirm_evaluations = 0usize;
    let mut artifact_confirm_policy_steps = 0usize;
    let mut scoped_candidate_count_sum = 0.0f32;
    let mut cached_root_candidates = 0usize;
    let mut cached_root_dedup = 0usize;
    let mut root_rule_equiv_prunes = 0usize;
    let mut cached_value_hits = 0usize;
    let mut cached_value_misses = 0usize;
    let mut cached_policy_steps = 0usize;
    let mut cached_entry_count_max = 0usize;
    let mut parallelism_used_max = 0usize;
    let mut candidate_eval_wall_ms = 0u64;

    for row in rows {
        *result_counts.entry(row.result.clone()).or_insert(0) += 1;
        *terminal_reason_counts
            .entry(row.terminal_reason.clone())
            .or_insert(0) += 1;
        if row.result == "defeat" {
            *death_floor_counts
                .entry(format!("act{}:floor{}", row.final_act, row.final_floor))
                .or_insert(0) += 1;
        }
        verified_decisions += row.stats.verified_decision_count;
        verified_overrides += row.stats.verified_override_count;
        verified_evaluations += row.stats.verified_candidate_evaluation_count;
        prefilter_evaluations += row.stats.verified_prefilter_candidate_evaluation_count;
        final_evaluations += row.stats.verified_final_candidate_evaluation_count;
        prefilter_policy_steps += row.stats.verified_prefilter_policy_step_eval_count;
        final_policy_steps += row.stats.verified_final_policy_step_eval_count;
        prefilter_decisions += row.stats.verified_prefilter_decision_count;
        prefilter_kept_decisions += row.stats.verified_prefilter_kept_decision_count;
        prefilter_kept += row.stats.verified_prefilter_kept_candidate_count;
        proposer_decisions += row.stats.verified_proposer_decision_count;
        proposer_non_rule += row.stats.verified_proposer_non_rule_candidate_count;
        proposer_kept += row.stats.verified_proposer_kept_candidate_count;
        confirm_decisions += row.stats.verified_confirm_decision_count;
        confirm_accepts += row.stats.verified_confirm_accept_count;
        confirm_rejects += row.stats.verified_confirm_reject_count;
        confirm_evaluations += row.stats.verified_confirm_candidate_evaluation_count;
        confirm_policy_steps += row.stats.verified_confirm_policy_step_eval_count;
        artifact_confirm_decisions += row.stats.verified_artifact_confirm_decision_count;
        artifact_confirm_accepts += row.stats.verified_artifact_confirm_accept_count;
        artifact_confirm_rejects += row.stats.verified_artifact_confirm_reject_count;
        artifact_confirm_evaluations += row
            .stats
            .verified_artifact_confirm_candidate_evaluation_count;
        artifact_confirm_policy_steps += row.stats.verified_artifact_confirm_policy_step_eval_count;
        verified_harmful += row.stats.verified_harmful_override_count;
        if let Some(mean_adv) = row.stats.verified_adv_mean_on_overrides {
            verified_adv_weighted_sum += mean_adv * row.stats.verified_override_count as f32;
        }
        scoped_candidate_count_sum += row.stats.verified_average_scoped_candidate_count
            * row.stats.verified_decision_count as f32;
        merge_counts(
            &mut decision_type_counts,
            &row.stats.verified_decision_type_counts,
        );
        merge_counts(
            &mut override_decision_type_counts,
            &row.stats.verified_override_decision_type_counts,
        );
        merge_counts(
            &mut decision_context_counts,
            &row.stats.verified_decision_context_counts,
        );
        merge_counts(
            &mut override_context_counts,
            &row.stats.verified_override_context_counts,
        );
        merge_counts(
            &mut best_adv_bucket_counts,
            &row.stats.verified_best_adv_bucket_counts,
        );
        merge_counts(
            &mut horizon_stop_reason_counts,
            &row.stats.verified_horizon_stop_reason_counts,
        );
        merge_counts(
            &mut payoff_reason_counts,
            &row.stats.verified_payoff_reason_counts,
        );
        merge_counts(
            &mut override_payoff_reason_counts,
            &row.stats.verified_override_payoff_reason_counts,
        );
        merge_counts(
            &mut horizon_artifact_reason_counts,
            &row.stats.verified_horizon_artifact_reason_counts,
        );
        merge_counts(&mut missing_counts, &row.stats.verified_missing_counts);
        cached_root_candidates += row.stats.verified_cached_root_candidate_count;
        cached_root_dedup += row.stats.verified_cached_root_exact_dedup_count;
        root_rule_equiv_prunes += row.stats.verified_root_rule_equivalent_prune_count;
        cached_value_hits += row.stats.verified_cached_value_hit_count;
        cached_value_misses += row.stats.verified_cached_value_miss_count;
        cached_policy_steps += row.stats.verified_cached_policy_step_eval_count;
        cached_entry_count_max =
            cached_entry_count_max.max(row.stats.verified_cached_cache_entry_count_max);
        parallelism_used_max = parallelism_used_max.max(row.stats.verified_parallelism_used_max);
        candidate_eval_wall_ms += row.stats.verified_candidate_eval_wall_ms;
    }

    VerifiedAdvOverridePolicySummary {
        episodes: rows.len(),
        crash_count: rows.iter().filter(|row| row.crash.is_some()).count(),
        result_counts,
        terminal_reason_counts,
        death_floor_counts,
        average_total_reward: mean_f32(&rewards),
        reward_stderr: stderr_f32(&rewards),
        average_combat_win_count: mean_f32(
            &rows
                .iter()
                .map(|row| row.combat_win_count as f32)
                .collect::<Vec<_>>(),
        ),
        average_final_floor: mean_f32(&final_floors),
        average_final_hp: mean_f32(&final_hps),
        average_steps: mean_f32(&rows.iter().map(|row| row.steps as f32).collect::<Vec<_>>()),
        verified_decision_count: verified_decisions,
        verified_override_count: verified_overrides,
        verified_override_rate: ratio(verified_overrides, verified_decisions).unwrap_or(0.0),
        verified_candidate_evaluation_count: verified_evaluations,
        verified_prefilter_candidate_evaluation_count: prefilter_evaluations,
        verified_final_candidate_evaluation_count: final_evaluations,
        verified_prefilter_policy_step_eval_count: prefilter_policy_steps,
        verified_final_policy_step_eval_count: final_policy_steps,
        verified_prefilter_decision_count: prefilter_decisions,
        verified_prefilter_kept_decision_count: prefilter_kept_decisions,
        verified_prefilter_kept_candidate_count: prefilter_kept,
        verified_prefilter_kept_rate: ratio(prefilter_kept_decisions, prefilter_decisions),
        verified_prefilter_average_kept_candidate_count: ratio(prefilter_kept, prefilter_decisions),
        verified_proposer_decision_count: proposer_decisions,
        verified_proposer_non_rule_candidate_count: proposer_non_rule,
        verified_proposer_kept_candidate_count: proposer_kept,
        verified_proposer_keep_rate: ratio(proposer_kept, proposer_non_rule),
        verified_average_scoped_candidate_count: if verified_decisions == 0 {
            0.0
        } else {
            scoped_candidate_count_sum / verified_decisions as f32
        },
        verified_adv_mean_on_overrides: ratio_f32(verified_adv_weighted_sum, verified_overrides),
        verified_harmful_override_count: verified_harmful,
        verified_harmful_override_rate: ratio(verified_harmful, verified_overrides),
        verified_low_evidence_reject_count: rows
            .iter()
            .map(|row| row.stats.verified_low_evidence_reject_count)
            .sum(),
        verified_confirm_decision_count: confirm_decisions,
        verified_confirm_accept_count: confirm_accepts,
        verified_confirm_reject_count: confirm_rejects,
        verified_confirm_candidate_evaluation_count: confirm_evaluations,
        verified_confirm_policy_step_eval_count: confirm_policy_steps,
        verified_artifact_confirm_decision_count: artifact_confirm_decisions,
        verified_artifact_confirm_accept_count: artifact_confirm_accepts,
        verified_artifact_confirm_reject_count: artifact_confirm_rejects,
        verified_artifact_confirm_candidate_evaluation_count: artifact_confirm_evaluations,
        verified_artifact_confirm_policy_step_eval_count: artifact_confirm_policy_steps,
        verified_decision_type_counts: decision_type_counts,
        verified_override_decision_type_counts: override_decision_type_counts,
        verified_decision_context_counts: decision_context_counts,
        verified_override_context_counts: override_context_counts,
        verified_best_adv_bucket_counts: best_adv_bucket_counts,
        verified_horizon_stop_reason_counts: horizon_stop_reason_counts,
        verified_payoff_reason_counts: payoff_reason_counts,
        verified_override_payoff_reason_counts: override_payoff_reason_counts,
        verified_horizon_artifact_reason_counts: horizon_artifact_reason_counts,
        verified_missing_counts: missing_counts,
        verified_cached_root_candidate_count: cached_root_candidates,
        verified_cached_root_exact_dedup_count: cached_root_dedup,
        verified_cached_root_exact_dedup_rate: ratio(cached_root_dedup, cached_root_candidates),
        verified_root_rule_equivalent_prune_count: root_rule_equiv_prunes,
        verified_root_rule_equivalent_prune_rate: ratio(
            root_rule_equiv_prunes,
            cached_root_candidates,
        ),
        verified_cached_value_hit_count: cached_value_hits,
        verified_cached_value_miss_count: cached_value_misses,
        verified_cached_value_hit_rate: ratio(
            cached_value_hits,
            cached_value_hits + cached_value_misses,
        ),
        verified_cached_policy_step_eval_count: cached_policy_steps,
        verified_cached_cache_entry_count_max: cached_entry_count_max,
        verified_parallelism_used_max: parallelism_used_max,
        verified_candidate_eval_wall_ms: candidate_eval_wall_ms,
    }
}

fn merge_counts(target: &mut BTreeMap<String, usize>, source: &BTreeMap<String, usize>) {
    for (key, value) in source {
        *target.entry(key.clone()).or_insert(0) += value;
    }
}

fn mean_f32(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn stderr_f32(values: &[f32]) -> Option<f32> {
    if values.len() <= 1 {
        return None;
    }
    let mean = mean_f32(values);
    let variance = values
        .iter()
        .map(|value| {
            let diff = *value - mean;
            diff * diff
        })
        .sum::<f32>()
        / (values.len() - 1) as f32;
    Some(variance.sqrt() / (values.len() as f32).sqrt())
}

fn ratio(numerator: usize, denominator: usize) -> Option<f32> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f32 / denominator as f32)
    }
}

fn ratio_f32(numerator: f32, denominator: usize) -> Option<f32> {
    if denominator == 0 {
        None
    } else {
        Some(numerator / denominator as f32)
    }
}

fn evaluate_one_candidate_independent(
    env: &FullRunEnv,
    state_before: &FullRunEnvState,
    action_index: usize,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    output: EvaluationOutputOptions,
    stats: &mut EvaluationStats,
) -> CandidateEvaluation {
    match evaluate_root_candidate(env, state_before, action_index, output) {
        Ok(root) => finish_independent_root(
            action_index,
            root,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            output.include_continuation_trace,
            stats,
        ),
        Err(value) => value,
    }
}
