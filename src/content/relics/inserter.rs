use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct Inserter;

impl Inserter {
    /// Inserter (Defect Boss)
    /// Java: atTurnStart() — counter increments, fires IncreaseMaxOrbAction(1) every 2 turns
    /// Same counter pattern as HappyFlower: counter = (counter == -1) ? counter+2 : ++counter
    /// Fire when counter == 2, reset to 0
    pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        // Java: this.counter = this.counter == -1 ? (this.counter += 2) : ++this.counter;
        let new_counter = if counter == -1 {
            counter + 2
        } else {
            counter + 1
        };

        if new_counter == 2 {
            // Java: addToBot(IncreaseMaxOrbAction(1))
            actions.push(ActionInfo {
                action: Action::IncreaseMaxOrb(1),
                insertion_mode: AddTo::Bottom,
            });
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::Inserter,
                    counter: 0,
                },
                insertion_mode: AddTo::Bottom,
            });
        } else {
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: crate::content::relics::RelicId::Inserter,
                    counter: new_counter,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }
}
