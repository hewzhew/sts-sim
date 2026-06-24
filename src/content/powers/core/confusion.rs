use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::EntityId;

// Confusion randomizes card costs when drawn.
pub fn on_card_draw(state: &mut CombatState, card: &mut CombatCard) {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.cost >= 0
        && def.card_type != crate::content::cards::CardType::Status
        && def.card_type != crate::content::cards::CardType::Curse
    {
        let new_cost = state.rng.card_random_rng.random(3) as u8;
        card.free_to_play_once = false;
        // Java Snecko-style randomization changes the combat copy cost and the
        // visible turn cost together.
        card.set_combat_and_turn_cost_java(new_cost as i32);
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
