use std::path::Path;

use serde_json::{json, Value};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::sim::combat::{combat_terminal, CombatPosition, CombatTerminal};
use sts_oracle_runtime::state::core::ClientInput;

use super::super::combat_trace_view::{
    combat_turn_snapshot, target_atomic_policy_trace, AtomicPolicyTraceStepV2,
};
use super::super::exact_turn_corridor::load_action_segments;
use super::super::print_json;
use super::{CombatContractArtifactV2, CombatContractTerminalCandidateV2};

pub(super) struct ReplayedActionTraceV2 {
    pub(super) final_exact_state_hash: String,
    pub(super) final_terminal: CombatTerminal,
    pub(super) final_hp: i32,
    pub(super) policy_trace: Vec<AtomicPolicyTraceStepV2>,
    pub(super) turn_checkpoints: Vec<Value>,
    pub(super) prefix_positions: Vec<CombatPosition>,
}

pub(super) fn replay_actions(
    root: &CombatPosition,
    actions: &[ClientInput],
) -> Result<ReplayedActionTraceV2, String> {
    let (policy_trace, final_exact_state_hash, prefix_positions) =
        target_atomic_policy_trace(root, actions, 250)?;
    let final_position = prefix_positions.last().unwrap_or(root);
    let final_terminal = combat_terminal(&final_position.engine, &final_position.combat);
    let final_hp = final_position.combat.entities.player.current_hp;
    let mut turn_checkpoints = vec![json!({
        "after_action_index": null,
        "terminal": format!("{:?}", combat_terminal(&root.engine, &root.combat)),
        "state": combat_turn_snapshot(root),
    })];
    let mut previous_turn = root.combat.turn.turn_count;
    for (index, position) in prefix_positions.iter().enumerate() {
        let terminal = combat_terminal(&position.engine, &position.combat);
        let current_turn = position.combat.turn.turn_count;
        if current_turn != previous_turn
            || terminal != CombatTerminal::Unresolved
            || index + 1 == prefix_positions.len()
        {
            turn_checkpoints.push(json!({
                "after_action_index": index,
                "terminal": format!("{terminal:?}"),
                "state": combat_turn_snapshot(position),
            }));
        }
        previous_turn = current_turn;
    }
    Ok(ReplayedActionTraceV2 {
        final_exact_state_hash,
        final_terminal,
        final_hp,
        policy_trace,
        turn_checkpoints,
        prefix_positions,
    })
}

pub(super) fn load_root(
    artifact_path: &Path,
    artifact: &CombatContractArtifactV2,
) -> Result<CombatPosition, String> {
    let loaded = load_combat_case(&artifact.request.case)?;
    let root = loaded.position;
    let root_exact_state_hash = combat_exact_state_hash_v2(&root.engine, &root.combat);
    if root_exact_state_hash != artifact.root_exact_state_hash
        || root_exact_state_hash != artifact.request.case_id
    {
        return Err(format!(
            "combat case root drifted: artifact '{}' expects {}, loaded case is {}",
            artifact_path.display(),
            artifact.root_exact_state_hash,
            root_exact_state_hash
        ));
    }
    Ok(root)
}

pub(super) fn replay_candidate(
    root: &CombatPosition,
    candidate: &CombatContractTerminalCandidateV2,
) -> Result<(Vec<ClientInput>, ReplayedActionTraceV2), String> {
    let actions = load_action_segments(std::slice::from_ref(&candidate.actions))?;
    let trace = replay_actions(root, &actions)?;
    if trace.final_terminal != CombatTerminal::Win {
        return Err(format!(
            "artifact candidate '{}' no longer replays to a win: {:?}",
            candidate.candidate_id, trace.final_terminal
        ));
    }
    if candidate.action_count != actions.len()
        || candidate.final_hp != trace.final_hp
        || candidate.terminal_exact_state_hash != trace.final_exact_state_hash
    {
        return Err(format!(
            "artifact candidate '{}' drifted: manifest records actions={}, final_hp={}, final_hash={}; replay produced actions={}, final_hp={}, final_hash={}",
            candidate.candidate_id,
            candidate.action_count,
            candidate.final_hp,
            candidate.terminal_exact_state_hash,
            actions.len(),
            trace.final_hp,
            trace.final_exact_state_hash,
        ));
    }
    Ok((actions, trace))
}

pub(super) fn run(artifact_path: &Path, artifact: &CombatContractArtifactV2) -> Result<(), String> {
    let candidate = artifact
        .terminal_candidates
        .iter()
        .find(|candidate| candidate.selected_by_contract_view)
        .ok_or_else(|| {
            format!(
                "V2 artifact '{}' has no candidate witness to trace",
                artifact_path.display()
            )
        })?;
    let root = load_root(artifact_path, artifact)?;
    let root_exact_state_hash = combat_exact_state_hash_v2(&root.engine, &root.combat);
    let (actions, trace) = replay_candidate(&root, candidate)?;
    if artifact.result.action_count != Some(actions.len())
        || artifact.result.final_hp != Some(trace.final_hp)
    {
        return Err(format!(
            "artifact candidate witness drifted: manifest records actions={:?}, final_hp={:?}; replay produced actions={}, final_hp={}",
            artifact.result.action_count,
            artifact.result.final_hp,
            actions.len(),
            trace.final_hp,
        ));
    }

    print_json(&json!({
        "schema_name": "OracleCombatContractWitnessTraceV2",
        "schema_version": 2,
        "artifact": artifact_path,
        "case_id": artifact.request.case_id,
        "classification": artifact.result.classification,
        "contract_passed": artifact.result.contract_passed,
        "candidate": candidate,
        "replay": {
            "root_exact_state_hash": root_exact_state_hash,
            "final_exact_state_hash": trace.final_exact_state_hash,
            "terminal": format!("{:?}", trace.final_terminal),
            "final_hp": trace.final_hp,
            "action_count": actions.len(),
        },
        "policy_trace": trace.policy_trace,
        "turn_checkpoints": trace.turn_checkpoints,
    }))
}
