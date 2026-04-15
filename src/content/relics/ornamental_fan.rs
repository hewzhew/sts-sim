use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Ornamental Fan: Every time you play 3 Attacks in a single turn, gain 4 Block.
pub fn on_use_card(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    let current = if counter < 0 { 0 } else { counter };
    let next_counter = current + 1;

    if next_counter >= 3 {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::OrnamentalFan,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: 4,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::OrnamentalFan,
                counter: next_counter,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}

pub fn at_turn_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    actions.push(ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::OrnamentalFan,
            counter: 0,
        },
        insertion_mode: AddTo::Bottom,
    });
    actions
}
