use crate::action::{Action, ActionInfo, AddTo, DamageType};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// Mercury Hourglass: At the start of your turn, deal 3 damage to all enemies.
pub fn at_turn_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Create damages array for all alive monsters
    let damages = vec![3; state.monsters.len()];

    actions.push(ActionInfo {
        action: Action::DamageAllEnemies {
            source: 0,
            damages: damages.into(),
            damage_type: DamageType::Thorns,
            is_modified: false,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
