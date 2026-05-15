use crate::runtime::action::ActionInfo;
use crate::runtime::combat::{CombatCard, CombatState};

fn apply_card_at_turn_start(card: &mut CombatCard) {
    if card.id == crate::content::cards::CardId::Eviscerate {
        card.cost_for_turn = None;
    }
}

pub fn at_turn_start_in_hand(state: &mut CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    // Java: Player.applyStartOfTurnCards()
    // Despite the historical Rust name, Java applies this to draw pile, hand,
    // then discard pile. Eviscerate resets its temporary discard reduction here.
    for card in state
        .zones
        .draw_pile
        .iter_mut()
        .chain(state.zones.hand.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
    {
        apply_card_at_turn_start(card);
    }
    smallvec::smallvec![]
}
