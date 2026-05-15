use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn escape_plan_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![
        ActionInfo {
            action: Action::DrawCardsWithHistory {
                amount: 1,
                clear_history: true,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::EscapePlanBlockIfSkill {
                block: evaluated.base_block_mut,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
