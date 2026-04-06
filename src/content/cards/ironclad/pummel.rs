use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo, DamageType, DamageInfo};
use smallvec::SmallVec;

pub fn pummel_play(_state: &CombatState, card: &CombatCard, target: Option<crate::core::EntityId>) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Pummel requires a valid target!");
    let mut actions = smallvec::SmallVec::new();
    let damage = card.base_damage_mut;
    let amount = card.base_magic_num_mut; // 4, upgraded 5
    
    for _ in 0..amount {
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
    }
    
    actions
}
