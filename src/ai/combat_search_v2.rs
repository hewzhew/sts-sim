use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::Instant;

use crate::content::cards;
use crate::content::monsters::EnemyId;
use crate::projection::combat::monster_preview_total_damage_in_combat;
use crate::runtime::combat::{CombatCard, CombatState, MonsterEntity, Power};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::state::core::{ClientInput, EngineState, PendingChoice};

mod types;

pub use types::*;

#[derive(Clone)]
struct SearchNode {
    engine: EngineState,
    combat: CombatState,
    actions: Vec<CombatSearchV2ActionTrace>,
    initial_hp: i32,
    potions_used: u32,
    potions_discarded: u32,
    cards_played: u32,
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
struct QueueEntry {
    priority: NodePriority,
    sequence_id: u64,
    node: SearchNode,
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
struct ResourceVector {
    hp: i32,
    block: i32,
    potions_used: u32,
    potions_discarded: u32,
    cards_played: u32,
    action_count: usize,
}

pub fn run_combat_search_v2(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
) -> CombatSearchV2Report {
    run_combat_search_v2_with_stepper(engine, combat, config, &EngineCombatStepper)
}

pub fn run_combat_search_v2_with_stepper(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
    stepper: &impl CombatStepper,
) -> CombatSearchV2Report {
    let started = Instant::now();
    let deadline = config.wall_time.map(|duration| started + duration);
    let mut stats = CombatSearchV2Stats::default();
    let initial_hp = combat.entities.player.current_hp;
    let mut dominance: HashMap<String, Vec<ResourceVector>> = HashMap::new();
    let mut frontier = BinaryHeap::new();
    let mut next_sequence_id = 0u64;
    let root = SearchNode {
        engine: engine.clone(),
        combat: combat.clone(),
        actions: Vec::new(),
        initial_hp,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
    };
    if terminal_label(&root.engine, &root.combat) == SearchTerminalLabel::Win {
        stats.nodes_to_first_win = Some(0);
    }
    push_frontier(&mut frontier, root, &mut next_sequence_id);

    let mut best_complete: Option<SearchNode> = None;
    let mut best_frontier: Option<SearchNode> = None;
    let mut unresolved_leaf_count = 0u64;
    let mut max_actions_cut_count = 0u64;
    let mut engine_step_limit_count = 0u64;
    let mut exhausted = false;

    while let Some(entry) = frontier.pop() {
        if stats.nodes_expanded as usize >= config.max_nodes {
            stats.node_budget_hit = true;
            exhausted = true;
            push_frontier(&mut frontier, entry.node, &mut next_sequence_id);
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            stats.deadline_hit = true;
            exhausted = true;
            push_frontier(&mut frontier, entry.node, &mut next_sequence_id);
            break;
        }

        let node = entry.node;
        remember_best_frontier(&mut best_frontier, &node);

        match terminal_label(&node.engine, &node.combat) {
            SearchTerminalLabel::Win => {
                stats.terminal_wins = stats.terminal_wins.saturating_add(1);
                if stats.nodes_to_first_win.is_none() {
                    stats.nodes_to_first_win = Some(stats.nodes_generated);
                }
                remember_best_complete(&mut best_complete, node);
                continue;
            }
            SearchTerminalLabel::Loss => {
                stats.terminal_losses = stats.terminal_losses.saturating_add(1);
                remember_best_complete(&mut best_complete, node);
                continue;
            }
            SearchTerminalLabel::Unresolved => {}
        }

        if node.actions.len() >= config.max_actions_per_line {
            max_actions_cut_count = max_actions_cut_count.saturating_add(1);
            continue;
        }

        let dominance_key = dominance_bucket_key(&node.engine, &node.combat);
        let resource = node.resource_vector();
        if is_dominated(&mut dominance, dominance_key, resource) {
            stats.dominance_prunes = stats.dominance_prunes.saturating_add(1);
            stats.transposition_prunes = stats.transposition_prunes.saturating_add(1);
            continue;
        }

        stats.nodes_expanded = stats.nodes_expanded.saturating_add(1);
        let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
        let legal = filtered_legal_moves(
            &node.engine,
            &node.combat,
            stepper.legal_actions(&position),
            config.potion_policy,
        );
        if legal.is_empty() {
            unresolved_leaf_count = unresolved_leaf_count.saturating_add(1);
            continue;
        }

        for (action_id, input) in legal.into_iter().enumerate() {
            if deadline.is_some_and(|limit| Instant::now() >= limit) {
                stats.deadline_hit = true;
                exhausted = true;
                break;
            }
            let step = stepper.apply_to_stable(
                &position,
                input.clone(),
                CombatStepLimits {
                    max_engine_steps: config.max_engine_steps_per_action,
                    deadline,
                },
            );
            if step.truncated && !step.timed_out {
                engine_step_limit_count = engine_step_limit_count.saturating_add(1);
            }
            if step.timed_out {
                stats.deadline_hit = true;
                exhausted = true;
            }

            let mut child = node.clone_for_child(step.position.engine, step.position.combat);
            child.note_input(&input);
            child.actions.push(CombatSearchV2ActionTrace {
                step_index: node.actions.len(),
                action_id,
                action_key: action_key(&node.combat, &input),
                action_debug: format!("{input:?}"),
            });
            stats.nodes_generated = stats.nodes_generated.saturating_add(1);

            if stats.nodes_to_first_win.is_none()
                && terminal_label(&child.engine, &child.combat) == SearchTerminalLabel::Win
            {
                stats.nodes_to_first_win = Some(stats.nodes_generated);
            }

            if !step.truncated {
                push_frontier(&mut frontier, child, &mut next_sequence_id);
            } else {
                unresolved_leaf_count = unresolved_leaf_count.saturating_add(1);
                remember_best_frontier(&mut best_frontier, &child);
            }
        }

        if exhausted {
            break;
        }
    }

    stats.elapsed_ms = started.elapsed().as_millis();
    let exhaustive = !exhausted && frontier.is_empty();
    let proof_status = if stats.deadline_hit {
        SearchProofStatus::DeadlineHit
    } else if stats.node_budget_hit {
        SearchProofStatus::BudgetExhausted
    } else if exhaustive {
        SearchProofStatus::Exhaustive
    } else {
        SearchProofStatus::FrontierUnresolved
    };
    let top_terminal = if exhaustive {
        best_complete
            .as_ref()
            .map(|node| terminal_label(&node.engine, &node.combat))
            .unwrap_or(SearchTerminalLabel::Unresolved)
    } else {
        SearchTerminalLabel::Unresolved
    };
    let reason = match proof_status {
        SearchProofStatus::Exhaustive => {
            "frontier exhausted; best_complete_trajectory is the best complete trajectory found by this exact-state search".to_string()
        }
        SearchProofStatus::BudgetExhausted => {
            "node budget exhausted; unresolved frontier remains, so no optimality claim is made".to_string()
        }
        SearchProofStatus::DeadlineHit => {
            "wall-clock deadline hit; unresolved frontier remains, so no optimality claim is made".to_string()
        }
        SearchProofStatus::FrontierUnresolved => {
            "frontier unresolved under current safety limits; no optimality claim is made".to_string()
        }
    };

    let sample_states = frontier
        .iter()
        .take(8)
        .map(|entry| summarize_state(&entry.node.engine, &entry.node.combat))
        .collect();
    CombatSearchV2Report {
        schema_name: "CombatSearchV2Report",
        schema_version: 1,
        input_label: config.input_label,
        information_boundary: "engine_state_snapshot_truth_v0",
        search_policy: CombatSearchV2PolicyReport {
            kind: "best_first_atomic_action_graph_search_v2",
            terminal_policy: "whole_combat_terminal_only",
            expansion_order: "lexicographic_priority_enemy_progress_hp_resource_line_length",
            potion_policy: config.potion_policy.label(),
            transposition_table: "exact_runtime_state_key_with_rng_signature",
            dominance_pruning: "same_abstraction_resource_dominance_hp_block_potions_cards_actions",
            rollout_value: "not_used_for_terminal_claims",
            llm_authority: "none",
        },
        budget: CombatSearchV2BudgetReport {
            max_nodes: config.max_nodes,
            max_actions_per_line: config.max_actions_per_line,
            max_engine_steps_per_action: config.max_engine_steps_per_action,
            wall_time_ms: config.wall_time.map(|duration| duration.as_millis()),
        },
        outcome: CombatSearchV2OutcomeReport {
            terminal: top_terminal,
            proof_status,
            reason,
            complete_trajectory_found: best_complete.is_some(),
            exhaustive,
        },
        best_complete_trajectory: best_complete
            .as_ref()
            .map(|node| trajectory_report(node, false)),
        best_frontier_trajectory: best_frontier.as_ref().map(|node| {
            trajectory_report(
                node,
                terminal_label(&node.engine, &node.combat) == SearchTerminalLabel::Unresolved,
            )
        }),
        frontier: CombatSearchV2FrontierReport {
            remaining_states: frontier.len(),
            unresolved_leaf_count,
            max_actions_cut_count,
            engine_step_limit_count,
            sample_states,
        },
        stats,
        evidence_reliability: CombatSearchV2EvidenceReport {
            hidden_info_policy: "uses_only_the_supplied_engine_state; if that state contains hidden draw/rng truth, the report is engine-evidence rather than public-agent evidence",
            random_policy: "rng state is part of the transposition key; belief particles are not implemented in this first runner",
            estimate_policy: "unresolved frontier summaries are estimates/partial evidence and are never reported as terminal outcomes",
            reliability: if exhaustive {
                "exact_under_supplied_state_and_engine_semantics"
            } else {
                "partial_budgeted_evidence"
            },
            warnings: vec![
                "unresolved_cannot_be_claimed_better_than_a_complete_baseline",
                "no_stepwise_human_action_agreement_objective",
                "no_llm_control_path",
                "combat_only_runner_does_not_validate_out_of_combat_strategy_quality",
                "default_potion_policy_disables_potions_until_a_real_potion_option_planner_exists",
            ],
        },
    }
}

fn filtered_legal_moves(
    engine: &EngineState,
    combat: &CombatState,
    legal: Vec<ClientInput>,
    potion_policy: CombatSearchV2PotionPolicy,
) -> Vec<ClientInput> {
    match potion_policy {
        CombatSearchV2PotionPolicy::All => legal,
        CombatSearchV2PotionPolicy::Never => legal
            .into_iter()
            .filter(|input| !is_potion_input(input))
            .collect(),
        CombatSearchV2PotionPolicy::LethalOnly => {
            let allow_potions = matches!(engine, EngineState::CombatPlayerTurn)
                && visible_incoming_damage(combat)
                    >= combat.entities.player.current_hp + combat.entities.player.block;
            legal
                .into_iter()
                .filter(|input| allow_potions || !is_potion_input(input))
                .collect()
        }
    }
}

fn is_potion_input(input: &ClientInput) -> bool {
    matches!(
        input,
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
    )
}

pub fn compare_trajectory_reports(
    search: Option<&CombatSearchV2TrajectoryReport>,
    search_exhaustive: bool,
    baseline: &CombatSearchV2TrajectoryReport,
) -> serde_json::Value {
    let Some(search) = search else {
        return serde_json::json!({
            "verdict": "inconclusive_no_search_complete_trajectory",
            "basis": "whole_combat_outcome",
        });
    };
    if !search_exhaustive || search.terminal == SearchTerminalLabel::Unresolved {
        return serde_json::json!({
            "verdict": "inconclusive_unresolved_search",
            "basis": "whole_combat_outcome",
            "reason": "search has unresolved frontier and cannot claim not-weaker-than-baseline",
            "baseline_terminal": baseline.terminal,
            "search_complete_candidate_terminal": search.terminal,
        });
    }

    let ordering = compare_complete_trajectory(search, baseline);
    serde_json::json!({
        "verdict": match ordering {
            Ordering::Greater => "search_better",
            Ordering::Equal => "search_tied",
            Ordering::Less => "baseline_better",
        },
        "basis": "whole_combat_outcome",
        "criteria_order": [
            "win_over_loss",
            "higher_final_hp",
            "fewer_potions_used",
            "fewer_turns",
            "fewer_cards_played"
        ],
        "search_terminal": search.terminal,
        "baseline_terminal": baseline.terminal,
        "search_final_hp": search.final_hp,
        "baseline_final_hp": baseline.final_hp,
        "search_potions_used": search.potions_used,
        "baseline_potions_used": baseline.potions_used,
        "search_turns": search.turns,
        "baseline_turns": baseline.turns,
    })
}

pub fn trajectory_from_state(
    engine: EngineState,
    combat: CombatState,
    initial_hp: i32,
    actions: Vec<CombatSearchV2ActionTrace>,
    potions_used: u32,
    potions_discarded: u32,
    cards_played: u32,
    estimated: bool,
) -> CombatSearchV2TrajectoryReport {
    let node = SearchNode {
        engine,
        combat,
        actions,
        initial_hp,
        potions_used,
        potions_discarded,
        cards_played,
    };
    trajectory_report(&node, estimated)
}

fn push_frontier(frontier: &mut BinaryHeap<QueueEntry>, node: SearchNode, sequence_id: &mut u64) {
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

fn remember_best_complete(best: &mut Option<SearchNode>, candidate: SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(&candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate);
    }
}

fn remember_best_frontier(best: &mut Option<SearchNode>, candidate: &SearchNode) {
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

fn compare_complete_trajectory(
    left: &CombatSearchV2TrajectoryReport,
    right: &CombatSearchV2TrajectoryReport,
) -> Ordering {
    terminal_rank(left.terminal)
        .cmp(&terminal_rank(right.terminal))
        .then_with(|| left.final_hp.cmp(&right.final_hp))
        .then_with(|| right.potions_used.cmp(&left.potions_used))
        .then_with(|| right.turns.cmp(&left.turns))
        .then_with(|| right.cards_played.cmp(&left.cards_played))
}

fn terminal_rank(label: SearchTerminalLabel) -> i32 {
    match label {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
}

fn is_dominated(
    dominance: &mut HashMap<String, Vec<ResourceVector>>,
    key: String,
    candidate: ResourceVector,
) -> bool {
    let bucket = dominance.entry(key).or_default();
    if bucket.iter().any(|existing| existing.dominates(candidate)) {
        return true;
    }
    bucket.retain(|existing| !candidate.dominates(*existing));
    bucket.push(candidate);
    false
}

impl ResourceVector {
    fn dominates(self, other: ResourceVector) -> bool {
        let no_worse = self.hp >= other.hp
            && self.block >= other.block
            && self.potions_used <= other.potions_used
            && self.potions_discarded <= other.potions_discarded
            && self.cards_played <= other.cards_played
            && self.action_count <= other.action_count;
        let strictly_better = self.hp > other.hp
            || self.block > other.block
            || self.potions_used < other.potions_used
            || self.potions_discarded < other.potions_discarded
            || self.cards_played < other.cards_played
            || self.action_count < other.action_count;
        no_worse && strictly_better
    }
}

impl SearchNode {
    fn clone_for_child(&self, engine: EngineState, combat: CombatState) -> Self {
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

    fn note_input(&mut self, input: &ClientInput) {
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

    fn resource_vector(&self) -> ResourceVector {
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

fn terminal_label(engine: &EngineState, combat: &CombatState) -> SearchTerminalLabel {
    match combat_terminal(engine, combat) {
        CombatTerminal::Win => SearchTerminalLabel::Win,
        CombatTerminal::Loss => SearchTerminalLabel::Loss,
        CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
    }
}

fn trajectory_report(node: &SearchNode, estimated: bool) -> CombatSearchV2TrajectoryReport {
    CombatSearchV2TrajectoryReport {
        terminal: terminal_label(&node.engine, &node.combat),
        estimated,
        actions: node.actions.clone(),
        final_hp: node.combat.entities.player.current_hp,
        final_block: node.combat.entities.player.block,
        hp_loss: (node.initial_hp - node.combat.entities.player.current_hp).max(0),
        turns: node.combat.turn.turn_count,
        potions_used: node.potions_used,
        potions_discarded: node.potions_discarded,
        cards_played: node.cards_played,
        enemy_final_state: node
            .combat
            .entities
            .monsters
            .iter()
            .enumerate()
            .map(|(slot, monster)| summarize_enemy(&node.combat, slot, monster))
            .collect(),
        final_state: summarize_state(&node.engine, &node.combat),
    }
}

fn summarize_state(engine: &EngineState, combat: &CombatState) -> CombatSearchV2StateSummary {
    CombatSearchV2StateSummary {
        engine_state: format!("{engine:?}"),
        terminal: terminal_label(engine, combat),
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        turn_count: combat.turn.turn_count,
        living_enemy_count: living_enemy_count(combat),
        total_enemy_hp: total_living_enemy_hp(combat),
        visible_incoming_damage: visible_incoming_damage(combat),
        hand_count: combat.zones.hand.len(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        limbo_count: combat.zones.limbo.len(),
        queued_cards_count: combat.zones.queued_cards.len(),
    }
}

fn summarize_enemy(
    combat: &CombatState,
    slot: usize,
    monster: &MonsterEntity,
) -> CombatSearchV2EnemySummary {
    CombatSearchV2EnemySummary {
        slot,
        entity_id: monster.id,
        enemy_id: EnemyId::from_id(monster.monster_type)
            .map(|id| format!("{id:?}"))
            .unwrap_or_else(|| format!("MonsterType{}", monster.monster_type)),
        hp: monster.current_hp,
        max_hp: monster.max_hp,
        block: monster.block,
        alive: monster.is_alive_for_action(),
        escaped: monster.is_escaped,
        dying: monster.is_dying,
        half_dead: monster.half_dead,
        planned_move_id: monster.planned_move_id(),
        visible_intent: format!("{:?}", combat.monster_protocol_visible_intent(monster.id)),
        visible_incoming_damage: if monster.is_alive_for_action() {
            monster_preview_total_damage_in_combat(combat, monster)
        } else {
            0
        },
    }
}

fn total_living_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn living_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
}

fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}

fn survival_margin(combat: &CombatState) -> i32 {
    combat.entities.player.current_hp + combat.entities.player.block
        - visible_incoming_damage(combat)
}

fn action_key(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                format!(
                    "combat/play_card/hand:{card_index}/card:{}#{}/target:{}",
                    card_label(card),
                    card.uuid,
                    target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| format!("{input:?}")),
        ClientInput::UsePotion {
            potion_index,
            target,
        } => combat
            .entities
            .potions
            .get(*potion_index)
            .and_then(|potion| potion.as_ref())
            .map(|potion| {
                format!(
                    "combat/use_potion/slot:{potion_index}/potion:{:?}#{}/target:{}",
                    potion.id,
                    potion.uuid,
                    target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "combat/use_potion/slot:{potion_index}/target:{}",
                    target_label(combat, *target)
                )
            }),
        ClientInput::DiscardPotion(slot) => format!("combat/discard_potion/slot:{slot}"),
        ClientInput::EndTurn => "combat/end_turn".to_string(),
        ClientInput::SubmitDiscoverChoice(index) => format!("combat/submit_choice/{index}"),
        ClientInput::SubmitHandSelect(uuids) => format!("combat/hand_select/{}", uuid_list(uuids)),
        ClientInput::SubmitGridSelect(uuids) => format!("combat/grid_select/{}", uuid_list(uuids)),
        ClientInput::SubmitScryDiscard(indices) => {
            format!(
                "combat/scry_discard/{}",
                indices
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        ClientInput::Cancel => "combat/cancel".to_string(),
        ClientInput::Proceed => "combat/proceed".to_string(),
        _ => format!("{input:?}"),
    }
}

fn card_label(card: &CombatCard) -> String {
    format!("{}+{}", cards::java_id(card.id), card.upgrades)
}

fn target_label(combat: &CombatState, target: Option<usize>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

fn uuid_list(uuids: &[u32]) -> String {
    uuids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn dominance_bucket_key(engine: &EngineState, combat: &CombatState) -> String {
    format!(
        "engine:{}|turn:{}|phase:{:?}|energy:{}|player:{}|zones:{}|monsters:{}|powers:{}|potions:{}|queue:{}|rng:{}",
        engine_key(engine),
        combat.turn.turn_count,
        combat.turn.current_phase,
        combat.turn.energy,
        player_non_hp_key(combat),
        zones_key(combat),
        monsters_key(combat),
        powers_key(combat),
        potions_key(combat),
        queue_key(combat),
        format!("{:?}", combat.rng.pool),
    )
}

fn engine_key(engine: &EngineState) -> String {
    match engine {
        EngineState::PendingChoice(choice) => {
            format!("PendingChoice:{}", pending_choice_key(choice))
        }
        _ => format!("{engine:?}"),
    }
}

fn pending_choice_key(choice: &PendingChoice) -> String {
    format!("{choice:?}")
}

fn player_non_hp_key(combat: &CombatState) -> String {
    let player = &combat.entities.player;
    format!(
        "stance:{:?}|orbs:{:?}|relics:{}|energy_master:{}|gold:{}",
        player.stance,
        player.orbs,
        player
            .relics
            .iter()
            .map(|relic| format!(
                "{:?}:{}:{}:{}",
                relic.id, relic.counter, relic.used_up, relic.amount
            ))
            .collect::<Vec<_>>()
            .join(","),
        player.energy_master,
        player.gold,
    )
}

fn zones_key(combat: &CombatState) -> String {
    format!(
        "hand:{}|draw:{}|discard:{}|exhaust:{}|limbo:{}|queued:{}",
        zone_key(&combat.zones.hand),
        zone_key(&combat.zones.draw_pile),
        zone_key(&combat.zones.discard_pile),
        zone_key(&combat.zones.exhaust_pile),
        zone_key(&combat.zones.limbo),
        combat
            .zones
            .queued_cards
            .iter()
            .map(|queued| format!(
                "{}:{}:{:?}",
                card_key(&queued.card),
                target_label(combat, queued.target),
                queued.source
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn zone_key(cards: &[CombatCard]) -> String {
    cards.iter().map(card_key).collect::<Vec<_>>().join(",")
}

fn card_key(card: &CombatCard) -> String {
    format!(
        "{:?}+{}#{}:misc{}:cost{}:{:?}:free{}",
        card.id,
        card.upgrades,
        card.uuid,
        card.misc_value,
        card.get_cost(),
        card.cost_for_turn,
        card.free_to_play_once
    )
}

fn monsters_key(combat: &CombatState) -> String {
    combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            format!(
                "id{}:type{}:hp{}:max{}:block{}:dying{}:escaped{}:half{}:move{}:hist{:?}:plan{:?}:runtime{:?}",
                monster.id,
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.block,
                monster.is_dying,
                monster.is_escaped,
                monster.half_dead,
                monster.planned_move_id(),
                monster.move_history(),
                monster.turn_plan(),
                monster_runtime_key(monster),
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn monster_runtime_key(monster: &MonsterEntity) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        monster.hexaghost,
        monster.louse,
        monster.jaw_worm,
        monster.thief,
        monster.byrd,
        monster.chosen,
        monster.snecko,
        monster.shelled_parasite,
        monster.bronze_automaton,
        monster.bronze_orb,
        monster.book_of_stabbing,
        monster.collector,
        monster.champ,
        monster.awakened_one,
        monster.corrupt_heart,
        monster.writhing_mass,
        monster.spiker,
        monster.spire_shield,
        monster.spire_spear,
        monster.slaver_red,
        monster.gremlin_leader,
        monster.gremlin_nob,
        monster.gremlin_wizard,
        monster.cultist,
        monster.sentry,
        monster.slime_boss,
        monster.large_slime,
        monster.spheric_guardian,
        monster.reptomancer,
        monster.darkling,
        monster.nemesis,
        monster.giant_head,
        monster.time_eater,
        monster.donu,
        monster.deca,
        monster.transient,
        monster.exploder,
        monster.maw,
    )
}

fn powers_key(combat: &CombatState) -> String {
    let mut entries = combat
        .entities
        .power_db
        .iter()
        .map(|(entity, powers)| {
            let mut power_keys = powers.iter().map(power_key).collect::<Vec<_>>();
            power_keys.sort();
            format!("{entity}:{}", power_keys.join(","))
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries.join("|")
}

fn power_key(power: &Power) -> String {
    format!(
        "{:?}:inst{:?}:amount{}:extra{}:payload{:?}:just{}",
        power.power_type,
        power.instance_id,
        power.amount,
        power.extra_data,
        power.payload,
        power.just_applied
    )
}

fn potions_key(combat: &CombatState) -> String {
    combat
        .entities
        .potions
        .iter()
        .enumerate()
        .map(|(index, potion)| match potion {
            Some(potion) => format!("{index}:{:?}:{}", potion.id, potion.uuid),
            None => format!("{index}:empty"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn queue_key(combat: &CombatState) -> String {
    combat
        .engine
        .action_queue
        .iter()
        .map(|action| format!("{action:?}"))
        .collect::<Vec<_>>()
        .join(",")
}
