use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use crate::types::{exact_hash, TurnOptionAction};

#[derive(Clone, Debug)]
pub struct ExactAtomicWitness {
    pub actions: Vec<TurnOptionAction>,
    pub final_position: CombatPosition,
    pub negative_log_policy: f64,
    pub replay_engine_steps: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AtomicWitnessReplayError {
    IllegalInput { action_index: usize },
    TransitionStepLimit { action_index: usize },
    SuccessorMismatch { action_index: usize },
}

pub(crate) fn replay_atomic_actions(
    stepper: &dyn CombatStepper,
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<(CombatPosition, usize), AtomicWitnessReplayError> {
    let mut position = root.clone();
    let mut engine_steps = 0usize;
    for (action_index, action) in actions.iter().enumerate() {
        if !stepper.is_legal_action(&position, &action.input) {
            return Err(AtomicWitnessReplayError::IllegalInput { action_index });
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        engine_steps = engine_steps.saturating_add(result.engine_steps);
        if result.truncated {
            return Err(AtomicWitnessReplayError::TransitionStepLimit { action_index });
        }
        if exact_hash(&result.position) != action.expected_successor_hash.as_str() {
            return Err(AtomicWitnessReplayError::SuccessorMismatch { action_index });
        }
        position = result.position;
    }
    Ok((position, engine_steps))
}
