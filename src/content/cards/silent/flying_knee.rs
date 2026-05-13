use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn flying_knee_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Flying Knee requires a valid target");
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
                target: 0,
                power_id: PowerId::Energized,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
