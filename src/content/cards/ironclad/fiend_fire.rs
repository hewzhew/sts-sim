use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn fiend_fire_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Fiend Fire requires a valid target!");
    smallvec::smallvec![ActionInfo {
        action: Action::FiendFire {
            target,
            damage_info: DamageInfo {
                source: 0,
                target,
                base: card.base_damage_mut,
                output: card.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: false,
            }
        },
        insertion_mode: AddTo::Bottom,
    }]
}
