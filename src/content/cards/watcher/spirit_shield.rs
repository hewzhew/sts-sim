use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::SpiritShield,
        name: "Spirit Shield",
        card_type: CardType::Skill,
        rarity: CardRarity::Rare,
        cost: 2,
        base_damage: 0,
        base_block: 0,
        base_magic: 3,
        target: CardTarget::SelfTarget,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 1,
    }
}

pub fn spirit_shield_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let other_hand_cards = state
        .zones
        .hand
        .iter()
        .filter(|hand_card| hand_card.uuid != card.uuid)
        .count() as i32;
    let amount = other_hand_cards * evaluated.base_magic_num_mut.max(0);
    smallvec::smallvec![ActionInfo {
        action: Action::GainBlock { target: 0, amount },
        insertion_mode: AddTo::Bottom,
    }]
}
