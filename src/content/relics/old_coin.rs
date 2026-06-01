use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    if !run_state
        .relics
        .iter()
        .any(|relic| relic.id == crate::content::relics::RelicId::Ectoplasm)
    {
        run_state.gold += 300;
    }
    None
}
