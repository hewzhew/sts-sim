use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn dropkick_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Dropkick requires a valid target!");
    let mut actions = SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));

    // The correct Java behavior: DropkickAction checks Vulnerable AT EXECUTION TIME, not at play time.
    // We defer the logic to Action::DropkickDamageAndEffect.
    actions.push(ActionInfo {
        action: Action::DropkickDamageAndEffect {
            target,
            damage_info: DamageInfo {
                source: 0,
                target,
                base: evaluated.base_damage_mut,
                output: evaluated.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
