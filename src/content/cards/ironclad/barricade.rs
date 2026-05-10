use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn barricade_play(state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    if crate::content::powers::store::has_power(
        state,
        0,
        crate::content::powers::PowerId::Barricade,
    ) {
        return smallvec::smallvec![];
    }

    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: crate::content::powers::PowerId::Barricade,
            amount: -1,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
