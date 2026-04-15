use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::content::powers::PowerId;
use crate::core::EntityId;
use smallvec::SmallVec;

pub struct ChampionBelt;

impl ChampionBelt {
    pub fn on_apply_power(power_id: PowerId, target: EntityId) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        // If the power is Vulnerable and applied to an enemy (not player, id 0)
        if power_id == PowerId::Vulnerable && target != 0 {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target,
                    power_id: PowerId::Weak,
                    amount: 1,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }
}
