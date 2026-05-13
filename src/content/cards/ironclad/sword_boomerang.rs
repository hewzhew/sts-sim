use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn sword_boomerang_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);

    for _ in 0..evaluated.base_magic_num_mut {
        actions.push(ActionInfo {
            action: Action::AttackDamageRandomEnemyCard {
                card: Box::new(card.clone()),
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
