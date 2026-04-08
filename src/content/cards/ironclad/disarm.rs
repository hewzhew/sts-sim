use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use smallvec::SmallVec;

pub fn disarm_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Disarm requires a valid target!");
    let mut actions = smallvec::SmallVec::new();
    let amount = card.base_magic_num_mut; // 2, upgraded 3

    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Strength,
            amount: -amount, // Reduces strength
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
