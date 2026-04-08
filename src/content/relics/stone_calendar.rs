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
        for monster in &state.monsters {
            if !monster.is_escaped && !monster.is_dying && monster.current_hp > 0 {
                actions.push(ActionInfo {
                    action: Action::LoseHp {
                        target: monster.id,
                        amount: 52,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
        }
    }
    actions
}
