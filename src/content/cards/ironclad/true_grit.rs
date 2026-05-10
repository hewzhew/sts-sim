use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn true_grit_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let mut actions = smallvec::smallvec![ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: evaluated.base_block_mut,
        },
        insertion_mode: AddTo::Bottom,
    }];

    let hand_len = state.zones.hand.len();
    if card.upgrades > 0 {
        if hand_len == 1 {
            actions.push(ActionInfo {
                action: Action::ExhaustCard {
                    card_uuid: state.zones.hand[0].uuid,
                    source_pile: crate::state::PileType::Hand,
                },
                insertion_mode: AddTo::Bottom,
            });
        } else if hand_len > 1 {
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
    } else if hand_len == 1 {
        actions.push(ActionInfo {
            action: Action::ExhaustCard {
                card_uuid: state.zones.hand[0].uuid,
                source_pile: crate::state::PileType::Hand,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else if hand_len > 1 {
        actions.push(ActionInfo {
            action: Action::ExhaustRandomCard { amount: 1 },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
