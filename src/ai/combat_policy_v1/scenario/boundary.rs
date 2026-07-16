use crate::sim::combat::CombatPosition;
use crate::state::core::EngineState;

use super::super::combat_policy_observation_v1;
use super::pending_choice::{pending_choice_kind, public_pending_choice_observation};
use super::types::{
    CombatPolicyObservationEnvelopeV1, CombatScenarioPolicyErrorV1,
    COMBAT_POLICY_INFORMATION_SET_SCHEMA_NAME, COMBAT_POLICY_INFORMATION_SET_SCHEMA_VERSION,
};

pub(super) fn policy_observation_envelope(
    scenario_id: &str,
    position: &CombatPosition,
) -> Result<CombatPolicyObservationEnvelopeV1, CombatScenarioPolicyErrorV1> {
    let (engine_state, pending_choice) = match &position.engine {
        EngineState::CombatPlayerTurn => {
            let pending_work = non_quiescent_player_turn_work(&position.combat);
            if !pending_work.is_empty() {
                return Err(CombatScenarioPolicyErrorV1::NonQuiescentBoundary {
                    scenario_id: scenario_id.to_string(),
                    pending_work,
                });
            }
            ("combat_player_turn", None)
        }
        EngineState::PendingChoice(choice) => {
            let public_choice = public_pending_choice_observation(&position.combat, choice)
                .map_err(|detail| CombatScenarioPolicyErrorV1::InvalidPendingChoice {
                    scenario_id: scenario_id.to_string(),
                    choice_kind: pending_choice_kind(choice),
                    detail,
                })?;
            ("combat_pending_choice", Some(public_choice))
        }
        other => {
            return Err(CombatScenarioPolicyErrorV1::UnsupportedBoundary {
                scenario_id: scenario_id.to_string(),
                engine_state: format!("{other:?}"),
            });
        }
    };

    Ok(CombatPolicyObservationEnvelopeV1 {
        schema_name: COMBAT_POLICY_INFORMATION_SET_SCHEMA_NAME.to_string(),
        schema_version: COMBAT_POLICY_INFORMATION_SET_SCHEMA_VERSION,
        engine_state: engine_state.to_string(),
        turn_count: position.combat.turn.turn_count,
        observation: combat_policy_observation_v1(&position.combat),
        pending_choice,
    })
}

fn non_quiescent_player_turn_work(combat: &crate::runtime::combat::CombatState) -> Vec<String> {
    let mut pending = Vec::new();
    if !combat.engine.action_queue.is_empty() {
        pending.push("action_queue".to_string());
    }
    if !combat.zones.queued_cards.is_empty() {
        pending.push("queued_cards".to_string());
    }
    if !combat.zones.limbo.is_empty() {
        pending.push("limbo".to_string());
    }
    if combat.runtime.using_card {
        pending.push("using_card".to_string());
    }
    if !combat.runtime.card_queue.is_empty() {
        pending.push("runtime_card_queue".to_string());
    }
    pending
}
