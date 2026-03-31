/// VelvetChoker: You may not play more than 6 cards per turn.
/// This is a passive constraint — checked in the engine when determining
/// if a card can be played. The relic itself doesn't generate actions.
/// The counter tracks cards played this turn.

/// Check if the player can play a card (called by engine before card play).
pub fn can_play_card(cards_played_this_turn: u32) -> bool {
    cards_played_this_turn < 6
}
