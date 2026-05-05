use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct CaptainsWheel;

impl CaptainsWheel {
    pub fn at_battle_start(relic_state: &mut crate::content::relics::RelicState) {
        relic_state.counter = 0;
    }

    pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        let current = if counter == -1 { 0 } else { counter };

        if current < 2 {
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::CaptainsWheel,
                    counter: current + 1,
                },
                insertion_mode: AddTo::Bottom,
            });
        } else if current == 2 {
            actions.push(ActionInfo {
                action: Action::GainBlock {
                    target: 0,
                    amount: 18,
                },
                insertion_mode: AddTo::Bottom,
            });
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::CaptainsWheel,
                    counter: 3,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }

    pub fn on_victory(relic_state: &mut crate::content::relics::RelicState) {
        relic_state.counter = -1;
    }
}
