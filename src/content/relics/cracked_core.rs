use crate::content::relics::RelicState;
use crate::runtime::action::ActionInfo;
use crate::runtime::combat::CombatState;

// Note: Defect Orb mechanics are unimplemented.
// This is currently a mock simulation for Cracked Core (Channels 1 Lightning).

pub fn at_battle_start(
    _state: &CombatState,
    _relic: &mut RelicState,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let actions = smallvec::SmallVec::new();

    // Placeholder Action. Orbs pipeline is outside scope.
    // AbstractDungeon.player.channelOrb(new Lightning());

    actions
}
