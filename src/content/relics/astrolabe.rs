use crate::content::cards::{get_card_definition, CardType};
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, return_state: EngineState) -> Option<EngineState> {
    let purgeable_count = run_state
        .master_deck
        .iter()
        .filter(|c| {
            let def = get_card_definition(c.id);
            def.card_type != CardType::Curse
        })
        .count();
    if purgeable_count > 0 {
        return Some(EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: purgeable_count.min(3),
            max_choices: purgeable_count.min(3),
            reason: RunPendingChoiceReason::TransformUpgraded,
            return_state: Box::new(return_state),
        }));
    }
    None
}
