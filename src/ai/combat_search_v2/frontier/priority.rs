use super::super::rollout_value::{rollout_priority_value, CombatSearchRolloutValueV1};
use super::super::transition::terminal_label;
use super::super::value::{
    combat_eval_from_rollout_estimate, combat_search_state_value, CombatEvalOutcomeClass,
    CombatEvalSurvivalBucket, CombatSearchStateValueV1,
};
use super::super::SearchTerminalLabel;
use super::super::{
    collector_tactic::{collector_tactic_value, CollectorTacticValueV0},
    CombatSearchActionPriorPluginId,
};
use super::node::SearchNode;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2::frontier) struct NodePriority {
    terminal_rank: i32,
    collector_tactic_gate: CollectorTacticFrontierGate,
    rollout_value: CombatSearchRolloutValueV1,
    action_prior_rank: i32,
    state_value: CombatSearchStateValueV1,
    potion_tactical_priority: i32,
    potion_conservation: i32,
    turn_branch_priority: i32,
    shorter_line: i32,
}

impl Ord for NodePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.terminal_rank
            .cmp(&other.terminal_rank)
            .then_with(|| self.collector_tactic_gate.cmp(&other.collector_tactic_gate))
            .then_with(|| self.rollout_value.cmp(&other.rollout_value))
            .then_with(|| self.action_prior_rank.cmp(&other.action_prior_rank))
            .then_with(|| self.state_value.cmp(&other.state_value))
            .then_with(|| {
                self.potion_tactical_priority
                    .cmp(&other.potion_tactical_priority)
            })
            .then_with(|| self.potion_conservation.cmp(&other.potion_conservation))
            .then_with(|| self.turn_branch_priority.cmp(&other.turn_branch_priority))
            .then_with(|| self.shorter_line.cmp(&other.shorter_line))
    }
}

impl PartialOrd for NodePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub(in crate::ai::combat_search_v2) struct QueueEntry {
    pub(in crate::ai::combat_search_v2::frontier) priority: NodePriority,
    pub(in crate::ai::combat_search_v2::frontier) sequence_id: u64,
    pub(in crate::ai::combat_search_v2) node: SearchNode,
}

impl Eq for QueueEntry {}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.sequence_id == other.sequence_id && self.priority == other.priority
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
pub(in crate::ai::combat_search_v2::frontier) fn priority_for_node(
    node: &SearchNode,
) -> NodePriority {
    priority_for_node_with_action_prior(node, CombatSearchActionPriorPluginId::Default)
}

pub(in crate::ai::combat_search_v2::frontier) fn priority_for_node_with_action_prior(
    node: &SearchNode,
    action_prior: CombatSearchActionPriorPluginId,
) -> NodePriority {
    let terminal_rank = match terminal_label(&node.engine, &node.combat) {
        SearchTerminalLabel::Win => 3,
        SearchTerminalLabel::Unresolved => 2,
        SearchTerminalLabel::Loss => 1,
    };
    NodePriority {
        terminal_rank,
        collector_tactic_gate: collector_tactic_frontier_gate(node, action_prior),
        rollout_value: rollout_priority_value(&node.rollout_estimate),
        action_prior_rank: action_prior_rank(node.action_prior_score),
        state_value: combat_search_state_value(node),
        potion_tactical_priority: node.potion_tactical_priority,
        potion_conservation: -((node.potions_used + node.potions_discarded) as i32),
        turn_branch_priority: node.last_turn_branch_priority,
        shorter_line: -(node.actions.len() as i32),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
struct CollectorTacticFrontierGate {
    enabled: i32,
    rollout_evaluated: i32,
    rollout_outcome: i32,
    rollout_survival: i32,
    rollout_risk_margin: i32,
    tactic: CollectorTacticValueV0,
}

fn collector_tactic_frontier_gate(
    node: &SearchNode,
    action_prior: CombatSearchActionPriorPluginId,
) -> CollectorTacticFrontierGate {
    if !action_prior.is_collector_tactic() {
        return CollectorTacticFrontierGate::default();
    }
    let tactic = collector_tactic_value(&node.combat, action_prior);
    if !tactic.is_applicable() {
        return CollectorTacticFrontierGate::default();
    }
    let eval = combat_eval_from_rollout_estimate(&node.rollout_estimate);
    CollectorTacticFrontierGate {
        enabled: 1,
        rollout_evaluated: i32::from(node.rollout_estimate.evaluated),
        rollout_outcome: match eval.outcome_class() {
            CombatEvalOutcomeClass::Loss => 0,
            CombatEvalOutcomeClass::Unresolved => 1,
            CombatEvalOutcomeClass::Win => 2,
        },
        rollout_survival: match eval.survival_bucket() {
            CombatEvalSurvivalBucket::DeadOrForcedLoss => 0,
            CombatEvalSurvivalBucket::LethalVisible => 1,
            CombatEvalSurvivalBucket::Critical => 2,
            CombatEvalSurvivalBucket::Stabilizing => 3,
            CombatEvalSurvivalBucket::Stable => 4,
        },
        rollout_risk_margin: eval.risk_margin(),
        tactic,
    }
}

fn action_prior_rank(score: Option<f64>) -> i32 {
    score
        .filter(|score| score.is_finite())
        .map(|score| (score.clamp(0.0, 1.0) * 1_000_000.0).round() as i32)
        .unwrap_or_default()
}
