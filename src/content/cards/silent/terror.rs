use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn terror_play(
    _state: &CombatState,
    _card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Terror requires a valid target");
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Vulnerable,
            amount: 99,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
