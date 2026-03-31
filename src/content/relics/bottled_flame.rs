pub struct BottledFlame;

impl BottledFlame {
    // Bottled Flame allows the player to select an Attack card to become Innate.
    // In the combat engine, Innate cards are already resolved during initialization,
    // so this relic holds no active combat loop hooks. It is processed in the overarching 
    // run simulation wrapper when building the initial deck.
}
