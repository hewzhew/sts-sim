use crate::content::powers::PowerId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn well_laid_plans_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::RetainCards,
            amount: card.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
