use crate::state::core::{EngineState, RunPendingChoiceState, RunPendingChoiceReason};
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, return_state: EngineState) -> Option<EngineState> {
    let purgeable_count = run_state.master_deck.len();
    if purgeable_count > 0 {
        return Some(EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: purgeable_count.min(2),
            max_choices: purgeable_count.min(2),
            reason: RunPendingChoiceReason::Purge, // Purge reason natively deletes selected cards
            return_state: Box::new(return_state),
        }));
    }
    None
}
