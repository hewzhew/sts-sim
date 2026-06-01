use std::time::Instant;

use crate::sim::combat::CombatStepper;

use super::super::frontier::SearchNode;
use super::super::types::CombatSearchV2Config;
use super::enumerate::enumerate_turn_plans;
use super::types::{TurnPlanBucket, TurnPlanStopReason, TurnPlannerConfigV1};

const TURN_PLAN_FRONTIER_SEED_MAX_INNER_NODES: usize = 128;
const TURN_PLAN_FRONTIER_SEED_MAX_END_STATES: usize = 8;
const TURN_PLAN_FRONTIER_SEED_PER_BUCKET_LIMIT: usize = 2;

#[derive(Default)]
pub(in crate::ai::combat_search_v2) struct TurnPlanFrontierSeedResult {
    pub(in crate::ai::combat_search_v2) nodes: Vec<SearchNode>,
}

pub(in crate::ai::combat_search_v2) fn root_turn_plan_frontier_seed(
    root: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
) -> TurnPlanFrontierSeedResult {
    if !config.turn_plan_policy.seeds_frontier() {
        return TurnPlanFrontierSeedResult::default();
    }

    let turn_config = TurnPlannerConfigV1 {
        max_inner_nodes: TURN_PLAN_FRONTIER_SEED_MAX_INNER_NODES,
        max_end_states: TURN_PLAN_FRONTIER_SEED_MAX_END_STATES,
        per_bucket_limit: TURN_PLAN_FRONTIER_SEED_PER_BUCKET_LIMIT,
        potion_policy: config.potion_policy,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
    };
    let enumeration = enumerate_turn_plans(root, stepper, &turn_config, deadline);
    let nodes = enumeration
        .plans
        .into_iter()
        .filter(|plan| should_seed_frontier(plan.bucket, plan.stop_reason, plan.actions.len()))
        .map(|plan| plan.end_node)
        .collect();

    TurnPlanFrontierSeedResult { nodes }
}

fn should_seed_frontier(
    bucket: TurnPlanBucket,
    stop_reason: TurnPlanStopReason,
    action_count: usize,
) -> bool {
    if action_count == 0 {
        return false;
    }
    if !matches!(
        bucket,
        TurnPlanBucket::TerminalWin | TurnPlanBucket::Progress
    ) {
        return false;
    }
    !matches!(
        stop_reason,
        TurnPlanStopReason::EngineStepLimit | TurnPlanStopReason::NoLegalActions
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontier_seed_keeps_only_terminal_win_or_progress_plans() {
        assert!(should_seed_frontier(
            TurnPlanBucket::TerminalWin,
            TurnPlanStopReason::Terminal,
            1
        ));
        assert!(should_seed_frontier(
            TurnPlanBucket::Progress,
            TurnPlanStopReason::NextTurn,
            1
        ));

        assert!(!should_seed_frontier(
            TurnPlanBucket::Setup,
            TurnPlanStopReason::NextTurn,
            1
        ));
        assert!(!should_seed_frontier(
            TurnPlanBucket::Balanced,
            TurnPlanStopReason::NextTurn,
            1
        ));
        assert!(!should_seed_frontier(
            TurnPlanBucket::Survival,
            TurnPlanStopReason::NextTurn,
            1
        ));
    }
}
