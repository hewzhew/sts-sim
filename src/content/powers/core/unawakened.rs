use crate::core::EntityId;
use crate::runtime::action::{Action, MonsterRuntimePatch};

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
        planned_steps: crate::runtime::combat::Intent::Unknown
            .to_legacy_move_spec()
            .to_steps(),
        planned_visible_spec: None,
    });
    actions.push(Action::UpdateMonsterRuntime {
        monster_id: owner,
        patch: MonsterRuntimePatch::AwakenedOne {
            form1: Some(false),
            first_turn: Some(true),
            protocol_seeded: Some(true),
        },
    });

    actions
}
