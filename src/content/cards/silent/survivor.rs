use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Survivor,
        name: "Survivor",
        card_type: CardType::Skill,
        rarity: CardRarity::Basic,
        cost: 1,
        base_damage: 0,
        base_block: 8,
        base_magic: 0,
        target: CardTarget::SelfTarget,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 3,
        upgrade_magic: 0,
    }
}

pub fn survivor_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: card.base_block_mut,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::DiscardFromHand {
                amount: 1,
                random: false,
                end_turn: false,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
