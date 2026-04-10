use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Pocketwatch: Whenever you end your turn with 3 or fewer cards played, draw 3 additional cards on the next turn.
/// Java: uses a counter flag to track. Simpler: at_end_of_turn checks, schedules extra draw.
pub fn at_end_of_turn(state: &crate::combat::CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if state.turn.counters.cards_played_this_turn <= 3 {
        // Queue extra draw for next turn — this will be added before the draw phase
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Pocketwatch,
                counter: 1, // Flag: draw 3 extra next turn
            },
            insertion_mode: AddTo::Bottom,
        });
    } else {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Pocketwatch,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}

/// At turn start, if counter is flagged, draw 3 extra cards.
pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if counter == 1 {
        actions.push(ActionInfo {
            action: Action::DrawCards(3),
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Pocketwatch,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}
