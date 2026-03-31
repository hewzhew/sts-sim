use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Incense Burner: Every 6 turns, gain 1 Intangible.
/// Counter persists across combats.
/// Java: atTurnStart() → counter = (counter == -1) ? counter + 2 : ++counter;
///   if (counter == 6) { counter = 0; addToBot(ApplyPowerAction(IntangiblePlayerPower, 1)) }
/// Java: onEquip() → counter = 0

pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Java: counter == -1 ? counter += 2 : ++counter
    // This handles uninitialized (-1) counter by jumping to 1 instead of 0
    let next_counter = if counter == -1 {
        1  // Java: -1 + 2 = 1
    } else if counter + 1 >= 6 {
        0
    } else {
        counter + 1
    };

    actions.push(ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::IncenseBurner,
            counter: next_counter,
        },
        insertion_mode: AddTo::Bottom, // Java: addToBot
    });

    if next_counter == 0 {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::Intangible,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom, // Java: addToBot
        });
    }

    actions
}
