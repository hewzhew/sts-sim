use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoProgressSignature {
    pub(crate) observation_key: String,
    pub(crate) action_mask_key: String,
    pub(crate) chosen_action_key: String,
}

#[derive(Clone, Debug)]
pub struct NoProgressTracker {
    pub(crate) last: Option<NoProgressSignature>,
    pub(crate) repeat_count: usize,
    pub(crate) start_step: usize,
}

impl NoProgressTracker {
    pub fn new() -> Self {
        Self {
            last: None,
            repeat_count: 0,
            start_step: 0,
        }
    }

    pub fn observe(
        &mut self,
        step_index: usize,
        signature: NoProgressSignature,
        observation: &RunObservationV0,
    ) -> Option<RunNoProgressLoop> {
        if self.last.as_ref() == Some(&signature) {
            self.repeat_count += 1;
        } else {
            self.last = Some(signature.clone());
            self.repeat_count = 1;
            self.start_step = step_index;
        }

        if self.repeat_count >= NO_PROGRESS_REPEAT_LIMIT {
            Some(RunNoProgressLoop {
                start_step: self.start_step,
                end_step: step_index,
                repeat_count: self.repeat_count,
                action_key: signature.chosen_action_key,
                decision_type: observation.decision_type.clone(),
                engine_state: observation.engine_state.clone(),
                floor: observation.floor,
                act: observation.act,
            })
        } else {
            None
        }
    }
}

pub fn make_contract_failure(
    config: &RunBatchConfig,
    episode_id: usize,
    seed: u64,
    kind: &str,
    terminal_reason: &str,
    floor: i32,
    act: u8,
    step: Option<usize>,
    action_key: Option<String>,
    decision_type: Option<String>,
    engine_state: Option<String>,
    details: String,
) -> RunContractFailure {
    RunContractFailure {
        kind: kind.to_string(),
        episode_id,
        seed,
        policy: config.policy.as_str().to_string(),
        step,
        action_key,
        decision_type,
        engine_state,
        floor,
        act,
        terminal_reason: terminal_reason.to_string(),
        details,
        trace_path: None,
        reproduce_command: reproduce_command(config, seed),
    }
}

pub fn reproduce_command(config: &RunBatchConfig, seed: u64) -> String {
    let mut parts = vec![
        "cargo".to_string(),
        "run".to_string(),
        "--release".to_string(),
        "--bin".to_string(),
        "sts_dev_tool".to_string(),
        "--".to_string(),
        "run-batch".to_string(),
        "--episodes".to_string(),
        "1".to_string(),
        "--seed".to_string(),
        seed.to_string(),
        "--policy".to_string(),
        config.policy.as_str().to_string(),
        "--ascension".to_string(),
        config.ascension.to_string(),
        "--class".to_string(),
        cli_class_arg(config.player_class).to_string(),
        "--max-steps".to_string(),
        config.max_steps.to_string(),
        "--reward-shaping-profile".to_string(),
        config.reward_shaping_profile.as_str().to_string(),
        "--determinism-check".to_string(),
        "--summary-out".to_string(),
        format!(
            "tools\\artifacts\\full_run_smoke\\repro_{}_seed_{}.json",
            config.policy.as_str(),
            seed
        ),
        "--trace-dir".to_string(),
        format!(
            "tools\\artifacts\\full_run_smoke\\repro_{}_seed_{}_trace",
            config.policy.as_str(),
            seed
        ),
    ];
    if config.final_act {
        parts.push("--final-act".to_string());
    }
    parts.join(" ")
}

pub fn cli_class_arg(player_class: &str) -> &'static str {
    match player_class {
        "Ironclad" => "ironclad",
        "Silent" => "silent",
        "Defect" => "defect",
        "Watcher" => "watcher",
        _ => "ironclad",
    }
}

pub fn run_batch(config: &RunBatchConfig) -> Result<RunBatchSummary, String> {
    if config.episodes == 0 {
        return Err("episodes must be greater than 0".to_string());
    }
    if config.max_steps == 0 {
        return Err("max_steps must be greater than 0".to_string());
    }
    if let Some(trace_dir) = &config.trace_dir {
        std::fs::create_dir_all(trace_dir).map_err(|err| {
            format!(
                "failed to create trace dir '{}': {err}",
                trace_dir.display()
            )
        })?;
    }

    let batch_start = Instant::now();
    let mut episodes = Vec::new();
    let mut crash_count = 0usize;
    let mut illegal_action_count = 0usize;
    let mut no_progress_loop_count = 0usize;
    let mut deterministic_replay_pass_count = 0usize;

    for episode_id in 0..config.episodes {
        let seed = config.base_seed.wrapping_add(episode_id as u64);
        let policy_seed = seed ^ 0x9e37_79b9_7f4a_7c15;
        let episode_policy = match config.policy {
            RunPolicyKind::RandomMasked => EpisodePolicy::RandomMasked {
                rng: StsRng::new(policy_seed),
            },
            RunPolicyKind::RuleBaselineV0 => EpisodePolicy::RuleBaselineV0,
            RunPolicyKind::PlanQueryV0 => EpisodePolicy::PlanQueryV0,
        };
        let mut episode = run_episode(config, episode_id, seed, episode_policy, true);

        if config.determinism_check {
            let replay = run_episode(
                config,
                episode_id,
                seed,
                EpisodePolicy::Replay {
                    actions: episode.actions.clone(),
                    cursor: 0,
                },
                false,
            );
            let replay_error = deterministic_replay_error(&episode.summary, &replay.summary);
            let passed = replay_error.is_none();
            episode.summary.deterministic_replay_pass = Some(passed);
            episode.summary.deterministic_replay_error = replay_error;
            if passed {
                deterministic_replay_pass_count += 1;
            } else if episode.summary.contract_failure.is_none() {
                let details = episode
                    .summary
                    .deterministic_replay_error
                    .clone()
                    .unwrap_or_else(|| "deterministic replay mismatch".to_string());
                episode.summary.contract_failure = Some(make_contract_failure(
                    config,
                    episode_id,
                    seed,
                    "deterministic_replay_mismatch",
                    "deterministic_replay_mismatch",
                    episode.summary.floor,
                    episode.summary.act,
                    None,
                    None,
                    None,
                    None,
                    details,
                ));
            }
        }

        if let Some(trace_dir) = &config.trace_dir {
            let trace_path = trace_dir.join(format!("episode_{episode_id:04}_seed_{seed}.json"));
            episode.summary.trace_path = Some(trace_path.display().to_string());
            if let Some(failure) = &mut episode.summary.contract_failure {
                failure.trace_path = episode.summary.trace_path.clone();
            }
            write_trace_file(&trace_path, config, &episode.summary, &episode.trace)?;
        }

        if episode.summary.crash.is_some() {
            crash_count += 1;
        }
        if episode.summary.no_progress_loop.is_some() {
            no_progress_loop_count += 1;
        }
        illegal_action_count += episode.summary.illegal_actions;
        episodes.push(episode.summary);
    }

    let elapsed = batch_start.elapsed().as_secs_f32().max(0.001);
    let total_steps = episodes.iter().map(|episode| episode.steps).sum::<usize>();
    let episodes_completed = episodes
        .iter()
        .filter(|episode| episode.crash.is_none())
        .count();
    let mut floors = episodes
        .iter()
        .map(|episode| episode.floor)
        .collect::<Vec<_>>();
    floors.sort_unstable();
    let average_floor = if floors.is_empty() {
        0.0
    } else {
        floors.iter().sum::<i32>() as f32 / floors.len() as f32
    };
    let median_floor = median_i32(&floors);
    let average_steps = total_steps as f32 / episodes.len().max(1) as f32;
    let average_total_reward = episodes
        .iter()
        .map(|episode| episode.total_reward)
        .sum::<f32>()
        / episodes.len().max(1) as f32;
    let average_combat_wins = episodes
        .iter()
        .map(|episode| episode.combat_win_count)
        .sum::<usize>() as f32
        / episodes.len().max(1) as f32;
    let legal_action_count_sum = episodes
        .iter()
        .map(|episode| episode.average_legal_action_count * episode.steps as f32)
        .sum::<f32>();
    let average_legal_action_count = legal_action_count_sum / total_steps.max(1) as f32;
    let max_legal_action_count = episodes
        .iter()
        .map(|episode| episode.max_legal_action_count)
        .max()
        .unwrap_or(0);
    let result_counts = count_by(episodes.iter().map(|episode| episode.result.clone()));
    let death_floor_counts = count_by(
        episodes
            .iter()
            .filter(|episode| episode.result == "defeat")
            .map(|episode| episode.floor.to_string()),
    );
    let act_counts = count_by(episodes.iter().map(|episode| episode.act.to_string()));
    let mut decision_type_counts = std::collections::BTreeMap::new();
    for episode in &episodes {
        for (decision_type, count) in &episode.decision_type_counts {
            *decision_type_counts
                .entry(decision_type.clone())
                .or_insert(0) += *count;
        }
    }
    let contract_failures = episodes
        .iter()
        .filter_map(|episode| episode.contract_failure.clone())
        .collect::<Vec<_>>();
    let contract_failure_count = contract_failures.len();

    Ok(RunBatchSummary {
        observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
        action_mask_kind: "per_decision_candidate_set".to_string(),
        policy: config.policy.as_str().to_string(),
        episodes_requested: config.episodes,
        base_seed: config.base_seed,
        ascension: config.ascension,
        final_act: config.final_act,
        player_class: config.player_class.to_string(),
        max_steps: config.max_steps,
        reward_shaping_profile: config.reward_shaping_profile.as_str().to_string(),
        episodes_completed,
        crash_count,
        illegal_action_count,
        no_progress_loop_count,
        deterministic_replay_pass_count,
        contract_failure_count,
        average_floor,
        median_floor,
        average_steps,
        average_total_reward,
        average_combat_wins,
        average_legal_action_count,
        max_legal_action_count,
        steps_per_second: total_steps as f32 / elapsed,
        episodes_per_hour: episodes.len() as f32 / elapsed * 3600.0,
        result_counts,
        death_floor_counts,
        act_counts,
        decision_type_counts,
        contract_failures,
        episodes,
    })
}

pub fn run_episode(
    config: &RunBatchConfig,
    episode_id: usize,
    seed: u64,
    policy: EpisodePolicy,
    capture_trace: bool,
) -> EpisodeRun {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_episode_inner(config, episode_id, seed, policy, capture_trace)
    }));
    match result {
        Ok(run) => run,
        Err(payload) => {
            let crash = if let Some(value) = payload.downcast_ref::<&str>() {
                (*value).to_string()
            } else if let Some(value) = payload.downcast_ref::<String>() {
                value.clone()
            } else {
                "panic without string payload".to_string()
            };
            let contract_failure = make_contract_failure(
                config,
                episode_id,
                seed,
                "panic",
                "panic",
                0,
                1,
                None,
                None,
                None,
                None,
                crash.clone(),
            );
            EpisodeRun {
                summary: RunEpisodeSummary {
                    episode_id,
                    seed,
                    result: "crash".to_string(),
                    terminal_reason: "panic".to_string(),
                    floor: 0,
                    act: 1,
                    steps: 0,
                    forced_engine_ticks: 0,
                    illegal_actions: 0,
                    no_progress_loop: None,
                    crash: Some(crash),
                    deterministic_replay_pass: None,
                    deterministic_replay_error: None,
                    contract_failure: Some(contract_failure),
                    duration_ms: 0,
                    total_reward: -100.0,
                    combat_win_count: 0,
                    decision_type_counts: std::collections::BTreeMap::new(),
                    average_legal_action_count: 0.0,
                    max_legal_action_count: 0,
                    hp: 0,
                    max_hp: 0,
                    gold: 0,
                    deck_size: 0,
                    relic_count: 0,
                    trace_path: None,
                },
                trace: Vec::new(),
                actions: Vec::new(),
            }
        }
    }
}

pub fn run_episode_inner(
    config: &RunBatchConfig,
    episode_id: usize,
    seed: u64,
    mut policy: EpisodePolicy,
    capture_trace: bool,
) -> EpisodeRun {
    let start = Instant::now();
    let mut ctx = EpisodeContext {
        engine_state: EngineState::EventRoom,
        run_state: RunState::new(
            seed,
            config.ascension,
            config.final_act,
            config.player_class,
        ),
        combat_state: None,
        stashed_event_combat: None,
        forced_engine_ticks: 0,
        combat_win_count: 0,
    };
    let mut trace = Vec::new();
    let mut actions = Vec::new();
    let mut illegal_actions = 0usize;
    let mut no_progress_loop = None;
    let mut crash = None;
    let mut contract_failure = None;
    let mut terminal_reason = "step_cap".to_string();
    let mut decision_type_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut legal_action_count_sum = 0usize;
    let mut max_legal_action_count = 0usize;
    let mut no_progress_tracker = NoProgressTracker::new();

    for step_index in 0..config.max_steps {
        if let Err(err) = prepare_decision_point(&mut ctx, config.max_steps) {
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "engine_error",
                "engine_error",
                ctx.run_state.floor_num,
                ctx.run_state.act_num,
                Some(step_index),
                None,
                Some(decision_type(&ctx.engine_state).to_string()),
                Some(engine_state_label(&ctx.engine_state).to_string()),
                err.clone(),
            ));
            crash = Some(err);
            terminal_reason = "engine_error".to_string();
            break;
        }

        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_reason = "game_over".to_string();
            break;
        }

        let legal_actions = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        if legal_actions.is_empty() {
            let details = format!(
                "no legal actions at {} on floor {}",
                engine_state_label(&ctx.engine_state),
                ctx.run_state.floor_num
            );
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "no_legal_actions",
                "no_legal_actions",
                ctx.run_state.floor_num,
                ctx.run_state.act_num,
                Some(step_index),
                None,
                Some(decision_type(&ctx.engine_state).to_string()),
                Some(engine_state_label(&ctx.engine_state).to_string()),
                details.clone(),
            ));
            crash = Some(details);
            terminal_reason = "no_legal_actions".to_string();
            break;
        }
        let current_decision_type = decision_type(&ctx.engine_state).to_string();
        *decision_type_counts
            .entry(current_decision_type.clone())
            .or_insert(0) += 1;
        legal_action_count_sum += legal_actions.len();
        max_legal_action_count = max_legal_action_count.max(legal_actions.len());

        let (chosen_action_index, action) = match choose_action(&mut policy, &ctx, &legal_actions) {
            Ok(action) => action,
            Err(err) => {
                illegal_actions += 1;
                contract_failure = Some(make_contract_failure(
                    config,
                    episode_id,
                    seed,
                    "illegal_replay_action",
                    "illegal_replay_action",
                    ctx.run_state.floor_num,
                    ctx.run_state.act_num,
                    Some(step_index),
                    None,
                    Some(current_decision_type.clone()),
                    Some(engine_state_label(&ctx.engine_state).to_string()),
                    err.clone(),
                ));
                crash = Some(err);
                terminal_reason = "illegal_replay_action".to_string();
                break;
            }
        };

        let observation = build_observation(&ctx);
        let action_mask = build_action_candidates(&legal_actions, Some(&ctx));
        let chosen = action_mask
            .get(chosen_action_index)
            .expect("chosen action index should be in legal action mask");
        let chosen_action_id = chosen.action_id;
        let chosen_action_key = chosen.action_key.clone();
        let signature =
            no_progress_signature(&observation, &action_mask, chosen_action_key.clone());

        if capture_trace {
            trace.push(RunStepTrace {
                step_index,
                floor: ctx.run_state.floor_num,
                act: ctx.run_state.act_num,
                engine_state: engine_state_label(&ctx.engine_state).to_string(),
                decision_type: current_decision_type.clone(),
                hp: ctx.run_state.current_hp,
                max_hp: ctx.run_state.max_hp,
                gold: ctx.run_state.gold,
                deck_size: ctx.run_state.master_deck.len(),
                relic_count: ctx.run_state.relics.len(),
                legal_action_count: legal_actions.len(),
                observation: observation.clone(),
                action_mask: action_mask.clone(),
                chosen_action_index,
                chosen_action_id,
                chosen_action_key: chosen_action_key.clone(),
                chosen_action: trace_input_from_client_input(&action),
            });
        }
        if let Some(loop_info) = no_progress_tracker.observe(step_index, signature, &observation) {
            let details = format!(
                "no progress loop: action {} repeated {} times from step {} to {} at {} floor {}",
                loop_info.action_key,
                loop_info.repeat_count,
                loop_info.start_step,
                loop_info.end_step,
                loop_info.decision_type,
                loop_info.floor
            );
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "no_progress_loop",
                "no_progress_loop",
                loop_info.floor,
                loop_info.act,
                Some(loop_info.end_step),
                Some(loop_info.action_key.clone()),
                Some(loop_info.decision_type.clone()),
                Some(loop_info.engine_state.clone()),
                details.clone(),
            ));
            crash = Some(details);
            terminal_reason = "no_progress_loop".to_string();
            no_progress_loop = Some(loop_info);
            break;
        }
        actions.push(action.clone());
        let executed_action_key = action_key_for_input(&action, ctx.combat_state.as_ref());

        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            Some(action),
        );
        if let Some(errors) = take_engine_error_diagnostics(&mut ctx) {
            illegal_actions += 1;
            let details = format!(
                "engine rejected legal action {executed_action_key}: {}",
                errors.join("; ")
            );
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "engine_rejected_action",
                "engine_rejected_action",
                ctx.run_state.floor_num,
                ctx.run_state.act_num,
                Some(step_index),
                Some(executed_action_key),
                Some(current_decision_type),
                Some(observation.engine_state.clone()),
                details.clone(),
            ));
            crash = Some(details);
            terminal_reason = "engine_rejected_action".to_string();
            break;
        }
        finish_combat_if_needed(&mut ctx);
        if !keep_running {
            terminal_reason = "engine_stopped".to_string();
            break;
        }
    }

    if crash.is_none() {
        let _ = prepare_decision_point(&mut ctx, config.max_steps);
        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_reason = "game_over".to_string();
        }
    }

    let result = match &ctx.engine_state {
        EngineState::GameOver(RunResult::Victory) => "victory",
        EngineState::GameOver(RunResult::Defeat) => "defeat",
        _ if crash.is_some() => "crash",
        _ => "step_cap",
    }
    .to_string();
    let average_legal_action_count = legal_action_count_sum as f32 / actions.len().max(1) as f32;
    let total_reward = episode_reward(
        &result,
        ctx.run_state.floor_num,
        ctx.combat_win_count,
        ctx.run_state.current_hp,
        ctx.run_state.max_hp,
    );

    EpisodeRun {
        summary: RunEpisodeSummary {
            episode_id,
            seed,
            result,
            terminal_reason,
            floor: ctx.run_state.floor_num,
            act: ctx.run_state.act_num,
            steps: actions.len(),
            forced_engine_ticks: ctx.forced_engine_ticks,
            illegal_actions,
            no_progress_loop,
            crash,
            deterministic_replay_pass: None,
            deterministic_replay_error: None,
            contract_failure,
            duration_ms: start.elapsed().as_millis(),
            total_reward,
            combat_win_count: ctx.combat_win_count,
            decision_type_counts,
            average_legal_action_count,
            max_legal_action_count,
            hp: ctx.run_state.current_hp,
            max_hp: ctx.run_state.max_hp,
            gold: ctx.run_state.gold,
            deck_size: ctx.run_state.master_deck.len(),
            relic_count: ctx.run_state.relics.len(),
            trace_path: None,
        },
        trace,
        actions,
    }
}

pub fn prepare_decision_point(ctx: &mut EpisodeContext, max_steps: usize) -> Result<(), String> {
    let forced_cap = max_steps.saturating_mul(10).max(1_000);
    let mut local_ticks = 0usize;
    loop {
        init_combat_if_needed(ctx)?;
        reconcile_terminal_combat_player_turn(ctx);
        finish_combat_if_needed(ctx);

        if !matches!(ctx.engine_state, EngineState::CombatProcessing) {
            return Ok(());
        }

        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            None,
        );
        ctx.forced_engine_ticks += 1;
        local_ticks += 1;
        reconcile_terminal_combat_player_turn(ctx);
        finish_combat_if_needed(ctx);
        if !keep_running || matches!(ctx.engine_state, EngineState::GameOver(_)) {
            return Ok(());
        }
        if local_ticks > forced_cap {
            return Err(format!(
                "forced engine ticks exceeded cap at floor {} state {}",
                ctx.run_state.floor_num,
                engine_state_label(&ctx.engine_state)
            ));
        }
    }
}

pub fn init_combat_if_needed(ctx: &mut EpisodeContext) -> Result<(), String> {
    if matches!(ctx.engine_state, EngineState::CombatPlayerTurn) && ctx.combat_state.is_none() {
        ctx.combat_state = Some(init_combat(&mut ctx.run_state));
        ctx.engine_state = EngineState::CombatProcessing;
        return Ok(());
    }

    if let EngineState::EventCombat(event_combat) = ctx.engine_state.clone() {
        if ctx.combat_state.is_none() {
            let encounter_id =
                encounter_key_to_id(event_combat.encounter_key).ok_or_else(|| {
                    format!("unknown event combat key '{}'", event_combat.encounter_key)
                })?;
            ctx.stashed_event_combat = Some(event_combat);
            ctx.combat_state = Some(init_event_combat(&mut ctx.run_state, encounter_id));
            ctx.engine_state = EngineState::CombatProcessing;
        }
    }

    Ok(())
}

pub fn reconcile_terminal_combat_player_turn(ctx: &mut EpisodeContext) {
    if !matches!(ctx.engine_state, EngineState::CombatPlayerTurn) {
        return;
    }
    let Some(combat) = ctx.combat_state.as_ref() else {
        return;
    };
    if combat_is_waiting_for_victory_settlement(combat) {
        ctx.engine_state = EngineState::CombatProcessing;
    }
}

pub fn combat_is_waiting_for_victory_settlement(combat: &CombatState) -> bool {
    !combat.entities.monsters.is_empty()
        && !combat.has_pending_actions()
        && combat.zones.queued_cards.is_empty()
        && combat
            .entities
            .monsters
            .iter()
            .all(|monster| monster_is_defeated_for_victory_settlement(combat, monster))
}

pub fn monster_is_defeated_for_victory_settlement(
    combat: &CombatState,
    monster: &crate::runtime::combat::MonsterEntity,
) -> bool {
    if monster.is_escaped {
        return true;
    }
    if monster.half_dead {
        return false;
    }
    if monster.current_hp > 0 && !monster.is_dying {
        return false;
    }
    !crate::content::powers::store::powers_for(combat, monster.id).is_some_and(|powers| {
        powers.iter().any(|power| {
            matches!(
                power.power_type,
                crate::content::powers::PowerId::Regrow
                    | crate::content::powers::PowerId::Unawakened
            )
        })
    })
}

pub fn finish_combat_if_needed(ctx: &mut EpisodeContext) {
    if matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn
            | EngineState::CombatProcessing
            | EngineState::PendingChoice(_)
            | EngineState::EventCombat(_)
    ) {
        return;
    }

    if ctx.combat_state.is_none() {
        return;
    }
    let survived_combat = !matches!(ctx.engine_state, EngineState::GameOver(_));
    ctx.combat_state = None;
    if survived_combat {
        ctx.combat_win_count += 1;
    }

    let Some(event_combat) = ctx.stashed_event_combat.take() else {
        return;
    };
    if matches!(ctx.engine_state, EngineState::GameOver(_)) {
        return;
    }
    if event_combat.reward_allowed {
        let mut rewards = event_combat.rewards;
        if !event_combat.no_cards_in_rewards {
            if let EngineState::RewardScreen(existing) = &ctx.engine_state {
                for item in &existing.items {
                    if matches!(item, RewardItem::Card { .. }) {
                        rewards.items.push(item.clone());
                    }
                }
            }
        }
        ctx.engine_state = EngineState::RewardScreen(rewards);
    } else {
        ctx.engine_state = match event_combat.post_combat_return {
            PostCombatReturn::EventRoom => EngineState::EventRoom,
            PostCombatReturn::MapNavigation => EngineState::MapNavigation,
        };
    }
}

pub fn take_engine_error_diagnostics(ctx: &mut EpisodeContext) -> Option<Vec<String>> {
    let combat = ctx.combat_state.as_mut()?;
    let diagnostics = combat.take_engine_diagnostics();
    let errors = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == EngineDiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>();
    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

