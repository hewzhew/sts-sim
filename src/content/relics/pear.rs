use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    run_state.max_hp += 10;
    run_state.current_hp = (run_state.current_hp + 10).min(run_state.max_hp);
    None
}
