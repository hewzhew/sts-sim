use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn true_grit_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::smallvec![
        ActionInfo { 
            action: Action::GainBlock { target: 0, amount: card.base_block_mut },
            insertion_mode: AddTo::Bottom 
        }
    ];

    if card.upgrades > 0 {
        actions.push(ActionInfo {
            action: Action::SuspendForHandSelect { min: 1, max: 1, reason: crate::state::HandSelectReason::Exhaust },
            insertion_mode: AddTo::Bottom
        });
    } else {
        actions.push(ActionInfo {
            action: Action::ExhaustRandomCard { amount: 1 },
            insertion_mode: AddTo::Bottom
        });
    }
    
    actions
}
