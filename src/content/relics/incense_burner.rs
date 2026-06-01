use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Incense Burner: Every 6 turns, gain 1 Intangible.
/// Counter persists across combats.
/// Java: atTurnStart() → counter = (counter == -1) ? counter + 2 : ++counter;
///   if (counter == 6) { counter = 0; addToBot(ApplyPowerAction(IntangiblePlayerPower, 1)) }
/// Java: onEquip() → counter = 0

pub fn at_turn_start(
    relic_state: &mut crate::content::relics::RelicState,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    relic_state.counter = if relic_state.counter == -1 {
        relic_state.counter + 2
    } else {
        relic_state.counter + 1
    };

    if relic_state.counter == 6 {
        relic_state.counter = 0;
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::IntangiblePlayer,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom, // Java: addToBot
        });
    }

    actions
}
