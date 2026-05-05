use crate::runtime::action::Action;
use smallvec::SmallVec;

/// Energized: At the start of your next turn, gain X Energy.
pub fn at_turn_start(entity_id: crate::core::EntityId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();

    // Gain Energy
    actions.push(Action::GainEnergy { amount });

    // Remove the power
    actions.push(Action::RemovePower {
        target: entity_id,
        power_id: crate::content::powers::PowerId::Energized,
    });

    actions
}
