use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// RedSkull: While your HP is at or below 50%, you have 3 additional Strength.
/// Java: atBattleStart() checks HP and applies Strength if bloodied.
/// During combat it also reacts to onBloodied/onNotBloodied threshold crossings.
pub fn at_battle_start(current_hp: i32, max_hp: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if current_hp <= max_hp / 2 {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::Strength,
                amount: 3,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}

pub fn on_player_hp_changed(
    previous_hp: i32,
    current_hp: i32,
    max_hp: i32,
) -> SmallVec<[ActionInfo; 4]> {
    let was_bloodied = previous_hp <= max_hp / 2;
    let is_bloodied = current_hp <= max_hp / 2;

    let mut actions = SmallVec::new();
    if !was_bloodied && is_bloodied {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::Strength,
                amount: 3,
            },
            insertion_mode: AddTo::Top,
        });
    } else if was_bloodied && !is_bloodied {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::Strength,
                amount: -3,
            },
            insertion_mode: AddTo::Top,
        });
    }
    actions
}
