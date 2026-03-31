use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo, DamageType, DamageInfo};
use smallvec::SmallVec;

pub fn body_slam_play(_state: &CombatState, card: &CombatCard, target: crate::core::EntityId) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let damage = card.base_damage_mut; // Correctly pre-calculated in cards/mod.rs router
    
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
