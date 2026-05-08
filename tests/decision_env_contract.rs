use sts_simulator::cli::full_run_smoke::{FullRunEnv, FullRunEnvConfig, RewardShapingProfile};
use sts_simulator::verification::decision_env::{
    DecisionEnv, DecisionRecord, DecisionRecordContext, EnvConfig, ObservationPayload,
    ObservationVisibility, PolicyInput, RunSeed, DECISION_ENV_CONTRACT_VERSION,
    DECISION_RECORD_SCHEMA_VERSION, POLICY_INPUT_SCHEMA_VERSION,
};

fn native_config(seed: u64) -> FullRunEnvConfig {
    FullRunEnvConfig {
        seed,
        ascension: 0,
        final_act: false,
        player_class: "ironclad",
        max_steps: 80,
        reward_shaping_profile: RewardShapingProfile::Baseline,
    }
}

fn contract_config(seed: u64) -> EnvConfig {
    EnvConfig {
        seed,
        ascension: 0,
        final_act: false,
        player_class: "ironclad".to_string(),
        max_steps: 80,
        reward_shaping_profile: "baseline".to_string(),
    }
}

#[test]
fn full_run_env_exposes_decision_env_contract() {
    let mut env = FullRunEnv::new(native_config(7)).expect("env");
    let timestep = DecisionEnv::reset(&mut env, RunSeed(7), contract_config(7)).expect("reset");

    assert_eq!(timestep.contract_version, DECISION_ENV_CONTRACT_VERSION);
    assert_eq!(
        timestep.observation.visibility,
        ObservationVisibility::Public
    );
    assert_eq!(
        timestep.observation.schema_version,
        "full_run_public_observation_v1"
    );
    assert!(timestep.observation.payload.get("plan_profile").is_none());
    assert!(!timestep
        .observation
        .payload
        .to_string()
        .contains("rule_score"));
    assert!(!timestep.candidates.is_empty());
    assert_eq!(
        timestep.candidates[0].action_schema_version,
        "full_run_public_action_candidate_v1"
    );
    assert!(timestep.candidates[0].payload.get("plan_delta").is_none());
    assert!(timestep.candidates[0]
        .payload
        .get("reward_structure")
        .is_none());
    assert_eq!(timestep.reward.scalar_reward, 0.0);
    assert!(!timestep.terminated);
    assert!(!timestep.truncated);

    for (index, candidate) in timestep.candidates.iter().enumerate() {
        assert_eq!(candidate.id.0, candidate.action_index);
        assert_eq!(candidate.id.0, index);
        assert!(!candidate.action_key.is_empty());
        assert!(!candidate.action_kind.is_empty());
    }
}

#[test]
fn full_run_env_snapshot_restore_replays_same_action() {
    let mut env = FullRunEnv::new(native_config(11)).expect("env");
    let timestep = DecisionEnv::reset(&mut env, RunSeed(11), contract_config(11)).expect("reset");
    let action = timestep
        .candidates
        .first()
        .expect("at least one candidate")
        .id;
    let snapshot = DecisionEnv::snapshot(&env).expect("snapshot");

    let first = DecisionEnv::step(&mut env, action).expect("first step");

    DecisionEnv::restore(&mut env, &snapshot).expect("restore");
    let second = DecisionEnv::step(&mut env, action).expect("second step");

    assert_eq!(first.info.state_hash, second.info.state_hash);
    assert_eq!(first.reward.scalar_reward, second.reward.scalar_reward);
    assert_eq!(
        first.observation.decision_type,
        second.observation.decision_type
    );
    assert_eq!(first.candidates.len(), second.candidates.len());
    assert_eq!(
        first
            .candidates
            .iter()
            .map(|candidate| candidate.action_key.clone())
            .collect::<Vec<_>>(),
        second
            .candidates
            .iter()
            .map(|candidate| candidate.action_key.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn decision_record_preserves_canonical_timestep_boundary() {
    let mut env = FullRunEnv::new(native_config(13)).expect("env");
    let timestep = DecisionEnv::reset(&mut env, RunSeed(13), contract_config(13)).expect("reset");
    let mut context = DecisionRecordContext::new("test_sim", "baseline_return_v0", 13);
    context.info = serde_json::json!({"source": "contract_test"});

    let record = DecisionRecord::from_timestep(&timestep, context);

    assert_eq!(record.schema_version, DECISION_RECORD_SCHEMA_VERSION);
    assert_eq!(record.decision_id, timestep.decision_id);
    assert_eq!(record.observation, timestep.observation);
    assert_eq!(record.candidates, timestep.candidates);
    assert_eq!(record.reward_since_prev, timestep.reward);
    assert_eq!(record.state_hash_before, timestep.info.state_hash);
    assert_eq!(record.sim_version, "test_sim");
    assert_eq!(record.return_spec_version, "baseline_return_v0");
    assert_eq!(record.seed, 13);
    assert_eq!(record.info["record_context"]["source"], "contract_test");
}

#[test]
fn decision_record_can_bind_behavior_action_to_outcome() {
    let mut env = FullRunEnv::new(native_config(17)).expect("env");
    let decision = DecisionEnv::reset(&mut env, RunSeed(17), contract_config(17)).expect("reset");
    let action = decision.candidates.first().expect("candidate").id;
    let outcome = DecisionEnv::step(&mut env, action).expect("step");
    let mut context = DecisionRecordContext::new("test_sim", "baseline_return_v0", 17);
    context.behavior_action = Some(action);

    let record = DecisionRecord::from_decision_and_outcome(&decision, &outcome, context);

    assert_eq!(record.schema_version, DECISION_RECORD_SCHEMA_VERSION);
    assert_eq!(record.decision_id, decision.decision_id);
    assert_eq!(record.observation, decision.observation);
    assert_eq!(record.candidates, decision.candidates);
    assert_eq!(record.behavior_action, Some(action));
    assert_eq!(record.reward_since_prev, outcome.reward);
    assert_eq!(record.terminated, outcome.terminated);
    assert_eq!(record.truncated, outcome.truncated);
    assert_eq!(record.state_hash_before, decision.info.state_hash);
    assert_eq!(record.state_hash_after, Some(outcome.info.state_hash));
    assert!(record.info.get("decision_timestep_info").is_some());
    assert!(record.info.get("outcome_timestep_info").is_some());
}

#[test]
fn public_candidate_payload_strips_legacy_heuristic_fields() {
    let mut env = FullRunEnv::new(native_config(1)).expect("env");
    let mut timestep = DecisionEnv::reset(&mut env, RunSeed(1), contract_config(1)).expect("reset");

    for _ in 0..12 {
        if timestep
            .candidates
            .iter()
            .any(|candidate| !candidate.payload["card"].is_null())
        {
            let serialized_candidates =
                serde_json::to_string(&timestep.candidates).expect("serialize candidates");
            assert!(!serialized_candidates.contains("rule_score"));
            assert!(!serialized_candidates.contains("plan_delta"));
            assert!(!serialized_candidates.contains("reward_structure"));
            assert!(!serialized_candidates.contains("dominated"));
            return;
        }
        let action = timestep
            .candidates
            .first()
            .expect("candidate before terminal")
            .id;
        timestep = DecisionEnv::step(&mut env, action).expect("step");
    }

    panic!("seed 1 did not reach a card-bearing candidate within the smoke horizon");
}

#[test]
fn policy_input_exposes_only_public_decision_payload() {
    let mut env = FullRunEnv::new(native_config(19)).expect("env");
    let timestep = DecisionEnv::reset(&mut env, RunSeed(19), contract_config(19)).expect("reset");

    let policy_input = PolicyInput::from_timestep(&timestep, 25).expect("policy input");

    assert_eq!(policy_input.schema_version, POLICY_INPUT_SCHEMA_VERSION);
    assert_eq!(policy_input.decision_id, timestep.decision_id);
    assert_eq!(
        policy_input.observation.visibility,
        ObservationVisibility::Public
    );
    assert_eq!(policy_input.candidates.len(), timestep.candidates.len());
    let serialized = serde_json::to_string(&policy_input).expect("serialize policy input");
    assert!(!serialized.contains("state_hash"));
    assert!(!serialized.contains("teacher_label"));
    assert!(!serialized.contains("timestep_info"));
    assert!(!serialized.contains("rule_score"));
    assert!(!serialized.contains("estimated_role_scores"));
}

#[test]
fn policy_input_rejects_non_public_observation() {
    let mut env = FullRunEnv::new(native_config(23)).expect("env");
    let mut timestep =
        DecisionEnv::reset(&mut env, RunSeed(23), contract_config(23)).expect("reset");
    timestep.observation = ObservationPayload {
        visibility: ObservationVisibility::Debug,
        ..timestep.observation
    };

    let err = PolicyInput::from_timestep(&timestep, 25).expect_err("debug observation rejected");
    assert!(err
        .message
        .contains("policy input requires public observation"));
}
