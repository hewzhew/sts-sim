use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Anger,
        name: "Anger",
        card_type: CardType::Attack,
        rarity: CardRarity::Common,
        cost: 0,
        base_damage: 6,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::Enemy,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 2,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}

pub fn anger_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Anger requires a valid target!");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    let mut actions = smallvec::SmallVec::new();
    let damage = evaluated.base_damage_mut; // 6, upgraded 8

    actions.push(ActionInfo {
        action: Action::Damage(DamageInfo {
            source: 0,
            target,
            base: damage,
            output: damage,
            damage_type: DamageType::Normal,
            is_modified: false,
        }),
        insertion_mode: AddTo::Bottom,
    });

    actions.push(ActionInfo {
        action: Action::MakeCopyInDiscard {
            original: Box::new(evaluated),
            amount: 1,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
