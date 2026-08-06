use super::super::action_priority::ActionOrderingPriority;
use super::types::ActionOrderingEntry;

pub(in crate::ai::combat_search_v2::action_ordering) fn compare_action_ordering_entries(
    left: &ActionOrderingEntry,
    right: &ActionOrderingEntry,
) -> std::cmp::Ordering {
    compare_action_ordering_priorities(
        &left.priority,
        left.root_action_prior_score,
        &right.priority,
        right.root_action_prior_score,
    )
}

pub(in crate::ai::combat_search_v2) fn compare_action_ordering_priorities(
    left: &ActionOrderingPriority,
    left_root_action_prior_score: Option<f64>,
    right: &ActionOrderingPriority,
    right_root_action_prior_score: Option<f64>,
) -> std::cmp::Ordering {
    right
        .recoverable_resource_urgency
        .cmp(&left.recoverable_resource_urgency)
        .then_with(|| right.role_rank.cmp(&left.role_rank))
        .then_with(|| {
            right
                .lethal_external_payoff
                .cmp(&left.lethal_external_payoff)
        })
        .then_with(|| {
            compare_prior_scores(right_root_action_prior_score, left_root_action_prior_score)
        })
        .then_with(|| right.cmp(left))
}

fn compare_prior_scores(left: Option<f64>, right: Option<f64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
