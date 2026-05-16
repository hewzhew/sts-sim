use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTag, CardTarget, CardType};

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Bite,
        name: "Bite",
        card_type: CardType::Attack,
        rarity: CardRarity::Special,
        cost: 1,
        base_damage: 7,
        base_block: 0,
        base_magic: 2,
        target: CardTarget::Enemy,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[CardTag::Healing],
        upgrade_damage: 1,
        upgrade_block: 0,
        upgrade_magic: 1,
    }
}
