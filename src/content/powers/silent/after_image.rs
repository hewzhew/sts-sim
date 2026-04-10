use crate::action::Action;
use crate::combat::CombatCard;
use crate::core::EntityId;

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
