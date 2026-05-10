use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn double_tap_play(
    state: &CombatState,
    card: &CombatCard,
    _target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);

    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::DoubleTap,
            amount: evaluated.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
