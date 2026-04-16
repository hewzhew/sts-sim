use crate::state::core::ClientInput;

use super::types::CombatCandidate;

pub(super) fn compare_candidates(
    left: &CombatCandidate,
    right: &CombatCandidate,
) -> std::cmp::Ordering {
    right
        .terminal_kind
        .cmp(&left.terminal_kind)
        .then_with(|| right.survives.cmp(&left.survives))
        .then_with(|| left.projected_unblocked.cmp(&right.projected_unblocked))
        .then_with(|| left.projected_enemy_total.cmp(&right.projected_enemy_total))
        .then_with(|| right.projected_hp.cmp(&left.projected_hp))
        .then_with(|| right.projected_block.cmp(&left.projected_block))
        .then_with(|| end_turn_last(&left.input, &right.input))
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

