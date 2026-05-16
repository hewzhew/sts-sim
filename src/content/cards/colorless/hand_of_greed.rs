use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::HandOfGreed,
        name: "Hand of Greed",
        card_type: CardType::Attack,
        rarity: CardRarity::Rare,
        cost: 2,
        base_damage: 20,
        base_block: 0,
        base_magic: 20,
        target: CardTarget::Enemy,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 5,
        upgrade_block: 0,
        upgrade_magic: 5,
    }
}
