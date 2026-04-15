use crate::runtime::action::{Action, ActionInfo, AddTo};
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
    state: &crate::runtime::combat::CombatState,
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
                damage_type: crate::runtime::action::DamageType::Thorns,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}
