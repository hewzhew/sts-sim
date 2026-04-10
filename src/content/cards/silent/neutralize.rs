use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn neutralize_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Neutralize requires a valid target");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(DamageInfo {
                source: 0,
                target,
                base: evaluated.base_damage_mut,
                output: evaluated.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: true,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target,
                power_id: crate::content::powers::PowerId::Weak,
                amount: evaluated.base_magic_num_mut,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
