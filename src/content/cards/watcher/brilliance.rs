use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Brilliance,
        name: "Brilliance",
        card_type: CardType::Attack,
        rarity: CardRarity::Rare,
        cost: 1,
        base_damage: 12,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::Enemy,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 4,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}

pub fn brilliance_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Brilliance requires a valid target");
    let mut dynamic_card = card.clone();
    let def = definition();
    let upgraded = if dynamic_card.upgrades > 0 { 1 } else { 0 };
    dynamic_card.base_damage_override = Some(
        def.base_damage
            + upgraded * def.upgrade_damage
            + state.turn.counters.mantra_gained_this_combat,
    );
    let evaluated =
        crate::content::cards::evaluate_card_for_play(&dynamic_card, state, Some(target));
    smallvec::smallvec![ActionInfo {
        action: Action::Damage(DamageInfo {
            source: 0,
            target,
            base: evaluated.base_damage_mut,
            output: evaluated.base_damage_mut,
            damage_type: DamageType::Normal,
            is_modified: true,
        }),
        insertion_mode: AddTo::Bottom,
    }]
}
