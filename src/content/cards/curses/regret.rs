use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::CombatState;
use smallvec::SmallVec;

/// Regret: Unplayable. At the end of your turn, lose HP equal to the number of cards in your hand.
pub fn on_end_turn_in_hand(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let hand_size = state.hand.len() as i32;

    smallvec::smallvec![ActionInfo {
        action: Action::Damage(DamageInfo {
            source: 0,
            target: 0, // 0 is player
            base: hand_size,
            output: hand_size,
            damage_type: DamageType::HpLoss,
            is_modified: false,
        }),
        insertion_mode: AddTo::Bottom,
    }]
}
