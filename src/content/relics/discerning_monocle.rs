/// DiscerningMonocle — Shop-specific relic.
/// Java: onEnterRoom(ShopRoom) → cosmetic pulse. MULTIPLIER = 0.8f.
/// Actual effect: 20% shop discount, applied in RunState::generate_shop() price chain.
pub fn on_equip() {
    // No combat hook. Price modifier is applied in generate_shop().
}
