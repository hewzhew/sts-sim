use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};

pub fn on_card_played(
    _state: &CombatState,
    _owner: EntityId,
    card: &CombatCard,
    power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type != crate::content::cards::CardType::Attack {
        actions.push(Action::MakeTempCardInDrawPile {
            card_id: crate::content::cards::CardId::Dazed,
            amount: power_amount as u8,
            random_spot: true,
            upgraded: false,
        });
    }

    actions
}
