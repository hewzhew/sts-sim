use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{Power, PowerPayload};
use smallvec::SmallVec;

pub fn at_start_of_turn(owner: crate::core::EntityId, power: &Power) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    if let PowerPayload::Card(card) = &power.payload {
        actions.push(Action::MakeCopyInHand {
            original: Box::new(card.clone()),
            amount: power.amount.max(0).min(u8::MAX as i32) as u8,
        });
    }

    if let Some(instance_id) = power.instance_id {
        actions.push(Action::RemovePowerInstance {
            target: owner,
            power_id: PowerId::Nightmare,
            instance_id,
        });
    } else {
        actions.push(Action::RemovePower {
            target: owner,
            power_id: PowerId::Nightmare,
        });
    }

    actions
}
