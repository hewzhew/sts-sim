use crate::action::ActionInfo;
use smallvec::SmallVec;

pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    actions.push(ActionInfo {
        action: crate::action::Action::IncreaseMaxOrb(3),
        insertion_mode: crate::action::AddTo::Bottom,
    });
    actions
}
