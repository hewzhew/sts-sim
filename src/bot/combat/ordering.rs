use crate::state::core::ClientInput;

use super::terminal::TerminalKind;
use super::types::CombatCandidate;
use super::value::compare_values;
use super::value::CombatValue;

pub(super) fn compare_candidates(
    left: &CombatCandidate,
    right: &CombatCandidate,
) -> std::cmp::Ordering {
    match (left.projection_truncated, right.projection_truncated) {
        (false, true) => return std::cmp::Ordering::Less,
        (true, false) => return std::cmp::Ordering::Greater,
        _ => {}
    }
    let value_cmp = compare_values(&left.value, &right.value);
    if !value_cmp.is_eq() {
        return value_cmp;
    }
    end_turn_tiebreak(&left.input, &right.input, &left.value)
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

pub(super) fn end_turn_tiebreak(
    left: &ClientInput,
    right: &ClientInput,
    value: &CombatValue,
) -> std::cmp::Ordering {
    if prefer_end_turn_on_tie(value) {
        end_turn_first(left, right)
    } else {
        end_turn_last(left, right)
    }
}

fn end_turn_first(left: &ClientInput, right: &ClientInput) -> std::cmp::Ordering {
    match (
        matches!(left, ClientInput::EndTurn),
        matches!(right, ClientInput::EndTurn),
    ) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}

fn prefer_end_turn_on_tie(value: &CombatValue) -> bool {
    matches!(
        value,
        CombatValue::Terminal(outcome) if outcome.kind == TerminalKind::Defeat
    )
}

#[cfg(test)]
mod tests {
    use super::end_turn_tiebreak;
    use crate::bot::combat::terminal::{TerminalKind, TerminalOutcome};
    use crate::bot::combat::value::CombatValue;
    use crate::state::core::ClientInput;

    #[test]
    fn terminal_defeat_ties_prefer_end_turn() {
        let value = CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Defeat,
            final_hp: 0,
            final_block: 0,
        });
        let cmp = end_turn_tiebreak(
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            &ClientInput::EndTurn,
            &value,
        );
        assert_eq!(cmp, std::cmp::Ordering::Greater);
    }
}
