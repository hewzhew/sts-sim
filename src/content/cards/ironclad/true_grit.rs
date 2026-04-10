use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn true_grit_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::smallvec![ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: card.base_block_mut,
        },
        insertion_mode: AddTo::Bottom,
    }];

    if card.upgrades > 0 {
        if _state.zones.hand.len() == 1 {
            actions.push(ActionInfo {
                action: Action::ExhaustCard {
                    card_uuid: _state.zones.hand[0].uuid,
                    source_pile: crate::state::PileType::Hand,
                },
                insertion_mode: AddTo::Bottom,
            });
        } else if _state.zones.hand.len() > 1 {
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
    } else {
        actions.push(ActionInfo {
            action: Action::ExhaustRandomCard { amount: 1 },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
