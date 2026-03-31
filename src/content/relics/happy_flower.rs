use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct HappyFlower;

impl HappyFlower {
    pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        // Java: this.counter = this.counter == -1 ? (this.counter += 2) : ++this.counter;
        // if (this.counter == 3) { this.counter = 0; fire energy }
        let new_counter = if counter == -1 { counter + 2 } else { counter + 1 };
        
        if new_counter == 3 {
            actions.push(ActionInfo {
                action: Action::GainEnergy { amount: 1 },
                insertion_mode: AddTo::Bottom,
            });
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::HappyFlower,
                    counter: 0,
                },
                insertion_mode: AddTo::Bottom,
            });
        } else {
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::HappyFlower,
                    counter: new_counter,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }
}
