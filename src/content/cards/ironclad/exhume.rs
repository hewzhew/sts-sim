use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn exhume_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let valid_count = state
        .zones
        .exhaust_pile
        .iter()
        .filter(|c| c.id != crate::content::cards::CardId::Exhume)
        .count();

    if valid_count == 0 {
        return smallvec::smallvec![];
    }

    smallvec::smallvec![ActionInfo {
        action: Action::SuspendForGridSelect {
            source_pile: crate::state::PileType::Exhaust,
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::GridSelectFilter::NonExhume,
            reason: crate::state::GridSelectReason::Exhume {
                upgrade: card.upgrades > 0
            },
        },
        insertion_mode: AddTo::Bottom,
    }]
}
