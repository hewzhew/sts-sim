use crate::content::cards::CardType;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn distraction_play(_state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::MakeRandomCardInHand {
            card_type: Some(CardType::Skill),
            cost_for_turn: Some(0),
        },
        insertion_mode: AddTo::Bottom,
    }]
}
