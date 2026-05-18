use crate::core::EntityId;

pub fn on_death(
    _owner: EntityId,
    _amount: i32,
) -> smallvec::SmallVec<[crate::runtime::action::Action; 2]> {
    // Java `UnawakenedPower` has no onDeath override. The first-phase death
    // transition belongs to `AwakenedOne.damage()`, because it mutates monster
    // state and move history immediately rather than as a normal power hook.
    smallvec::SmallVec::new()
}
