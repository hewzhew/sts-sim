use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct BurningBlood;

impl BurningBlood {
    pub fn on_victory() -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        actions.push(ActionInfo {
            action: Action::Heal {
                target: 0,
                amount: 6,
            },
            insertion_mode: AddTo::Top,
        });
        actions
    }
}
