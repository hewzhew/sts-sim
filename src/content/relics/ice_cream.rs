/// Ice Cream: Energy is now conserved between turns.
/// Handled statically inside `engine::resolve_action` -> `Action::EndTurnTrigger` or `StartTurnTrigger` where energy is normally reset.

pub fn is_ice_cream() -> bool {
    true
}
