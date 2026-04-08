use crate::action::ActionInfo;
use smallvec::SmallVec;

pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    actions.push(ActionInfo {
        action: crate::action::Action::MakeTempCardInHand {
            card_id: crate::content::cards::CardId::Miracle,
            amount: 1,
            upgraded: false,
        },
        insertion_mode: crate::action::AddTo::Bottom,
    });
    actions
}
