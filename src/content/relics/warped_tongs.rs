use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// WarpedTongs: At the start of your turn, upgrade a random card in your hand for this combat.
/// Java: addToBot(UpgradeRandomCardAction()) — random selection deferred to engine handler.
pub fn at_turn_start(_state: &crate::combat::CombatState) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::UpgradeRandomCard,
        insertion_mode: AddTo::Bottom,
    }]
}
