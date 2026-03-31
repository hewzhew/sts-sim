use crate::combat::CombatState;
use crate::content::relics::RelicState;
use smallvec::SmallVec;
use crate::action::{Action, ActionInfo, AddTo};

/// Frozen Core (Defect Boss)
/// Java: onPlayerEndTurn() → if hasEmptyOrb(): channelOrb(new Frost())
/// "If you end your turn with empty Orb slots, channel 1 Frost."
pub fn at_end_of_turn(state: &CombatState, _relic: &mut RelicState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    // Java: hasEmptyOrb() — returns true if any orb slot is EmptyOrbSlot
    let has_empty = state.player.orbs.iter().any(|o| o.id == crate::combat::OrbId::Empty);
    if has_empty {
        actions.push(ActionInfo {
            action: Action::ChannelOrb(crate::combat::OrbId::Frost),
            insertion_mode: AddTo::Bottom, // Java: channelOrb is direct call, but fires via action queue
        });
    }
    actions
}
