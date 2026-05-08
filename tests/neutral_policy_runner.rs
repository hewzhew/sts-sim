use std::fs;

use sts_simulator::app::policy_runner::NeutralProbeEvaluator;
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::runtime::combat::{CombatCard, CombatState};
use sts_simulator::state::core::ClientInput;
use sts_simulator::state::EngineState;
use sts_simulator::test_support::{blank_test_combat, planned_monster};
use sts_simulator::verification::decision_env::{
    ActionCandidate, ActionId, DecisionId, ObservationPayload, ObservationVisibility, PolicyInput,
    TimeStep, POLICY_INPUT_SCHEMA_VERSION,
};
use sts_simulator::verification::neutral_engine_query::SearchExecutionContext;
use sts_simulator::verification::search_policy::DecisionMode;

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn cultist_combat() -> CombatState {
    cultist_combat_with_hp(48)
}

fn cultist_combat_with_hp(hp: i32) -> CombatState {
    let mut combat = blank_test_combat();
    combat.turn.energy = 3;
    combat.turn.turn_count = 2;
    let mut cultist = planned_monster(EnemyId::Cultist, 1);
    cultist.current_hp = hp;
    combat.entities.monsters.push(cultist);
    combat.zones.hand.push(card(CardId::Strike, 1));
    combat.zones.hand.push(card(CardId::Strike, 2));
    combat.zones.hand.push(card(CardId::Defend, 3));
    combat
}

fn decision_id() -> DecisionId {
    DecisionId {
        episode_id: "neutral-policy-test".to_string(),
        step_index: 0,
        decision_type: "combat".to_string(),
    }
}

fn candidates() -> Vec<ClientInput> {
    vec![
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        ClientInput::PlayCard {
            card_index: 1,
            target: Some(1),
        },
        ClientInput::PlayCard {
            card_index: 2,
            target: None,
        },
    ]
}

fn policy_input(inputs: &[ClientInput]) -> PolicyInput {
    let timestep = TimeStep {
        contract_version: "decision_env_contract_v0".to_string(),
        decision_id: decision_id(),
        observation: ObservationPayload {
            schema_version: "neutral_policy_test_public_obs_v0".to_string(),
            visibility: ObservationVisibility::Public,
            decision_type: "combat".to_string(),
            payload: serde_json::json!({"fixture": "cultist_combat"}),
        },
        candidates: inputs
            .iter()
            .enumerate()
            .map(|(index, input)| ActionCandidate {
                id: ActionId(index),
                action_schema_version: "neutral_policy_test_action_v0".to_string(),
                action_index: index,
                action_key: format!("{input:?}"),
                action_kind: match input {
                    ClientInput::PlayCard { .. } => "play_card",
                    ClientInput::UsePotion { .. } => "use_potion",
                    ClientInput::EndTurn => "end_turn",
                    _ => "other",
                }
                .to_string(),
                payload: serde_json::Value::Null,
            })
            .collect(),
        reward: sts_simulator::verification::decision_env::RewardEvent {
            schema_version: "reward_event_v0".to_string(),
            scalar_reward: 0.0,
            components: serde_json::Value::Null,
        },
        terminated: false,
        truncated: false,
        info: sts_simulator::verification::decision_env::StepInfo {
            state_hash: String::new(),
            payload: serde_json::Value::Null,
        },
    };
    PolicyInput::from_timestep(&timestep, 100).unwrap()
}

fn policy_input_with_action_views(
    inputs: &[ClientInput],
    kinds: &[&str],
    keys: &[&str],
) -> PolicyInput {
    let timestep = TimeStep {
        contract_version: "decision_env_contract_v0".to_string(),
        decision_id: decision_id(),
        observation: ObservationPayload {
            schema_version: "neutral_policy_test_public_obs_v0".to_string(),
            visibility: ObservationVisibility::Public,
            decision_type: "combat".to_string(),
            payload: serde_json::json!({"fixture": "cultist_combat"}),
        },
        candidates: inputs
            .iter()
            .enumerate()
            .map(|(index, _input)| ActionCandidate {
                id: ActionId(index),
                action_schema_version: "neutral_policy_test_action_v0".to_string(),
                action_index: index,
                action_key: keys[index].to_string(),
                action_kind: kinds[index].to_string(),
                payload: serde_json::Value::Null,
            })
            .collect(),
        reward: sts_simulator::verification::decision_env::RewardEvent {
            schema_version: "reward_event_v0".to_string(),
            scalar_reward: 0.0,
            components: serde_json::Value::Null,
        },
        terminated: false,
        truncated: false,
        info: sts_simulator::verification::decision_env::StepInfo {
            state_hash: String::new(),
            payload: serde_json::Value::Null,
        },
    };
    PolicyInput::from_timestep(&timestep, 100).unwrap()
}

#[test]
fn neutral_runner_uses_effect_groups_without_legacy_or_exact() {
    let inputs = candidates();
    let combat = cultist_combat();
    let input = policy_input_with_action_views(
        &inputs,
        &["play_card", "play_card", "play_card"],
        &[
            "combat/play_card/card:Strike/hand:0/target:monster_slot:0",
            "combat/play_card/card:Strike/hand:1/target:monster_slot:0",
            "combat/play_card/card:Defend/hand:2",
        ],
    );
    assert_eq!(input.schema_version, POLICY_INPUT_SCHEMA_VERSION);
    let context = SearchExecutionContext::from_policy_input(
        &input,
        EngineState::CombatPlayerTurn,
        combat,
        inputs,
    );
    let runner = NeutralProbeEvaluator::default();
    let trace = runner.deliberate(&input, &context);

    assert_eq!(trace.decision.mode, DecisionMode::EvidenceInsufficient);
    assert_eq!(trace.decision.selected_action_id, None);
    assert_eq!(
        trace.decision.fallback_reason.as_deref(),
        Some("neutral_runner_signal_only")
    );
    assert_eq!(
        trace
            .decision
            .payload
            .pointer("/controller_decision")
            .and_then(|value| value.as_str()),
        Some("abstain")
    );
    assert_eq!(
        trace
            .decision
            .payload
            .pointer("/short_horizon_signal_candidate_id")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
    assert_eq!(trace.proposal.policy_id, "neutral_probe_evaluator_v1");
    assert!(trace.evidence.iter().all(|evidence| matches!(
        evidence.search_kind,
        sts_simulator::verification::search_policy::SearchKind::NeutralBranchCompression { .. }
    )));
    assert_eq!(
        trace
            .search_plan
            .payload
            .pointer("/evaluation_trace/expanded_branch_groups")
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(2)
    );
}

#[test]
fn neutral_runner_keeps_resource_actions_as_evidence_not_selection() {
    let inputs = vec![
        ClientInput::UsePotion {
            potion_index: 0,
            target: Some(1),
        },
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    ];
    let combat = cultist_combat();
    let input = policy_input_with_action_views(
        &inputs,
        &["use_potion", "play_card"],
        &[
            "combat/use_potion/slot:0/target:monster_slot:0",
            "combat/play_card/card:Strike/hand:0/target:monster_slot:0",
        ],
    );
    let context = SearchExecutionContext::from_policy_input(
        &input,
        EngineState::CombatPlayerTurn,
        combat,
        inputs,
    );
    let runner = NeutralProbeEvaluator::default();
    let trace = runner.deliberate(&input, &context);
    let evaluations = trace
        .decision
        .payload
        .pointer("/candidate_evaluations")
        .and_then(|value| value.as_array())
        .expect("candidate evaluations");
    assert_eq!(evaluations.len(), 2);
    assert_eq!(evaluations[0]["resource_action"], true);
    assert_eq!(evaluations[0]["dominance_eligible"], false);
    assert_eq!(evaluations[0]["reason_code"], "resource_ineligible");
    assert_eq!(evaluations[0]["hypothesis_class"], "audit_only");
    assert!(evaluations[0]["risk_buckets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|bucket| bucket == "resource"));
    assert_ne!(trace.decision.selected_action_id, Some(ActionId(0)));
}

#[test]
fn neutral_runner_evaluations_include_audit_classification_fields() {
    let inputs = candidates();
    let combat = cultist_combat();
    let input = policy_input_with_action_views(
        &inputs,
        &["play_card", "play_card", "play_card"],
        &[
            "combat/play_card/card:Strike/hand:0/target:monster_slot:0",
            "combat/play_card/card:Strike/hand:1/target:monster_slot:0",
            "combat/play_card/card:Defend/hand:2",
        ],
    );
    let context = SearchExecutionContext::from_policy_input(
        &input,
        EngineState::CombatPlayerTurn,
        combat,
        inputs,
    );
    let runner = NeutralProbeEvaluator::default();
    let trace = runner.deliberate(&input, &context);
    let evaluations = trace
        .decision
        .payload
        .pointer("/candidate_evaluations")
        .and_then(|value| value.as_array())
        .expect("candidate evaluations");
    let attack = evaluations
        .iter()
        .find(|eval| eval["action_id"] == 0)
        .expect("attack eval");
    assert_eq!(attack["reason_code"], "damage_delta_only");
    assert_eq!(attack["label_role"], "SearchSignalOnly");
    assert_eq!(
        attack["hypothesis_class"],
        "short_horizon_tactical_hypothesis"
    );
    assert_eq!(attack["evidence_scope"], "stable_boundary");
    let block = evaluations
        .iter()
        .find(|eval| eval["action_id"] == 2)
        .expect("block eval");
    assert_eq!(block["reason_code"], "defense_horizon_missing");
}

#[test]
fn neutral_runner_marks_terminal_clear_as_certificate() {
    let inputs = candidates();
    let combat = cultist_combat_with_hp(6);
    let input = policy_input(&inputs);
    let context = SearchExecutionContext::from_policy_input(
        &input,
        EngineState::CombatPlayerTurn,
        combat,
        inputs,
    );
    let runner = NeutralProbeEvaluator::default();
    let trace = runner.deliberate(&input, &context);
    let evaluations = trace
        .decision
        .payload
        .pointer("/candidate_evaluations")
        .and_then(|value| value.as_array())
        .expect("candidate evaluations");
    let terminal = evaluations
        .iter()
        .find(|eval| eval["action_id"] == 0)
        .expect("terminal eval");
    assert_eq!(terminal["reason_code"], "terminal_clear");
    assert_eq!(terminal["hypothesis_class"], "terminal_certificate");
    assert_eq!(terminal["label_role"], "DecisionCertificate");
    assert_eq!(trace.decision.selected_action_id, None);
    assert_eq!(
        trace
            .decision
            .payload
            .pointer("/short_horizon_signal_candidate_id")
            .and_then(|value| value.as_u64()),
        Some(0)
    );
}

#[test]
fn neutral_policy_runner_source_does_not_reference_legacy_or_exact_turn() {
    let source = fs::read_to_string("src/app/policy_runner/mod.rs").unwrap();
    for forbidden in [
        "legacy",
        "frontier",
        "diagnose_root_search",
        "best_line",
        "best_first_input",
        "exact_turn_takeover",
    ] {
        assert!(
            !source.contains(forbidden),
            "neutral policy runner must not reference {forbidden}"
        );
    }
}
