use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::HandSelectFilter;
use crate::state::HandSelectReason;
use smallvec::SmallVec;

pub fn dual_wield_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let amount = evaluated.base_magic_num_mut as u8;
    let valid_cards: Vec<_> = state
        .zones
        .hand
        .iter()
        .filter(|c| {
            matches!(
                crate::content::cards::get_card_definition(c.id).card_type,
                crate::content::cards::CardType::Attack | crate::content::cards::CardType::Power
            )
        })
        .cloned()
        .collect();

    if valid_cards.is_empty() {
        return actions;
    }

    if valid_cards.len() == 1 {
        actions.push(ActionInfo {
            action: Action::MakeCopyInHand {
                original: Box::new(valid_cards[0].clone()),
                amount,
            },
            insertion_mode: AddTo::Bottom,
        });
        return actions;
    }

    // Java DualWieldAction's multi-candidate selected branch queues one copy
    // before its `dupeAmount` loop; preserve that source-visible behavior.
    actions.push(ActionInfo {
        action: Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: HandSelectFilter::AttackOrPower,
            reason: HandSelectReason::Copy {
                amount: amount.saturating_add(1),
            },
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
