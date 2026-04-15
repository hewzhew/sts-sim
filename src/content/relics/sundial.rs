use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Sundial: Every 3 times you shuffle your draw pile, gain 2 Energy.
pub fn on_shuffle(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let next_counter = if counter + 1 >= 3 { 0 } else { counter + 1 };

    actions.push(ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::Sundial,
            counter: next_counter,
        },
        insertion_mode: AddTo::Bottom, // Java: addToBot
    });

    if next_counter == 0 {
        actions.push(ActionInfo {
            action: Action::GainEnergy { amount: 2 },
            insertion_mode: AddTo::Bottom, // Java: addToBot
        });
    }

    actions
}
