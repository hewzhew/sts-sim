use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn bloodletting_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![
        ActionInfo {
            action: Action::LoseHp {
                target: 0,
                amount: 3,
                triggers_rupture: true,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::GainEnergy {
                amount: evaluated.base_magic_num_mut
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
