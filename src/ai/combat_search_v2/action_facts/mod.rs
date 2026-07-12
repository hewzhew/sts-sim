use crate::sim::combat::CombatStepResult;
#[cfg(test)]
use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::*;

mod card;
mod delta;
mod mechanics;
mod payload;
mod target;
mod types;
use card::card_facts;
use delta::{action_kind, exact_delta_facts_from_step};
use mechanics::immediate_and_mechanics_facts;
pub use types::{
    CombatSearchV2ActionAccessMechanicsFacts, CombatSearchV2ActionCardFacts,
    CombatSearchV2ActionDerivedMechanicsFacts, CombatSearchV2ActionDirectMechanicsFacts,
    CombatSearchV2ActionExactDeltaFacts, CombatSearchV2ActionFacts,
    CombatSearchV2ActionImmediateFacts, CombatSearchV2ActionMechanicsFacts,
    CombatSearchV2ActionReactiveMechanicsFacts, CombatSearchV2ActionResourceTimingFacts,
    CombatSearchV2ActionTargetFacts, CombatSearchV2TimedEnemyThreatTargetFacts,
};

#[cfg(test)]
pub(super) fn summarize_action_facts(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    stepper: &impl CombatStepper,
    max_engine_steps: usize,
) -> CombatSearchV2ActionFacts {
    let step = stepper.apply_to_stable(
        &CombatPosition::new(engine.clone(), combat.clone()),
        input.clone(),
        CombatStepLimits {
            max_engine_steps,
            deadline: None,
        },
    );
    summarize_action_facts_from_step(combat, input, &step)
}

pub(super) fn summarize_action_facts_from_step(
    combat: &CombatState,
    input: &ClientInput,
    step: &CombatStepResult,
) -> CombatSearchV2ActionFacts {
    let card = card_facts(combat, input);
    let target = target::target_facts(combat, input);
    let (immediate, mechanics) = immediate_and_mechanics_facts(combat, input, card.as_ref());
    let exact_one_step_delta = exact_delta_facts_from_step(combat, step);

    CombatSearchV2ActionFacts {
        action_kind: action_kind(input),
        card,
        target,
        immediate: CombatSearchV2ActionImmediateFacts {
            creates_pending_choice_after_one_step: exact_one_step_delta.pending_choice_present,
            ..immediate
        },
        mechanics,
        exact_one_step_delta,
    }
}

#[cfg(test)]
mod tests;
