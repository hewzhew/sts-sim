use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn adrenaline_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainEnergy {
                amount: if card.upgrades > 0 { 2 } else { 1 },
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::DrawCards(card.base_magic_num_mut.max(0) as u32),
            insertion_mode: AddTo::Bottom,
        },
    ]
}
