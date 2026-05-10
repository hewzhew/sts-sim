use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn warcry_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![
        ActionInfo {
            action: Action::DrawCards(evaluated.base_magic_num_mut as u32),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::PutOnDeck {
                amount: 1,
                random: false
            },
            insertion_mode: AddTo::Bottom,
        }
    ] // Exhaust is handled by card.exhaust = true in definitions
}
