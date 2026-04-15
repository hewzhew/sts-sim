use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use smallvec::SmallVec;

pub fn uppercut_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Uppercut requires a valid target!");
    let mut actions = smallvec::SmallVec::new();
    let damage = card.base_damage_mut;
    let amount = card.base_magic_num_mut; // 1, upgraded 2

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
        action: Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Weak,
            amount,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions.push(ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Vulnerable,
            amount,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
