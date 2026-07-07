use std::time::Instant;

use super::super::*;
use super::loop_state::SearchLoopState;
use super::rollout_timing::{timed_rollout_estimate, RolloutEstimateSource};

pub(super) fn child_rollout_estimate(
    loop_state: &mut SearchLoopState,
    child: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> RolloutNodeEstimate {
    if terminal_label(&child.engine, &child.combat) != SearchTerminalLabel::Unresolved {
        loop_state.performance.terminal_child_rollout_skips = loop_state
            .performance
            .terminal_child_rollout_skips
            .saturating_add(1);
        return RolloutNodeEstimate::from_node(
            child,
            0,
            RolloutStopReason::TerminalState,
            Some("terminal_child_no_rollout"),
            super::super::rollout_pending_choice::RolloutPendingChoiceProgress::default(),
        );
    }
    if loop_state.plugins.child_rollout == CombatSearchChildRolloutPluginId::LazyOnPop
        && loop_state.plugins.rollout != CombatSearchRolloutPluginId::Disabled
    {
        loop_state.performance.deferred_child_rollout_nodes = loop_state
            .performance
            .deferred_child_rollout_nodes
            .saturating_add(1);
        return RolloutNodeEstimate::unevaluated();
    }
    timed_rollout_estimate(
        &mut loop_state.rollout_cache,
        child,
        stepper,
        config,
        deadline,
        &mut loop_state.performance,
        RolloutEstimateSource::Child,
    )
}
