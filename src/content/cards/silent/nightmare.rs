use crate::runtime::action::{Action, ActionInfo};
use crate::runtime::combat::{CombatCard, CombatState};

pub fn nightmare_play(
    state: &CombatState,
    card: &CombatCard,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![ActionInfo {
        action: Action::Nightmare {
            amount: evaluated.base_magic_num_mut.max(0).min(u8::MAX as i32) as u8,
        },
        insertion_mode: crate::runtime::action::AddTo::Bottom,
    }]
}
