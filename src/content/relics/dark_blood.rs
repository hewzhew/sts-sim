use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct DarkBlood;

impl DarkBlood {
    pub fn on_victory() -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        actions.push(ActionInfo {
            action: Action::Heal {
                target: 0,
                amount: 12,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions
    }
}
