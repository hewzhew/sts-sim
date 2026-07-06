use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;

pub(super) fn enqueue_child_or_remember_leaf(
    loop_state: &mut SearchLoopState,
    child: SearchNode,
    truncated: bool,
) {
    let started = Instant::now();
    if loop_state.stats.nodes_to_first_win.is_none()
        && terminal_label(&child.engine, &child.combat) == SearchTerminalLabel::Win
    {
        loop_state.stats.nodes_to_first_win = Some(loop_state.stats.nodes_generated);
    }

    if !truncated {
        loop_state.push_frontier(child);
    } else {
        loop_state.unresolved_leaf_count = loop_state.unresolved_leaf_count.saturating_add(1);
        loop_state.remember_best_frontier(&child);
    }
    loop_state.performance.child_bookkeeping_elapsed_us = loop_state
        .performance
        .child_bookkeeping_elapsed_us
        .saturating_add(started.elapsed().as_micros());
}
