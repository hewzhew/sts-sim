use crate::ai::combat_search_v2::frontier::SearchNode;
use crate::ai::combat_search_v2::turn_planner::types::TurnPlanStepStateV1;
use crate::ai::combat_search_v2::CombatSearchV2ActionFacts;

#[derive(Clone)]
pub(super) struct TurnPlanWorkNode {
    pub(super) node: SearchNode,
    pub(super) action_facts: Vec<CombatSearchV2ActionFacts>,
    pub(super) step_states: Vec<TurnPlanStepStateV1>,
}
