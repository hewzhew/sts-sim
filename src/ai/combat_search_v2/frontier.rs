use super::*;
use std::hash::Hash;

#[derive(Clone)]
pub(super) struct SearchNode {
    pub(super) engine: EngineState,
    pub(super) combat: CombatState,
    pub(super) actions: Vec<CombatSearchV2ActionTrace>,
    pub(super) initial_hp: i32,
    pub(super) potions_used: u32,
    pub(super) potions_discarded: u32,
    pub(super) cards_played: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodePriority {
    terminal_rank: i32,
    fewer_living_enemies: i32,
    enemy_progress: i32,
    survival_margin: i32,
    player_hp: i32,
    player_block: i32,
    potion_conservation: i32,
    shorter_line: i32,
}

impl Ord for NodePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.terminal_rank
            .cmp(&other.terminal_rank)
            .then_with(|| self.fewer_living_enemies.cmp(&other.fewer_living_enemies))
            .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
            .then_with(|| self.survival_margin.cmp(&other.survival_margin))
            .then_with(|| self.player_hp.cmp(&other.player_hp))
            .then_with(|| self.player_block.cmp(&other.player_block))
            .then_with(|| self.potion_conservation.cmp(&other.potion_conservation))
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
        fewer_living_enemies: -(living_enemy_count(&node.combat) as i32),
        enemy_progress: -total_living_enemy_hp(&node.combat),
        survival_margin: survival_margin(&node.combat),
        player_hp: node.combat.entities.player.current_hp,
        player_block: node.combat.entities.player.block,
        potion_conservation: -((node.potions_used + node.potions_discarded) as i32),
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
    compare_node_terminal(left, right)
        .then_with(|| {
            left.combat
                .entities
                .player
                .current_hp
                .cmp(&right.combat.entities.player.current_hp)
        })
        .then_with(|| right.potions_used.cmp(&left.potions_used))
        .then_with(|| {
            right
                .combat
                .turn
                .turn_count
                .cmp(&left.combat.turn.turn_count)
        })
        .then_with(|| right.cards_played.cmp(&left.cards_played))
        .then_with(|| {
            total_living_enemy_hp(&right.combat).cmp(&total_living_enemy_hp(&left.combat))
        })
        .then_with(|| right.actions.len().cmp(&left.actions.len()))
}

fn compare_node_terminal(left: &SearchNode, right: &SearchNode) -> Ordering {
    terminal_rank(terminal_label(&left.engine, &left.combat))
        .cmp(&terminal_rank(terminal_label(&right.engine, &right.combat)))
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
    fn covers(self, other: ResourceVector) -> bool {
        self.hp >= other.hp
            && self.block >= other.block
            && self.potions_used <= other.potions_used
            && self.potions_discarded <= other.potions_discarded
            && self.cards_played <= other.cards_played
            && self.action_count <= other.action_count
    }
}

impl SearchNode {
    pub(super) fn clone_for_child(&self, engine: EngineState, combat: CombatState) -> Self {
        Self {
            engine,
            combat,
            actions: self.actions.clone(),
            initial_hp: self.initial_hp,
            potions_used: self.potions_used,
            potions_discarded: self.potions_discarded,
            cards_played: self.cards_played,
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
