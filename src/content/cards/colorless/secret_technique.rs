use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::SecretTechnique,
        name: "Secret Technique",
        card_type: CardType::Skill,
        rarity: CardRarity::Rare,
        cost: 0,
        base_damage: 0,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::None,
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

pub fn secret_technique_play(state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let skills: Vec<_> = state
        .zones
        .draw_pile
        .iter()
        .filter(|c| crate::content::cards::get_card_definition(c.id).card_type == CardType::Skill)
        .map(|c| c.uuid)
        .collect();
    if skills.len() == 1 {
        smallvec::smallvec![ActionInfo {
            action: Action::MoveCard {
                card_uuid: skills[0],
                from: crate::state::PileType::Draw,
                to: crate::state::PileType::Hand,
            },
            insertion_mode: AddTo::Bottom,
        }]
    } else if !skills.is_empty() {
        smallvec::smallvec![ActionInfo {
            action: Action::SuspendForGridSelect {
                source_pile: crate::state::PileType::Draw,
                min: 1,
                max: 1,
                can_cancel: false,
                filter: crate::state::GridSelectFilter::Skill,
                reason: crate::state::GridSelectReason::SkillFromDeckToHand,
            },
            insertion_mode: AddTo::Bottom,
        }]
    } else {
        SmallVec::new()
    }
}
