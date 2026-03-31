use crate::action::Action;
use crate::content::cards::{get_card_definition, CardType};
use smallvec::SmallVec;

pub fn on_card_drawn(card_id: crate::content::cards::CardId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    let def = get_card_definition(card_id);
    if def.card_type == CardType::Status {
        actions.push(Action::DrawCards(amount as u32));
    }
    actions
}
