use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    // +7 maxHP, heal proportionally
    run_state.max_hp += 7;
    run_state.current_hp = (run_state.current_hp + 7).min(run_state.max_hp);
    None
}
