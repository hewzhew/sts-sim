use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::DramaticEntrance,
        name: "Dramatic Entrance",
        card_type: CardType::Attack,
        rarity: CardRarity::Uncommon,
        cost: 0,
        base_damage: 8,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::AllEnemy,
        is_multi_damage: false,
        exhaust: true,
        ethereal: false,
        innate: true,
        tags: &[],
        upgrade_damage: 4,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}
