use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// RedSkull: While your HP is at or below 50%, you have 3 additional Strength.
/// Java: atBattleStart() checks HP and applies LoseStr/GainStr accordingly.
/// Also has an onBloodied / onNotBloodied callback, but for simplicity
/// we implement it as at_battle_start check + the engine should re-check on HP change.
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
