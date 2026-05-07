// Mechanical split from main.rs for candidate evaluation and policy preview helpers.

fn evaluate_candidates(
    env: &mut FullRunEnv,
    episode_cache: &mut ValueCache,
    action_indices: Vec<usize>,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    runtime: EvaluationRuntimeOptions,
    output: EvaluationOutputOptions,
) -> Result<CandidateEvaluationPayload, String> {
    let gamma = if gamma.is_finite() {
        gamma.clamp(0.0, 1.0)
    } else {
        return Err("gamma must be finite".to_string());
    };
    let state_before = env.state()?;
    let before_value = if output.check_live_env_unchanged {
        Some(
            serde_json::to_value(&state_before)
                .map_err(|err| format!("state serialize failed: {err}"))?,
        )
    } else {
        None
    };
    let indices = if action_indices.is_empty() {
        (0..state_before.action_candidates.len()).collect::<Vec<_>>()
    } else {
        action_indices
    };
    let mut stats = EvaluationStats {
        root_candidate_count: indices.len(),
        ..EvaluationStats::default()
    };
    let mut request_cache = ValueCache::default();
    let eval_started = Instant::now();

    let evaluations = match runtime.mode {
        EvaluationMode::Independent => evaluate_candidates_independent(
            env,
            &state_before,
            indices,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            output,
            runtime.parallelism,
            runtime.exact_root_dedup,
            &mut stats,
        ),
        EvaluationMode::BellmanCachedV1 => {
            if horizon_mode != HorizonMode::FixedDecisions {
                return Err(
                    "bellman_cached_v1 currently supports only fixed_decisions horizon_mode"
                        .to_string(),
                );
            }
            stats.parallelism_used = 1;
            let cache = match runtime.cache_scope {
                ValueCacheScope::Request => &mut request_cache,
                ValueCacheScope::Episode => episode_cache,
            };
            evaluate_candidates_bellman_cached(
                env,
                &state_before,
                indices,
                continuation_policy,
                horizon_decisions,
                horizon_mode,
                gamma,
                runtime.cache_max_entries,
                output,
                cache,
                &mut stats,
            )
        }
    };
    let candidate_eval_wall_ms = eval_started.elapsed().as_millis() as u64;

    let need_state_after = output.include_state || output.check_live_env_unchanged;
    let state_after = if need_state_after {
        Some(env.state()?)
    } else {
        None
    };
    let after_value = match (&state_after, output.check_live_env_unchanged) {
        (Some(state_after), true) => Some(
            serde_json::to_value(state_after)
                .map_err(|err| format!("state serialize failed: {err}"))?,
        ),
        _ => None,
    };
    let live_env_unchanged = match (&before_value, &after_value) {
        (Some(before), Some(after)) => Some(before == after),
        _ => None,
    };
    Ok(CandidateEvaluationPayload {
        schema_version: "return_q_candidate_evaluation_v0".to_string(),
        continuation_policy: continuation_policy.as_str().to_string(),
        horizon_decisions,
        horizon_mode: horizon_mode.as_str().to_string(),
        gamma,
        evaluation_mode: runtime.mode.as_str().to_string(),
        value_cache_scope: runtime.cache_scope.as_str().to_string(),
        root_candidate_count: stats.root_candidate_count,
        root_exact_dedup_count: stats.root_exact_dedup_count,
        root_rule_equivalent_prune_count: stats.root_rule_equivalent_prune_count,
        value_cache_hit_count: stats.value_cache_hit_count,
        value_cache_miss_count: stats.value_cache_miss_count,
        policy_step_eval_count: stats.policy_step_eval_count,
        cache_entry_count: match runtime.cache_scope {
            ValueCacheScope::Request => request_cache.entry_count,
            ValueCacheScope::Episode => episode_cache.entry_count,
        },
        parallelism_requested: runtime.parallelism,
        parallelism_used: stats.parallelism_used,
        candidate_eval_wall_ms,
        live_env_unchanged,
        state_before: output.include_state.then_some(state_before),
        state_after: if output.include_state {
            state_after
        } else {
            None
        },
        evaluations,
    })
}

fn preview_policy_action(
    env: &mut FullRunEnv,
    policy: RunPolicyKind,
    output: PreviewOutputOptions,
) -> Result<PolicyPreviewPayload, String> {
    if !output.include_state && !output.include_next_state && !output.check_live_env_unchanged {
        let (chosen_action_index, chosen_action_key) = env.preview_policy_action_index(policy)?;
        let info = env.info();
        return Ok(PolicyPreviewPayload {
            schema_version: "return_q_policy_preview_v0".to_string(),
            policy: policy.as_str().to_string(),
            live_env_unchanged: None,
            state_before: None,
            state_after: None,
            chosen_action_index,
            chosen_action_key,
            reward: 0.0,
            done: info.result != "ongoing",
            next_state: None,
            info,
        });
    }

    let state_before = env.state()?;
    let before_value = if output.check_live_env_unchanged {
        Some(
            serde_json::to_value(&state_before)
                .map_err(|err| format!("state serialize failed: {err}"))?,
        )
    } else {
        None
    };
    let mut trial = env.clone();
    let step = trial.step_policy(policy)?;
    let chosen_action_index = step.chosen_action_key.as_ref().and_then(|key| {
        state_before
            .action_candidates
            .iter()
            .position(|candidate| candidate.action_key == *key)
    });
    let state_after = env.state()?;
    let after_value = if output.check_live_env_unchanged {
        Some(
            serde_json::to_value(&state_after)
                .map_err(|err| format!("state serialize failed: {err}"))?,
        )
    } else {
        None
    };
    let live_env_unchanged = match (&before_value, &after_value) {
        (Some(before), Some(after)) => Some(before == after),
        _ => None,
    };
    Ok(PolicyPreviewPayload {
        schema_version: "return_q_policy_preview_v0".to_string(),
        policy: policy.as_str().to_string(),
        live_env_unchanged,
        state_before: output.include_state.then_some(state_before),
        state_after: output.include_state.then_some(state_after),
        chosen_action_index,
        chosen_action_key: step.chosen_action_key,
        reward: step.reward,
        done: step.done,
        next_state: output.include_next_state.then_some(step.state),
        info: step.info,
    })
}

include!("verified_override_impl.rs");

fn evaluate_candidates_independent(
    env: &FullRunEnv,
    state_before: &FullRunEnvState,
    action_indices: Vec<usize>,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    output: EvaluationOutputOptions,
    parallelism_requested: usize,
    exact_root_dedup: bool,
    stats: &mut EvaluationStats,
) -> Vec<CandidateEvaluation> {
    if action_indices.is_empty() {
        stats.parallelism_used = 1;
        return Vec::new();
    }
    if exact_root_dedup {
        return evaluate_candidates_independent_with_root_dedup(
            env,
            state_before,
            action_indices,
            continuation_policy,
            horizon_decisions,
            horizon_mode,
            gamma,
            output,
            parallelism_requested,
            stats,
        );
    }

    let effective_parallelism = effective_parallelism(parallelism_requested, action_indices.len());
    stats.parallelism_used = effective_parallelism;
    if effective_parallelism <= 1 || action_indices.len() <= 1 {
        return action_indices
            .into_iter()
            .map(|action_index| {
                evaluate_one_candidate_independent(
                    env,
                    state_before,
                    action_index,
                    continuation_policy,
                    horizon_decisions,
                    horizon_mode,
                    gamma,
                    output,
                    stats,
                )
            })
            .collect::<Vec<_>>();
    }

    let indexed = action_indices.into_iter().enumerate().collect::<Vec<_>>();
    let chunk_size = indexed.len().div_ceil(effective_parallelism);
    let mut joined = Vec::new();
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in indexed.chunks(chunk_size) {
            let work = chunk.to_vec();
            let base_env = env.clone();
            let base_state = state_before.clone();
            handles.push(scope.spawn(move || {
                let mut local_stats = EvaluationStats::default();
                let mut local = Vec::with_capacity(work.len());
                for (position, action_index) in work {
                    let evaluation = evaluate_one_candidate_independent(
                        &base_env,
                        &base_state,
                        action_index,
                        continuation_policy,
                        horizon_decisions,
                        horizon_mode,
                        gamma,
                        output,
                        &mut local_stats,
                    );
                    local.push((position, evaluation));
                }
                (local, local_stats)
            }));
        }
        for handle in handles {
            let (local, local_stats) = handle
                .join()
                .expect("candidate evaluation worker should not panic");
            stats.merge(local_stats);
            joined.extend(local);
        }
    });
    joined.sort_by_key(|(position, _evaluation)| *position);
    joined
        .into_iter()
        .map(|(_position, evaluation)| evaluation)
        .collect()
}

fn evaluate_candidates_independent_with_root_dedup(
    env: &FullRunEnv,
    state_before: &FullRunEnvState,
    action_indices: Vec<usize>,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    output: EvaluationOutputOptions,
    parallelism_requested: usize,
    stats: &mut EvaluationStats,
) -> Vec<CandidateEvaluation> {
    enum RootEval {
        Failed(CandidateEvaluation),
        Success(RootCandidateSuccess),
    }

    let rule_index = preview_policy_index_from_env(env, continuation_policy);
    let rule_root =
        rule_index.and_then(|index| evaluate_root_candidate(env, state_before, index, output).ok());
    let mut roots = Vec::with_capacity(action_indices.len());
    for (position, action_index) in action_indices.into_iter().enumerate() {
        let root = match evaluate_root_candidate(env, state_before, action_index, output) {
            Ok(root) => {
                if let (Some(rule_index), Some(rule_root)) = (rule_index, &rule_root) {
                    if action_index != rule_index
                        && root.one_step_reward.to_bits() == rule_root.one_step_reward.to_bits()
                        && root.env_after == rule_root.env_after
                    {
                        stats.root_rule_equivalent_prune_count += 1;
                    }
                }
                RootEval::Success(root)
            }
            Err(value) => RootEval::Failed(value),
        };
        roots.push((position, action_index, root));
    }

    let mut unique_roots: Vec<(usize, FullRunEnv, PayoffHorizonProfile)> = Vec::new();
    let mut root_to_unique = vec![None; roots.len()];
    for (position, _action_index, root_eval) in &roots {
        let RootEval::Success(root) = root_eval else {
            continue;
        };
        if let Some((unique_index, _env)) = unique_roots.iter().enumerate().find(
            |(_unique_index, (_first_position, env_after, profile))| {
                env_after == &root.env_after && profile == &root.payoff_profile
            },
        ) {
            stats.root_exact_dedup_count += 1;
            root_to_unique[*position] = Some(unique_index);
        } else {
            let unique_index = unique_roots.len();
            unique_roots.push((
                *position,
                root.env_after.clone(),
                root.payoff_profile.clone(),
            ));
            root_to_unique[*position] = Some(unique_index);
        }
    }

    let effective_parallelism = effective_parallelism(parallelism_requested, unique_roots.len());
    stats.parallelism_used = effective_parallelism.max(1);
    let suffixes = evaluate_unique_suffixes_independent(
        unique_roots,
        continuation_policy,
        horizon_decisions,
        horizon_mode,
        gamma,
        output.include_continuation_trace,
        effective_parallelism,
        stats,
    );

    let mut evaluations = Vec::with_capacity(roots.len());
    for (position, action_index, root_eval) in roots {
        match root_eval {
            RootEval::Failed(value) => evaluations.push((position, value)),
            RootEval::Success(root) => {
                let Some(unique_index) = root_to_unique[position] else {
                    evaluations.push((
                        position,
                        failed_candidate_evaluation(
                            action_index,
                            root.candidate,
                            "missing exact root suffix group".to_string(),
                        ),
                    ));
                    continue;
                };
                match suffixes.get(unique_index) {
                    Some(Ok(suffix)) => evaluations.push((
                        position,
                        candidate_evaluation_from_root(action_index, root, suffix.clone(), gamma),
                    )),
                    Some(Err(err)) => evaluations.push((
                        position,
                        candidate_evaluation_from_root_error(action_index, root, err.clone()),
                    )),
                    None => evaluations.push((
                        position,
                        failed_candidate_evaluation(
                            action_index,
                            root.candidate,
                            "missing exact root suffix value".to_string(),
                        ),
                    )),
                }
            }
        }
    }
    evaluations.sort_by_key(|(position, _evaluation)| *position);
    evaluations
        .into_iter()
        .map(|(_position, evaluation)| evaluation)
        .collect()
}

fn preview_policy_index_from_env(env: &FullRunEnv, policy: RunPolicyKind) -> Option<usize> {
    let mut preview = env.clone();
    preview
        .preview_policy_action_index(policy)
        .ok()
        .and_then(|(index, _key)| index)
}

fn evaluate_unique_suffixes_independent(
    unique_roots: Vec<(usize, FullRunEnv, PayoffHorizonProfile)>,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    include_trace: bool,
    parallelism: usize,
    stats: &mut EvaluationStats,
) -> Vec<Result<SuffixValue, String>> {
    if unique_roots.is_empty() {
        return Vec::new();
    }
    let effective_parallelism = effective_parallelism(parallelism, unique_roots.len());
    if effective_parallelism <= 1 || unique_roots.len() <= 1 {
        return unique_roots
            .into_iter()
            .map(|(_position, env_after, payoff_profile)| {
                evaluate_suffix_rollout(
                    &env_after,
                    continuation_policy,
                    horizon_decisions,
                    horizon_mode,
                    &payoff_profile,
                    gamma,
                    include_trace,
                    stats,
                )
            })
            .collect();
    }

    let indexed = unique_roots.into_iter().enumerate().collect::<Vec<_>>();
    let chunk_size = indexed.len().div_ceil(effective_parallelism);
    let mut joined = Vec::new();
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in indexed.chunks(chunk_size) {
            let work = chunk.to_vec();
            handles.push(scope.spawn(move || {
                let mut local_stats = EvaluationStats::default();
                let mut local = Vec::with_capacity(work.len());
                for (unique_index, (_position, env_after, payoff_profile)) in work {
                    let suffix = evaluate_suffix_rollout(
                        &env_after,
                        continuation_policy,
                        horizon_decisions,
                        horizon_mode,
                        &payoff_profile,
                        gamma,
                        include_trace,
                        &mut local_stats,
                    );
                    local.push((unique_index, suffix));
                }
                (local, local_stats)
            }));
        }
        for handle in handles {
            let (local, local_stats) = handle
                .join()
                .expect("suffix evaluation worker should not panic");
            stats.merge(local_stats);
            joined.extend(local);
        }
    });
    joined.sort_by_key(|(unique_index, _suffix)| *unique_index);
    joined
        .into_iter()
        .map(|(_unique_index, suffix)| suffix)
        .collect()
}

fn finish_independent_root(
    action_index: usize,
    root: RootCandidateSuccess,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    include_trace: bool,
    stats: &mut EvaluationStats,
) -> CandidateEvaluation {
    match evaluate_suffix_rollout(
        &root.env_after,
        continuation_policy,
        horizon_decisions,
        horizon_mode,
        &root.payoff_profile,
        gamma,
        include_trace,
        stats,
    ) {
        Ok(suffix) => candidate_evaluation_from_root(action_index, root, suffix, gamma),
        Err(err) => candidate_evaluation_from_root_error(action_index, root, err),
    }
}

fn evaluate_suffix_rollout(
    env: &FullRunEnv,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    payoff_profile: &PayoffHorizonProfile,
    gamma: f32,
    include_trace: bool,
    stats: &mut EvaluationStats,
) -> Result<SuffixValue, String> {
    if horizon_decisions == 0 || env.info().result != "ongoing" {
        return Ok(base_suffix_value(env, include_trace));
    }
    let mut trial = env.clone();
    let start_state = trial.state()?;
    let start_turn = combat_turn_count(&start_state);
    let mut discounted_return = 0.0f32;
    let mut discount = 1.0f32;
    let mut continuation_steps = 0usize;
    let mut continuation_action_keys = Vec::new();
    let mut rollout_done = trial.info().result != "ongoing";
    let mut final_info = trial.info();
    let mut post_turn_normal_decisions = 0usize;
    let mut horizon_stop_reason = "horizon_decision_cap".to_string();

    while !rollout_done && continuation_steps < horizon_decisions {
        let step = trial.step_policy(continuation_policy)?;
        discounted_return += discount * step.reward;
        discount *= gamma;
        continuation_steps += 1;
        stats.policy_step_eval_count += 1;
        if include_trace {
            if let Some(key) = step.chosen_action_key {
                continuation_action_keys.push(key);
            }
        }
        rollout_done = step.done;
        final_info = step.info;
        if rollout_done {
            horizon_stop_reason = "terminal".to_string();
            break;
        }
        if let Some(reason) = should_stop_adaptive_suffix(
            horizon_mode,
            start_turn,
            continuation_steps,
            &step.state,
            payoff_profile,
            &mut post_turn_normal_decisions,
        ) {
            horizon_stop_reason = reason.to_string();
            break;
        }
    }

    Ok(SuffixValue {
        discounted_return,
        continuation_steps,
        continuation_action_keys,
        rollout_done,
        rollout_terminal_reason: final_info.terminal_reason.clone(),
        horizon_stop_reason,
        final_info,
    })
}

fn candidate_evaluation_from_root(
    action_index: usize,
    root: RootCandidateSuccess,
    suffix: SuffixValue,
    gamma: f32,
) -> CandidateEvaluation {
    CandidateEvaluation {
        action_index,
        candidate: root.candidate,
        ok: true,
        error: None,
        chosen_action_key: root.chosen_action_key,
        one_step_reward: root.one_step_reward,
        discounted_return: root.one_step_reward + gamma * suffix.discounted_return,
        next_state: root.next_state,
        done: root.done,
        terminal_reason: root.terminal_reason,
        continuation_steps: suffix.continuation_steps,
        continuation_action_keys: suffix.continuation_action_keys,
        rollout_done: if root.done { true } else { suffix.rollout_done },
        rollout_terminal_reason: suffix.rollout_terminal_reason,
        horizon_stop_reason: suffix.horizon_stop_reason,
        payoff_reasons: root
            .payoff_profile
            .reasons
            .iter()
            .map(|reason| (*reason).to_string())
            .collect(),
        final_info: Some(suffix.final_info),
    }
}

fn combat_turn_count(state: &FullRunEnvState) -> Option<u32> {
    state
        .observation
        .combat
        .as_ref()
        .map(|combat| combat.turn_count)
}

fn payoff_horizon_profile_for_root(
    state_before: &FullRunEnvState,
    state_after: &FullRunEnvState,
    candidate: Option<&RunActionCandidate>,
) -> PayoffHorizonProfile {
    let mut profile = PayoffHorizonProfile::default();
    let Some(before_combat) = &state_before.observation.combat else {
        return profile;
    };
    let Some(after_combat) = &state_after.observation.combat else {
        return profile;
    };

    if let Some(pending) = &after_combat.pending_choice {
        match pending.kind.as_str() {
            "grid_select" | "hand_select" | "discovery_select" | "scry_select" => {
                add_payoff_reason(&mut profile, "pending_card_choice", 0);
            }
            _ => {}
        }
        if pending.source_pile.as_deref() == Some("Draw") {
            add_payoff_reason(&mut profile, "draw_pile_order_choice", 1);
        }
    }

    if after_combat.hand_count > before_combat.hand_count {
        add_payoff_reason(&mut profile, "hand_access_changed", 0);
    }

    if let Some(candidate) = candidate {
        let key = candidate.action_key.as_str();
        if key.contains("card:Headbutt") {
            add_payoff_reason(&mut profile, "headbutt_topdeck_payoff", 2);
        }
        if key.contains("card:Warcry") {
            add_payoff_reason(&mut profile, "card_order_or_hand_choice_payoff", 1);
        }
        if key.contains("card:Armaments") {
            add_payoff_reason(&mut profile, "hand_upgrade_payoff", 0);
        }
        if key.contains("card:DemonForm") || key.contains("card:Barricade") {
            add_payoff_reason(&mut profile, "slow_power_payoff", 2);
        } else if key.contains("card:Combust") || key.contains("card:Metallicize") {
            add_payoff_reason(&mut profile, "end_of_turn_power_payoff", 1);
        }
        if let Some(card) = &candidate.card {
            if card.draws_cards || card.gains_energy {
                add_payoff_reason(&mut profile, "same_turn_resource_payoff", 0);
            }
            if card.applies_vulnerable || card.applies_weak {
                add_payoff_reason(&mut profile, "debuff_combat_window_payoff", 0);
            }
            if card.scaling_piece {
                add_payoff_reason(&mut profile, "setup_or_scaling_detected", 0);
            }
            if card.exhaust {
                add_payoff_reason(&mut profile, "self_exhaust_state_changed", 0);
            }
        }
    }

    profile
}

fn add_payoff_reason(
    profile: &mut PayoffHorizonProfile,
    reason: &'static str,
    post_turn_normal_decision_budget: usize,
) {
    if !profile.reasons.contains(&reason) {
        profile.reasons.push(reason);
    }
    profile.post_turn_normal_decision_budget = profile
        .post_turn_normal_decision_budget
        .max(post_turn_normal_decision_budget);
}

fn should_stop_adaptive_suffix(
    horizon_mode: HorizonMode,
    start_turn: Option<u32>,
    continuation_steps: usize,
    state: &FullRunEnvState,
    payoff_profile: &PayoffHorizonProfile,
    post_turn_normal_decisions: &mut usize,
) -> Option<&'static str> {
    if horizon_mode == HorizonMode::FixedDecisions || continuation_steps == 0 {
        return None;
    }
    let Some(start_turn) = start_turn else {
        return None;
    };
    let decision_type = state.observation.decision_type.as_str();
    if !decision_type.starts_with("combat") {
        return Some("left_combat_decision_space");
    }
    let is_later_normal_combat =
        decision_type == "combat" && combat_turn_count(state).is_some_and(|turn| turn > start_turn);
    if !is_later_normal_combat {
        return None;
    }
    match horizon_mode {
        HorizonMode::FixedDecisions => None,
        HorizonMode::AdaptiveNextPlayerTurnV1 => Some("next_player_turn"),
        HorizonMode::AdaptivePayoffWindowV1 => {
            *post_turn_normal_decisions += 1;
            if payoff_profile.post_turn_normal_decision_budget == 0 {
                return Some("next_player_turn_no_payoff_extension");
            }
            if *post_turn_normal_decisions > payoff_profile.post_turn_normal_decision_budget {
                Some("payoff_window_complete")
            } else {
                None
            }
        }
    }
}

fn candidate_evaluation_from_root_error(
    action_index: usize,
    root: RootCandidateSuccess,
    err: String,
) -> CandidateEvaluation {
    CandidateEvaluation {
        action_index,
        candidate: root.candidate,
        ok: false,
        error: Some(err),
        chosen_action_key: root.chosen_action_key,
        one_step_reward: root.one_step_reward,
        discounted_return: root.one_step_reward,
        next_state: root.next_state,
        done: root.done,
        terminal_reason: root.terminal_reason,
        continuation_steps: 0,
        continuation_action_keys: Vec::new(),
        rollout_done: true,
        rollout_terminal_reason: "engine_error".to_string(),
        horizon_stop_reason: "engine_error".to_string(),
        payoff_reasons: root
            .payoff_profile
            .reasons
            .iter()
            .map(|reason| (*reason).to_string())
            .collect(),
        final_info: Some(root.final_info),
    }
}

fn effective_parallelism(requested: usize, work_len: usize) -> usize {
    if work_len <= 1 {
        return 1;
    }
    let requested = if requested == 0 {
        thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1)
    } else {
        requested
    };
    requested.max(1).min(work_len)
}

struct RootCandidateSuccess {
    env_after: FullRunEnv,
    candidate: Option<RunActionCandidate>,
    chosen_action_key: Option<String>,
    one_step_reward: f32,
    next_state: Option<FullRunEnvState>,
    done: bool,
    terminal_reason: String,
    payoff_profile: PayoffHorizonProfile,
    final_info: FullRunEnvInfo,
}

struct SuffixPathStep {
    env_before: FullRunEnv,
    horizon_remaining: usize,
    reward: f32,
    chosen_action_key: Option<String>,
    done: bool,
}

fn evaluate_candidates_bellman_cached(
    env: &FullRunEnv,
    state_before: &FullRunEnvState,
    action_indices: Vec<usize>,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    cache_max_entries: usize,
    output: EvaluationOutputOptions,
    cache: &mut ValueCache,
    stats: &mut EvaluationStats,
) -> Vec<CandidateEvaluation> {
    let mut suffix_dedup: Vec<(FullRunEnv, SuffixValue)> = Vec::new();
    let mut evaluations = Vec::new();
    for action_index in action_indices {
        let root = match evaluate_root_candidate(env, state_before, action_index, output) {
            Ok(value) => value,
            Err(value) => {
                evaluations.push(value);
                continue;
            }
        };
        let suffix = if root.done || horizon_decisions == 0 {
            base_suffix_value(&root.env_after, output.include_continuation_trace)
        } else if let Some((_env, value)) = suffix_dedup
            .iter()
            .find(|(existing_env, _value)| existing_env == &root.env_after)
        {
            stats.root_exact_dedup_count += 1;
            value.clone()
        } else {
            match evaluate_suffix_value(
                &root.env_after,
                continuation_policy,
                horizon_decisions,
                horizon_mode,
                gamma,
                output.include_continuation_trace,
                cache_max_entries,
                cache,
                stats,
            ) {
                Ok(value) => {
                    suffix_dedup.push((root.env_after.clone(), value.clone()));
                    value
                }
                Err(err) => {
                    evaluations.push(CandidateEvaluation {
                        action_index,
                        candidate: root.candidate,
                        ok: false,
                        error: Some(err),
                        chosen_action_key: root.chosen_action_key,
                        one_step_reward: root.one_step_reward,
                        discounted_return: root.one_step_reward,
                        next_state: root.next_state,
                        done: root.done,
                        terminal_reason: root.terminal_reason,
                        continuation_steps: 0,
                        continuation_action_keys: Vec::new(),
                        rollout_done: true,
                        rollout_terminal_reason: "engine_error".to_string(),
                        horizon_stop_reason: "engine_error".to_string(),
                        payoff_reasons: root
                            .payoff_profile
                            .reasons
                            .iter()
                            .map(|reason| (*reason).to_string())
                            .collect(),
                        final_info: Some(root.final_info),
                    });
                    continue;
                }
            }
        };
        evaluations.push(CandidateEvaluation {
            action_index,
            candidate: root.candidate,
            ok: true,
            error: None,
            chosen_action_key: root.chosen_action_key,
            one_step_reward: root.one_step_reward,
            discounted_return: root.one_step_reward + gamma * suffix.discounted_return,
            next_state: root.next_state,
            done: root.done,
            terminal_reason: root.terminal_reason,
            continuation_steps: suffix.continuation_steps,
            continuation_action_keys: suffix.continuation_action_keys,
            rollout_done: if root.done { true } else { suffix.rollout_done },
            rollout_terminal_reason: suffix.rollout_terminal_reason,
            horizon_stop_reason: suffix.horizon_stop_reason,
            payoff_reasons: root
                .payoff_profile
                .reasons
                .iter()
                .map(|reason| (*reason).to_string())
                .collect(),
            final_info: Some(suffix.final_info),
        });
    }
    evaluations
}

fn evaluate_root_candidate(
    env: &FullRunEnv,
    state_before: &FullRunEnvState,
    action_index: usize,
    output: EvaluationOutputOptions,
) -> Result<RootCandidateSuccess, CandidateEvaluation> {
    let candidate = state_before.action_candidates.get(action_index).cloned();
    if action_index >= state_before.action_candidates.len() {
        return Err(failed_candidate_evaluation(
            action_index,
            candidate,
            format!(
                "action index {action_index} out of range for {} candidates",
                state_before.action_candidates.len()
            ),
        ));
    }
    if !state_before
        .action_mask
        .get(action_index)
        .copied()
        .unwrap_or(false)
    {
        return Err(failed_candidate_evaluation(
            action_index,
            candidate,
            format!("action index {action_index} is currently illegal"),
        ));
    }

    let mut trial = env.clone();
    let root_step = match trial.step(action_index) {
        Ok(step) => step,
        Err(err) => return Err(failed_candidate_evaluation(action_index, candidate, err)),
    };
    let payoff_profile =
        payoff_horizon_profile_for_root(state_before, &root_step.state, candidate.as_ref());
    let next_state = output.include_next_state.then_some(root_step.state.clone());
    Ok(RootCandidateSuccess {
        env_after: trial,
        candidate,
        chosen_action_key: root_step.chosen_action_key,
        one_step_reward: root_step.reward,
        next_state,
        done: root_step.done,
        terminal_reason: root_step.info.terminal_reason.clone(),
        payoff_profile,
        final_info: root_step.info,
    })
}

fn evaluate_suffix_value(
    env: &FullRunEnv,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    include_trace: bool,
    cache_max_entries: usize,
    cache: &mut ValueCache,
    stats: &mut EvaluationStats,
) -> Result<SuffixValue, String> {
    if horizon_decisions == 0 || env.info().result != "ongoing" {
        return Ok(base_suffix_value(env, include_trace));
    }
    let mut trial = env.clone();
    let mut remaining = horizon_decisions;
    let mut path = Vec::new();
    let mut tail_value = None;

    while remaining > 0 && trial.info().result == "ongoing" {
        if let Some(value) = cache.get(
            &trial,
            continuation_policy,
            remaining,
            horizon_mode,
            gamma,
            include_trace,
        ) {
            stats.value_cache_hit_count += 1;
            tail_value = Some(value);
            break;
        }
        stats.value_cache_miss_count += 1;
        let env_before = trial.clone();
        let step = trial.step_policy(continuation_policy)?;
        stats.policy_step_eval_count += 1;
        path.push(SuffixPathStep {
            env_before,
            horizon_remaining: remaining,
            reward: step.reward,
            chosen_action_key: step.chosen_action_key,
            done: step.done,
        });
        remaining = remaining.saturating_sub(1);
        if step.done {
            break;
        }
    }

    let mut value = tail_value.unwrap_or_else(|| base_suffix_value(&trial, include_trace));
    for step in path.into_iter().rev() {
        let mut continuation_action_keys = Vec::new();
        if include_trace {
            if let Some(key) = step.chosen_action_key {
                continuation_action_keys.push(key);
            }
            continuation_action_keys.extend(value.continuation_action_keys.clone());
        }
        value = SuffixValue {
            discounted_return: step.reward + gamma * value.discounted_return,
            continuation_steps: 1 + value.continuation_steps,
            continuation_action_keys,
            rollout_done: if step.done { true } else { value.rollout_done },
            rollout_terminal_reason: value.rollout_terminal_reason.clone(),
            horizon_stop_reason: value.horizon_stop_reason.clone(),
            final_info: value.final_info.clone(),
        };
        cache.insert(
            step.env_before,
            continuation_policy,
            step.horizon_remaining,
            horizon_mode,
            gamma,
            include_trace,
            value.clone(),
            cache_max_entries,
        );
    }
    Ok(value)
}

fn base_suffix_value(env: &FullRunEnv, _include_trace: bool) -> SuffixValue {
    let info = env.info();
    SuffixValue {
        discounted_return: 0.0,
        continuation_steps: 0,
        continuation_action_keys: Vec::new(),
        rollout_done: info.result != "ongoing",
        rollout_terminal_reason: info.terminal_reason.clone(),
        horizon_stop_reason: if info.result == "ongoing" {
            "horizon_zero_or_base".to_string()
        } else {
            "terminal".to_string()
        },
        final_info: info,
    }
}

fn cache_bucket(
    env: &FullRunEnv,
    continuation_policy: RunPolicyKind,
    horizon_decisions: usize,
    horizon_mode: HorizonMode,
    gamma: f32,
    include_trace: bool,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    continuation_policy.as_str().hash(&mut hasher);
    horizon_decisions.hash(&mut hasher);
    horizon_mode.hash(&mut hasher);
    gamma.to_bits().hash(&mut hasher);
    include_trace.hash(&mut hasher);
    env.cache_bucket_hint().hash(&mut hasher);
    hasher.finish()
}

fn failed_candidate_evaluation(
    action_index: usize,
    candidate: Option<RunActionCandidate>,
    error: String,
) -> CandidateEvaluation {
    CandidateEvaluation {
        action_index,
        candidate,
        ok: false,
        error: Some(error),
        chosen_action_key: None,
        one_step_reward: 0.0,
        discounted_return: 0.0,
        next_state: None,
        done: true,
        terminal_reason: "evaluation_error".to_string(),
        continuation_steps: 0,
        continuation_action_keys: Vec::new(),
        rollout_done: true,
        rollout_terminal_reason: "evaluation_error".to_string(),
        horizon_stop_reason: "evaluation_error".to_string(),
        payoff_reasons: Vec::new(),
        final_info: None,
    }
}
