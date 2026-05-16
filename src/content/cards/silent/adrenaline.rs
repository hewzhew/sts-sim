use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Adrenaline,
        name: "Adrenaline",
        card_type: CardType::Skill,
        rarity: CardRarity::Rare,
        cost: 0,
        base_damage: 0,
        base_block: 0,
        base_magic: 2,
        target: CardTarget::SelfTarget,
        is_multi_damage: false,
        exhaust: true,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}

pub fn adrenaline_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainEnergy {
                amount: if card.upgrades > 0 { 2 } else { 1 },
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::DrawCards(card.base_magic_num_mut.max(0) as u32),
            insertion_mode: AddTo::Bottom,
        },
    ]
}
