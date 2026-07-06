use super::types::ActionOrderingEntry;

pub(in crate::ai::combat_search_v2::action_ordering) fn compare_action_ordering_entries(
    left: &ActionOrderingEntry,
    right: &ActionOrderingEntry,
) -> std::cmp::Ordering {
    right
        .priority
        .role_rank
        .cmp(&left.priority.role_rank)
        .then_with(|| {
            compare_prior_scores(right.root_action_prior_score, left.root_action_prior_score)
        })
        .then_with(|| right.priority.cmp(&left.priority))
}

fn compare_prior_scores(left: Option<f64>, right: Option<f64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
