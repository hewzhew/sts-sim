use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};

pub(crate) fn get_legal_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    crate::sim::combat_legal_actions::get_legal_moves(engine, combat)
}

pub fn legal_moves_for_audit(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    crate::sim::combat_legal_actions::legal_moves_for_audit(engine, combat)
}
