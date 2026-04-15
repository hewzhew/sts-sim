use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use crate::core::EntityId;
use smallvec::{smallvec, SmallVec};

pub fn on_hp_lost(state: &CombatState, owner: EntityId, _amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = smallvec![];

    // Split triggers when HP drops to or below 50%
    if let Some(monster) = state.entities.monsters.iter().find(|m| m.id == owner) {
        if monster.current_hp <= monster.max_hp / 2 && monster.next_move_byte != 3 {
            // 3 is SPLIT
            actions.push(Action::SetMonsterMove {
                monster_id: owner,
                next_move_byte: 3,
                intent: crate::runtime::combat::Intent::Unknown,
            });
        }
    }

    actions
}
