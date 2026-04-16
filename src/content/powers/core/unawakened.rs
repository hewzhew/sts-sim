use crate::core::EntityId;
use crate::runtime::action::Action;

pub fn on_death(owner: EntityId, _amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();

    // Java AwakenedOne.damage():
    // - addToTop(new ClearCardQueueAction())
    // - remove debuffs, Curiosity, Unawakened, Shackled
    // - setMove(3, UNKNOWN)
    // The actual heal back to max happens later during REBIRTH / changeState,
    // not on the death hit itself.
    actions.push(Action::ClearCardQueue);
    actions.push(Action::AwakenedRebirthClear { target: owner });
    actions.push(Action::SetMonsterMove {
        monster_id: owner,
        next_move_byte: 3,
        intent: crate::runtime::combat::Intent::Unknown,
    });

    actions
}
