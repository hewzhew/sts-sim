use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct BloodyIdol;

impl BloodyIdol {
    pub fn on_gain_gold() -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        actions.push(ActionInfo {
            action: Action::Heal {
                target: 0,
                amount: 5,
            },
            insertion_mode: AddTo::Top,
        });
        actions
    }
}
