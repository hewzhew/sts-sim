use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::cli::full_run_smoke::{
    FullRunEnv, FullRunEnvConfig, FullRunEnvInfo, FullRunEnvState,
};
use sts_simulator::verification::decision_env::{
    ActionId, DecisionEnv, DecisionRecord, DecisionRecordContext,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
enum DriverRequest {
    Ping,
    Reset {
        seed: Option<u64>,
        ascension: Option<u8>,
        final_act: Option<bool>,
        class: Option<String>,
        max_steps: Option<usize>,
    },
    Observation,
    DecisionEnvObservation,
    Step {
        action_index: usize,
    },
    DecisionEnvStep {
        action_id: usize,
    },
    DecisionRecordStep {
        action_id: usize,
        sim_version: Option<String>,
        return_spec_version: Option<String>,
        context: Option<Value>,
    },
    Close,
}

#[derive(Debug, Serialize)]
struct DriverResponse {
    ok: bool,
    error: Option<String>,
    payload: Option<Value>,
    reward: Option<f32>,
    done: Option<bool>,
    chosen_action_key: Option<String>,
    info: Option<FullRunEnvInfo>,
}

#[derive(Default)]
struct DriverSession {
    env: Option<FullRunEnv>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = DriverSession::default();
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request = serde_json::from_str::<DriverRequest>(&line);
        let should_close = matches!(request.as_ref(), Ok(DriverRequest::Close));
        let response = match request {
            Ok(request) => handle_request(&mut session, request),
            Err(err) => error_response(format!("invalid request: {err}")),
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

fn handle_request(session: &mut DriverSession, request: DriverRequest) -> DriverResponse {
    match request {
        DriverRequest::Ping => ok_response(None, None, None, None, None),
        DriverRequest::Close => ok_response(
            None,
            None,
            None,
            None,
            session.env.as_ref().map(FullRunEnv::info),
        ),
        DriverRequest::Reset {
            seed,
            ascension,
            final_act,
            class,
            max_steps,
        } => reset_env(
            session,
            seed,
            ascension,
            final_act,
            class.as_deref(),
            max_steps,
        ),
        DriverRequest::Observation => with_env(session, |current| match current.state() {
            Ok(state) => ok_response(
                Some(state_payload(state)),
                None,
                Some(current.info().result != "ongoing"),
                None,
                Some(current.info()),
            ),
            Err(err) => error_response(err),
        }),
        DriverRequest::DecisionEnvObservation => with_env(session, |current| {
            match DecisionEnv::current_timestep(current) {
                Ok(timestep) => ok_response(
                    Some(
                        serde_json::to_value(timestep)
                            .expect("decision env timestep should serialize"),
                    ),
                    None,
                    Some(current.info().result != "ongoing"),
                    None,
                    Some(current.info()),
                ),
                Err(err) => error_response(err.to_string()),
            }
        }),
        DriverRequest::Step { action_index } => {
            with_env(session, |current| match current.step(action_index) {
                Ok(step) => ok_response(
                    Some(state_payload(step.state)),
                    Some(step.reward),
                    Some(step.done),
                    step.chosen_action_key,
                    Some(step.info),
                ),
                Err(err) => error_response(err),
            })
        }
        DriverRequest::DecisionEnvStep { action_id } => with_env(session, |current| {
            match DecisionEnv::step(current, ActionId(action_id)) {
                Ok(timestep) => {
                    let reward = timestep.reward.scalar_reward;
                    let done = timestep.terminated || timestep.truncated;
                    let chosen_action_key = timestep
                        .reward
                        .components
                        .get("chosen_action_key")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    ok_response(
                        Some(
                            serde_json::to_value(timestep)
                                .expect("decision env timestep should serialize"),
                        ),
                        Some(reward),
                        Some(done),
                        chosen_action_key,
                        Some(current.info()),
                    )
                }
                Err(err) => error_response(err.to_string()),
            }
        }),
        DriverRequest::DecisionRecordStep {
            action_id,
            sim_version,
            return_spec_version,
            context,
        } => with_env(session, |current| {
            let seed = current.info().seed;
            let decision = match DecisionEnv::current_timestep(current) {
                Ok(timestep) => timestep,
                Err(err) => return error_response(err.to_string()),
            };
            let outcome = match DecisionEnv::step(current, ActionId(action_id)) {
                Ok(timestep) => timestep,
                Err(err) => return error_response(err.to_string()),
            };
            let mut record_context = DecisionRecordContext::new(
                sim_version.unwrap_or_else(|| "full_run_env".to_string()),
                return_spec_version.unwrap_or_else(|| "driver_reward_v0".to_string()),
                seed,
            );
            record_context.behavior_action = Some(ActionId(action_id));
            record_context.info =
                context.unwrap_or_else(|| json!({"source": "full_run_env_driver"}));
            let record =
                DecisionRecord::from_decision_and_outcome(&decision, &outcome, record_context);
            ok_response(
                Some(serde_json::to_value(&record).expect("decision record should serialize")),
                Some(record.reward_since_prev.scalar_reward),
                Some(record.terminated || record.truncated),
                record
                    .reward_since_prev
                    .components
                    .get("chosen_action_key")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                Some(current.info()),
            )
        }),
    }
}

fn reset_env(
    session: &mut DriverSession,
    seed: Option<u64>,
    ascension: Option<u8>,
    final_act: Option<bool>,
    class: Option<&str>,
    max_steps: Option<usize>,
) -> DriverResponse {
    let player_class = match normalize_player_class(class) {
        Ok(value) => value,
        Err(err) => return error_response(err),
    };
    let config = FullRunEnvConfig {
        seed: seed.unwrap_or(1),
        ascension: ascension.unwrap_or(0),
        final_act: final_act.unwrap_or(false),
        player_class,
        max_steps: max_steps.unwrap_or(5000),
    };
    match FullRunEnv::new(config) {
        Ok(mut next_env) => match next_env.state() {
            Ok(state) => {
                let done = next_env.info().result != "ongoing";
                session.env = Some(next_env);
                ok_response(
                    Some(state_payload(state)),
                    Some(0.0),
                    Some(done),
                    None,
                    session.env.as_ref().map(FullRunEnv::info),
                )
            }
            Err(err) => error_response(err),
        },
        Err(err) => error_response(err),
    }
}

fn with_env(
    session: &mut DriverSession,
    f: impl FnOnce(&mut FullRunEnv) -> DriverResponse,
) -> DriverResponse {
    match session.env.as_mut() {
        Some(current) => f(current),
        None => error_response("full-run env not initialized; send reset first".to_string()),
    }
}

fn ok_response(
    payload: Option<Value>,
    reward: Option<f32>,
    done: Option<bool>,
    chosen_action_key: Option<String>,
    info: Option<FullRunEnvInfo>,
) -> DriverResponse {
    DriverResponse {
        ok: true,
        error: None,
        payload,
        reward,
        done,
        chosen_action_key,
        info,
    }
}

fn error_response(error: String) -> DriverResponse {
    DriverResponse {
        ok: false,
        error: Some(error),
        payload: None,
        reward: None,
        done: None,
        chosen_action_key: None,
        info: None,
    }
}

fn state_payload(state: FullRunEnvState) -> Value {
    serde_json::to_value(state).expect("full-run state should serialize")
}

fn normalize_player_class(value: Option<&str>) -> Result<&'static str, String> {
    match value.unwrap_or("ironclad").to_ascii_lowercase().as_str() {
        "ironclad" | "red" => Ok("Ironclad"),
        "silent" | "green" => Ok("Silent"),
        "defect" | "blue" => Ok("Defect"),
        "watcher" | "purple" => Ok("Watcher"),
        other => Err(format!(
            "unsupported class '{other}'; expected ironclad, silent, defect, or watcher"
        )),
    }
}
