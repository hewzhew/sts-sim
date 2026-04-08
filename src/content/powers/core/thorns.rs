use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::CombatState;
use crate::core::EntityId;

pub fn on_attacked(
    _state: &CombatState,
    _owner: EntityId,
    _damage: i32,
    source: EntityId,
    power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    // Thorns only triggers if the source is not the owner (e.g., self-damage doesn't trigger it)
    if source != 0 && source != _owner {
        actions.push(Action::Damage(DamageInfo {
            source: _owner,
            target: source,
            base: power_amount,
            output: power_amount,
            damage_type: DamageType::Thorns,
            is_modified: false,
        }));
    } else if source == 0 && _owner != 0 {
        // Player attacked monster
        actions.push(Action::Damage(DamageInfo {
            source: _owner,
            target: 0,
            base: power_amount,
            output: power_amount,
            damage_type: DamageType::Thorns,
            is_modified: false,
        }));
    }

    actions
}
