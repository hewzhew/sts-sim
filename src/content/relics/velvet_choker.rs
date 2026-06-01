/// VelvetChoker: You may not play more than 6 cards per turn.
/// Java also keeps the public relic counter synchronized with cards played.
/// The engine still uses the turn counter for the hard play-limit check, but
/// the relic counter must match Java for public observation and replay.

pub fn at_battle_start(relic_state: &mut crate::content::relics::RelicState) {
    relic_state.counter = 0;
}

pub fn at_turn_start(relic_state: &mut crate::content::relics::RelicState) {
    relic_state.counter = 0;
}

pub fn on_use_card(relic_state: &mut crate::content::relics::RelicState) {
    if relic_state.counter < 6 {
        relic_state.counter += 1;
    }
}

pub fn on_victory(relic_state: &mut crate::content::relics::RelicState) {
    relic_state.counter = -1;
}

/// Check if the player can play a card (called by engine before card play).
pub fn can_play_card(cards_played_this_turn: u32) -> bool {
    cards_played_this_turn < 6
}
