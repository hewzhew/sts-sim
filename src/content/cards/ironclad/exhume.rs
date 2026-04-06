use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn exhume_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let valid_count = state.exhaust_pile.iter()
        .filter(|c| c.id != crate::content::cards::CardId::Exhume)
        .count();
        
    if valid_count == 0 {
        return smallvec::smallvec![];
    }

    smallvec::smallvec![
        ActionInfo {
            action: Action::SuspendForGridSelect {
                source_pile: crate::state::PileType::Exhaust,
                min: 1,
                max: 1,
                can_cancel: false,
                reason: crate::state::GridSelectReason::Exhume { upgrade: card.upgrades > 0 },
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
