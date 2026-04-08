use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn iron_wave_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Iron Wave requires a valid target!");
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: card.base_block_mut
            },
            insertion_mode: AddTo::Bottom
        },
        ActionInfo {
            action: Action::Damage(DamageInfo {
                source: 0,
                target,
                base: card.base_damage_mut,
                output: card.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: false,
            }),
            insertion_mode: AddTo::Bottom
        }
    ]
}
