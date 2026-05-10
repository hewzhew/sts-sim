use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn offering_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![
        ActionInfo {
            action: Action::LoseHp {
                target: 0,
                amount: 6,
                triggers_rupture: true,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::GainEnergy { amount: 2 },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::DrawCards(evaluated.base_magic_num_mut as u32),
            insertion_mode: AddTo::Bottom,
        }
    ]
}
