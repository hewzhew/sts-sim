use crate::action::ActionInfo;
use smallvec::SmallVec;

/// TungstenRod: Whenever you would lose HP, lose 1 less.
pub fn on_lose_hp(_amount: i32) -> SmallVec<[ActionInfo; 4]> {
    // This is a modifier, not an action generator.
    // The actual reduction should be applied in the damage resolution code.
    // This hook signals that TungstenRod is present.
    SmallVec::new()
}

/// Modifier: reduce HP loss by 1 (minimum 0)
pub fn modify_hp_loss(amount: i32) -> i32 {
    (amount - 1).max(0)
}
