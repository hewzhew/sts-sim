use crate::rewards::state::BossRelicChoiceState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn handle(
    run_state: &mut RunState,
    state: &mut BossRelicChoiceState,
    input: Option<ClientInput>,
) -> Option<EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::SubmitRelicChoice(idx) => {
                if idx < state.relics.len() {
                    let chosen_relic = state.relics[idx];

                    // apply_on_obtain_effect might trigger a PendingChoice (e.g. for Calling Bell or Astrolabe),
                    // which we then wrap and return. When the inner state resolves, it will surface
                    // the fallback state we give it. We will use EngineState::MapNavigation as default,
                    // but we must remember to advance_act() BEFORE taking the relic? Or after?
                    // Java: advance_act usually happens on entering next floor / leaving Boss Room.
                    // We can safely advance act here because boss reward is over.

                    run_state.advance_act();

                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        chosen_relic,
                        EngineState::MapNavigation,
                        DomainEventSource::BossRelicChoice,
                    ) {
                        return Some(next_state);
                    }

                    return Some(EngineState::MapNavigation);
                }
            }
            ClientInput::Proceed | ClientInput::Cancel => {
                // Skipping Boss Relic
                run_state.advance_act();
                return Some(EngineState::MapNavigation);
            }
            _ => {}
        }
    }
    None
}
