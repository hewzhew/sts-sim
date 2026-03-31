use crate::combat::CombatState;
use crate::action::{Action, ActionInfo};

pub fn at_turn_start_in_hand(_state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    // Java: Player.applyStartOfTurnCards()
    // e.g. Decay (take 2 damage at end of turn... wait Decay is end of turn. Regret is end of turn. 
    // Doubt is end of turn. Pride is end of turn. Shame is end of turn.
    // What about Start of turn cards? Unplayable usually. 
    // We provide the hook for completeness.
    smallvec::smallvec![]
}
