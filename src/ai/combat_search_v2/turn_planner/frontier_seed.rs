use std::time::Instant;

use crate::sim::combat::CombatStepper;

use super::super::frontier::SearchNode;
use super::super::{CombatSearchPluginStack, CombatSearchV2Config};
use super::enumerate::enumerate_turn_plans;
use super::types::{TurnPlanBucket, TurnPlanStopReason, TurnPlanV1, TurnPlannerConfigV1};

const TURN_PLAN_FRONTIER_SEED_MAX_INNER_NODES: usize = 128;
const TURN_PLAN_FRONTIER_SEED_MAX_END_STATES: usize = 8;
const TURN_PLAN_FRONTIER_SEED_PER_BUCKET_LIMIT: usize = 2;

#[derive(Default)]
pub(in crate::ai::combat_search_v2) struct TurnPlanFrontierSeedResult {
    pub(in crate::ai::combat_search_v2) plans: Vec<TurnPlanV1>,
    pub(in crate::ai::combat_search_v2) turn_plan_prior_scored_plans: usize,
}

pub(in crate::ai::combat_search_v2) fn turn_plan_frontier_seed(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    plugins: &CombatSearchPluginStack,
    deadline: Option<Instant>,
) -> TurnPlanFrontierSeedResult {
    let turn_config = TurnPlannerConfigV1 {
        max_inner_nodes: TURN_PLAN_FRONTIER_SEED_MAX_INNER_NODES,
        max_end_states: TURN_PLAN_FRONTIER_SEED_MAX_END_STATES,
        per_bucket_limit: TURN_PLAN_FRONTIER_SEED_PER_BUCKET_LIMIT,
        potion_policy: plugins.potion.policy,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        turn_plan_prior: config.turn_plan_prior.clone(),
    };
    let enumeration = enumerate_turn_plans(node, stepper, &turn_config, deadline);
    let turn_plan_prior_scored_plans = enumeration.turn_plan_prior_scored_plans;
    let plans = enumeration
        .plans
        .into_iter()
        .filter(|plan| should_seed_frontier(plan.bucket, plan.stop_reason, plan.actions.len()))
        .collect();

    TurnPlanFrontierSeedResult {
        plans,
        turn_plan_prior_scored_plans,
    }
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
        TurnPlanBucket::TerminalWin
            | TurnPlanBucket::Progress
            | TurnPlanBucket::Survival
            | TurnPlanBucket::Setup
            | TurnPlanBucket::Boundary
    ) {
        return false;
    }
    if bucket == TurnPlanBucket::Boundary && stop_reason != TurnPlanStopReason::PendingChoice {
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
    fn frontier_seed_keeps_terminal_progress_and_survival_plans() {
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
        assert!(should_seed_frontier(
            TurnPlanBucket::Survival,
            TurnPlanStopReason::NextTurn,
            1
        ));
        assert!(should_seed_frontier(
            TurnPlanBucket::Boundary,
            TurnPlanStopReason::PendingChoice,
            1
        ));
        assert!(should_seed_frontier(
            TurnPlanBucket::Setup,
            TurnPlanStopReason::NextTurn,
            1
        ));

        assert!(!should_seed_frontier(
            TurnPlanBucket::Balanced,
            TurnPlanStopReason::NextTurn,
            1
        ));
    }
}
