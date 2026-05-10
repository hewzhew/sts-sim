use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn rampage_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Rampage requires a valid target!");
    let mut actions = smallvec::SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, _state, Some(target));
    let damage = evaluated.base_damage_mut;
    let increase_amount = evaluated.base_magic_num_mut;

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
        action: Action::ModifyCardDamage {
            card_uuid: evaluated.uuid,
            amount: increase_amount,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
