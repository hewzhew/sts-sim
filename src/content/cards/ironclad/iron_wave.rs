use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn iron_wave_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Iron Wave requires a valid target!");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    smallvec::smallvec![
        ActionInfo {
            action: Action::GainBlock {
                target: 0,
                amount: evaluated.base_block_mut
            },
            insertion_mode: AddTo::Bottom
        },
        ActionInfo {
            action: Action::Damage(DamageInfo {
                source: 0,
                target,
                base: evaluated.base_damage_mut,
                output: evaluated.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: true,
            }),
            insertion_mode: AddTo::Bottom
        }
    ]
}
