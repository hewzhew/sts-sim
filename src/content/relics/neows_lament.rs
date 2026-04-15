use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// NeowsLament: Enemies in your first 3 combats have 1 HP.
/// Uses relic counter: starts at 3, decremented each combat until 0.
pub fn at_battle_start(state: &CombatState, counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if counter > 0 {
        // Java mutates monster currentHealth directly to 1; this is not HP-loss damage.
        for monster in &state.entities.monsters {
            if !monster.is_escaped && !monster.is_dying && monster.current_hp > 1 {
                actions.push(ActionInfo {
                    action: Action::SetCurrentHp {
                        target: monster.id,
                        hp: 1,
                    },
                    insertion_mode: AddTo::Top,
                });
            }
        }
        let next_counter = counter - 1;
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::NeowsLament,
                counter: if next_counter == 0 { -2 } else { next_counter },
            },
            insertion_mode: AddTo::Top,
        });
        if next_counter == 0 {
            actions.push(ActionInfo {
                action: Action::UpdateRelicUsedUp {
                    relic_id: crate::content::relics::RelicId::NeowsLament,
                    used_up: true,
                },
                insertion_mode: AddTo::Top,
            });
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::at_battle_start;
    use crate::action::{Action, AddTo};
    use crate::content::test_support::{basic_combat, CombatTestExt};

    #[test]
    fn neows_lament_sets_monsters_to_one_hp_directly() {
        let combat = basic_combat()
            .with_rng_seed(1)
            .with_player_hp(40)
            .with_player_max_hp(80)
            .with_monster_max_hp(1, 36)
            .with_monster_hp(1, 36);
        let actions = at_battle_start(&combat, 3);
        assert!(actions
            .iter()
            .any(|info| matches!(info.action, Action::SetCurrentHp { target: 1, hp: 1 })));
        assert!(actions.iter().all(|info| info.insertion_mode == AddTo::Top));
        assert!(!actions
            .iter()
            .any(|info| matches!(info.action, Action::LoseHp { .. })));
    }
}
