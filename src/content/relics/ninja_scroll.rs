use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Ninja Scroll: Start each combat with 3 Shivs in hand.
pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Add 3 Shivs to hand
    actions.push(ActionInfo {
        action: Action::MakeTempCardInHand {
            card_id: crate::content::cards::CardId::Shiv,
            amount: 3,
            upgraded: false,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
