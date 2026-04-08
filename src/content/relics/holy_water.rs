use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// Holy Water: Replaces Pure Water. At the start of each combat, add 3 Miracles to your hand.
pub fn at_battle_start(_state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    actions.push(ActionInfo {
        action: Action::MakeTempCardInHand {
            card_id: crate::content::cards::CardId::Miracle,
            amount: 3,
            upgraded: false,
        },
        insertion_mode: AddTo::Bottom,
    });
    actions
}
