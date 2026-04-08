use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// NeowsLament: Enemies in your first 3 combats have 1 HP.
/// Uses relic counter: starts at 3, decremented each combat until 0.
pub fn at_battle_start(state: &CombatState, counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if counter > 0 {
        // Set all enemies HP to 1 by dealing (currentHP - 1) non-blockable damage
        for monster in &state.monsters {
            if !monster.is_escaped && !monster.is_dying && monster.current_hp > 1 {
                actions.push(ActionInfo {
                    action: Action::LoseHp {
                        target: monster.id,
                        amount: monster.current_hp - 1,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
        }
        // Decrement counter
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::NeowsLament,
                counter: counter - 1,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}
