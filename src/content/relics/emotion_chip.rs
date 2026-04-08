use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::content::relics::RelicState;
use smallvec::SmallVec;

/// EmotionChip (Defect Rare)
/// Java: wasHPLost(amount) → if in combat && amount > 0: set pulse=true
///       atTurnStart() → if pulse: fire ImpulseAction (trigger all orbs), reset pulse
///       onVictory() → reset pulse
///
/// In Rust, we use relic.counter as the pulse flag:
///   counter == 0 → no pulse
///   counter == 1 → pulse active (will fire on next turn start)

pub fn on_lose_hp(
    _state: &CombatState,
    relic: &mut RelicState,
    amount: i32,
) -> SmallVec<[ActionInfo; 4]> {
    // Java: wasHPLost(damageAmount) — if damageAmount > 0, set pulse=true
    if amount > 0 && relic.counter != 1 {
        relic.counter = 1; // Set pulse
    }
    SmallVec::new() // No actions generated here — the effect fires at turn start
}

pub fn at_turn_start(_state: &CombatState, relic: &mut RelicState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    // Java: atTurnStart() — if pulse: addToBot(ImpulseAction), reset pulse
    if relic.counter == 1 {
        relic.counter = 0; // Reset pulse
                           // ImpulseAction triggers all orb passives
        actions.push(ActionInfo {
            action: Action::TriggerPassiveOrbs,
            insertion_mode: AddTo::Bottom, // Java: addToBot
        });
    }
    actions
}
