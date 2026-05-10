use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn rupture_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, _state, None);
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::Rupture,
            amount: evaluated.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
