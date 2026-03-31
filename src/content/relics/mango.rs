use crate::action::ActionInfo;
use smallvec::SmallVec;

/// Mango: Raise Max HP by 14 upon pickup.
/// No combat hooks. Max HP is preserved stat-side inside PlayerEntity / RunState out of combat.

pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    SmallVec::new()
}
