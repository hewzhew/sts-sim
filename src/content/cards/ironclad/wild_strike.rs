use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTag, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::WildStrike,
        name: "Wild Strike",
        card_type: CardType::Attack,
        rarity: CardRarity::Common,
        cost: 1,
        base_damage: 12,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::Enemy,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[CardTag::Strike],
        upgrade_damage: 5,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}

pub fn wild_strike_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<crate::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Wild Strike requires a valid target!");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(crate::runtime::action::DamageInfo {
                source: 0,
                target,
                base: evaluated.base_damage_mut,
                output: evaluated.base_damage_mut,
                damage_type: crate::runtime::action::DamageType::Normal,
                is_modified: false,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDrawPile {
                card_id: crate::content::cards::CardId::Wound,
                amount: 1,
                random_spot: true,
                to_bottom: false,
                upgraded: false
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
