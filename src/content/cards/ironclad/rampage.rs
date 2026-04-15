use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn rampage_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Rampage requires a valid target!");
    let mut actions = smallvec::SmallVec::new();
    let damage = card.base_damage_mut;
    let increase_amount = card.base_magic_num_mut;

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
            card_uuid: card.uuid,
            amount: increase_amount,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
