use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn spot_weakness_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Spot Weakness requires a valid target!");
    let mut actions = smallvec::SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));

    actions.push(ActionInfo {
        action: Action::SpotWeakness {
            target,
            amount: evaluated.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
