use crate::combat::CombatState;
use crate::content::relics::RelicState;
use smallvec::SmallVec;
use crate::action::{Action, ActionInfo, AddTo};

/// Fusion Hammer
/// Boss Relic
/// Gain 1 Energy at the start of each turn. You can no longer Smith at Rest Sites.
/// 
/// Energy is handled via `energy_master` increment at onEquip.
/// Rest Site restriction handled via `RunState`.
pub fn out_of_combat() {}
