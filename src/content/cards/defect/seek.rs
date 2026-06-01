use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Seek,
        name: "Seek",
        card_type: CardType::Skill,
        rarity: CardRarity::Rare,
        cost: 0,
        base_damage: 0,
        base_block: 0,
        base_magic: 1,
        target: CardTarget::None,
        is_multi_damage: false,
        exhaust: true,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 1,
    }
}

pub fn seek_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let amount = evaluated.base_magic_num_mut.max(0) as usize;
    if amount == 0 || state.zones.draw_pile.is_empty() {
        return SmallVec::new();
    }

    if state.zones.draw_pile.len() <= amount {
        return state
            .zones
            .draw_pile
            .iter()
            .rev()
            .map(|draw_card| ActionInfo {
                action: Action::MoveCard {
                    card_uuid: draw_card.uuid,
                    from: crate::state::PileType::Draw,
                    to: crate::state::PileType::Hand,
                },
                insertion_mode: AddTo::Bottom,
            })
            .collect();
    }

    smallvec::smallvec![ActionInfo {
        action: Action::SuspendForGridSelect {
            source_pile: crate::state::PileType::Draw,
            min: amount.min(u8::MAX as usize) as u8,
            max: amount.min(u8::MAX as usize) as u8,
            can_cancel: false,
            filter: crate::state::GridSelectFilter::Any,
            reason: crate::state::GridSelectReason::DrawPileToHand,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
