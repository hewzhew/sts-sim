use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn deadly_poison_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Deadly Poison requires a valid target");
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target,
            power_id: crate::content::powers::PowerId::Poison,
            amount: card.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
