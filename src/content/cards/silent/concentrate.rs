use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn concentrate_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![
        ActionInfo {
            action: Action::DiscardFromHand {
                amount: evaluated.base_magic_num_mut,
                random: false,
                end_turn: false,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::GainEnergy { amount: 2 },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
