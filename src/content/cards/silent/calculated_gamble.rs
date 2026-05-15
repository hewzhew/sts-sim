use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn calculated_gamble_play(
    _state: &CombatState,
    _card: &CombatCard,
) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        // Java CalculatedGamble.use passes `false` even when the card is
        // upgraded. The upgrade changes exhaust only, not draw count.
        action: Action::CalculatedGamble { draw_extra: false },
        insertion_mode: AddTo::Bottom,
    }]
}
