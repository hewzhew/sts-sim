use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct BronzeScales;

impl BronzeScales {
    pub fn at_battle_start(player_id: crate::core::EntityId) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: player_id,
                target: player_id,
                power_id: crate::content::powers::PowerId::Thorns,
                amount: 3,
            },
            insertion_mode: AddTo::Top,
        });
        actions
    }
}
