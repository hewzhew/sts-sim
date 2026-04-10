use crate::action::{ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// Gambling Chip: At the start of each combat, after drawing cards,
/// discard any number of cards then draw that many.
///
/// Java: atTurnStartPostDraw() (NOT atBattleStart!)
///   - activated flag ensures it only fires on the first turn.
///   - addToBot(GamblingChipAction) which opens hand select to discard any number, then draw that many.
///
/// In our engine, at_turn_start hooks fire AFTER DrawCards(5) has been queued,
/// so the hand will already be drawn when this executes.
pub fn at_turn_start(
    state: &CombatState,
    _player: &crate::combat::PlayerEntity,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    // Java: activated flag — only fires on the first turn of combat
    if state.turn.turn_count == 1 {
        actions.push(ActionInfo {
            action: crate::action::Action::SuspendForHandSelect {
                min: 0,
                max: 99,
                can_cancel: true,
                filter: crate::state::HandSelectFilter::Any,
                reason: crate::state::HandSelectReason::GamblingChip,
            },
            insertion_mode: AddTo::Bottom, // Java: addToBot
        });
    }
    actions
}
