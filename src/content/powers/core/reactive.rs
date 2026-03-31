use crate::action::Action;
use crate::core::EntityId;
use crate::combat::CombatState;

pub fn on_attacked(
    _state: &CombatState,
    owner: EntityId,
    damage: i32,
    _source: EntityId,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if damage > 0 {
        actions.push(Action::RollMonsterMove { monster_id: owner });
    }

    actions
}
