use crate::state::core::{EngineState, RunPendingChoiceState, RunPendingChoiceReason};
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, return_state: EngineState) -> Option<EngineState> {
    if !run_state.master_deck.is_empty() {
        return Some(EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Duplicate,
            return_state: Box::new(return_state),
        }));
    }
    None
}
