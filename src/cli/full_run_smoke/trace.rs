use super::*;

pub fn probe_combat_plan_from_trace(
    config: &FullRunTracePlanProbeConfig,
) -> Result<crate::bot::combat::CombatTurnPlanProbeReport, String> {
    let raw = std::fs::read_to_string(&config.trace_file).map_err(|err| {
        format!(
            "failed to read trace file '{}': {err}",
            config.trace_file.display()
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
    let ascension = config.ascension.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("ascension"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u8
    });
    let final_act = config.final_act.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("final_act"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });
    let player_class = config.player_class.clone().unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("player_class"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Ironclad")
            .to_string()
    });
    let max_steps = config.max_steps.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("max_steps"))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| config.step_index.saturating_add(128).max(512))
    });
    let steps = trace
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "trace missing steps[]".to_string())?;
    if config.step_index >= steps.len() {
        return Err(format!(
            "step-index {} out of range for trace with {} step(s)",
            config.step_index,
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

    for (step_idx, step) in steps.iter().take(config.step_index).enumerate() {
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
                config.step_index
            ));
        }
    }

    prepare_decision_point(&mut ctx, max_steps)?;
    let Some(combat) = ctx.combat_state.as_ref() else {
        return Err(format!(
            "trace step {} replayed to non-combat state {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    };
    if !matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return Err(format!(
            "trace step {} is not a combat turn frontier: {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    }

    let target_trace_step = &steps[config.step_index];
    let mut report =
        crate::bot::combat::probe_turn_plans(&ctx.engine_state, combat, config.probe_config);
    report.source_trace = serde_json::json!({
        "trace_file": config.trace_file.display().to_string(),
        "step_index": config.step_index,
        "seed": seed,
        "ascension": ascension,
        "final_act": final_act,
        "player_class": player_class,
        "trace_observation_schema_version": trace.get("observation_schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "trace_action_schema_version": trace.get("action_schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_decision_type": target_trace_step.get("decision_type").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_engine_state": target_trace_step.get("engine_state").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_chosen_action_key": target_trace_step.get("chosen_action_key").cloned().unwrap_or(serde_json::Value::Null),
    });
    Ok(report)
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

