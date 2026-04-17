use crate::state::core::ClientInput;

use super::types::CombatCandidate;
use super::value::compare_values;

pub(super) fn compare_candidates(
    left: &CombatCandidate,
    right: &CombatCandidate,
) -> std::cmp::Ordering {
    compare_values(&left.value, &right.value).then_with(|| end_turn_last(&left.input, &right.input))
}

pub(super) fn end_turn_last(left: &ClientInput, right: &ClientInput) -> std::cmp::Ordering {
    match (
        matches!(left, ClientInput::EndTurn),
        matches!(right, ClientInput::EndTurn),
    ) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    }
}
