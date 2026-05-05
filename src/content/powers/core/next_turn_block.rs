use crate::runtime::action::Action;
use smallvec::SmallVec;

/// Next Turn Block: At the start of your next turn, gain X Block. (Used by Self-Forming Clay, Dodge and Roll, etc.)
pub fn at_turn_start(entity_id: crate::core::EntityId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();

    // Gain block
    actions.push(Action::GainBlock {
        target: entity_id,
        amount,
    });

    // Remove the power
    actions.push(Action::RemovePower {
        target: entity_id,
        power_id: crate::content::powers::PowerId::NextTurnBlock,
    });

    actions
}
