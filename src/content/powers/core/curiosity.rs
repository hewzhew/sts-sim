use crate::action::Action;
use crate::combat::CombatCard;
use crate::content::powers::PowerId;
use crate::core::EntityId;

pub fn on_player_card_played(
    owner: EntityId,
    amount: i32,
    card: &CombatCard,
) -> smallvec::SmallVec<[Action; 2]> {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Power {
        smallvec::smallvec![
            Action::ApplyPower {
                source: owner,
                target: owner,
                power_id: PowerId::Strength,
                amount,
            }
        ]
    } else {
        smallvec::smallvec![]
    }
}
