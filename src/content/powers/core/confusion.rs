use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};

// Confusion randomizes card costs when drawn.
pub fn on_card_draw(state: &mut CombatState, card: &mut CombatCard) {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.cost >= 0
        && def.card_type != crate::content::cards::CardType::Status
        && def.card_type != crate::content::cards::CardType::Curse
    {
        let new_cost = state.rng.card_random_rng.random(3) as u8;
        card.cost_for_turn = Some(new_cost);
        card.free_to_play_once = false;
        // In STS, cost also gets mapped over, but dynamically evaluated cards in combat will use cost_for_turn.
        // We set cost_modifier so that absolute get_cost() reflects the permanent cost change in combat.
        card.cost_modifier = new_cost as i8 - def.cost as i8;
    }
}

pub fn on_card_drawn(
    _state: &mut CombatState,
    _owner: EntityId,
    _card_uuid: u32,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    smallvec::smallvec![]
}
