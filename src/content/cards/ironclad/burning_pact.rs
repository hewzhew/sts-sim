use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn burning_pact_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let mut actions = SmallVec::new();

    actions.push(ActionInfo {
        action: Action::ExhaustFromHand {
            amount: 1,
            random: false,
            any_number: false,
            can_pick_zero: false,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions.push(ActionInfo {
        action: Action::DrawCards(evaluated.base_magic_num_mut as u32),
        insertion_mode: AddTo::Bottom,
    });

    actions
}
