use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Mummified Hand: Whenever you play a Power card, a random card in your hand costs 0 for the turn.
pub fn on_use_card(card: &CombatCard, _state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card.id);
    
    if def.card_type == crate::content::cards::CardType::Power {
        actions.push(ActionInfo {
            action: Action::MummifiedHandEffect, // We need to add this engine action or directly modify random card in queue
            insertion_mode: AddTo::Top, // Java: addToTop
        });
    }

    actions
}
