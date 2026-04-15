use crate::runtime::combat::CombatState;
use crate::content::relics::RelicState;

// Note: The Courier is strictly an out-of-combat relic.
// It discounts Shop prices by 20% and instantly replenishes purchased items.

pub fn on_equip(_state: &mut CombatState, _relic: &mut RelicState) {
    // Handled purely in RunState Shop Logic.
}
