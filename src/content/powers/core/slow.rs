use crate::action::Action;
use crate::combat::{CombatState, CombatCard};
use crate::core::EntityId;

pub fn on_calculate_damage_from_player(
    _state: &CombatState,
    _card: &CombatCard,
    _target: EntityId,
    base_damage: f32,
    amount: i32,
) -> f32 {
    // Each stack of Slow (which increases by 1 for each card played this turn) adds +10% damage
    // The amount given here is the CURRENT amount of Slow stacks
    let multiplier = 1.0 + (amount as f32 * 0.1);
    base_damage * multiplier
}

pub fn at_end_of_turn(_owner: EntityId) -> smallvec::SmallVec<[Action; 2]> {
    // In actual game logic, Slow resets to 0 at the end of the *player's* turn, 
    // or practically it just acts as a counter of cards played this turn.
    // For engine simplicity we evaluate it dynamically or reset it here if the owner turn ends.
    smallvec::smallvec![]
}
