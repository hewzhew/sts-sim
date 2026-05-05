use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use sts_simulator::cli::full_run_smoke::{
    FullRunEnv, FullRunEnvConfig, FullRunEnvInfo, FullRunEnvState, RewardShapingProfile,
    RunPolicyKind,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum DriverRequest {
    Ping,
    Reset {
        seed: Option<u64>,
        ascension: Option<u8>,
        final_act: Option<bool>,
        class: Option<String>,
        max_steps: Option<usize>,
        reward_shaping_profile: Option<String>,
    },
    Observation,
    Step {
        action_index: usize,
    },
    StepPolicy {
        policy: String,
    },
    Close,
}

#[derive(Debug, Serialize)]
struct DriverResponse {
    ok: bool,
    error: Option<String>,
    payload: Option<FullRunEnvState>,
    reward: Option<f32>,
    done: Option<bool>,
    chosen_action_key: Option<String>,
    info: Option<FullRunEnvInfo>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut env: Option<FullRunEnv> = None;
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
            Ok(request) => handle_request(&mut env, request),
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

fn handle_request(env: &mut Option<FullRunEnv>, request: DriverRequest) -> DriverResponse {
    match request {
        DriverRequest::Ping => DriverResponse {
            ok: true,
            error: None,
            payload: None,
            reward: None,
            done: None,
            chosen_action_key: None,
            info: None,
        },
        DriverRequest::Close => DriverResponse {
            ok: true,
            error: None,
            payload: None,
            reward: None,
            done: None,
            chosen_action_key: None,
            info: env.as_ref().map(|current| current.info()),
        },
        DriverRequest::Reset {
            seed,
            ascension,
            final_act,
            class,
            max_steps,
            reward_shaping_profile,
        } => {
            let player_class = match normalize_player_class(class.as_deref()) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            let config = FullRunEnvConfig {
                seed: seed.unwrap_or(1),
                ascension: ascension.unwrap_or(0),
                final_act: final_act.unwrap_or(false),
                player_class,
                max_steps: max_steps.unwrap_or(5000),
                reward_shaping_profile: match reward_shaping_profile {
                    Some(value) => match RewardShapingProfile::parse(&value) {
                        Ok(profile) => profile,
                        Err(err) => return error_response(err),
                    },
                    None => RewardShapingProfile::Baseline,
                },
            };
            match FullRunEnv::new(config) {
                Ok(mut next_env) => match next_env.state() {
                    Ok(state) => {
                        let info = next_env.info();
                        let done = info.result != "ongoing";
                        *env = Some(next_env);
                        DriverResponse {
                            ok: true,
                            error: None,
                            payload: Some(state),
                            reward: Some(0.0),
                            done: Some(done),
                            chosen_action_key: None,
                            info: env.as_ref().map(|current| current.info()),
                        }
                    }
                    Err(err) => error_response(err),
                },
                Err(err) => error_response(err),
            }
        }
        DriverRequest::Observation => match env.as_mut() {
            Some(current) => match current.state() {
                Ok(state) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(state),
                    reward: None,
                    done: Some(current.info().result != "ongoing"),
                    chosen_action_key: None,
                    info: Some(current.info()),
                },
                Err(err) => error_response(err),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::Step { action_index } => match env.as_mut() {
            Some(current) => match current.step(action_index) {
                Ok(step) => DriverResponse {
                    ok: true,
                    error: None,
                    payload: Some(step.state),
                    reward: Some(step.reward),
                    done: Some(step.done),
                    chosen_action_key: step.chosen_action_key,
                    info: Some(step.info),
                },
                Err(err) => error_response(err),
            },
            None => error_response("full-run env not initialized; send reset first".to_string()),
        },
        DriverRequest::StepPolicy { policy } => {
            let policy_kind = match normalize_policy(&policy) {
                Ok(value) => value,
                Err(err) => return error_response(err),
            };
            match env.as_mut() {
                Some(current) => match current.step_policy(policy_kind) {
                    Ok(step) => DriverResponse {
                        ok: true,
                        error: None,
                        payload: Some(step.state),
                        reward: Some(step.reward),
                        done: Some(step.done),
                        chosen_action_key: step.chosen_action_key,
                        info: Some(step.info),
                    },
                    Err(err) => error_response(err),
                },
                None => {
                    error_response("full-run env not initialized; send reset first".to_string())
                }
            }
        }
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

fn normalize_policy(value: &str) -> Result<RunPolicyKind, String> {
    match value.to_ascii_lowercase().as_str() {
        "rule_baseline_v0" => Ok(RunPolicyKind::RuleBaselineV0),
        "plan_query_v0" => Ok(RunPolicyKind::PlanQueryV0),
        other => Err(format!(
            "unsupported policy '{other}'; expected rule_baseline_v0 or plan_query_v0"
        )),
    }
}
