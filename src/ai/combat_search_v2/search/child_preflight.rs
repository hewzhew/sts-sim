use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ChildPreflightOutcome {
    Continue {
        potion_tactical_priority: Option<i32>,
    },
    Advanced,
    DeadlineReached,
}

pub(super) fn prepare_child_for_expansion(
    loop_state: &mut SearchLoopState,
    parent: &SearchNode,
    ordered_choice: &IndexedActionChoice,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> ChildPreflightOutcome {
    let potion_tactical_priority =
        potions::semantic_potion_tactical_priority(&parent.combat, &ordered_choice.choice.input);
    if config.max_potions_used.is_some_and(|max| {
        parent.potions_used >= max && is_use_potion_input(&ordered_choice.choice.input)
    }) {
        loop_state.potion_budget_cut_count = loop_state.potion_budget_cut_count.saturating_add(1);
        return ChildPreflightOutcome::Advanced;
    }
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        loop_state.stats.deadline_hit = true;
        loop_state.exhausted = true;
        return ChildPreflightOutcome::DeadlineReached;
    }
    ChildPreflightOutcome::Continue {
        potion_tactical_priority,
    }
}
