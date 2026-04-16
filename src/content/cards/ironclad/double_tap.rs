use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn double_tap_play(
    _state: &CombatState,
    card: &CombatCard,
    _target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::DoubleTap,
            amount: card.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
