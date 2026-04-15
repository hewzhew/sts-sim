use crate::runtime::action::Action;
use crate::runtime::combat::PowerId;
use crate::core::EntityId;

pub fn on_calculate_block(mut block: f32, amount: i32) -> f32 {
    if amount > 0 {
        block = (block * 0.75).floor();
    }
    block
}

pub fn at_end_of_round(
    owner: EntityId,
    amount: i32,
    just_applied: bool,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    if amount > 0 && !just_applied {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Frail,
            amount: -1,
        });
    }
    actions
}
