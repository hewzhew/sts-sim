use crate::action::ActionInfo;
use crate::combat::CombatState;

/// SlaversCollar: During Elite and Boss combats, gain 1 additional Energy each turn.
///
/// Java: beforeEnergyPrep() → ++energyMaster (permanent for combat duration)
///       onVictory()         → --energyMaster
///
/// In Rust, we handle this in at_battle_start by setting a flag on the relic.
/// The energy system reads this flag to add +1 energy per turn reset.
/// We use the simpler approach: just increase energy by 1 at battle start each turn
/// via the existing at_turn_start mechanism — but Java does `++energyMaster` which is
/// permanent. So we replicate by adding +1 to the base energy calculation.
///
/// Simplest correct approach: at_battle_start emits no action, but the energy reset
/// in core.rs checks `is_elite_fight || is_boss_fight` + has SlaversCollar → energy += 1.
///
/// However, to be truly Java-accurate where it modifies energyMaster once:
/// We store state in relic.counter (1 = bonus active) and check it at energy reset.

pub fn at_battle_start(state: &CombatState, relic: &mut crate::content::relics::RelicState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let actions = smallvec::SmallVec::new();
    
    // Java: room.eliteTrigger || any monster.type == BOSS
    let is_elite_or_boss = state.is_elite_fight || state.is_boss_fight;
    
    if is_elite_or_boss {
        // Mark relic as active (counter = 1) for energy reset check
        relic.counter = 1;
    } else {
        relic.counter = 0;
    }
    
    actions
}
