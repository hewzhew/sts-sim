use std::time::Instant;

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::types::{
    exact_hash, supported_boundary, CombatDecisionRoot, CompleteTurnOption,
    CompleteTurnOptionBoundary,
};

#[derive(Clone, Copy, Debug)]
pub struct ReplayLimits {
    pub max_engine_steps: usize,
    pub deadline: Option<Instant>,
}

impl ReplayLimits {
    pub fn deterministic(max_engine_steps: usize) -> Self {
        Self {
            max_engine_steps,
            deadline: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    RootFingerprintMismatch,
    EngineStepBudget,
    Deadline,
    IllegalInput {
        action_index: usize,
    },
    TransitionStepLimit {
        action_index: usize,
    },
    SuccessorMismatch {
        action_index: usize,
    },
    UnsupportedFinalBoundary,
    BoundaryMismatch {
        expected: CompleteTurnOptionBoundary,
        actual: CompleteTurnOptionBoundary,
    },
    FinalSuccessorMismatch,
}

#[derive(Clone, Debug)]
pub struct VerifiedTurnOptionReplay {
    pub position: CombatPosition,
    pub boundary: CompleteTurnOptionBoundary,
    pub engine_steps: usize,
}

pub fn replay_turn_option(
    root: &CombatDecisionRoot,
    option: &CompleteTurnOption,
    stepper: &dyn CombatStepper,
    limits: ReplayLimits,
) -> Result<VerifiedTurnOptionReplay, ReplayError> {
    if root.exact_state_hash() != option.root_exact_state_hash() {
        return Err(ReplayError::RootFingerprintMismatch);
    }
    let mut position = root.position().clone();
    let mut engine_steps = 0usize;

    for (action_index, action) in option.actions().iter().enumerate() {
        if limits
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ReplayError::Deadline);
        }
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(ReplayError::IllegalInput { action_index });
        }
        let remaining = limits.max_engine_steps.saturating_sub(engine_steps);
        let expected_steps = action.engine_steps.max(1);
        if remaining < expected_steps {
            return Err(ReplayError::EngineStepBudget);
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: expected_steps,
                deadline: limits.deadline,
            },
        );
        engine_steps = engine_steps.saturating_add(result.engine_steps);
        if result.timed_out {
            return Err(ReplayError::Deadline);
        }
        if result.truncated {
            return Err(ReplayError::TransitionStepLimit { action_index });
        }
        if exact_hash(&result.position) != action.expected_successor_hash {
            return Err(ReplayError::SuccessorMismatch { action_index });
        }
        position = result.position;
    }

    let Some(boundary) = supported_boundary(root, &position, stepper.terminal(&position)) else {
        return Err(ReplayError::UnsupportedFinalBoundary);
    };
    if boundary != option.boundary() {
        return Err(ReplayError::BoundaryMismatch {
            expected: option.boundary(),
            actual: boundary,
        });
    }
    if position != *option.exact_successor() {
        return Err(ReplayError::FinalSuccessorMismatch);
    }
    Ok(VerifiedTurnOptionReplay {
        position,
        boundary,
        engine_steps,
    })
}
