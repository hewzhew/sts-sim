use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn bludgeon_play(_state: &CombatState, card: &CombatCard, target: EntityId) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    actions.push(ActionInfo {
        action: Action::Damage(DamageInfo {
            source: 0,
            target,
            base: card.base_damage_mut,
            output: card.base_damage_mut,
            damage_type: DamageType::Normal,
            is_modified: false,
        }),
        insertion_mode: AddTo::Bottom
    });

    actions
}
