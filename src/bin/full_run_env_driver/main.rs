use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::bot::combat::{
    CombatPlanReport, CombatPlanSequenceClass, CombatTurnPlanProbeConfig,
    CombatTurnPlanProbeReport, PlanScoreBreakdown,
};
use sts_simulator::cli::full_run_smoke::{
    card_applies_vulnerable, card_applies_weak, card_draws_cards, card_exhausts_other_cards,
    card_gains_energy, card_is_block_core, card_is_multi_hit, card_is_scaling_piece, FullRunEnv,
    FullRunEnvConfig, FullRunEnvInfo, FullRunEnvState,
};
use sts_simulator::content::cards::{self, CardId, CardRarity, CardType};
use sts_simulator::verification::decision_env::{
    ActionCandidate, ActionId, DecisionEnv, DecisionRecord, DecisionRecordContext, TimeStep,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
enum DriverRequest {
    Ping,
    ExportCardFacts,
    Reset {
        seed: Option<u64>,
        ascension: Option<u8>,
        final_act: Option<bool>,
        class: Option<String>,
        max_steps: Option<usize>,
    },
    Observation,
    DecisionEnvObservation,
    CombatPlanProbe {
        max_depth: Option<usize>,
        max_nodes: Option<usize>,
        beam_width: Option<usize>,
        max_engine_steps_per_action: Option<usize>,
    },
    CombatSearchEngine {
        horizon_turns: Option<usize>,
        max_nodes: Option<usize>,
        beam_width: Option<usize>,
        particles: Option<usize>,
        max_engine_steps_per_action: Option<usize>,
        include_branch_clusters: Option<bool>,
    },
    CandidateAfterstateSummary {
        action_ids: Vec<usize>,
    },
    DecisionLabProbe {
        action_ids: Vec<usize>,
        max_rollout_steps: Option<usize>,
        max_depth: Option<usize>,
        max_nodes: Option<usize>,
        beam_width: Option<usize>,
        max_engine_steps_per_action: Option<usize>,
    },
    CampfireRestSmithEval,
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
        DriverRequest::ExportCardFacts => export_card_facts(),
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
        DriverRequest::CombatPlanProbe {
            max_depth,
            max_nodes,
            beam_width,
            max_engine_steps_per_action,
        } => with_env(session, |current| {
            let config = combat_probe_config_from_options(
                max_depth,
                max_nodes,
                beam_width,
                max_engine_steps_per_action,
            );
            match current.combat_plan_probe(config) {
                Ok(report) => ok_response(
                    Some(
                        serde_json::to_value(report)
                            .expect("combat plan probe report should serialize"),
                    ),
                    None,
                    Some(current.info().result != "ongoing"),
                    None,
                    Some(current.info()),
                ),
                Err(err) => error_response(err),
            }
        }),
        DriverRequest::CombatSearchEngine {
            horizon_turns,
            max_nodes,
            beam_width,
            particles,
            max_engine_steps_per_action,
            include_branch_clusters,
        } => with_env(session, |current| {
            combat_search_engine(
                current,
                horizon_turns.unwrap_or(2),
                max_nodes.unwrap_or(4_000),
                beam_width.unwrap_or(48),
                particles.unwrap_or(32),
                max_engine_steps_per_action.unwrap_or(200),
                include_branch_clusters.unwrap_or(true),
            )
        }),
        DriverRequest::CandidateAfterstateSummary { action_ids } => with_env(session, |current| {
            candidate_afterstate_summary(current, action_ids)
        }),
        DriverRequest::DecisionLabProbe {
            action_ids,
            max_rollout_steps,
            max_depth,
            max_nodes,
            beam_width,
            max_engine_steps_per_action,
        } => with_env(session, |current| {
            let config = combat_probe_config_from_options(
                max_depth,
                max_nodes,
                beam_width,
                max_engine_steps_per_action,
            );
            decision_lab_probe(current, action_ids, max_rollout_steps.unwrap_or(4), config)
        }),
        DriverRequest::CampfireRestSmithEval => with_env(session, campfire_rest_smith_eval),
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

fn candidate_afterstate_summary(
    current: &mut FullRunEnv,
    action_ids: Vec<usize>,
) -> DriverResponse {
    let decision = match DecisionEnv::current_timestep(current) {
        Ok(timestep) => timestep,
        Err(err) => return error_response(err.to_string()),
    };
    let snapshot = match DecisionEnv::snapshot(current) {
        Ok(snapshot) => snapshot,
        Err(err) => return error_response(err.to_string()),
    };
    let mut summaries = Vec::new();
    for action_id in action_ids {
        let candidate = decision
            .candidates
            .iter()
            .find(|candidate| candidate.id.0 == action_id);
        let Some(candidate) = candidate else {
            summaries.push(json!({
                "action_id": action_id,
                "ok": false,
                "error": "action_id not in current candidate set",
            }));
            continue;
        };
        let outcome = match DecisionEnv::step(current, ActionId(action_id)) {
            Ok(outcome) => {
                candidate_afterstate_item(&decision, &outcome, action_id, &candidate.action_key)
            }
            Err(err) => json!({
                "action_id": action_id,
                "action_key": candidate.action_key,
                "ok": false,
                "error": err.to_string(),
            }),
        };
        if let Err(err) = DecisionEnv::restore(current, &snapshot) {
            return error_response(format!(
                "failed to restore snapshot after afterstate probe: {err}"
            ));
        }
        summaries.push(outcome);
    }
    ok_response(
        Some(json!({
            "schema_name": "CandidateAfterstateSummary",
            "schema_version": 2,
            "decision_id": decision.decision_id,
            "information_boundary": "engine_search",
            "label_role": "not_a_label",
            "trainable_as_action_label": false,
            "probability_model": "not_implemented_v0",
            "worldline_model": "one_step_afterstate_only",
            "truth_warnings": [
                "one_step_afterstate_only",
                "uses_engine_snapshot_restore",
                "not_a_policy_label"
            ],
            "before_summary": compact_public_observation_summary(
                &decision.observation.payload,
                decision.candidates.len()
            ),
            "summaries": summaries,
        })),
        None,
        Some(current.info().result != "ongoing"),
        None,
        Some(current.info()),
    )
}

fn decision_lab_probe(
    current: &mut FullRunEnv,
    action_ids: Vec<usize>,
    max_rollout_steps: usize,
    combat_config: CombatTurnPlanProbeConfig,
) -> DriverResponse {
    let decision = match DecisionEnv::current_timestep(current) {
        Ok(timestep) => timestep,
        Err(err) => return error_response(err.to_string()),
    };
    let snapshot = match DecisionEnv::snapshot(current) {
        Ok(snapshot) => snapshot,
        Err(err) => return error_response(err.to_string()),
    };
    let mut branches = Vec::new();
    for action_id in action_ids {
        let branch = decision_lab_branch(
            current,
            &decision,
            action_id,
            max_rollout_steps,
            combat_config,
        );
        if let Err(err) = DecisionEnv::restore(current, &snapshot) {
            return error_response(format!(
                "failed to restore snapshot after decision lab branch: {err}"
            ));
        }
        branches.push(branch);
    }
    ok_response(
        Some(json!({
            "schema_name": "DecisionLabProbe",
            "schema_version": 1,
            "decision_id": decision.decision_id,
            "information_boundary": "engine_search",
            "label_role": "not_a_label",
            "trainable_as_action_label": false,
            "probability_model": "not_implemented_v0",
            "worldline_model": "bounded_branch_lab_v0",
            "max_rollout_steps": max_rollout_steps,
            "truth_warnings": [
                "branch_lab_uses_engine_snapshot_restore",
                "rollout_policy_is_heuristic_v0",
                "not_exhaustive_worldline_search",
                "not_a_policy_label"
            ],
            "before_summary": compact_public_observation_summary(
                &decision.observation.payload,
                decision.candidates.len()
            ),
            "branches": branches,
        })),
        None,
        Some(current.info().result != "ongoing"),
        None,
        Some(current.info()),
    )
}

fn decision_lab_branch(
    current: &mut FullRunEnv,
    decision: &TimeStep,
    action_id: usize,
    max_rollout_steps: usize,
    combat_config: CombatTurnPlanProbeConfig,
) -> Value {
    let Some(candidate) = decision
        .candidates
        .iter()
        .find(|candidate| candidate.id.0 == action_id)
    else {
        return json!({
            "root_action_id": action_id,
            "ok": false,
            "error": "action_id not in current candidate set",
        });
    };
    let first_outcome = match DecisionEnv::step(current, ActionId(action_id)) {
        Ok(outcome) => outcome,
        Err(err) => {
            return json!({
                "root_action_id": action_id,
                "root_action_key": candidate.action_key,
                "ok": false,
                "error": err.to_string(),
            });
        }
    };
    let first_step =
        candidate_afterstate_item(decision, &first_outcome, action_id, &candidate.action_key);
    let mut rollout_steps = Vec::new();
    let mut last_timestep = first_outcome;
    let mut stop_reason = if last_timestep.terminated || last_timestep.truncated {
        "terminal_after_root_action".to_string()
    } else {
        "max_rollout_steps_reached".to_string()
    };
    for rollout_index in 0..max_rollout_steps {
        if last_timestep.terminated || last_timestep.truncated {
            break;
        }
        let current_timestep = match DecisionEnv::current_timestep(current) {
            Ok(timestep) => timestep,
            Err(err) => {
                stop_reason = format!("current_timestep_failed: {err}");
                break;
            }
        };
        let Some(choice) = choose_lab_rollout_action(current, &current_timestep, combat_config)
        else {
            stop_reason = format!(
                "stopped_at_ambiguous_{}",
                current_timestep.observation.decision_type
            );
            last_timestep = current_timestep;
            break;
        };
        let Some(rollout_candidate) = current_timestep
            .candidates
            .iter()
            .find(|candidate| candidate.id.0 == choice.action_id)
        else {
            stop_reason = "selected_rollout_action_missing_from_candidates".to_string();
            last_timestep = current_timestep;
            break;
        };
        let outcome = match DecisionEnv::step(current, ActionId(choice.action_id)) {
            Ok(outcome) => outcome,
            Err(err) => {
                stop_reason = format!("rollout_step_failed: {err}");
                last_timestep = current_timestep;
                break;
            }
        };
        rollout_steps.push(json!({
            "rollout_index": rollout_index,
            "authority": choice.authority,
            "reason": choice.reason,
            "selected_plan": choice.selected_plan,
            "action_id": choice.action_id,
            "action_key": rollout_candidate.action_key,
            "afterstate": candidate_afterstate_item(
                &current_timestep,
                &outcome,
                choice.action_id,
                &rollout_candidate.action_key,
            ),
        }));
        if outcome.terminated || outcome.truncated {
            stop_reason = "terminal_during_rollout".to_string();
            last_timestep = outcome;
            break;
        }
        last_timestep = outcome;
    }
    let rollout_step_count = rollout_steps.len();
    json!({
        "root_action_id": action_id,
        "root_action_key": candidate.action_key,
        "ok": true,
        "root_afterstate": first_step,
        "rollout_steps": rollout_steps,
        "rollout_step_count": rollout_step_count,
        "stop_reason": stop_reason,
        "final_summary": compact_public_observation_summary(
            &last_timestep.observation.payload,
            last_timestep.candidates.len(),
        ),
        "final_risk_flags": compact_risk_flags(&last_timestep.observation.payload),
        "terminal": compact_terminal_summary(&last_timestep.reward.components),
    })
}

struct LabRolloutChoice {
    action_id: usize,
    authority: &'static str,
    reason: String,
    selected_plan: Option<String>,
}

fn choose_lab_rollout_action(
    current: &mut FullRunEnv,
    timestep: &TimeStep,
    combat_config: CombatTurnPlanProbeConfig,
) -> Option<LabRolloutChoice> {
    let candidates = &timestep.candidates;
    if candidates.len() == 1 {
        return Some(LabRolloutChoice {
            action_id: candidates[0].id.0,
            authority: "routine_policy",
            reason: "single_legal_action".to_string(),
            selected_plan: None,
        });
    }
    let decision_type = timestep.observation.decision_type.as_str();
    if decision_type == "combat" {
        let probe = current.combat_plan_probe(combat_config).ok()?;
        let (action_id, plan_name) = choose_lab_combat_search_action(&probe, candidates)?;
        return Some(LabRolloutChoice {
            action_id,
            authority: "search_verifier",
            reason: "combat_turn_probe_selected_action".to_string(),
            selected_plan: Some(plan_name),
        });
    }
    if decision_type == "reward" {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.action_key.starts_with("reward/claim/"))
        {
            return Some(LabRolloutChoice {
                action_id: candidate.id.0,
                authority: "routine_policy",
                reason: "claim_visible_reward".to_string(),
                selected_plan: None,
            });
        }
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.action_key == "proceed")
        {
            return Some(LabRolloutChoice {
                action_id: candidate.id.0,
                authority: "routine_policy",
                reason: "reward_proceed_after_claims".to_string(),
                selected_plan: None,
            });
        }
    }
    if decision_type == "map" {
        let map_candidates = candidates
            .iter()
            .filter(|candidate| candidate.action_key.starts_with("map/select"))
            .collect::<Vec<_>>();
        if map_candidates.len() == 1 {
            return Some(LabRolloutChoice {
                action_id: map_candidates[0].id.0,
                authority: "routine_policy",
                reason: "single_map_route".to_string(),
                selected_plan: None,
            });
        }
    }
    if decision_type == "treasure" || decision_type == "event" {
        let structural = candidates
            .iter()
            .filter(|candidate| {
                !candidate.action_key.starts_with("potion/")
                    && !candidate.action_key.starts_with("discard_potion/")
            })
            .collect::<Vec<_>>();
        if structural.len() == 1 {
            return Some(LabRolloutChoice {
                action_id: structural[0].id.0,
                authority: "routine_policy",
                reason: format!("single_{decision_type}_choice"),
                selected_plan: None,
            });
        }
    }
    if decision_type == "campfire" {
        let current_hp = value_i64(&timestep.observation.payload, "current_hp");
        let max_hp = value_i64(&timestep.observation.payload, "max_hp");
        if let (Some(current_hp), Some(max_hp)) = (current_hp, max_hp) {
            if max_hp > 0 && current_hp * 100 <= max_hp * 50 {
                if let Some(candidate) = candidates
                    .iter()
                    .find(|candidate| candidate.action_key == "campfire/rest")
                {
                    return Some(LabRolloutChoice {
                        action_id: candidate.id.0,
                        authority: "routine_policy",
                        reason: "low_hp_campfire_rest".to_string(),
                        selected_plan: None,
                    });
                }
            }
        }
    }
    None
}

fn choose_lab_combat_search_action(
    probe: &CombatTurnPlanProbeReport,
    candidates: &[ActionCandidate],
) -> Option<(usize, String)> {
    let player_hp = probe.state_summary.player_hp;
    let unblocked = probe.state_summary.unblocked_incoming_damage;
    if let Some(action_id) = first_action_for_plan(probe, candidates, "Lethal", false) {
        return Some((action_id, "Lethal".to_string()));
    }
    if player_hp > 0 && unblocked >= player_hp {
        if let Some(action_id) = first_action_for_plan(probe, candidates, "FullBlock", true) {
            return Some((action_id, "FullBlock".to_string()));
        }
    }
    if unblocked > 0 {
        if let Some(action_id) =
            first_action_for_plan(probe, candidates, "BlockEnoughThenDamage", true)
        {
            return Some((action_id, "BlockEnoughThenDamage".to_string()));
        }
    }
    if let Some(action_id) = first_action_for_plan(probe, candidates, "MaxDamage", false) {
        return Some((action_id, "MaxDamage".to_string()));
    }
    let mut affordances = probe.first_action_affordances.iter().collect::<Vec<_>>();
    affordances.sort_by(|left, right| {
        left.best_plan_rank
            .unwrap_or(usize::MAX)
            .cmp(&right.best_plan_rank.unwrap_or(usize::MAX))
            .then_with(|| right.sequence_count.cmp(&left.sequence_count))
            .then_with(|| left.action_key.cmp(&right.action_key))
    });
    for affordance in affordances {
        if let Some(action_id) = action_id_for_key(candidates, &affordance.action_key) {
            return Some((action_id, "FirstActionAffordance".to_string()));
        }
    }
    None
}

fn first_action_for_plan(
    probe: &CombatTurnPlanProbeReport,
    candidates: &[ActionCandidate],
    plan_name: &str,
    require_score_signal: bool,
) -> Option<usize> {
    let plan = probe
        .plans
        .iter()
        .find(|plan| plan.plan_name == plan_name)?;
    if plan_name == "Lethal"
        && plan
            .best_score
            .as_ref()
            .map_or(0, |score| score.lethal_score)
            <= 0
    {
        return None;
    }
    if require_score_signal && !has_score_signal(plan) {
        return None;
    }
    let action_key = plan.best_action_keys.first()?;
    action_id_for_key(candidates, action_key)
}

fn has_score_signal(plan: &CombatPlanReport) -> bool {
    plan.best_score
        .as_ref()
        .map(has_positive_plan_score_component)
        .unwrap_or(false)
}

fn has_positive_plan_score_component(score: &PlanScoreBreakdown) -> bool {
    score.block_score > 0
        || score.hp_loss_score > 0
        || score.damage_score > 0
        || score.enemy_death_score > 0
}

fn action_id_for_key(candidates: &[ActionCandidate], action_key: &str) -> Option<usize> {
    candidates
        .iter()
        .find(|candidate| candidate.action_key == action_key)
        .map(|candidate| candidate.id.0)
}

fn candidate_afterstate_item(
    decision: &TimeStep,
    outcome: &TimeStep,
    action_id: usize,
    action_key: &str,
) -> Value {
    let before_summary = compact_public_observation_summary(
        &decision.observation.payload,
        decision.candidates.len(),
    );
    let after_summary =
        compact_public_observation_summary(&outcome.observation.payload, outcome.candidates.len());
    json!({
        "action_id": action_id,
        "action_key": action_key,
        "ok": true,
        "reward": outcome.reward.scalar_reward,
        "done": outcome.terminated || outcome.truncated,
        "terminated": outcome.terminated,
        "truncated": outcome.truncated,
        "chosen_action_key": outcome.reward.components.get("chosen_action_key").cloned().unwrap_or(Value::Null),
        "terminal": compact_terminal_summary(&outcome.reward.components),
        "after_env_info": compact_afterstate_env_info(&outcome.info.payload),
        "state_delta": compact_state_delta(&before_summary, &after_summary),
        "after_summary": after_summary,
        "risk_flags_after": compact_risk_flags(&outcome.observation.payload),
        "next_legal_action_count": outcome.candidates.len(),
    })
}

fn compact_public_observation_summary(payload: &Value, legal_action_count: usize) -> Value {
    let combat = payload.get("combat").filter(|value| value.is_object());
    json!({
        "decision_type": payload.get("decision_type").cloned().unwrap_or(Value::Null),
        "engine_state": payload.get("engine_state").cloned().unwrap_or(Value::Null),
        "act": payload.get("act").cloned().unwrap_or(Value::Null),
        "floor": payload.get("floor").cloned().unwrap_or(Value::Null),
        "current_hp": payload.get("current_hp").cloned().unwrap_or(Value::Null),
        "max_hp": payload.get("max_hp").cloned().unwrap_or(Value::Null),
        "gold": payload.get("gold").cloned().unwrap_or(Value::Null),
        "deck_size": payload.get("deck_size").cloned().unwrap_or(Value::Null),
        "act_boss": payload.get("act_boss").cloned().unwrap_or(Value::Null),
        "legal_action_count": legal_action_count,
        "combat": combat.map(compact_combat_summary).unwrap_or(Value::Null),
    })
}

fn compact_combat_summary(combat: &Value) -> Value {
    json!({
        "player_hp": combat.get("player_hp").cloned().unwrap_or(Value::Null),
        "player_block": combat.get("player_block").cloned().unwrap_or(Value::Null),
        "energy": combat.get("energy").cloned().unwrap_or(Value::Null),
        "turn_count": combat.get("turn_count").cloned().unwrap_or(Value::Null),
        "player_powers": combat.get("player_powers").cloned().unwrap_or(Value::Null),
        "visible_incoming_damage": combat.get("visible_incoming_damage").cloned().unwrap_or(Value::Null),
        "total_monster_hp": combat.get("total_monster_hp").cloned().unwrap_or(Value::Null),
        "alive_monster_count": combat.get("alive_monster_count").cloned().unwrap_or(Value::Null),
        "monsters": combat.get("monsters").cloned().unwrap_or(Value::Null),
        "encounter_hints": combat.get("encounter_hints").cloned().unwrap_or(Value::Null),
        "draw_count": combat.get("draw_count").cloned().unwrap_or(Value::Null),
        "discard_count": combat.get("discard_count").cloned().unwrap_or(Value::Null),
    })
}

fn compact_terminal_summary(reward_components: &Value) -> Value {
    json!({
        "result": reward_components.get("result").cloned().unwrap_or(Value::Null),
        "terminal_reason": reward_components.get("terminal_reason").cloned().unwrap_or(Value::Null),
        "floor": reward_components.get("floor").cloned().unwrap_or(Value::Null),
        "hp": reward_components.get("hp").cloned().unwrap_or(Value::Null),
        "combat_win_count": reward_components.get("combat_win_count").cloned().unwrap_or(Value::Null),
    })
}

fn compact_afterstate_env_info(info_payload: &Value) -> Value {
    let env_info = info_payload.get("env_info").unwrap_or(&Value::Null);
    json!({
        "step": env_info.get("step").cloned().unwrap_or(Value::Null),
        "floor": env_info.get("floor").cloned().unwrap_or(Value::Null),
        "act": env_info.get("act").cloned().unwrap_or(Value::Null),
        "hp": env_info.get("hp").cloned().unwrap_or(Value::Null),
        "max_hp": env_info.get("max_hp").cloned().unwrap_or(Value::Null),
        "gold": env_info.get("gold").cloned().unwrap_or(Value::Null),
        "deck_size": env_info.get("deck_size").cloned().unwrap_or(Value::Null),
        "result": env_info.get("result").cloned().unwrap_or(Value::Null),
        "terminal_reason": env_info.get("terminal_reason").cloned().unwrap_or(Value::Null),
        "combat_win_count": env_info.get("combat_win_count").cloned().unwrap_or(Value::Null),
        "chosen_action_key": info_payload.get("chosen_action_key").cloned().unwrap_or(Value::Null),
        "legal_action_count": info_payload.get("legal_action_count").cloned().unwrap_or(Value::Null),
    })
}

fn compact_state_delta(before: &Value, after: &Value) -> Value {
    let hp_before = value_i64(before, "current_hp");
    let hp_after = value_i64(after, "current_hp");
    let gold_before = value_i64(before, "gold");
    let gold_after = value_i64(after, "gold");
    let floor_before = value_i64(before, "floor");
    let floor_after = value_i64(after, "floor");
    let monster_before = nested_i64(before, "combat", "total_monster_hp");
    let monster_after = nested_i64(after, "combat", "total_monster_hp");
    json!({
        "hp_delta": option_delta(hp_before, hp_after),
        "gold_delta": option_delta(gold_before, gold_after),
        "floor_delta": option_delta(floor_before, floor_after),
        "monster_hp_delta": option_delta(monster_before, monster_after),
        "decision_type_before": before.get("decision_type").cloned().unwrap_or(Value::Null),
        "decision_type_after": after.get("decision_type").cloned().unwrap_or(Value::Null),
    })
}

fn compact_risk_flags(payload: &Value) -> Vec<&'static str> {
    let mut flags = Vec::new();
    let current_hp = value_i64(payload, "current_hp");
    let max_hp = value_i64(payload, "max_hp");
    if let (Some(current_hp), Some(max_hp)) = (current_hp, max_hp) {
        if max_hp > 0 && current_hp * 100 <= max_hp * 35 {
            flags.push("low_hp");
        }
    }
    if let Some(combat) = payload.get("combat").filter(|value| value.is_object()) {
        let hp = value_i64(combat, "player_hp");
        let block = value_i64(combat, "player_block").unwrap_or(0);
        let incoming = value_i64(combat, "visible_incoming_damage").unwrap_or(0);
        let monster_hp = value_i64(combat, "total_monster_hp").unwrap_or(0);
        if incoming > 0 {
            flags.push("incoming_damage");
        }
        if let Some(hp) = hp {
            if hp > 0 && std::cmp::max(0, incoming - block) >= hp {
                flags.push("lethal_incoming");
            }
        }
        if monster_hp <= 15 {
            flags.push("possible_lethal_window");
        }
    }
    flags
}

fn value_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn nested_i64(value: &Value, outer_key: &str, inner_key: &str) -> Option<i64> {
    value
        .get(outer_key)
        .and_then(|outer| outer.get(inner_key))
        .and_then(Value::as_i64)
}

fn option_delta(before: Option<i64>, after: Option<i64>) -> Value {
    match (before, after) {
        (Some(before), Some(after)) => json!(after - before),
        _ => Value::Null,
    }
}

fn combat_probe_config_from_options(
    max_depth: Option<usize>,
    max_nodes: Option<usize>,
    beam_width: Option<usize>,
    max_engine_steps_per_action: Option<usize>,
) -> CombatTurnPlanProbeConfig {
    let mut config = CombatTurnPlanProbeConfig::default();
    if let Some(value) = max_depth {
        config.max_depth = value;
    }
    if let Some(value) = max_nodes {
        config.max_nodes = value;
    }
    if let Some(value) = beam_width {
        config.beam_width = value;
    }
    if let Some(value) = max_engine_steps_per_action {
        config.max_engine_steps_per_action = value;
    }
    config
}

fn campfire_rest_smith_eval(current: &mut FullRunEnv) -> DriverResponse {
    let decision = match DecisionEnv::current_timestep(current) {
        Ok(timestep) => timestep,
        Err(err) => return error_response(err.to_string()),
    };
    if decision.observation.decision_type != "campfire" {
        return error_response(format!(
            "campfire_rest_smith_eval requires campfire decision, got {}",
            decision.observation.decision_type
        ));
    }
    let payload = &decision.observation.payload;
    let current_hp = payload
        .get("current_hp")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let max_hp = payload
        .get("max_hp")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let rest_candidates = decision
        .candidates
        .iter()
        .filter(|candidate| candidate.action_key == "campfire/rest")
        .map(|candidate| {
            json!({
                "action_id": candidate.id.0,
                "action_key": candidate.action_key,
            })
        })
        .collect::<Vec<_>>();
    let smith_candidates = decision
        .candidates
        .iter()
        .filter(|candidate| candidate.action_key.starts_with("campfire/smith/"))
        .map(|candidate| {
            json!({
                "action_id": candidate.id.0,
                "action_key": candidate.action_key,
            })
        })
        .collect::<Vec<_>>();
    ok_response(
        Some(json!({
            "schema_name": "CampfireRestSmithEval",
            "schema_version": 1,
            "decision_id": decision.decision_id,
            "information_boundary": "public_observation_plus_legal_actions",
            "label_role": "not_a_label",
            "trainable_as_action_label": false,
            "current_hp": current_hp,
            "max_hp": max_hp,
            "hp_ratio_milli": if max_hp > 0 { current_hp * 1000 / max_hp } else { 0 },
            "low_hp_rest_guardrail_threshold_milli": 500,
            "rest_legal": !rest_candidates.is_empty(),
            "rest_candidates": rest_candidates,
            "smith_candidates": smith_candidates,
        })),
        None,
        Some(current.info().result != "ongoing"),
        None,
        Some(current.info()),
    )
}

fn export_card_facts() -> DriverResponse {
    let card_ids = export_card_fact_ids();
    let cards = card_ids
        .iter()
        .map(|&card_id| card_fact_payload(card_id))
        .collect::<Vec<_>>();
    ok_response(
        Some(json!({
            "schema_name": "CardFactsDatabase",
            "schema_version": 1,
            "artifact_role": "semantic_tool_knowledge_base",
            "information_boundary": "static_game_content",
            "label_role": "not_a_label",
            "trainable_as_action_label": false,
            "policy_quality_claim": false,
            "producer": {
                "component": "full_run_env_driver",
                "command": "export_card_facts"
            },
            "scope": "base_game_card_definitions_v1",
            "cards": cards,
        })),
        None,
        None,
        None,
        None,
    )
}

fn export_card_fact_ids() -> Vec<CardId> {
    let mut ids = Vec::new();
    let rarities = [CardRarity::Common, CardRarity::Uncommon, CardRarity::Rare];
    for rarity in rarities {
        for &card_id in cards::ironclad_pool_for_rarity(rarity) {
            push_unique_card_id(&mut ids, card_id);
        }
        for &card_id in cards::silent_pool_for_rarity(rarity) {
            push_unique_card_id(&mut ids, card_id);
        }
        for &card_id in cards::defect_pool_for_rarity(rarity) {
            push_unique_card_id(&mut ids, card_id);
        }
        for &card_id in cards::watcher_pool_for_rarity(rarity) {
            push_unique_card_id(&mut ids, card_id);
        }
        for &card_id in cards::colorless_pool_for_rarity(rarity) {
            push_unique_card_id(&mut ids, card_id);
        }
    }
    for card_id in [
        CardId::Strike,
        CardId::Defend,
        CardId::Bash,
        CardId::StrikeG,
        CardId::DefendG,
        CardId::StrikeB,
        CardId::DefendB,
        CardId::Zap,
        CardId::Dualcast,
        CardId::StrikeP,
        CardId::DefendP,
        CardId::Eruption,
        CardId::Vigilance,
        CardId::Wound,
        CardId::Burn,
        CardId::Dazed,
        CardId::Slimed,
        CardId::Void,
        CardId::AscendersBane,
        CardId::CurseOfTheBell,
        CardId::Necronomicurse,
        CardId::Pride,
    ] {
        push_unique_card_id(&mut ids, card_id);
    }
    for &card_id in cards::get_curse_pool() {
        push_unique_card_id(&mut ids, card_id);
    }
    ids
}

fn push_unique_card_id(ids: &mut Vec<CardId>, card_id: CardId) {
    if !ids.contains(&card_id) {
        ids.push(card_id);
    }
}

fn card_fact_payload(card_id: CardId) -> Value {
    let def = cards::get_card_definition(card_id);
    let roles = derived_card_roles(card_id, def.card_type, def.base_damage, def.base_block);
    let tags = derived_card_tags(card_id, def.card_type, def.cost, def.target, &def);
    json!({
        "card_id": format!("{card_id:?}"),
        "java_id": cards::java_id(card_id),
        "name": def.name,
        "card_type": format!("{:?}", def.card_type),
        "rarity": format!("{:?}", def.rarity),
        "cost": def.cost,
        "base_damage": def.base_damage,
        "base_block": def.base_block,
        "base_magic": def.base_magic,
        "upgrade_damage": def.upgrade_damage,
        "upgrade_block": def.upgrade_block,
        "upgrade_magic": def.upgrade_magic,
        "upgraded_damage": def.base_damage + def.upgrade_damage,
        "upgraded_block": def.base_block + def.upgrade_block,
        "upgraded_magic": def.base_magic + def.upgrade_magic,
        "target": format!("{:?}", def.target),
        "is_multi_damage": def.is_multi_damage,
        "exhaust": def.exhaust,
        "ethereal": def.ethereal,
        "innate": def.innate,
        "engine_tags": def.tags.iter().map(|tag| format!("{tag:?}")).collect::<Vec<_>>(),
        "derived_roles": roles,
        "derived_tags": tags,
    })
}

fn derived_card_roles(
    card_id: CardId,
    card_type: CardType,
    base_damage: i32,
    base_block: i32,
) -> Vec<&'static str> {
    let mut roles = Vec::new();
    if card_type == CardType::Attack || base_damage > 0 {
        roles.push("attack");
    }
    if base_damage > 0 {
        roles.push("frontload");
    }
    if base_block > 0 || card_is_block_core(card_id) {
        roles.push("block");
    }
    if card_draws_cards(card_id) {
        roles.push("draw");
    }
    if card_gains_energy(card_id) {
        roles.push("energy");
    }
    if card_applies_weak(card_id) {
        roles.push("weak");
        roles.push("mitigation");
    }
    if card_applies_vulnerable(card_id) {
        roles.push("vulnerable");
    }
    if card_is_scaling_piece(card_id) {
        roles.push("scaling");
    }
    if card_is_multi_hit(card_id) {
        roles.push("multi_hit");
    }
    if card_type == CardType::Status || card_type == CardType::Curse {
        roles.push("bad_draw");
    }
    sort_dedup_static_strs(&mut roles);
    roles
}

fn derived_card_tags(
    card_id: CardId,
    card_type: CardType,
    cost: i8,
    target: cards::CardTarget,
    def: &cards::CardDefinition,
) -> Vec<&'static str> {
    let mut tags = Vec::new();
    match card_type {
        CardType::Attack => tags.push("attack"),
        CardType::Skill => tags.push("skill"),
        CardType::Power => tags.push("power"),
        CardType::Status => tags.push("status"),
        CardType::Curse => tags.push("curse"),
    }
    if cards::is_starter_basic(card_id) {
        tags.push("basic");
    }
    if cost == 0 {
        tags.push("zero_cost");
    } else if cost == -1 {
        tags.push("x_cost");
    } else if cost == -2 {
        tags.push("unplayable");
    }
    if matches!(target, cards::CardTarget::AllEnemy) {
        tags.push("aoe");
    }
    if def.is_multi_damage || card_is_multi_hit(card_id) {
        tags.push("multi_damage");
    }
    if def.exhaust || card_exhausts_other_cards(card_id) {
        tags.push("exhaust");
    }
    if def.ethereal {
        tags.push("ethereal");
    }
    if def.innate {
        tags.push("innate");
    }
    if card_is_block_core(card_id) {
        tags.push("block_core");
    }
    sort_dedup_static_strs(&mut tags);
    tags
}

fn sort_dedup_static_strs(values: &mut Vec<&'static str>) {
    values.sort_unstable();
    values.dedup();
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

fn combat_search_engine(
    current: &mut FullRunEnv,
    horizon_turns: usize,
    max_nodes: usize,
    beam_width: usize,
    particles: usize,
    max_engine_steps_per_action: usize,
    include_branch_clusters: bool,
) -> DriverResponse {
    let decision = match DecisionEnv::current_timestep(current) {
        Ok(timestep) => timestep,
        Err(err) => return error_response(err.to_string()),
    };
    if decision.observation.decision_type != "combat" {
        return error_response(format!(
            "combat_search_engine requires combat decision, got {}",
            decision.observation.decision_type
        ));
    }
    let max_depth = horizon_turns.saturating_mul(3).max(1);
    let config = CombatTurnPlanProbeConfig {
        max_depth,
        max_nodes,
        beam_width,
        max_engine_steps_per_action,
    };
    let probe = match current.combat_plan_probe(config) {
        Ok(report) => report,
        Err(err) => return error_response(err),
    };
    let payload = build_combat_search_report(
        &decision,
        &probe,
        horizon_turns,
        particles,
        include_branch_clusters,
    );
    ok_response(
        Some(payload),
        None,
        Some(current.info().result != "ongoing"),
        None,
        Some(current.info()),
    )
}

#[derive(Clone, Debug)]
struct RootActionMetrics {
    action_id: usize,
    action_key: String,
    action_label: String,
    branch_count: usize,
    search_status: &'static str,
    survival_rate: f64,
    fatality_rate: f64,
    hp_min: i32,
    hp_mean: f64,
    hp_p10: i32,
    block_shortfall_worst: i32,
    damage_done_mean: f64,
    enemy_hp_remaining_mean: f64,
    lethal_now: bool,
    lethal_next_turn_rate: f64,
    potion_cost: f64,
    hp_cost: f64,
    exhaust_cost: f64,
    energy_left_mean: f64,
    next_turn_draw_quality: f64,
    remaining_hand_count_mean: f64,
    draw_pile_pressure: f64,
    nob_skill_punish: f64,
    lagavulin_wake_risk: f64,
    sentry_daze_pollution: f64,
    hexaghost_burn_pressure: f64,
    generic_mechanic_risk: f64,
    branch_entropy: f64,
    tail_risk: f64,
    major_tradeoffs: Vec<String>,
    risk_note_kinds: Vec<String>,
    representative_branch: Vec<String>,
}

fn build_combat_search_report(
    decision: &TimeStep,
    probe: &CombatTurnPlanProbeReport,
    horizon_turns: usize,
    particles: usize,
    include_branch_clusters: bool,
) -> Value {
    let roots = decision
        .candidates
        .iter()
        .map(|candidate| build_root_action_metrics(candidate, probe))
        .collect::<Vec<_>>();
    let dominated_pairs = dominated_root_pairs(&roots);
    let frontier = roots
        .iter()
        .filter(|root| {
            root.branch_count > 0
                && !dominated_pairs
                    .iter()
                    .any(|(dominated, _)| *dominated == root.action_key)
        })
        .map(|root| frontier_item(root, &roots))
        .collect::<Vec<_>>();
    let dominated_actions = dominated_pairs
        .iter()
        .map(|(dominated, dominator)| dominated_item(dominated, dominator, &roots))
        .collect::<Vec<_>>();
    let failure_mode_clusters = if include_branch_clusters {
        build_failure_mode_clusters(probe, &dominated_pairs)
    } else {
        Vec::new()
    };
    json!({
        "schema_name": "CombatSearchReport",
        "schema_version": 1,
        "search_paradigm": "abstraction_refining_frontier_search_v1",
        "information_boundary": "engine_search",
        "decision_authority": "evidence_only",
        "controller_role": "evidence_provider",
        "not_final_action": true,
        "label_role": "not_a_label",
        "trainable_as_action_label": false,
        "policy_quality_claim": false,
        "source_probe_schema_version": probe.schema_version,
        "decision_id": decision.decision_id,
        "search_config": {
            "horizon_turns": horizon_turns,
            "effective_max_depth": probe.probe_limits.max_depth,
            "max_nodes": probe.probe_limits.max_nodes,
            "beam_width": probe.probe_limits.beam_width,
            "particles_requested": particles,
            "particles_evaluated": 0,
            "max_engine_steps_per_action": probe.probe_limits.max_engine_steps_per_action,
            "include_branch_clusters": include_branch_clusters,
            "belief_particle_model": "next_turn_draw_particles_reserved_not_evaluated_v1",
            "budget_model": "anytime_frontier_budget",
            "depth_semantics": "effective_max_depth_is_a_budget_guard_not_an_optimality_claim"
        },
        "state_summary": probe.state_summary,
        "root_action_outcomes": roots.iter().map(root_action_outcome_json).collect::<Vec<_>>(),
        "pareto_frontier": frontier,
        "dominated_actions": dominated_actions,
        "failure_mode_clusters": failure_mode_clusters,
        "search_geometry": build_search_geometry_report(
            probe,
            &roots,
            &dominated_pairs,
            particles,
        ),
        "search_reliability": combat_search_reliability(probe, horizon_turns, particles),
        "truth_warnings": combat_search_truth_warnings(probe),
    })
}

fn build_root_action_metrics(
    candidate: &ActionCandidate,
    probe: &CombatTurnPlanProbeReport,
) -> RootActionMetrics {
    let action_key = candidate.action_key.clone();
    let affordance = probe
        .first_action_affordances
        .iter()
        .find(|item| item.action_key == action_key);
    let sequences = probe
        .sequence_classes
        .iter()
        .filter(|sequence| {
            sequence
                .action_keys
                .first()
                .map(|first| first == &action_key)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    let branch_count = sequences.len();
    let player_hp = probe.state_summary.player_hp;
    let total_monster_hp = probe.state_summary.total_monster_hp;
    let mut hp_after_values = Vec::new();
    let mut fatal_count = 0usize;
    let mut damage_sum = 0i32;
    let mut enemy_hp_sum = 0i32;
    let mut hp_cost_sum = 0i32;
    let mut energy_sum = 0i32;
    let mut remaining_hand_sum = 0i32;
    let mut block_shortfall_worst = 0i32;
    let mut lethal_now = false;
    let mut exhaust_cost_sum = 0i32;
    let mut mechanic_projection = 0i32;
    for sequence in &sequences {
        let outcome = &sequence.outcome;
        let hp_loss = outcome
            .projected_unblocked_damage
            .max(outcome.hp_loss_actual)
            .max(0);
        let hp_after = player_hp - hp_loss;
        hp_after_values.push(hp_after);
        if hp_after <= 0 {
            fatal_count += 1;
        }
        damage_sum += outcome.damage_done;
        enemy_hp_sum += outcome.total_monster_hp;
        hp_cost_sum += hp_loss;
        energy_sum += outcome.remaining_energy;
        remaining_hand_sum += outcome.remaining_hand_count as i32;
        block_shortfall_worst = block_shortfall_worst.max(outcome.projected_unblocked_damage);
        lethal_now |= outcome.living_monster_count == 0 || outcome.total_monster_hp <= 0;
        exhaust_cost_sum += sequence.diagnostics.exhaust_value.abs();
        mechanic_projection = mechanic_projection
            .max(sequence.diagnostics.strength_projection.max(0))
            .max(sequence.diagnostics.random_risk.max(0));
    }
    let mut risk_note_kinds = Vec::new();
    if let Some(affordance) = affordance {
        for kind in &affordance.risk_note_kinds {
            push_unique_string(&mut risk_note_kinds, kind.clone());
        }
    }
    for note in &probe.risk_notes {
        if note.action_key == action_key {
            push_unique_string(&mut risk_note_kinds, note.kind.clone());
        }
    }
    let survival_rate = if branch_count == 0 {
        0.0
    } else {
        (branch_count - fatal_count) as f64 / branch_count as f64
    };
    let fatality_rate = if branch_count == 0 {
        0.0
    } else {
        fatal_count as f64 / branch_count as f64
    };
    let hp_min = hp_after_values.iter().copied().min().unwrap_or(player_hp);
    let hp_mean = mean_i32(&hp_after_values, player_hp as f64);
    let hp_p10 = percentile_i32(&hp_after_values, 10, player_hp);
    let damage_done_mean = mean_sum(damage_sum, branch_count);
    let enemy_hp_remaining_mean = if branch_count == 0 {
        total_monster_hp as f64
    } else {
        enemy_hp_sum as f64 / branch_count as f64
    };
    let hp_cost = mean_sum(hp_cost_sum, branch_count);
    let energy_left_mean = mean_sum(energy_sum, branch_count);
    let remaining_hand_count_mean = mean_sum(remaining_hand_sum, branch_count);
    let action_potion_cost = if action_key.contains("potion") {
        1.0
    } else {
        0.0
    };
    let generic_mechanic_risk = risk_note_kinds.len() as f64 + mechanic_projection as f64;
    let tail_risk =
        fatality_rate * 100.0 + block_shortfall_worst.max(0) as f64 + generic_mechanic_risk * 2.0;
    RootActionMetrics {
        action_id: candidate.id.0,
        action_key: action_key.clone(),
        action_label: affordance
            .map(|item| item.action_label.clone())
            .unwrap_or_else(|| action_key.clone()),
        branch_count,
        search_status: if branch_count == 0 {
            "not_expanded"
        } else {
            "expanded"
        },
        survival_rate,
        fatality_rate,
        hp_min,
        hp_mean,
        hp_p10,
        block_shortfall_worst,
        damage_done_mean,
        enemy_hp_remaining_mean,
        lethal_now,
        lethal_next_turn_rate: if lethal_now { 1.0 } else { 0.0 },
        potion_cost: action_potion_cost,
        hp_cost,
        exhaust_cost: mean_sum(exhaust_cost_sum, branch_count),
        energy_left_mean,
        next_turn_draw_quality: remaining_hand_count_mean,
        remaining_hand_count_mean,
        draw_pile_pressure: probe.state_summary.draw_count as f64,
        nob_skill_punish: mechanic_component_for(&risk_note_kinds, "nob", mechanic_projection),
        lagavulin_wake_risk: mechanic_component_for(
            &risk_note_kinds,
            "lagavulin",
            mechanic_projection,
        ),
        sentry_daze_pollution: mechanic_component_for(
            &risk_note_kinds,
            "daze",
            mechanic_projection,
        ),
        hexaghost_burn_pressure: mechanic_component_for(
            &risk_note_kinds,
            "hexaghost",
            mechanic_projection,
        ),
        generic_mechanic_risk,
        branch_entropy: if branch_count > 1 {
            (branch_count as f64).ln()
        } else {
            0.0
        },
        tail_risk,
        major_tradeoffs: affordance
            .map(|item| item.major_tradeoffs.clone())
            .unwrap_or_default(),
        risk_note_kinds,
        representative_branch: sequences
            .first()
            .map(|sequence| sequence.actions.clone())
            .or_else(|| affordance.map(|item| item.best_sequence_actions.clone()))
            .unwrap_or_default(),
    }
}

fn root_action_outcome_json(root: &RootActionMetrics) -> Value {
    json!({
        "root_action_id": root.action_id,
        "root_action_key": root.action_key,
        "root_action_label": root.action_label,
        "search_status": root.search_status,
        "representative_branch": root.representative_branch,
        "major_tradeoffs": root.major_tradeoffs,
        "risk_note_kinds": root.risk_note_kinds,
        "outcome_vector": {
            "schema_name": "OutcomeVectorV1",
            "survival_component": {
                "fatality_rate": root.fatality_rate,
                "survival_rate": root.survival_rate,
                "hp_min": root.hp_min,
                "hp_mean": root.hp_mean,
                "hp_p10": root.hp_p10,
                "block_shortfall_worst": root.block_shortfall_worst,
            },
            "tempo_component": {
                "damage_done_mean": root.damage_done_mean,
                "enemy_hp_remaining_mean": root.enemy_hp_remaining_mean,
                "lethal_now": root.lethal_now,
                "lethal_next_turn_rate": root.lethal_next_turn_rate,
            },
            "resource_component": {
                "potion_cost": root.potion_cost,
                "hp_cost": root.hp_cost,
                "exhaust_cost": root.exhaust_cost,
                "energy_left_mean": root.energy_left_mean,
            },
            "future_component": {
                "next_turn_draw_quality": root.next_turn_draw_quality,
                "remaining_hand_count_mean": root.remaining_hand_count_mean,
                "draw_pile_pressure": root.draw_pile_pressure,
            },
            "mechanic_component": {
                "nob_skill_punish": root.nob_skill_punish,
                "lagavulin_wake_risk": root.lagavulin_wake_risk,
                "sentry_daze_pollution": root.sentry_daze_pollution,
                "hexaghost_burn_pressure": root.hexaghost_burn_pressure,
                "generic_mechanic_risk": root.generic_mechanic_risk,
            },
            "distribution_component": {
                "branch_count": root.branch_count,
                "branch_entropy": root.branch_entropy,
                "tail_risk": root.tail_risk,
            },
        },
    })
}

fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn mean_i32(values: &[i32], default: f64) -> f64 {
    if values.is_empty() {
        default
    } else {
        values.iter().sum::<i32>() as f64 / values.len() as f64
    }
}

fn mean_sum(sum: i32, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    }
}

fn percentile_i32(values: &[i32], percentile: usize, default: i32) -> i32 {
    if values.is_empty() {
        return default;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) * percentile) / 100;
    sorted[index]
}

fn mechanic_component_for(kinds: &[String], needle: &str, projection: i32) -> f64 {
    if kinds
        .iter()
        .any(|kind| kind.to_ascii_lowercase().contains(needle))
    {
        1.0 + projection.max(0) as f64
    } else {
        0.0
    }
}

fn dominated_root_pairs(roots: &[RootActionMetrics]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for root in roots {
        if root.branch_count == 0 {
            continue;
        }
        if let Some(dominator) = roots
            .iter()
            .find(|other| other.action_key != root.action_key && root_dominates(other, root))
        {
            pairs.push((root.action_key.clone(), dominator.action_key.clone()));
        }
    }
    pairs
}

fn root_dominates(a: &RootActionMetrics, b: &RootActionMetrics) -> bool {
    if a.branch_count == 0 || b.branch_count == 0 {
        return false;
    }
    let eps = 0.0001;
    let not_worse = a.survival_rate + eps >= b.survival_rate
        && a.fatality_rate <= b.fatality_rate + eps
        && a.hp_p10 >= b.hp_p10
        && a.damage_done_mean + eps >= b.damage_done_mean
        && a.enemy_hp_remaining_mean <= b.enemy_hp_remaining_mean + eps
        && a.potion_cost <= b.potion_cost + eps
        && a.generic_mechanic_risk <= b.generic_mechanic_risk + eps
        && a.tail_risk <= b.tail_risk + eps;
    let strictly_better = a.survival_rate > b.survival_rate + eps
        || a.fatality_rate + eps < b.fatality_rate
        || a.hp_p10 > b.hp_p10
        || a.damage_done_mean > b.damage_done_mean + eps
        || a.enemy_hp_remaining_mean + eps < b.enemy_hp_remaining_mean
        || a.potion_cost + eps < b.potion_cost
        || a.generic_mechanic_risk + eps < b.generic_mechanic_risk
        || a.tail_risk + eps < b.tail_risk;
    not_worse && strictly_better
}

fn frontier_item(root: &RootActionMetrics, roots: &[RootActionMetrics]) -> Value {
    let axes = frontier_axes(root, roots);
    json!({
        "root_action_key": root.action_key,
        "frontier_axes": axes,
        "tradeoff_label": frontier_tradeoff_label(root),
        "reason": format!(
            "survival={:.2} fatality={:.2} hp_p10={} damage_mean={:.1} enemy_hp_mean={:.1} tail_risk={:.1}",
            root.survival_rate,
            root.fatality_rate,
            root.hp_p10,
            root.damage_done_mean,
            root.enemy_hp_remaining_mean,
            root.tail_risk,
        ),
    })
}

fn frontier_axes(root: &RootActionMetrics, roots: &[RootActionMetrics]) -> Vec<&'static str> {
    let mut axes = Vec::new();
    if roots
        .iter()
        .all(|other| root.survival_rate + 0.0001 >= other.survival_rate)
    {
        axes.push("survival");
    }
    if roots.iter().all(|other| root.hp_p10 >= other.hp_p10) {
        axes.push("hp_tail");
    }
    if roots
        .iter()
        .all(|other| root.damage_done_mean + 0.0001 >= other.damage_done_mean)
    {
        axes.push("tempo");
    }
    if roots
        .iter()
        .all(|other| root.potion_cost <= other.potion_cost + 0.0001)
    {
        axes.push("resource");
    }
    if roots
        .iter()
        .all(|other| root.tail_risk <= other.tail_risk + 0.0001)
    {
        axes.push("tail_risk");
    }
    if axes.is_empty() {
        axes.push("mixed_tradeoff");
    }
    axes
}

fn frontier_tradeoff_label(root: &RootActionMetrics) -> &'static str {
    if root.lethal_now {
        "lethal_line"
    } else if root.fatality_rate > 0.0 {
        "risky_high_variance_line"
    } else if root.potion_cost > 0.0 {
        "potion_for_safety_or_tempo"
    } else if root.damage_done_mean >= 20.0 {
        "tempo_line"
    } else {
        "survival_resource_tradeoff"
    }
}

fn dominated_item(dominated: &str, dominator: &str, roots: &[RootActionMetrics]) -> Value {
    let dominated_root = roots.iter().find(|root| root.action_key == dominated);
    let dominator_root = roots.iter().find(|root| root.action_key == dominator);
    let axes = match (dominated_root, dominator_root) {
        (Some(a), Some(b)) => dominance_axes(b, a),
        _ => Vec::new(),
    };
    json!({
        "root_action_key": dominated,
        "dominated_by": dominator,
        "dominated_axes": axes,
        "reason": "dominated on evaluated survival/tempo/resource/risk axes",
    })
}

fn dominance_axes(a: &RootActionMetrics, b: &RootActionMetrics) -> Vec<&'static str> {
    let mut axes = Vec::new();
    if a.survival_rate > b.survival_rate {
        axes.push("survival_rate");
    }
    if a.fatality_rate < b.fatality_rate {
        axes.push("fatality_rate");
    }
    if a.hp_p10 > b.hp_p10 {
        axes.push("hp_p10");
    }
    if a.damage_done_mean > b.damage_done_mean {
        axes.push("damage_done_mean");
    }
    if a.enemy_hp_remaining_mean < b.enemy_hp_remaining_mean {
        axes.push("enemy_hp_remaining_mean");
    }
    if a.potion_cost < b.potion_cost {
        axes.push("potion_cost");
    }
    if a.generic_mechanic_risk < b.generic_mechanic_risk {
        axes.push("mechanic_risk");
    }
    if a.tail_risk < b.tail_risk {
        axes.push("tail_risk");
    }
    axes
}

#[derive(Clone, Debug)]
struct FailureClusterAgg {
    label: &'static str,
    count: usize,
    hp_sum: i32,
    damage_sum: i32,
    enemy_hp_sum: i32,
    representative_branch: Vec<String>,
}

fn build_failure_mode_clusters(
    probe: &CombatTurnPlanProbeReport,
    dominated_pairs: &[(String, String)],
) -> Vec<Value> {
    let mut clusters: Vec<FailureClusterAgg> = Vec::new();
    let player_hp = probe.state_summary.player_hp;
    for sequence in &probe.sequence_classes {
        let hp_loss = sequence
            .outcome
            .projected_unblocked_damage
            .max(sequence.outcome.hp_loss_actual)
            .max(0);
        let label = classify_sequence_failure(
            player_hp,
            hp_loss,
            sequence.outcome.projected_unblocked_damage,
            sequence.outcome.living_monster_count,
            sequence.outcome.total_monster_hp,
            sequence.diagnostics.strength_projection,
            &sequence.actions,
        );
        push_failure_cluster(
            &mut clusters,
            label,
            player_hp - hp_loss,
            sequence.outcome.damage_done,
            sequence.outcome.total_monster_hp,
            sequence.actions.clone(),
        );
    }
    if !dominated_pairs.is_empty() {
        push_failure_cluster(
            &mut clusters,
            "dominated_no_progress",
            player_hp,
            0,
            probe.state_summary.total_monster_hp,
            dominated_pairs
                .iter()
                .take(4)
                .map(|(dominated, dominator)| format!("{dominated} dominated_by {dominator}"))
                .collect(),
        );
    }
    let total = clusters
        .iter()
        .map(|cluster| cluster.count)
        .sum::<usize>()
        .max(1);
    clusters
        .into_iter()
        .map(|cluster| {
            let count = cluster.count.max(1);
            json!({
                "label": cluster.label,
                "probability_weight": cluster.count as f64 / total as f64,
                "branch_count": cluster.count,
                "representative_branch": cluster.representative_branch,
                "centroid_outcome": {
                    "hp_after_mean": cluster.hp_sum as f64 / count as f64,
                    "damage_done_mean": cluster.damage_sum as f64 / count as f64,
                    "enemy_hp_remaining_mean": cluster.enemy_hp_sum as f64 / count as f64,
                },
            })
        })
        .collect()
}

fn classify_sequence_failure(
    player_hp: i32,
    hp_loss: i32,
    projected_unblocked_damage: i32,
    living_monster_count: usize,
    total_monster_hp: i32,
    strength_projection: i32,
    actions: &[String],
) -> &'static str {
    if living_monster_count == 0 || total_monster_hp <= 0 {
        return "lethal_found";
    }
    let hp_after = player_hp - hp_loss;
    if hp_after <= 0 && strength_projection > 0 {
        return "fatal_due_to_mechanic_scaling";
    }
    if hp_after <= 0 && projected_unblocked_damage > 0 {
        return "fatal_due_to_block_shortfall";
    }
    if hp_after <= 0 {
        return "fatal_due_to_low_tempo";
    }
    if actions
        .iter()
        .any(|action| action.to_ascii_lowercase().contains("potion"))
        && hp_after > 0
    {
        return "potion_required_to_survive";
    }
    if hp_after * 100 >= player_hp.max(1) * 60 {
        "safe_high_hp"
    } else {
        "safe_low_hp"
    }
}

fn push_failure_cluster(
    clusters: &mut Vec<FailureClusterAgg>,
    label: &'static str,
    hp_after: i32,
    damage_done: i32,
    enemy_hp: i32,
    representative_branch: Vec<String>,
) {
    if let Some(existing) = clusters.iter_mut().find(|cluster| cluster.label == label) {
        existing.count += 1;
        existing.hp_sum += hp_after;
        existing.damage_sum += damage_done;
        existing.enemy_hp_sum += enemy_hp;
        if existing.representative_branch.is_empty() {
            existing.representative_branch = representative_branch;
        }
        return;
    }
    clusters.push(FailureClusterAgg {
        label,
        count: 1,
        hp_sum: hp_after,
        damage_sum: damage_done,
        enemy_hp_sum: enemy_hp,
        representative_branch,
    });
}

fn combat_search_reliability(
    probe: &CombatTurnPlanProbeReport,
    horizon_turns: usize,
    particles: usize,
) -> Value {
    let limits = &probe.probe_limits;
    let nodes_pruned = limits.pruned_by_budget
        + limits.pruned_by_dominated_state
        + limits.pruned_by_optimistic_bound
        + limits.pruned_as_equivalent
        + limits.pruned_by_abstract_equivalence
        + limits.pruned_by_generation_canonical_order
        + limits.pruned_by_plan_expansion_gate;
    let budget_exhausted = limits.pruned_by_budget > 0 || limits.nodes_expanded >= limits.max_nodes;
    let confidence_level = if budget_exhausted {
        "low"
    } else if probe.sequence_classes.is_empty() {
        "low"
    } else if probe
        .truth_warnings
        .iter()
        .any(|warning| warning.contains("random") || warning.contains("budget"))
    {
        "medium"
    } else {
        "high"
    };
    json!({
        "horizon_turns": horizon_turns,
        "nodes_expanded": limits.nodes_expanded,
        "nodes_pruned": nodes_pruned,
        "particle_count": particles,
        "particles_evaluated": 0,
        "budget_exhausted": budget_exhausted,
        "used_exact_draw_enumeration": false,
        "used_sampled_draw_particles": false,
        "unsupported_randomness": [
            "next_turn_draw_particles_not_evaluated_v1",
            "enemy_intent_particles_not_implemented_v1",
            "rng_particles_not_implemented_v1"
        ],
        "depth_is_budget_guard": true,
        "conclusion_scope": "partial_budget_limited_evidence_not_global_optimality",
        "confidence_level": confidence_level,
    })
}

fn combat_search_truth_warnings(probe: &CombatTurnPlanProbeReport) -> Vec<String> {
    let mut warnings = probe.truth_warnings.clone();
    push_unique_string(
        &mut warnings,
        "combat_search_engine_v1_derives_vectors_from_current_turn_probe".to_string(),
    );
    push_unique_string(
        &mut warnings,
        "next_turn_draw_particles_reserved_but_not_evaluated_v1".to_string(),
    );
    push_unique_string(
        &mut warnings,
        "pareto_frontier_is_diagnostic_not_final_action".to_string(),
    );
    push_unique_string(
        &mut warnings,
        "effective_max_depth_is_budget_guard_not_decision_horizon_claim".to_string(),
    );
    push_unique_string(
        &mut warnings,
        "search_geometry_reports_merged_clusters_and_unresolved_frontiers".to_string(),
    );
    warnings
}

#[derive(Clone, Debug)]
struct OrderSensitivityAgg {
    action_multiset: String,
    count: usize,
    representative_orders: Vec<Vec<String>>,
    damage_min: i32,
    damage_max: i32,
    enemy_hp_min: i32,
    enemy_hp_max: i32,
    hp_loss_min: i32,
    hp_loss_max: i32,
    reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct AbstractClusterAgg {
    key: String,
    count: usize,
    hp_sum: i32,
    hp_min: i32,
    hp_max: i32,
    damage_min: i32,
    damage_max: i32,
    enemy_hp_min: i32,
    enemy_hp_max: i32,
    lethal_count: usize,
    fatal_count: usize,
    representative_branch: Vec<String>,
}

fn build_search_geometry_report(
    probe: &CombatTurnPlanProbeReport,
    roots: &[RootActionMetrics],
    dominated_pairs: &[(String, String)],
    particles: usize,
) -> Value {
    json!({
        "schema_name": "SearchGeometry",
        "schema_version": 1,
        "scope": "combat_current_budget",
        "decision_authority": "evidence_only",
        "not_final_action": true,
        "budget_model": {
            "kind": "anytime_frontier_budget",
            "max_nodes": probe.probe_limits.max_nodes,
            "nodes_expanded": probe.probe_limits.nodes_expanded,
            "depth_semantics": "depth_is_a_budget_guard_not_a_claim_that_long_range_dependencies_are_resolved",
        },
        "order_sensitivity": build_order_sensitivity_report(probe),
        "abstract_state_clusters": build_abstract_state_clusters(probe),
        "frontier_queue": build_frontier_priority_queue(roots, dominated_pairs),
        "unresolved_frontier": build_unresolved_frontier_report(probe, roots, particles),
        "belief_particle_status": {
            "draw_particles_requested": particles,
            "draw_particles_evaluated": 0,
            "enemy_intent_particles": "not_implemented_v1",
            "rng_particles": "not_implemented_v1",
            "particle_clustering": "reserved_for_tactical_outcome_clusters",
        },
        "refinement_policy": [
            "expand death/lethal boundaries before stable low-variance branches",
            "split abstract clusters when hp/damage/enemy_hp spread crosses thresholds",
            "preserve action orders when symbolic effects or empirical outcome spread indicate order sensitivity",
            "merge only conservative abstract states and report max outcome spread"
        ],
    })
}

fn build_order_sensitivity_report(probe: &CombatTurnPlanProbeReport) -> Value {
    let mut groups: std::collections::BTreeMap<String, OrderSensitivityAgg> =
        std::collections::BTreeMap::new();
    for sequence in &probe.sequence_classes {
        let action_keys = sequence_action_keys(sequence);
        if action_keys.len() < 2 {
            continue;
        }
        let mut sorted = action_keys.clone();
        sorted.sort();
        let key = sorted.join(" + ");
        let hp_loss = sequence_hp_loss(sequence);
        let reasons = order_sensitivity_reasons(&action_keys);
        let entry = groups
            .entry(key.clone())
            .or_insert_with(|| OrderSensitivityAgg {
                action_multiset: key,
                count: 0,
                representative_orders: Vec::new(),
                damage_min: sequence.outcome.damage_done,
                damage_max: sequence.outcome.damage_done,
                enemy_hp_min: sequence.outcome.total_monster_hp,
                enemy_hp_max: sequence.outcome.total_monster_hp,
                hp_loss_min: hp_loss,
                hp_loss_max: hp_loss,
                reasons: Vec::new(),
            });
        entry.count += 1;
        if entry.representative_orders.len() < 3 {
            entry.representative_orders.push(action_keys);
        }
        entry.damage_min = entry.damage_min.min(sequence.outcome.damage_done);
        entry.damage_max = entry.damage_max.max(sequence.outcome.damage_done);
        entry.enemy_hp_min = entry.enemy_hp_min.min(sequence.outcome.total_monster_hp);
        entry.enemy_hp_max = entry.enemy_hp_max.max(sequence.outcome.total_monster_hp);
        entry.hp_loss_min = entry.hp_loss_min.min(hp_loss);
        entry.hp_loss_max = entry.hp_loss_max.max(hp_loss);
        for reason in reasons {
            push_unique_string(&mut entry.reasons, reason);
        }
    }
    let mut items = groups.into_values().collect::<Vec<_>>();
    items.sort_by(|a, b| {
        order_sensitivity_rank(b)
            .cmp(&order_sensitivity_rank(a))
            .then_with(|| b.count.cmp(&a.count))
    });
    let sensitive_count = items
        .iter()
        .filter(|item| order_sensitivity_rank(item) > 0)
        .count();
    json!({
        "method": "symbolic_effect_flags_plus_observed_outcome_spread",
        "groups_total": items.len(),
        "sensitive_or_potentially_sensitive_groups": sensitive_count,
        "items": items
            .into_iter()
            .take(8)
            .map(order_sensitivity_item_json)
            .collect::<Vec<_>>(),
    })
}

fn order_sensitivity_item_json(item: OrderSensitivityAgg) -> Value {
    let empirical_spread = item.damage_max != item.damage_min
        || item.enemy_hp_max != item.enemy_hp_min
        || item.hp_loss_max != item.hp_loss_min;
    let status = if empirical_spread {
        "observed_order_sensitive"
    } else if !item.reasons.is_empty() {
        "potentially_order_sensitive"
    } else {
        "order_invariant_under_observed_probe"
    };
    json!({
        "action_multiset": item.action_multiset,
        "status": status,
        "observed_order_count": item.count,
        "representative_orders": item.representative_orders,
        "sensitivity_axes": {
            "damage_done_spread": item.damage_max - item.damage_min,
            "enemy_hp_remaining_spread": item.enemy_hp_max - item.enemy_hp_min,
            "hp_loss_spread": item.hp_loss_max - item.hp_loss_min,
        },
        "symbolic_reasons": item.reasons,
    })
}

fn order_sensitivity_rank(item: &OrderSensitivityAgg) -> i32 {
    let empirical_spread = item.damage_max != item.damage_min
        || item.enemy_hp_max != item.enemy_hp_min
        || item.hp_loss_max != item.hp_loss_min;
    if empirical_spread {
        2
    } else if !item.reasons.is_empty() {
        1
    } else {
        0
    }
}

fn order_sensitivity_reasons(action_keys: &[String]) -> Vec<String> {
    let joined = action_keys
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let mut reasons = Vec::new();
    if joined.contains("bash") || joined.contains("vulnerable") || joined.contains("weak") {
        reasons.push("debuff_before_damage_or_survival_can_change_outcome".to_string());
    }
    if joined.contains("draw") {
        reasons.push("draw_effect_changes_future_action_set".to_string());
    }
    if joined.contains("energy") || joined.contains("bloodletting") || joined.contains("seeing_red")
    {
        reasons.push("energy_generation_order_sensitive".to_string());
    }
    if joined.contains("potion") {
        reasons.push("potion_resource_branch_order_sensitive".to_string());
    }
    if joined.contains("exhaust") || joined.contains("discard") {
        reasons.push("hand_or_deck_mutation_order_sensitive".to_string());
    }
    if joined.contains("kill") || joined.contains("monster_slot") {
        reasons.push("target_hp_and_kill_order_may_change_followup_value".to_string());
    }
    reasons
}

fn build_abstract_state_clusters(probe: &CombatTurnPlanProbeReport) -> Value {
    let mut clusters: std::collections::BTreeMap<String, AbstractClusterAgg> =
        std::collections::BTreeMap::new();
    let player_hp = probe.state_summary.player_hp;
    for sequence in &probe.sequence_classes {
        let hp_loss = sequence_hp_loss(sequence);
        let hp_after = player_hp - hp_loss;
        let lethal =
            sequence.outcome.living_monster_count == 0 || sequence.outcome.total_monster_hp <= 0;
        let fatal = hp_after <= 0;
        let key = abstract_cluster_key(
            player_hp,
            hp_after,
            sequence.outcome.total_monster_hp,
            sequence.outcome.remaining_hand_count,
            sequence.diagnostics.strength_projection,
            lethal,
            fatal,
        );
        let entry = clusters
            .entry(key.clone())
            .or_insert_with(|| AbstractClusterAgg {
                key,
                count: 0,
                hp_sum: 0,
                hp_min: hp_after,
                hp_max: hp_after,
                damage_min: sequence.outcome.damage_done,
                damage_max: sequence.outcome.damage_done,
                enemy_hp_min: sequence.outcome.total_monster_hp,
                enemy_hp_max: sequence.outcome.total_monster_hp,
                lethal_count: 0,
                fatal_count: 0,
                representative_branch: sequence.actions.clone(),
            });
        entry.count += 1;
        entry.hp_sum += hp_after;
        entry.hp_min = entry.hp_min.min(hp_after);
        entry.hp_max = entry.hp_max.max(hp_after);
        entry.damage_min = entry.damage_min.min(sequence.outcome.damage_done);
        entry.damage_max = entry.damage_max.max(sequence.outcome.damage_done);
        entry.enemy_hp_min = entry.enemy_hp_min.min(sequence.outcome.total_monster_hp);
        entry.enemy_hp_max = entry.enemy_hp_max.max(sequence.outcome.total_monster_hp);
        if lethal {
            entry.lethal_count += 1;
        }
        if fatal {
            entry.fatal_count += 1;
        }
    }
    let mut items = clusters.into_values().collect::<Vec<_>>();
    items.sort_by(|a, b| {
        cluster_refinement_rank(b)
            .cmp(&cluster_refinement_rank(a))
            .then_with(|| b.count.cmp(&a.count))
    });
    let refine_count = items
        .iter()
        .filter(|item| cluster_needs_refinement(item))
        .count();
    json!({
        "method": "conservative_symbolic_outcome_buckets",
        "clusters_total": items.len(),
        "clusters_needing_refinement": refine_count,
        "items": items
            .into_iter()
            .take(12)
            .map(abstract_cluster_item_json)
            .collect::<Vec<_>>(),
    })
}

fn abstract_cluster_item_json(cluster: AbstractClusterAgg) -> Value {
    let count = cluster.count.max(1);
    let needs_refinement = cluster_needs_refinement(&cluster);
    json!({
        "cluster_key": cluster.key,
        "branch_count": cluster.count,
        "merge_basis": "terminality|hp_bucket|enemy_hp_bucket|hand_bucket|mechanic_bucket",
        "representative_branch": cluster.representative_branch,
        "centroid": {
            "hp_after_mean": cluster.hp_sum as f64 / count as f64,
        },
        "max_outcome_spread": {
            "hp_after_spread": cluster.hp_max - cluster.hp_min,
            "damage_done_spread": cluster.damage_max - cluster.damage_min,
            "enemy_hp_remaining_spread": cluster.enemy_hp_max - cluster.enemy_hp_min,
        },
        "refinement": {
            "needed": needs_refinement,
            "reason": if needs_refinement {
                "high_outcome_spread_or_terminality_disagreement"
            } else {
                "outcome_spread_within_conservative_bucket_thresholds"
            },
        },
    })
}

fn cluster_needs_refinement(cluster: &AbstractClusterAgg) -> bool {
    let terminality_disagreement = (cluster.lethal_count > 0
        && cluster.lethal_count < cluster.count)
        || (cluster.fatal_count > 0 && cluster.fatal_count < cluster.count);
    terminality_disagreement
        || cluster.hp_max - cluster.hp_min >= 12
        || cluster.damage_max - cluster.damage_min >= 20
        || cluster.enemy_hp_max - cluster.enemy_hp_min >= 20
}

fn cluster_refinement_rank(cluster: &AbstractClusterAgg) -> i32 {
    if cluster_needs_refinement(cluster) {
        1
    } else {
        0
    }
}

fn abstract_cluster_key(
    player_hp: i32,
    hp_after: i32,
    enemy_hp: i32,
    remaining_hand_count: usize,
    strength_projection: i32,
    lethal: bool,
    fatal: bool,
) -> String {
    let terminal = if fatal {
        "fatal"
    } else if lethal {
        "lethal"
    } else {
        "ongoing"
    };
    format!(
        "{terminal}|hp:{}|enemy:{}|hand:{}|mechanic:{}",
        hp_bucket(player_hp, hp_after),
        enemy_hp_bucket(enemy_hp),
        hand_bucket(remaining_hand_count),
        if strength_projection > 0 {
            "scaling_pressure"
        } else {
            "none"
        },
    )
}

fn hp_bucket(player_hp: i32, hp_after: i32) -> &'static str {
    if hp_after <= 0 {
        "dead"
    } else if hp_after * 100 <= player_hp.max(1) * 25 {
        "critical"
    } else if hp_after * 100 <= player_hp.max(1) * 50 {
        "low"
    } else if hp_after * 100 <= player_hp.max(1) * 75 {
        "stable"
    } else {
        "high"
    }
}

fn enemy_hp_bucket(enemy_hp: i32) -> &'static str {
    if enemy_hp <= 0 {
        "dead"
    } else if enemy_hp <= 12 {
        "low"
    } else if enemy_hp <= 35 {
        "medium"
    } else {
        "high"
    }
}

fn hand_bucket(remaining_hand_count: usize) -> &'static str {
    match remaining_hand_count {
        0 => "empty",
        1 | 2 => "thin",
        _ => "cards_remaining",
    }
}

fn build_frontier_priority_queue(
    roots: &[RootActionMetrics],
    dominated_pairs: &[(String, String)],
) -> Value {
    let dominated = dominated_pairs
        .iter()
        .map(|(action, _)| action.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut items = roots
        .iter()
        .map(|root| frontier_priority_item(root, &dominated))
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        b.get("priority_score")
            .and_then(Value::as_f64)
            .partial_cmp(&a.get("priority_score").and_then(Value::as_f64))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    json!({
        "method": "risk_variance_leverage_priority",
        "items": items.into_iter().take(12).collect::<Vec<_>>(),
    })
}

fn frontier_priority_item(
    root: &RootActionMetrics,
    dominated: &std::collections::BTreeSet<String>,
) -> Value {
    let mut score = 0.0;
    let mut components = Vec::new();
    if root.branch_count == 0 {
        score += 100.0;
        components.push("unexpanded_root");
    }
    if root.survival_rate > 0.0 && root.fatality_rate > 0.0 {
        score += 90.0;
        components.push("death_boundary");
    }
    if root.lethal_now {
        score += 80.0;
        components.push("lethal_boundary");
    }
    if !dominated.contains(&root.action_key) && root.branch_count > 0 {
        score += 45.0;
        components.push("pareto_competitor");
    }
    if root.tail_risk > 0.0 || root.branch_entropy > 1.0 {
        score += root.tail_risk.min(50.0) + root.branch_entropy.min(5.0);
        components.push("variance_or_tail_risk");
    }
    if root.potion_cost > 0.0 || root.hp_cost > 0.0 {
        score += 12.0;
        components.push("resource_swing");
    }
    json!({
        "root_action_key": root.action_key,
        "priority_score": score,
        "priority_components": components,
        "current_evidence": {
            "branch_count": root.branch_count,
            "survival_rate": root.survival_rate,
            "fatality_rate": root.fatality_rate,
            "lethal_now": root.lethal_now,
            "tail_risk": root.tail_risk,
        },
    })
}

fn build_unresolved_frontier_report(
    probe: &CombatTurnPlanProbeReport,
    roots: &[RootActionMetrics],
    particles: usize,
) -> Value {
    let mut items = Vec::new();
    for root in roots.iter().filter(|root| root.branch_count == 0) {
        items.push(json!({
            "kind": "unexpanded_root_action",
            "root_action_key": root.action_key,
            "reason": "no_sequence_class_observed_for_root_under_current_budget_or_probe_limits",
        }));
    }
    let limits = &probe.probe_limits;
    if limits.pruned_by_budget > 0 || limits.nodes_expanded >= limits.max_nodes {
        items.push(json!({
            "kind": "budget_exhausted",
            "reason": "node_budget_or_budget_pruning_hit",
            "nodes_expanded": limits.nodes_expanded,
            "max_nodes": limits.max_nodes,
            "pruned_by_budget": limits.pruned_by_budget,
        }));
    }
    let order_sensitive = count_order_sensitive_groups(probe);
    if order_sensitive > 0 {
        items.push(json!({
            "kind": "order_sensitive_frontier",
            "reason": "some action multisets are potentially or empirically order sensitive and may need refinement",
            "group_count": order_sensitive,
        }));
    }
    let refine_clusters = count_refinement_clusters(probe);
    if refine_clusters > 0 {
        items.push(json!({
            "kind": "abstract_cluster_refinement_needed",
            "reason": "merged clusters contain high outcome spread or terminality disagreement",
            "cluster_count": refine_clusters,
        }));
    }
    if particles > 0 {
        items.push(json!({
            "kind": "belief_particles_not_evaluated",
            "reason": "draw/enemy/rng belief particles are reserved but not evaluated in this report",
            "particles_requested": particles,
        }));
    }
    let unresolved_count = items.len();
    json!({
        "items": items,
        "unresolved_count": unresolved_count,
        "conclusion": if unresolved_count == 0 {
            "no_unresolved_frontier_detected_by_current_report"
        } else {
            "search_result_is_partial_evidence_with_unresolved_frontiers"
        },
    })
}

fn count_order_sensitive_groups(probe: &CombatTurnPlanProbeReport) -> usize {
    let report = build_order_sensitivity_report(probe);
    report
        .get("sensitive_or_potentially_sensitive_groups")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn count_refinement_clusters(probe: &CombatTurnPlanProbeReport) -> usize {
    let report = build_abstract_state_clusters(probe);
    report
        .get("clusters_needing_refinement")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn sequence_action_keys(sequence: &CombatPlanSequenceClass) -> Vec<String> {
    if !sequence.action_keys.is_empty() {
        sequence.action_keys.clone()
    } else {
        sequence.actions.clone()
    }
}

fn sequence_hp_loss(sequence: &CombatPlanSequenceClass) -> i32 {
    sequence
        .outcome
        .projected_unblocked_damage
        .max(sequence.outcome.hp_loss_actual)
        .max(0)
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
