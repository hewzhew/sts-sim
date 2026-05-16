use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::PanicButton,
        name: "Panic Button",
        card_type: CardType::Skill,
        rarity: CardRarity::Uncommon,
        cost: 0,
        base_damage: 0,
        base_block: 30,
        base_magic: 2,
        target: CardTarget::SelfTarget,
        is_multi_damage: false,
        exhaust: true,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 10,
        upgrade_magic: 0,
    }
}
