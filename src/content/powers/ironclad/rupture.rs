use crate::action::Action;
use crate::content::powers::PowerId;
use smallvec::SmallVec;

pub fn on_hp_lost(amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    actions.push(Action::ApplyPower {
        source: 0, target: 0, power_id: PowerId::Strength, amount,
    });
    actions
}
