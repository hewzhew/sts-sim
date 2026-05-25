use super::rollout_value::{rollout_priority_value, CombatSearchRolloutValueV1};
use super::value::{combat_search_state_value, CombatSearchStateValueV1};
use super::*;
use std::hash::Hash;

#[derive(Clone)]
pub(super) struct SearchNode {
    pub(super) engine: EngineState,
    pub(super) combat: CombatState,
    pub(super) actions: Vec<CombatSearchV2ActionTrace>,
    pub(super) turn_prefix: TurnPrefixState,
    pub(super) initial_hp: i32,
    pub(super) potions_used: u32,
    pub(super) potions_discarded: u32,
    pub(super) cards_played: u32,
    pub(super) potion_tactical_priority: i32,
    pub(super) last_turn_branch_priority: i32,
    pub(super) rollout_estimate: RolloutNodeEstimate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodePriority {
    terminal_rank: i32,
    rollout_value: CombatSearchRolloutValueV1,
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
            .then_with(|| self.rollout_value.cmp(&other.rollout_value))
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
pub(super) struct QueueEntry {
    priority: NodePriority,
    sequence_id: u64,
    pub(super) node: SearchNode,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ResourceVector {
    hp: i32,
    block: i32,
    potions_used: u32,
    potions_discarded: u32,
    cards_played: u32,
    action_count: usize,
}

pub(super) fn push_frontier(
    frontier: &mut BinaryHeap<QueueEntry>,
    node: SearchNode,
    sequence_id: &mut u64,
) {
    let priority = priority_for_node(&node);
    frontier.push(QueueEntry {
        priority,
        sequence_id: *sequence_id,
        node,
    });
    *sequence_id = sequence_id.saturating_add(1);
}

fn priority_for_node(node: &SearchNode) -> NodePriority {
    let terminal_rank = match terminal_label(&node.engine, &node.combat) {
        SearchTerminalLabel::Win => 3,
        SearchTerminalLabel::Unresolved => 2,
        SearchTerminalLabel::Loss => 1,
    };
    NodePriority {
        terminal_rank,
        rollout_value: rollout_priority_value(node.rollout_estimate),
        state_value: combat_search_state_value(node),
        potion_tactical_priority: node.potion_tactical_priority,
        potion_conservation: -((node.potions_used + node.potions_discarded) as i32),
        turn_branch_priority: node.last_turn_branch_priority,
        shorter_line: -(node.actions.len() as i32),
    }
}

pub(super) fn remember_best_complete(best: &mut Option<SearchNode>, candidate: SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(&candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate);
    }
}

pub(super) fn remember_best_frontier(best: &mut Option<SearchNode>, candidate: &SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate.clone());
    }
}

fn compare_nodes(left: &SearchNode, right: &SearchNode) -> Ordering {
    CombatOutcomeScore::from_node(left).cmp(&CombatOutcomeScore::from_node(right))
}

pub(super) fn is_resource_covered<K: Eq + Hash>(
    table: &mut HashMap<K, Vec<ResourceVector>>,
    key: K,
    candidate: ResourceVector,
) -> bool {
    let bucket = table.entry(key).or_default();
    if bucket.iter().any(|existing| existing.covers(candidate)) {
        return true;
    }
    bucket.retain(|existing| !candidate.covers(*existing));
    bucket.push(candidate);
    false
}

impl ResourceVector {
    pub(super) fn diagnostic_parts(self) -> ResourceVectorDiagnosticParts {
        ResourceVectorDiagnosticParts {
            hp: self.hp,
            block: self.block,
            potions_used: self.potions_used,
            potions_discarded: self.potions_discarded,
            cards_played: self.cards_played,
            action_count: self.action_count,
        }
    }

    fn covers(self, other: ResourceVector) -> bool {
        self.hp >= other.hp
            && self.block >= other.block
            && self.potions_used <= other.potions_used
            && self.potions_discarded <= other.potions_discarded
            && self.cards_played <= other.cards_played
            && self.action_count <= other.action_count
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ResourceVectorDiagnosticParts {
    pub hp: i32,
    pub block: i32,
    pub potions_used: u32,
    pub potions_discarded: u32,
    pub cards_played: u32,
    pub action_count: usize,
}

impl SearchNode {
    pub(super) fn clone_for_child(&self, engine: EngineState, combat: CombatState) -> Self {
        Self {
            engine,
            combat,
            actions: self.actions.clone(),
            turn_prefix: self.turn_prefix.clone(),
            initial_hp: self.initial_hp,
            potions_used: self.potions_used,
            potions_discarded: self.potions_discarded,
            cards_played: self.cards_played,
            potion_tactical_priority: self.potion_tactical_priority,
            last_turn_branch_priority: self.last_turn_branch_priority,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        }
    }

    pub(super) fn note_input(&mut self, input: &ClientInput) {
        match input {
            ClientInput::UsePotion { .. } => {
                self.potions_used = self.potions_used.saturating_add(1);
            }
            ClientInput::DiscardPotion(_) => {
                self.potions_discarded = self.potions_discarded.saturating_add(1);
            }
            ClientInput::PlayCard { .. } => {
                self.cards_played = self.cards_played.saturating_add(1);
            }
            _ => {}
        }
    }

    pub(super) fn note_potion_tactical_priority(&mut self, priority: Option<i32>) {
        if let Some(priority) = priority {
            self.potion_tactical_priority = self.potion_tactical_priority.max(priority);
        }
    }

    pub(super) fn note_turn_branch_priority(&mut self, priority: i32) {
        self.last_turn_branch_priority = priority;
    }

    pub(super) fn note_turn_prefix(
        &mut self,
        parent_combat: &CombatState,
        input: &ClientInput,
        transition: TurnBranchTransition,
    ) {
        self.turn_prefix = advance_turn_prefix(&self.turn_prefix, parent_combat, input, transition);
    }

    pub(super) fn resource_vector(&self) -> ResourceVector {
        ResourceVector {
            hp: self.combat.entities.player.current_hp,
            block: self.combat.entities.player.block,
            potions_used: self.potions_used,
            potions_discarded: self.potions_discarded,
            cards_played: self.cards_played,
            action_count: self.actions.len(),
        }
    }
}

#[cfg(test)]
mod tests;
