use crate::runtime::action::ActionInfo;
use crate::runtime::combat::CombatState;
use crate::content::relics::RelicState;
use smallvec::SmallVec;

/// Ectoplasm
/// Boss Relic
/// Gain 1 Energy at the start of each turn. You can no longer gain Gold.
///
/// Note: The +1 Energy is typically handled by `energy_master` increment at onEquip out of combat.
/// The gold restriction is handled by a check `if !player.has_relic(Ectoplasm)` globally in out-of-combat.
/// No direct combat hooks for Ectoplasm are needed unless we manually hook turn energy override,
/// but standard StS implementation bumps `energy_master` directly in `RunState::on_equip`.
pub fn at_battle_start(_state: &CombatState, _relic: &mut RelicState) -> SmallVec<[ActionInfo; 4]> {
    SmallVec::new()
}
