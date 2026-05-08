use serde_json::{json, Value};

use crate::verification::decision_env::{
    ActionCandidate, ActionId, DecisionEnv, DecisionEnvError, DecisionId, EnvConfig,
    ObservationPayload, ObservationVisibility, RewardEvent, RunSeed, StepInfo, TimeStep,
    DECISION_ENV_CONTRACT_VERSION, REWARD_EVENT_SCHEMA_VERSION,
};

use super::{
    FullRunEnv, FullRunEnvConfig, FullRunEnvInfo, FullRunEnvState,
    FullRunPublicActionCandidatePayloadV1, FullRunPublicObservationV1, RewardShapingProfile,
    RunActionCandidate, TraceClientInput, FULL_RUN_PUBLIC_ACTION_SCHEMA_VERSION,
    FULL_RUN_PUBLIC_OBSERVATION_SCHEMA_VERSION,
};

impl DecisionEnv for FullRunEnv {
    type Snapshot = FullRunEnv;

    fn reset(&mut self, seed: RunSeed, config: EnvConfig) -> Result<TimeStep, DecisionEnvError> {
        let config = full_run_config_from_env(seed, config)?;
        *self = FullRunEnv::new(config).map_err(DecisionEnvError::new)?;
        let state = self.state().map_err(DecisionEnvError::new)?;
        timestep_from_state(self, state, 0.0, None)
    }

    fn current_timestep(&mut self) -> Result<TimeStep, DecisionEnvError> {
        let state = self.state().map_err(DecisionEnvError::new)?;
        timestep_from_state(self, state, 0.0, None)
    }

    fn step(&mut self, action: ActionId) -> Result<TimeStep, DecisionEnvError> {
        let step = FullRunEnv::step(self, action.0).map_err(DecisionEnvError::new)?;
        timestep_from_state(self, step.state, step.reward, step.chosen_action_key)
    }

    fn snapshot(&self) -> Result<Self::Snapshot, DecisionEnvError> {
        Ok(self.clone())
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), DecisionEnvError> {
        *self = snapshot.clone();
        Ok(())
    }
}

fn full_run_config_from_env(
    seed: RunSeed,
    mut config: EnvConfig,
) -> Result<FullRunEnvConfig, DecisionEnvError> {
    config.seed = seed.0;
    let player_class = match config.player_class.to_ascii_lowercase().as_str() {
        "" | "ironclad" => "ironclad",
        other => {
            return Err(DecisionEnvError::new(format!(
                "FullRunEnv DecisionEnv adapter supports player_class=ironclad, got {other}"
            )))
        }
    };
    let reward_shaping_profile = RewardShapingProfile::parse(&config.reward_shaping_profile)
        .map_err(DecisionEnvError::new)?;

    Ok(FullRunEnvConfig {
        seed: config.seed,
        ascension: config.ascension,
        final_act: config.final_act,
        player_class,
        max_steps: config.max_steps,
        reward_shaping_profile,
    })
}

fn timestep_from_state(
    env: &FullRunEnv,
    state: FullRunEnvState,
    scalar_reward: f32,
    chosen_action_key: Option<String>,
) -> Result<TimeStep, DecisionEnvError> {
    let info = env.info();
    let state_hash = format!("{:016x}", env.cache_bucket_hint());
    let terminated = is_natural_terminal(&info);
    let truncated = is_truncated_terminal(&info);
    let decision_type = state.observation.decision_type.clone();
    let candidates = state
        .action_candidates
        .iter()
        .map(action_candidate_from_full_run)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TimeStep {
        contract_version: DECISION_ENV_CONTRACT_VERSION.to_string(),
        decision_id: DecisionId {
            episode_id: format!("seed:{}", info.seed),
            step_index: info.step,
            decision_type: decision_type.clone(),
        },
        observation: ObservationPayload {
            schema_version: FULL_RUN_PUBLIC_OBSERVATION_SCHEMA_VERSION.to_string(),
            visibility: ObservationVisibility::Public,
            decision_type,
            payload: serde_json::to_value(FullRunPublicObservationV1::from_observation(
                &state.observation,
                &state.observation_schema_version,
            ))
            .map_err(|err| {
                DecisionEnvError::new(format!(
                    "serialize full-run public observation failed: {err}"
                ))
            })?,
        },
        candidates,
        reward: RewardEvent {
            schema_version: REWARD_EVENT_SCHEMA_VERSION.to_string(),
            scalar_reward,
            components: reward_components(&info, chosen_action_key.as_deref()),
        },
        terminated,
        truncated,
        info: StepInfo {
            state_hash,
            payload: timestep_info_payload(&state, &info, chosen_action_key),
        },
    })
}

fn action_candidate_from_full_run(
    candidate: &RunActionCandidate,
) -> Result<ActionCandidate, DecisionEnvError> {
    Ok(ActionCandidate {
        id: ActionId(candidate.action_index),
        action_schema_version: FULL_RUN_PUBLIC_ACTION_SCHEMA_VERSION.to_string(),
        action_index: candidate.action_index,
        action_key: candidate.action_key.clone(),
        action_kind: action_kind(&candidate.action).to_string(),
        payload: serde_json::to_value(
            FullRunPublicActionCandidatePayloadV1::from_candidate(candidate).map_err(|err| {
                DecisionEnvError::new(format!(
                    "serialize full-run public action candidate failed: {err}"
                ))
            })?,
        )
        .map_err(|err| {
            DecisionEnvError::new(format!(
                "serialize full-run public action candidate payload failed: {err}"
            ))
        })?,
    })
}

fn action_kind(action: &TraceClientInput) -> &'static str {
    match action {
        TraceClientInput::PlayCard { .. } => "play_card",
        TraceClientInput::UsePotion { .. } => "use_potion",
        TraceClientInput::DiscardPotion { .. } => "discard_potion",
        TraceClientInput::EndTurn => "end_turn",
        TraceClientInput::SubmitCardChoice { .. } => "card_choice",
        TraceClientInput::SubmitDiscoverChoice { .. } => "discover_choice",
        TraceClientInput::SelectMapNode { .. } | TraceClientInput::FlyToNode { .. } => "map",
        TraceClientInput::SelectEventOption { .. } | TraceClientInput::EventChoice { .. } => {
            "event"
        }
        TraceClientInput::CampfireOption { .. } => "campfire",
        TraceClientInput::SubmitScryDiscard { .. } => "scry",
        TraceClientInput::SubmitSelection { .. }
        | TraceClientInput::SubmitHandSelect { .. }
        | TraceClientInput::SubmitGridSelect { .. }
        | TraceClientInput::SubmitDeckSelect { .. } => "selection",
        TraceClientInput::ClaimReward { .. } => "claim_reward",
        TraceClientInput::SelectCard { .. } => "select_card",
        TraceClientInput::BuyCard { .. } => "buy_card",
        TraceClientInput::BuyRelic { .. } => "buy_relic",
        TraceClientInput::BuyPotion { .. } => "buy_potion",
        TraceClientInput::PurgeCard { .. } => "purge_card",
        TraceClientInput::SubmitRelicChoice { .. } => "relic_choice",
        TraceClientInput::Proceed => "proceed",
        TraceClientInput::Cancel => "cancel",
    }
}

fn is_natural_terminal(info: &FullRunEnvInfo) -> bool {
    matches!(info.result.as_str(), "victory" | "defeat")
}

fn is_truncated_terminal(info: &FullRunEnvInfo) -> bool {
    matches!(info.result.as_str(), "truncated" | "crash")
        || matches!(
            info.terminal_reason.as_str(),
            "step_cap" | "no_progress_loop" | "script_error"
        )
}

fn reward_components(info: &FullRunEnvInfo, chosen_action_key: Option<&str>) -> Value {
    json!({
        "chosen_action_key": chosen_action_key,
        "terminal_reason": info.terminal_reason,
        "result": info.result,
        "floor": info.floor,
        "act": info.act,
        "hp": info.hp,
        "max_hp": info.max_hp,
        "combat_win_count": info.combat_win_count,
    })
}

fn timestep_info_payload(
    state: &FullRunEnvState,
    info: &FullRunEnvInfo,
    chosen_action_key: Option<String>,
) -> Value {
    json!({
        "env_info": info,
        "chosen_action_key": chosen_action_key,
        "observation_schema_version": state.observation_schema_version,
        "action_schema_version": state.action_schema_version,
        "action_mask_kind": state.action_mask_kind,
        "legal_action_count": state.legal_action_count,
    })
}
