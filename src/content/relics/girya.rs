use crate::content::powers::PowerId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct Girya;

impl Girya {
    pub fn at_battle_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        // Since the counter tracks the rest site usage (0 to 3), counter.max(0) applies here safely
        let strength = counter.max(0);
        if strength > 0 {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target: 0,
                    power_id: PowerId::Strength,
                    amount: strength,
                },
                insertion_mode: AddTo::Top, // Java: addToTop
            });
        }
        actions
    }
}
