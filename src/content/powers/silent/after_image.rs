use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatCard;

pub fn on_card_played(
    owner: EntityId,
    amount: i32,
    _card: &CombatCard,
) -> smallvec::SmallVec<[Action; 2]> {
    if amount <= 0 {
        return smallvec::smallvec![];
    }
    smallvec::smallvec![Action::GainBlock {
        target: owner,
        amount,
    }]
}
