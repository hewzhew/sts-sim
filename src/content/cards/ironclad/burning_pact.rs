use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn burning_pact_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    if state.zones.hand.len() == 1 {
        actions.push(ActionInfo {
            action: Action::ExhaustCard {
                card_uuid: state.zones.hand[0].uuid,
                source_pile: crate::state::PileType::Hand,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else if state.zones.hand.len() > 1 {
        actions.push(ActionInfo {
            action: Action::SuspendForHandSelect {
                min: 1,
                max: 1,
                can_cancel: false,
                filter: crate::state::HandSelectFilter::Any,
                reason: crate::state::HandSelectReason::Exhaust,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions.push(ActionInfo {
        action: Action::DrawCards(card.base_magic_num_mut as u32),
        insertion_mode: AddTo::Bottom,
    });

    actions
}
