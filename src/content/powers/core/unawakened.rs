use crate::action::Action;
use crate::core::EntityId;

pub fn on_death(owner: EntityId, _amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    
    // Engine detects Unawakened and skips is_dying (halfDead behavior).
    // Queue transition actions for phase 2.

    // Set next move to dummy move 3 (DARK ECHO init)
    actions.push(Action::SetMonsterMove {
        monster_id: owner,
        next_move_byte: 3,
        intent: crate::combat::Intent::Unknown,
    });

    // Heal back to max
    actions.push(Action::Heal { target: owner, amount: 9999 });

    // Clear targeted debuffs, Curiosity, Unawakened, and Shackled
    actions.push(Action::AwakenedRebirthClear { target: owner });
    
    actions
}
