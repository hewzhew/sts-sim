use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Java StoneCalendar:
/// - atBattleStart() => counter = 0
/// - atTurnStart() => ++counter
/// - onPlayerEndTurn() => if counter == 7, deal 52 to all enemies
pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::StoneCalendar,
            counter: counter + 1,
        },
        insertion_mode: AddTo::Bottom,
    }]
}

pub fn at_end_of_turn(
    state: &crate::combat::CombatState,
    counter: i32,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if counter == 7 {
        let damages: smallvec::SmallVec<[i32; 5]> =
            state.entities.monsters.iter().map(|_| 52).collect();
        actions.push(ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages,
                damage_type: crate::action::DamageType::Thorns,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::at_end_of_turn;
    use crate::action::{Action, DamageType};
    use crate::content::test_support::{basic_combat, CombatTestExt};

    #[test]
    fn stone_calendar_uses_thorns_damage_all_enemies() {
        let combat = basic_combat()
            .with_rng_seed(1)
            .with_monster_max_hp(1, 36)
            .with_monster_hp(1, 36);
        let actions = at_end_of_turn(&combat, 7);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0].action,
            Action::DamageAllEnemies {
                source: 0,
                damages,
                damage_type: DamageType::Thorns,
                is_modified: false,
            } if damages.as_slice() == &[52]
        ));
    }
}
