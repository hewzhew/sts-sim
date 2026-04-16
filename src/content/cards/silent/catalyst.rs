use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn catalyst_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Catalyst requires a valid target");
    let poison = state.get_power(target, PowerId::Poison).max(0);
    if poison == 0 {
        return smallvec::smallvec![];
    }

    let extra = poison * (card.base_magic_num_mut - 1).max(1);
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Poison,
            amount: extra,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
