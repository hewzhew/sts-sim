use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::RitualDagger,
        name: "Ritual Dagger",
        card_type: CardType::Attack,
        rarity: CardRarity::Special,
        cost: 1,
        base_damage: 15,
        base_block: 0,
        base_magic: 3,
        target: CardTarget::Enemy,
        is_multi_damage: false,
        exhaust: true,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 2,
    }
}
