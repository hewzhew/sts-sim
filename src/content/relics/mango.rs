use crate::action::ActionInfo;
use crate::state::core::EngineState;
use crate::state::run::RunState;
use smallvec::SmallVec;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    run_state.max_hp += 14;
    run_state.current_hp = (run_state.current_hp + 14).min(run_state.max_hp);
    None
}

pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    SmallVec::new()
}
