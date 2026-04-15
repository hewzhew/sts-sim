use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn clash_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Clash requires a valid target!");
    let mut actions = smallvec::SmallVec::new();

    // Unplayable if there are non-attacks in hand
    let has_non_attacks = state.zones.hand.iter().any(|c| {
        crate::content::cards::get_card_definition(c.id).card_type
            != crate::content::cards::CardType::Attack
    });
    if has_non_attacks {
        return actions;
    }

    let damage = card.base_damage_mut;

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

    actions
}
