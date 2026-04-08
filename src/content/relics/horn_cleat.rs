use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct HornCleat;

impl HornCleat {
    pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        // Native reset for per-battle turn counters
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::HornCleat,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions
    }

    pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        let current = if counter == -1 { 0 } else { counter };

        // At start of 2nd turn
        if current == 1 {
            actions.push(ActionInfo {
                action: Action::GainBlock {
                    target: 0,
                    amount: 14,
                },
                insertion_mode: AddTo::Bottom,
            });
        }

        if current < 2 {
            // Keep incrementing internally so counter represents current_turn mapped accurately.
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::HornCleat,
                    counter: current + 1,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }
}
