use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};
use sts_simulator::bot::combat::{load_fixture_path, DecisionAuditEngineState};
use sts_simulator::bot::harness::{
    ActionMask, CombatAction, CombatEnv, CombatEnvSpec, CombatEpisodeOutcome, CombatObservation,
    CombatRewardBreakdown,
};
use sts_simulator::diff::replay::{
    derive_combat_replay_view, find_combat_step_index_by_before_frame_id,
    load_live_session_replay_path, reconstruct_combat_replay_step,
};
use sts_simulator::diff::state_sync::build_combat_state_from_snapshots;
use sts_simulator::fixtures::author_spec::{compile_combat_author_spec, CombatAuthorSpec};
use sts_simulator::fixtures::combat_start_spec::CombatStartSpec;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    initial_author_spec: Option<PathBuf>,
    #[arg(long)]
    initial_start_spec: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    initial_seed_hint: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum DriverRequest {
    Ping,
    Reset {
        author_spec: Option<PathBuf>,
        start_spec: Option<PathBuf>,
        fixture: Option<PathBuf>,
        replay_raw: Option<PathBuf>,
        replay_frame: Option<u64>,
        seed_hint: Option<u64>,
    },
    Observation,
    Step {
        action_index: usize,
    },
    Close,
}

#[derive(Debug, Serialize)]
struct DriverActionCandidate {
    index: usize,
    legal: bool,
    label: String,
    action_family: String,
    choice_kind: Option<String>,
    card_id: Option<String>,
    card_name: Option<String>,
    potion_id: Option<String>,
    potion_name: Option<String>,
    slot_index: Option<usize>,
    target: Option<usize>,
    target_slot: Option<usize>,
    selection_indices: Vec<usize>,
    selection_uuids: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct DriverStatePayload {
    observation: CombatObservation,
    action_candidates: Vec<DriverActionCandidate>,
    action_mask: Vec<bool>,
    legal_action_count: usize,
}

#[derive(Debug, Serialize)]
struct DriverResponse {
    ok: bool,
    error: Option<String>,
    payload: Option<DriverStatePayload>,
    reward: Option<f32>,
    reward_breakdown: Option<CombatRewardBreakdown>,
    done: Option<bool>,
    outcome: Option<String>,
    chosen_action_label: Option<String>,
    spec_name: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut env = match (args.initial_author_spec, args.initial_start_spec) {
        (Some(_), Some(_)) => {
            return Err("initial_author_spec and initial_start_spec are mutually exclusive".into())
        }
        (Some(path), None) => Some(load_env_from_author_spec(path, args.initial_seed_hint)?),
        (None, Some(path)) => Some(load_env_from_start_spec(path, args.initial_seed_hint)?),
        (None, None) => None,
    };

    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Result<DriverRequest, _> = serde_json::from_str(&line);
        let should_close = matches!(request.as_ref(), Ok(DriverRequest::Close));
        let response = match request {
            Ok(request) => handle_request(&mut env, request),
            Err(err) => DriverResponse {
                ok: false,
                error: Some(format!("invalid request: {err}")),
                payload: None,
                reward: None,
                reward_breakdown: None,
                done: None,
                outcome: None,
                chosen_action_label: None,
                spec_name: None,
            },
        };
        serde_json::to_writer(&mut stdout, &response)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        if should_close {
            break;
        }
    }
    Ok(())
}

fn handle_request(env: &mut Option<CombatEnv>, request: DriverRequest) -> DriverResponse {
    match request {
        DriverRequest::Ping => DriverResponse {
            ok: true,
            error: None,
            payload: None,
            reward: None,
            reward_breakdown: None,
            done: None,
            outcome: None,
            chosen_action_label: None,
            spec_name: None,
        },
        DriverRequest::Close => DriverResponse {
            ok: true,
            error: None,
            payload: None,
            reward: None,
            reward_breakdown: None,
            done: None,
            outcome: None,
            chosen_action_label: None,
            spec_name: env.as_ref().map(|current| current.observation().env_name),
        },
        DriverRequest::Reset {
            author_spec,
            start_spec,
            fixture,
            replay_raw,
            replay_frame,
            seed_hint,
        } => match reset_env(
            env,
            author_spec,
            start_spec,
            fixture,
            replay_raw,
            replay_frame,
            seed_hint.unwrap_or(0),
        ) {
            Ok(response) => response,
            Err(err) => error_response(err),
        },
        DriverRequest::Observation => match env.as_ref() {
            Some(current_env) => state_response(current_env, None, None, None, None),
            None => {
                error_response(
                    "combat env not initialized; send reset with author_spec, start_spec, fixture, or replay_raw/replay_frame"
                        .into(),
                )
            }
        },
        DriverRequest::Step { action_index } => {
            let Some(current_env) = env.as_mut() else {
                return error_response(
                    "combat env not initialized; send reset with author_spec, start_spec, fixture, or replay_raw/replay_frame"
                        .into(),
                );
            };
            let mask = current_env.action_mask();
            if action_index >= mask.candidate_actions.len() {
                return error_response(format!(
                    "action index {action_index} out of range for {} candidates",
                    mask.candidate_actions.len()
                ));
            }
            if !mask.legal[action_index] {
                return error_response(format!("action index {action_index} is currently illegal"));
            }
            let action = mask.candidate_actions[action_index].clone();
            match current_env.step(action) {
                Ok(step) => state_response(
                    current_env,
                    Some(step.reward),
                    Some(step.reward_breakdown),
                    Some(step.done),
                    Some(match step.outcome {
                        Some(CombatEpisodeOutcome::Victory) => "victory".to_string(),
                        Some(CombatEpisodeOutcome::Defeat) => "defeat".to_string(),
                        None => "ongoing".to_string(),
                    }),
                )
                .with_action_label(step.chosen_action_label),
                Err(err) => error_response(err),
            }
        }
    }
}

fn reset_env(
    env: &mut Option<CombatEnv>,
    author_spec: Option<PathBuf>,
    start_spec: Option<PathBuf>,
    fixture: Option<PathBuf>,
    replay_raw: Option<PathBuf>,
    replay_frame: Option<u64>,
    seed_hint: u64,
) -> Result<DriverResponse, String> {
    let requested_sources = [
        author_spec.as_ref().map(|_| "author_spec"),
        start_spec.as_ref().map(|_| "start_spec"),
        fixture.as_ref().map(|_| "fixture"),
        replay_raw.as_ref().map(|_| "replay"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if requested_sources.len() > 1 {
        return Err(format!(
            "reset accepts at most one source selector, got: {}",
            requested_sources.join(", ")
        ));
    }
    if replay_raw.is_some() ^ replay_frame.is_some() {
        return Err(
            "replay reset requires both replay_raw and replay_frame to be provided".to_string(),
        );
    }

    let requested_spec = if let Some(spec_path) = author_spec {
        Some(load_spec_from_author_spec(spec_path, seed_hint).map_err(|err| err.to_string())?)
    } else if let Some(spec_path) = start_spec {
        Some(load_spec_from_start_spec(spec_path, seed_hint)?)
    } else if let Some(fixture_path) = fixture {
        Some(load_spec_from_fixture_path(fixture_path, seed_hint)?)
    } else if let (Some(raw_path), Some(frame)) = (replay_raw, replay_frame) {
        Some(load_spec_from_replay_frame(raw_path, frame, seed_hint)?)
    } else {
        None
    };

    match (env.as_mut(), requested_spec) {
        (Some(current_env), None) => {
            current_env.reset(None);
            Ok(state_response(
                current_env,
                Some(0.0),
                Some(CombatRewardBreakdown::default()),
                Some(false),
                Some("ongoing".to_string()),
            ))
        }
        (Some(current_env), Some(spec)) => {
            current_env.reset(Some(spec));
            Ok(state_response(
                current_env,
                Some(0.0),
                Some(CombatRewardBreakdown::default()),
                Some(false),
                Some("ongoing".to_string()),
            ))
        }
        (None, Some(spec)) => {
            let new_env = CombatEnv::new(spec);
            *env = Some(new_env);
            Ok(state_response(
                env.as_ref().expect("env just initialized"),
                Some(0.0),
                Some(CombatRewardBreakdown::default()),
                Some(false),
                Some("ongoing".to_string()),
            ))
        }
        (None, None) => Err(
            "combat env not initialized; send reset with author_spec, start_spec, fixture, or replay_raw/replay_frame"
                .into(),
        ),
    }
}

fn error_response(error: String) -> DriverResponse {
    DriverResponse {
        ok: false,
        error: Some(error),
        payload: None,
        reward: None,
        reward_breakdown: None,
        done: None,
        outcome: None,
        chosen_action_label: None,
        spec_name: None,
    }
}

fn state_response(
    env: &CombatEnv,
    reward: Option<f32>,
    reward_breakdown: Option<CombatRewardBreakdown>,
    done: Option<bool>,
    outcome: Option<String>,
) -> DriverResponse {
    let observation = env.observation();
    let action_mask = env.action_mask();
    let spec_name = Some(observation.env_name.clone());
    let action_candidates = build_action_candidates(
        &action_mask,
        env.current_engine_state(),
        &observation,
        env.current_combat(),
    );
    let payload = DriverStatePayload {
        observation,
        legal_action_count: action_mask.legal.iter().filter(|value| **value).count(),
        action_candidates,
        action_mask: action_mask.legal.clone(),
    };
    DriverResponse {
        ok: true,
        error: None,
        payload: Some(payload),
        reward,
        reward_breakdown,
        done,
        outcome,
        chosen_action_label: None,
        spec_name,
    }
}

fn load_env_from_author_spec(
    path: PathBuf,
    seed_hint: u64,
) -> Result<CombatEnv, Box<dyn std::error::Error>> {
    let spec = load_spec_from_author_spec(path, seed_hint)?;
    Ok(CombatEnv::new(spec))
}

fn load_env_from_start_spec(
    path: PathBuf,
    seed_hint: u64,
) -> Result<CombatEnv, Box<dyn std::error::Error>> {
    let spec = load_spec_from_start_spec(path, seed_hint)?;
    Ok(CombatEnv::new(spec))
}

fn load_spec_from_author_spec(
    path: PathBuf,
    seed_hint: u64,
) -> Result<CombatEnvSpec, Box<dyn std::error::Error>> {
    let spec_payload = std::fs::read_to_string(&path)?;
    let spec: CombatAuthorSpec = serde_json::from_str(&spec_payload)?;
    let fixture = compile_combat_author_spec(&spec)?;
    Ok(CombatEnvSpec::from_fixture(&fixture, seed_hint))
}

fn load_spec_from_start_spec(path: PathBuf, seed_hint: u64) -> Result<CombatEnvSpec, String> {
    let payload = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to read start_spec {}: {err}", path.display()))?;
    let spec: CombatStartSpec = serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse start_spec {}: {err}", path.display()))?;
    let effective_seed = if seed_hint == 0 { spec.seed } else { seed_hint };
    CombatEnvSpec::from_start_spec_with_seed(&spec, effective_seed)
}

fn load_spec_from_fixture_path(path: PathBuf, seed_hint: u64) -> Result<CombatEnvSpec, String> {
    let fixture = load_fixture_path(&path)?;
    let combat = build_combat_state_from_snapshots(
        &fixture.truth_snapshot,
        &fixture.observation_snapshot,
        &fixture.relics,
    );
    let engine = match fixture.engine_state {
        DecisionAuditEngineState::CombatPlayerTurn => {
            sts_simulator::state::core::EngineState::CombatPlayerTurn
        }
    };
    Ok(CombatEnvSpec::from_combat(
        fixture.name,
        seed_hint,
        engine,
        combat,
    ))
}

fn load_spec_from_replay_frame(
    raw_path: PathBuf,
    frame: u64,
    seed_hint: u64,
) -> Result<CombatEnvSpec, String> {
    let replay = load_live_session_replay_path(&raw_path)?;
    let view = derive_combat_replay_view(&replay);
    let step_index = find_combat_step_index_by_before_frame_id(&view, frame)
        .ok_or_else(|| format!("no executable combat step found for before frame_id={frame}"))?;
    let reconstructed = reconstruct_combat_replay_step(&view, step_index)?;
    let replay_name = raw_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("replay_frame");
    Ok(CombatEnvSpec::from_combat(
        format!("{replay_name}::frame_{frame}"),
        seed_hint,
        reconstructed.before_engine,
        reconstructed.before_combat,
    ))
}

fn build_action_candidates(
    mask: &ActionMask,
    engine_state: &sts_simulator::state::core::EngineState,
    observation: &CombatObservation,
    combat: &sts_simulator::runtime::combat::CombatState,
) -> Vec<DriverActionCandidate> {
    let pending_choice_kind = observation.pending_choice_kind.clone();
    mask.candidate_actions
        .iter()
        .enumerate()
        .map(|(index, action)| match action {
            CombatAction::EndTurn => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "end_turn".to_string(),
                choice_kind: None,
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: None,
                target: None,
                target_slot: None,
                selection_indices: Vec::new(),
                selection_uuids: Vec::new(),
            },
            CombatAction::PlayCard { card_index, target } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "play_card".to_string(),
                choice_kind: None,
                card_id: combat
                    .zones
                    .hand
                    .get(*card_index)
                    .map(|card| format!("{:?}", card.id)),
                card_name: combat.zones.hand.get(*card_index).map(|card| {
                    let mut label = sts_simulator::content::cards::get_card_definition(card.id)
                        .name
                        .to_string();
                    for _ in 0..card.upgrades {
                        label.push('+');
                    }
                    label
                }),
                potion_id: None,
                potion_name: None,
                slot_index: Some(*card_index),
                target: *target,
                target_slot: (*target).and_then(|entity_id| {
                    observation
                        .monsters
                        .iter()
                        .position(|monster| monster.entity_id == entity_id)
                }),
                selection_indices: Vec::new(),
                selection_uuids: Vec::new(),
            },
            CombatAction::UsePotion {
                potion_index,
                target,
            } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "use_potion".to_string(),
                choice_kind: None,
                card_id: None,
                card_name: None,
                potion_id: combat
                    .entities
                    .potions
                    .get(*potion_index)
                    .and_then(|potion| potion.as_ref())
                    .map(|potion| format!("{:?}", potion.id)),
                potion_name: combat
                    .entities
                    .potions
                    .get(*potion_index)
                    .and_then(|potion| potion.as_ref())
                    .map(|potion| {
                        sts_simulator::content::potions::get_potion_definition(potion.id)
                            .name
                            .to_string()
                    }),
                slot_index: Some(*potion_index),
                target: *target,
                target_slot: (*target).and_then(|entity_id| {
                    observation
                        .monsters
                        .iter()
                        .position(|monster| monster.entity_id == entity_id)
                }),
                selection_indices: Vec::new(),
                selection_uuids: Vec::new(),
            },
            CombatAction::SubmitDiscoverChoice {
                index: choice_index,
            } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: choice_family_name(engine_state, pending_choice_kind.as_deref()),
                choice_kind: pending_choice_kind.clone(),
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: Some(*choice_index),
                target: None,
                target_slot: None,
                selection_indices: vec![*choice_index],
                selection_uuids: Vec::new(),
            },
            CombatAction::SubmitCardChoice { indices } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "card_select".to_string(),
                choice_kind: pending_choice_kind.clone(),
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: indices.first().copied(),
                target: None,
                target_slot: None,
                selection_indices: indices.clone(),
                selection_uuids: Vec::new(),
            },
            CombatAction::SubmitHandSelect { uuids } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "hand_select".to_string(),
                choice_kind: pending_choice_kind.clone(),
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: None,
                target: None,
                target_slot: None,
                selection_indices: Vec::new(),
                selection_uuids: uuids.clone(),
            },
            CombatAction::SubmitGridSelect { uuids } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "grid_select".to_string(),
                choice_kind: pending_choice_kind.clone(),
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: None,
                target: None,
                target_slot: None,
                selection_indices: Vec::new(),
                selection_uuids: uuids.clone(),
            },
            CombatAction::Proceed => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "proceed".to_string(),
                choice_kind: None,
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: None,
                target: None,
                target_slot: None,
                selection_indices: Vec::new(),
                selection_uuids: Vec::new(),
            },
            CombatAction::Cancel => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "cancel".to_string(),
                choice_kind: pending_choice_kind.clone(),
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: None,
                target: None,
                target_slot: None,
                selection_indices: Vec::new(),
                selection_uuids: Vec::new(),
            },
            CombatAction::Raw { .. } => DriverActionCandidate {
                index,
                legal: mask.legal[index],
                label: action.label(combat),
                action_family: "raw".to_string(),
                choice_kind: None,
                card_id: None,
                card_name: None,
                potion_id: None,
                potion_name: None,
                slot_index: None,
                target: None,
                target_slot: None,
                selection_indices: Vec::new(),
                selection_uuids: Vec::new(),
            },
        })
        .collect()
}

fn choice_family_name(
    _engine_state: &sts_simulator::state::core::EngineState,
    pending_choice_kind: Option<&str>,
) -> String {
    match pending_choice_kind {
        Some("card_reward_select") => "card_reward_select",
        Some("stance_choice") => "stance_choice",
        Some("scry_select") => "scry_select",
        Some("discovery_select") => "discovery_select",
        _ => "discover_select",
    }
    .to_string()
}

trait DriverResponseExt {
    fn with_action_label(self, label: String) -> Self;
}

impl DriverResponseExt for DriverResponse {
    fn with_action_label(mut self, label: String) -> Self {
        self.chosen_action_label = Some(label);
        self
    }
}
