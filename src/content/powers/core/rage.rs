use crate::action::Action;
use crate::combat::CombatCard;
use crate::core::EntityId;

pub fn on_card_played(
    owner: EntityId,
    card: &CombatCard,
    power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Attack {
        smallvec::smallvec![Action::GainBlock {
            target: owner,
            amount: power_amount.max(0),
        }]
    } else {
        smallvec::smallvec![]
    }
}
