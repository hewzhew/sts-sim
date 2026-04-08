use crate::action::Action;
use crate::combat::{CombatState, PowerId};
use crate::core::EntityId;

pub fn on_death(
    _state: &CombatState,
    _owner: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    // Spore Cloud applies Vulnerable to player on death
    actions.push(Action::ApplyPower {
        source: _owner, // Technically dead, but source still traceable
        target: 0,      // Player
        power_id: PowerId::Vulnerable,
        amount,
    });

    actions
}
