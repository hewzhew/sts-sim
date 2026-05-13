use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn true_grit_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let mut actions = smallvec::smallvec![ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: evaluated.base_block_mut,
        },
        insertion_mode: AddTo::Bottom,
    }];

    if card.upgrades > 0 {
        actions.push(ActionInfo {
            action: Action::ExhaustFromHand {
                amount: 1,
                random: false,
                any_number: false,
                can_pick_zero: false,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else {
        actions.push(ActionInfo {
            action: Action::ExhaustFromHand {
                amount: 1,
                random: true,
                any_number: false,
                can_pick_zero: false,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
