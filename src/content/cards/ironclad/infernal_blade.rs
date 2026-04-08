use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn infernal_blade_play(_state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();

    actions.push(ActionInfo {
        action: Action::MakeRandomCardInHand {
            card_type: Some(crate::content::cards::CardType::Attack),
            cost_for_turn: Some(0),
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
