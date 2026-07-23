use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::time::Instant;

use serde::Serialize;
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use sts_core::state::core::ClientInput;

use super::generator::TurnOptionGeneratorPreferredLane;
use super::policy::{
    CombatGuideLaneId, CombatStateGuide, CombatStateGuideRank, SharedCombatActionPolicy,
};
use super::types::{
    exact_hash, CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOption,
    CompleteTurnOptionBoundary, TurnOptionAction, TurnOptionGenerationGap,
    TurnOptionGeneratorConfig,
};
use super::witness_search::{
    OracleCombatDeepStateSnapshot, OracleCombatWitness, OracleCombatWitnessDiscoverySource,
    OracleCombatWitnessProgressSnapshot, OracleCombatWitnessReplayError,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessStateProgressSnapshot,
};
use super::TurnOptionGeneratorSession;

/// Resumable search over a shared graph of exact player-turn boundaries.
///
/// Complete-turn generation remains lazy, but Widen and Deepen are decided at
/// the node that owns the alternatives. A deep path therefore does not have
/// to compete against every shallower generator in one global queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalTurnGraphWitnessConfig {
    pub generator: TurnOptionGeneratorConfig,
    /// One deterministic service unit for a selected node's resumable turn
    /// generator. This controls preemption granularity, not search quality.
    pub generation_quantum_work: usize,
    pub max_turn_depth: usize,
    pub satisfaction: OracleCombatWitnessSatisfaction,
}

impl Default for LocalTurnGraphWitnessConfig {
    fn default() -> Self {
        Self {
            generator: TurnOptionGeneratorConfig::default(),
            generation_quantum_work: 4,
            max_turn_depth: 32,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LocalTurnGraphWitnessQuantum {
    pub additional_selections: usize,
    pub additional_generation_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalTurnGraphWitnessInterruption {
    SelectionBudget,
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalTurnGraphWitnessStatus {
    WitnessFound,
    Partial(LocalTurnGraphWitnessInterruption),
    FrontierExhausted,
    MechanicsGap,
    ReplayMismatch(OracleCombatWitnessReplayError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LocalTurnGraphWitnessCounters {
    pub selections: usize,
    pub node_visits: usize,
    pub generation_work: usize,
    pub engine_steps: usize,
    pub exact_nodes: usize,
    pub exact_edges: usize,
    pub completed_turn_options: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub duplicate_successor_edges: usize,
    pub terminal_losses: usize,
    pub depth_limited_successors: usize,
    pub exhausted_nodes: usize,
    pub maximum_turn_depth: usize,
}

#[derive(Clone, Debug)]
pub struct LocalTurnGraphWitnessReport {
    pub status: LocalTurnGraphWitnessStatus,
    pub counters: LocalTurnGraphWitnessCounters,
    pub root_visits: usize,
    pub root_generated_options: usize,
    pub root_children: usize,
    pub generation_gaps: Vec<TurnOptionGenerationGap>,
    pub witness: Option<OracleCombatWitness>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct LocalTurnGraphStateSnapshot {
    pub exact_state_hash: String,
    pub relative_turn_depth: usize,
    pub visits: usize,
    pub generated_options: usize,
    pub children: usize,
    pub exhausted: bool,
}

/// Read-only root-action attribution using the local graph's own semantics.
///
/// Descendant counts are non-exclusive reachability counts: an exact node
/// shared by two root-action families is truthfully reachable from both.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalTurnGraphRootActionFamilySnapshot {
    pub first_action: ClientInput,
    pub best_root_negative_log_policy: Option<f64>,
    pub completed_root_turn_options: usize,
    pub terminal_wins: usize,
    pub terminal_losses: usize,
    pub escapes: usize,
    pub unique_next_turn_successors: usize,
    pub retained_next_turn_successors: usize,
    pub reachable_exact_states: usize,
    pub reachable_retained_states: usize,
    pub reachable_generation_work: usize,
    pub reachable_completed_turn_options: usize,
    pub max_player_turn: u32,
    pub best_hp_at_max_turn: Option<i32>,
    pub lowest_enemy_hp_at_max_turn: Option<i32>,
}

#[derive(Clone)]
struct LocalRootActionFamilyAccumulator {
    first_action: ClientInput,
    best_root_negative_log_policy: Option<f64>,
    completed_root_turn_options: usize,
    terminal_wins: usize,
    terminal_losses: usize,
    escapes: usize,
}

struct GraphNode {
    generator: TurnOptionGeneratorSession,
    /// One exact incoming path retained for diagnostics only. Search ownership
    /// and scheduling continue to use the shared exact node.
    diagnostic_parent: Option<(usize, usize)>,
    relative_turn_depth: usize,
    visits: usize,
    generated_options: usize,
    children: Vec<GraphEdge>,
    guides: Vec<CombatStateGuide>,
    boundary_service_views: Vec<LocalServiceView>,
    next_boundary_service_view: usize,
    generation_service_views: Vec<LocalServiceView>,
    next_generation_service_view: usize,
    widen_anchor_visits: usize,
    synced_gaps: usize,
    exhausted: bool,
}

struct GraphEdge {
    successor: usize,
    actions: Vec<TurnOptionAction>,
    negative_log_policy: f64,
    visits: usize,
    anchor_visits: usize,
    guide_visits: BTreeMap<CombatGuideLaneId, usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalServiceView {
    Anchor,
    Guide(CombatGuideLaneId),
}

enum SelectedWork {
    Widen {
        node_id: usize,
        path: Vec<(usize, usize)>,
        view: LocalServiceView,
    },
    Restart,
    Exhausted,
}

/// A resumable session. Exact successor nodes and their service statistics are
/// shared across all incoming edges.
pub struct LocalTurnGraphWitnessSession {
    original_root: CombatPosition,
    config: LocalTurnGraphWitnessConfig,
    policy: SharedCombatActionPolicy,
    nodes: Vec<GraphNode>,
    nodes_by_hash: HashMap<String, usize>,
    used: LocalTurnGraphWitnessCounters,
    granted_selections: usize,
    granted_generation_work: usize,
    granted_engine_steps: usize,
    generation_gaps: Vec<TurnOptionGenerationGap>,
    root_action_families: Vec<LocalRootActionFamilyAccumulator>,
    witness: Option<OracleCombatWitness>,
    replay_failure: Option<OracleCombatWitnessReplayError>,
}

impl LocalTurnGraphWitnessSession {
    pub fn with_policy(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let original_root = root.position().clone();
        let root_hash = root.exact_state_hash().to_owned();
        let root_guides = policy.state_guides(root.position());
        let root_boundary_service_views = boundary_service_views(policy.as_ref(), root.position());
        let root_generation_service_views =
            generation_service_views(policy.as_ref(), root.position());
        let generator =
            TurnOptionGeneratorSession::with_policy(root.clone(), config.generator, policy.clone());
        Self {
            original_root,
            config,
            policy,
            nodes: vec![GraphNode {
                generator,
                diagnostic_parent: None,
                relative_turn_depth: 0,
                visits: 0,
                generated_options: 0,
                children: Vec::new(),
                guides: root_guides,
                boundary_service_views: root_boundary_service_views,
                next_boundary_service_view: 0,
                generation_service_views: root_generation_service_views,
                next_generation_service_view: 0,
                widen_anchor_visits: 0,
                synced_gaps: 0,
                exhausted: false,
            }],
            nodes_by_hash: HashMap::from([(root_hash, 0)]),
            used: LocalTurnGraphWitnessCounters {
                exact_nodes: 1,
                ..LocalTurnGraphWitnessCounters::default()
            },
            granted_selections: 0,
            granted_generation_work: 0,
            granted_engine_steps: 0,
            generation_gaps: Vec::new(),
            root_action_families: Vec::new(),
            witness: None,
            replay_failure: None,
        }
    }

    pub fn witness(&self) -> Option<&OracleCombatWitness> {
        self.witness.as_ref()
    }

    pub fn restore_verified_witness(&mut self, witness: OracleCombatWitness) -> Result<(), String> {
        if witness.final_position.combat.runtime.combat_smoked {
            return Err(
                "restored local-turn-graph witness is a Smoke Bomb escape, not a terminal victory"
                    .to_string(),
            );
        }
        if sts_core::sim::combat::combat_terminal(
            &witness.final_position.engine,
            &witness.final_position.combat,
        ) != CombatTerminal::Win
        {
            return Err("restored local-turn-graph witness is not terminal victory".to_string());
        }
        if self
            .witness
            .as_ref()
            .is_none_or(|current| witness_better(&witness, current))
        {
            self.witness = Some(witness);
        }
        Ok(())
    }

    pub fn counters(&self) -> LocalTurnGraphWitnessCounters {
        self.used.clone()
    }

    pub fn retained_state_work(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.generator.retained_work_items())
            .sum::<usize>()
            .saturating_add(self.nodes.iter().filter(|node| !node.exhausted).count())
    }

    pub fn progress_snapshot(&self) -> OracleCombatWitnessProgressSnapshot {
        let root = &self.nodes[0];
        let root_counters = root.generator.counters();
        let mut survival_by_turn =
            BTreeMap::<u32, (OracleCombatDeepStateSnapshot, Vec<TurnOptionAction>)>::new();
        let mut deepest_survival = None::<(OracleCombatDeepStateSnapshot, Vec<TurnOptionAction>)>;
        let mut deepest_progress = None::<(OracleCombatDeepStateSnapshot, Vec<TurnOptionAction>)>;
        let mut max_path_atomic_depth = 0usize;
        for node_id in 0..self.nodes.len() {
            let actions = self.diagnostic_actions_to_node(node_id);
            max_path_atomic_depth = max_path_atomic_depth.max(actions.len());
            let state = local_deep_state_snapshot(&self.nodes[node_id], actions.len());
            let replace_turn =
                survival_by_turn
                    .get(&state.player_turn)
                    .is_none_or(|(current, _)| {
                        (state.player_hp, -state.enemy_total_hp, state.player_block)
                            > (
                                current.player_hp,
                                -current.enemy_total_hp,
                                current.player_block,
                            )
                    });
            if replace_turn {
                survival_by_turn.insert(state.player_turn, (state.clone(), actions.clone()));
            }
            let replace_survival = deepest_survival.as_ref().is_none_or(|(current, _)| {
                (
                    state.player_turn,
                    state.player_hp,
                    -state.enemy_total_hp,
                    state.player_block,
                ) > (
                    current.player_turn,
                    current.player_hp,
                    -current.enemy_total_hp,
                    current.player_block,
                )
            });
            if replace_survival {
                deepest_survival = Some((state.clone(), actions.clone()));
            }
            let replace_progress = deepest_progress.as_ref().is_none_or(|(current, _)| {
                (
                    state.player_turn,
                    -state.enemy_total_hp,
                    state.player_hp,
                    state.player_block,
                ) > (
                    current.player_turn,
                    -current.enemy_total_hp,
                    current.player_hp,
                    current.player_block,
                )
            });
            if replace_progress {
                deepest_progress = Some((state, actions));
            }
        }
        let recent_turn_survival_envelope = survival_by_turn
            .into_values()
            .rev()
            .take(32)
            .map(|(state, _)| state)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        OracleCombatWitnessProgressSnapshot {
            retained_states: self.nodes.iter().filter(|node| !node.exhausted).count(),
            queued_anchor_entries: self.nodes.iter().filter(|node| !node.exhausted).count(),
            queued_guided_entries: Vec::new(),
            guide_queues: Vec::new(),
            generation_gap_count: self.generation_gaps.len(),
            pending_witness_replay: false,
            root_state: Some(OracleCombatWitnessStateProgressSnapshot {
                exact_state_hash: exact_hash(root.generator.root().position()),
                path_atomic_depth: 0,
                path_negative_log_policy: 0.0,
                generator_work: root_counters.generation_work,
                generator_engine_steps: root_counters.engine_steps,
                completed_turn_options: root.generator.total_completed_options(),
                retained_generator_work_items: root.generator.retained_work_items(),
                synced_options: root.generated_options,
                anchor_states_ahead: None,
                guided_states_ahead: None,
                guided_lane_ranks: None,
            }),
            max_player_turn: self
                .nodes
                .iter()
                .map(|node| node.generator.root().position().combat.turn.turn_count)
                .max()
                .unwrap_or_default(),
            deepest_survival_state: deepest_survival.as_ref().map(|(state, _)| state.clone()),
            deepest_progress_state: deepest_progress.as_ref().map(|(state, _)| state.clone()),
            deepest_survival_actions: deepest_survival
                .map(|(_, actions)| actions)
                .unwrap_or_default(),
            deepest_progress_actions: deepest_progress
                .map(|(_, actions)| actions)
                .unwrap_or_default(),
            recent_turn_survival_envelope,
            max_path_atomic_depth,
            max_completed_turn_options_at_state: self
                .nodes
                .iter()
                .map(|node| node.generator.total_completed_options())
                .max()
                .unwrap_or_default(),
            ..OracleCombatWitnessProgressSnapshot::default()
        }
    }

    pub fn advance(
        &mut self,
        quantum: LocalTurnGraphWitnessQuantum,
        stepper: &dyn CombatStepper,
    ) -> LocalTurnGraphWitnessReport {
        self.granted_selections = self
            .granted_selections
            .saturating_add(quantum.additional_selections);
        self.granted_generation_work = self
            .granted_generation_work
            .saturating_add(quantum.additional_generation_work);
        self.granted_engine_steps = self
            .granted_engine_steps
            .saturating_add(quantum.additional_engine_steps);

        let status = loop {
            if self.witness_satisfies() {
                break LocalTurnGraphWitnessStatus::WitnessFound;
            }
            if let Some(error) = self.replay_failure.clone() {
                break LocalTurnGraphWitnessStatus::ReplayMismatch(error);
            }
            if deadline_reached(quantum.deadline) {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::Deadline,
                );
            }
            if self.used.selections >= self.granted_selections {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::SelectionBudget,
                );
            }
            if self.used.generation_work >= self.granted_generation_work {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::GenerationWorkBudget,
                );
            }
            if self.used.engine_steps >= self.granted_engine_steps {
                break LocalTurnGraphWitnessStatus::Partial(
                    LocalTurnGraphWitnessInterruption::EngineStepBudget,
                );
            }

            match self.select_work() {
                SelectedWork::Widen {
                    node_id,
                    path,
                    view,
                } => {
                    self.used.selections = self.used.selections.saturating_add(1);
                    if !self.widen(node_id, &path, view, quantum.deadline, stepper) {
                        break LocalTurnGraphWitnessStatus::Partial(
                            if deadline_reached(quantum.deadline) {
                                LocalTurnGraphWitnessInterruption::Deadline
                            } else {
                                LocalTurnGraphWitnessInterruption::EngineStepBudget
                            },
                        );
                    }
                }
                SelectedWork::Restart => continue,
                SelectedWork::Exhausted => {
                    break if self.generation_gaps.is_empty() {
                        LocalTurnGraphWitnessStatus::FrontierExhausted
                    } else {
                        LocalTurnGraphWitnessStatus::MechanicsGap
                    };
                }
            }
        };
        self.snapshot(status)
    }

    pub fn state_snapshot_by_exact_hash(
        &self,
        exact_state_hash: &str,
    ) -> Option<LocalTurnGraphStateSnapshot> {
        let node_id = *self.nodes_by_hash.get(exact_state_hash)?;
        let node = &self.nodes[node_id];
        Some(LocalTurnGraphStateSnapshot {
            exact_state_hash: exact_state_hash.to_owned(),
            relative_turn_depth: node.relative_turn_depth,
            visits: node.visits,
            generated_options: node.generated_options,
            children: node.children.len(),
            exhausted: node.exhausted,
        })
    }

    pub fn root_action_families(&self) -> Vec<LocalTurnGraphRootActionFamilySnapshot> {
        let mut snapshots = self
            .root_action_families
            .iter()
            .map(|family| self.root_action_family_snapshot(family))
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            left.best_root_negative_log_policy
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.best_root_negative_log_policy.unwrap_or(f64::INFINITY))
        });
        snapshots
    }

    fn root_action_family_snapshot(
        &self,
        family: &LocalRootActionFamilyAccumulator,
    ) -> LocalTurnGraphRootActionFamilySnapshot {
        let root_successors = self.nodes[0]
            .children
            .iter()
            .filter(|edge| {
                edge.actions
                    .first()
                    .is_some_and(|action| action.input == family.first_action)
            })
            .map(|edge| edge.successor)
            .collect::<BTreeSet<_>>();
        let retained_next_turn_successors = root_successors
            .iter()
            .filter(|node_id| !self.nodes[**node_id].exhausted)
            .count();
        let mut pending = root_successors.iter().copied().collect::<VecDeque<_>>();
        let mut reachable = BTreeSet::new();
        while let Some(node_id) = pending.pop_front() {
            if !reachable.insert(node_id) {
                continue;
            }
            pending.extend(
                self.nodes[node_id]
                    .children
                    .iter()
                    .map(|edge| edge.successor),
            );
        }

        let mut max_player_turn = 0;
        let mut best_hp_at_max_turn = None;
        let mut lowest_enemy_hp_at_max_turn = None;
        let mut reachable_generation_work = 0usize;
        let mut reachable_completed_turn_options = 0usize;
        let mut reachable_retained_states = 0usize;
        for node_id in &reachable {
            let node = &self.nodes[*node_id];
            let position = node.generator.root().position();
            let turn = position.combat.turn.turn_count;
            let hp = position.combat.entities.player.current_hp;
            let enemy_hp = position
                .combat
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .map(|monster| monster.current_hp.max(0))
                .sum::<i32>();
            if turn > max_player_turn {
                max_player_turn = turn;
                best_hp_at_max_turn = Some(hp);
                lowest_enemy_hp_at_max_turn = Some(enemy_hp);
            } else if turn == max_player_turn {
                best_hp_at_max_turn =
                    Some(best_hp_at_max_turn.map_or(hp, |current| current.max(hp)));
                lowest_enemy_hp_at_max_turn = Some(
                    lowest_enemy_hp_at_max_turn.map_or(enemy_hp, |current| current.min(enemy_hp)),
                );
            }
            let counters = node.generator.counters();
            reachable_generation_work =
                reachable_generation_work.saturating_add(counters.generation_work);
            reachable_completed_turn_options = reachable_completed_turn_options
                .saturating_add(node.generator.total_completed_options());
            if !node.exhausted {
                reachable_retained_states = reachable_retained_states.saturating_add(1);
            }
        }

        LocalTurnGraphRootActionFamilySnapshot {
            first_action: family.first_action.clone(),
            best_root_negative_log_policy: family.best_root_negative_log_policy,
            completed_root_turn_options: family.completed_root_turn_options,
            terminal_wins: family.terminal_wins,
            terminal_losses: family.terminal_losses,
            escapes: family.escapes,
            unique_next_turn_successors: root_successors.len(),
            retained_next_turn_successors,
            reachable_exact_states: reachable.len(),
            reachable_retained_states,
            reachable_generation_work,
            reachable_completed_turn_options,
            max_player_turn,
            best_hp_at_max_turn,
            lowest_enemy_hp_at_max_turn,
        }
    }

    fn record_root_option(&mut self, option: &CompleteTurnOption) {
        let Some(first_action) = option.actions().first() else {
            return;
        };
        let family_index = self
            .root_action_families
            .iter()
            .position(|family| family.first_action == first_action.input)
            .unwrap_or_else(|| {
                self.root_action_families
                    .push(LocalRootActionFamilyAccumulator {
                        first_action: first_action.input.clone(),
                        best_root_negative_log_policy: None,
                        completed_root_turn_options: 0,
                        terminal_wins: 0,
                        terminal_losses: 0,
                        escapes: 0,
                    });
                self.root_action_families.len() - 1
            });
        let family = &mut self.root_action_families[family_index];
        family.best_root_negative_log_policy = Some(
            family
                .best_root_negative_log_policy
                .map_or(option.negative_log_policy(), |current| {
                    current.min(option.negative_log_policy())
                }),
        );
        family.completed_root_turn_options = family.completed_root_turn_options.saturating_add(1);
        match option.boundary() {
            CompleteTurnOptionBoundary::TerminalWin => {
                family.terminal_wins = family.terminal_wins.saturating_add(1);
            }
            CompleteTurnOptionBoundary::TerminalLoss => {
                family.terminal_losses = family.terminal_losses.saturating_add(1);
            }
            CompleteTurnOptionBoundary::Escape => {
                family.escapes = family.escapes.saturating_add(1);
            }
            CompleteTurnOptionBoundary::NextPlayerTurn => {}
        }
    }

    fn select_work(&mut self) -> SelectedWork {
        let mut node_id = 0usize;
        let mut path = Vec::new();
        loop {
            self.refresh_exhaustion(node_id);
            if self.nodes[node_id].exhausted {
                return if node_id == 0 {
                    SelectedWork::Exhausted
                } else {
                    SelectedWork::Restart
                };
            }

            self.nodes[node_id].visits = self.nodes[node_id].visits.saturating_add(1);
            self.used.node_visits = self.used.node_visits.saturating_add(1);
            let requested_view = {
                let node = &mut self.nodes[node_id];
                let view = node.boundary_service_views
                    [node.next_boundary_service_view % node.boundary_service_views.len()];
                node.next_boundary_service_view = node.next_boundary_service_view.saturating_add(1);
                view
            };
            let selected = select_local_work(&self.nodes[node_id], &self.nodes, requested_view)
                .or_else(|| {
                    select_local_work(&self.nodes[node_id], &self.nodes, LocalServiceView::Anchor)
                });
            let Some(selected) = selected else {
                self.nodes[node_id].exhausted = true;
                self.used.exhausted_nodes = self.used.exhausted_nodes.saturating_add(1);
                return SelectedWork::Restart;
            };
            let LocalWorkChoice::Edge {
                edge_index,
                view: actual_view,
            } = selected
            else {
                let LocalWorkChoice::Widen { view } = selected else {
                    unreachable!()
                };
                let node = &mut self.nodes[node_id];
                debug_assert_eq!(view, LocalServiceView::Anchor);
                node.widen_anchor_visits = node.widen_anchor_visits.saturating_add(1);
                let generation_view = node.generation_service_views
                    [node.next_generation_service_view % node.generation_service_views.len()];
                node.next_generation_service_view =
                    node.next_generation_service_view.saturating_add(1);
                return SelectedWork::Widen {
                    node_id,
                    path,
                    view: generation_view,
                };
            };
            self.nodes[node_id].children[edge_index].visits = self.nodes[node_id].children
                [edge_index]
                .visits
                .saturating_add(1);
            match actual_view {
                LocalServiceView::Anchor => {
                    self.nodes[node_id].children[edge_index].anchor_visits = self.nodes[node_id]
                        .children[edge_index]
                        .anchor_visits
                        .saturating_add(1);
                }
                LocalServiceView::Guide(lane) => {
                    let visits = self.nodes[node_id].children[edge_index]
                        .guide_visits
                        .entry(lane)
                        .or_default();
                    *visits = visits.saturating_add(1);
                }
            }
            let successor = self.nodes[node_id].children[edge_index].successor;
            path.push((node_id, edge_index));
            node_id = successor;
        }
    }

    fn widen(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        view: LocalServiceView,
        deadline: Option<Instant>,
        stepper: &dyn CombatStepper,
    ) -> bool {
        let remaining_work = self
            .granted_generation_work
            .saturating_sub(self.used.generation_work);
        let remaining_steps = self
            .granted_engine_steps
            .saturating_sub(self.used.engine_steps);
        let work = self
            .config
            .generation_quantum_work
            .max(1)
            .min(remaining_work);
        if work == 0 || remaining_steps == 0 {
            return false;
        }

        let (before, after, before_diagnostics, after_diagnostics, options, new_gaps) = {
            let node = &mut self.nodes[node_id];
            node.generator.prefer_lane(match view {
                LocalServiceView::Anchor => TurnOptionGeneratorPreferredLane::Anchor,
                LocalServiceView::Guide(lane) => TurnOptionGeneratorPreferredLane::Guide(lane),
            });
            let before = node.generator.counters();
            let before_diagnostics = node.generator.diagnostics();
            node.generator.advance(
                stepper,
                CombatPlanningQuantum {
                    additional_generation_work: work,
                    additional_engine_steps: remaining_steps.min(work.saturating_mul(
                        self.config.generator.max_engine_steps_per_transition.max(1),
                    )),
                    deadline,
                },
            );
            let after = node.generator.counters();
            let after_diagnostics = node.generator.diagnostics();
            let options = node.generator.take_completed_options();
            let gaps = node.generator.gaps()[node.synced_gaps..].to_vec();
            node.synced_gaps = node.generator.gaps().len();
            (
                before,
                after,
                before_diagnostics,
                after_diagnostics,
                options,
                gaps,
            )
        };

        let used_work = after.generation_work.saturating_sub(before.generation_work);
        let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
        if used_work == 0 && used_steps == 0 {
            return false;
        }
        self.used.generation_work = self.used.generation_work.saturating_add(used_work);
        self.used.engine_steps = self.used.engine_steps.saturating_add(used_steps);
        self.used.applied_action_transitions = self.used.applied_action_transitions.saturating_add(
            after_diagnostics
                .applied_action_transitions
                .saturating_sub(before_diagnostics.applied_action_transitions),
        );
        self.used.unique_successor_states = self.used.unique_successor_states.saturating_add(
            after_diagnostics
                .unique_successor_states
                .saturating_sub(before_diagnostics.unique_successor_states),
        );
        self.used.duplicate_exact_successors = self.used.duplicate_exact_successors.saturating_add(
            after_diagnostics
                .duplicate_exact_successors
                .saturating_sub(before_diagnostics.duplicate_exact_successors),
        );
        self.generation_gaps.extend(new_gaps);

        for option in options {
            if node_id == 0 {
                self.record_root_option(&option);
            }
            self.nodes[node_id].generated_options =
                self.nodes[node_id].generated_options.saturating_add(1);
            self.used.completed_turn_options = self.used.completed_turn_options.saturating_add(1);
            match option.boundary() {
                CompleteTurnOptionBoundary::TerminalWin => {
                    let (mut actions, prefix_negative_log_policy) = self.path_actions(path);
                    actions.extend_from_slice(option.actions());
                    match replay_witness(
                        &self.original_root,
                        &actions,
                        prefix_negative_log_policy + option.negative_log_policy(),
                        stepper,
                    ) {
                        Ok(witness) => {
                            if self
                                .witness
                                .as_ref()
                                .is_none_or(|current| witness_better(&witness, current))
                            {
                                self.witness = Some(witness);
                            }
                        }
                        Err(error) => self.replay_failure = Some(error),
                    }
                    if self.witness_satisfies() {
                        return true;
                    }
                }
                CompleteTurnOptionBoundary::TerminalLoss => {
                    self.used.terminal_losses = self.used.terminal_losses.saturating_add(1);
                }
                CompleteTurnOptionBoundary::Escape => {}
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    self.accept_successor(node_id, option);
                }
            }
        }
        self.refresh_exhaustion(node_id);
        true
    }

    fn witness_satisfies(&self) -> bool {
        let Some(witness) = self.witness.as_ref() else {
            return false;
        };
        match self.config.satisfaction {
            OracleCombatWitnessSatisfaction::FirstWitness => true,
            OracleCombatWitnessSatisfaction::HpLossAtMost(limit) => {
                let initial_hp = self.original_root.combat.entities.player.current_hp;
                let final_hp = witness.final_position.combat.entities.player.current_hp;
                initial_hp.saturating_sub(final_hp).max(0) as u32 <= limit
            }
            OracleCombatWitnessSatisfaction::BudgetOrExhaustion => false,
        }
    }

    fn accept_successor(&mut self, parent_id: usize, option: CompleteTurnOption) {
        let relative_turn_depth = self.nodes[parent_id].relative_turn_depth.saturating_add(1);
        if relative_turn_depth > self.config.max_turn_depth {
            self.used.depth_limited_successors =
                self.used.depth_limited_successors.saturating_add(1);
            return;
        }

        let successor_hash = option.exact_successor_hash().to_owned();
        let successor = if let Some(existing) = self.nodes_by_hash.get(&successor_hash) {
            *existing
        } else {
            let Ok(root) = CombatDecisionRoot::new(option.exact_successor().clone()) else {
                return;
            };
            let guides = self.policy.state_guides(root.position());
            let boundary_service_views =
                boundary_service_views(self.policy.as_ref(), root.position());
            let generation_service_views =
                generation_service_views(self.policy.as_ref(), root.position());
            let node_id = self.nodes.len();
            let generator = TurnOptionGeneratorSession::with_policy(
                root.clone(),
                self.config.generator,
                self.policy.clone(),
            );
            self.nodes.push(GraphNode {
                generator,
                diagnostic_parent: Some((parent_id, self.nodes[parent_id].children.len())),
                relative_turn_depth,
                visits: 0,
                generated_options: 0,
                children: Vec::new(),
                guides,
                boundary_service_views,
                next_boundary_service_view: 0,
                generation_service_views,
                next_generation_service_view: 0,
                widen_anchor_visits: 0,
                synced_gaps: 0,
                exhausted: false,
            });
            self.nodes_by_hash.insert(successor_hash, node_id);
            self.used.exact_nodes = self.nodes.len();
            self.used.maximum_turn_depth = self.used.maximum_turn_depth.max(relative_turn_depth);
            node_id
        };

        let successor_lanes = self.nodes[successor]
            .guides
            .iter()
            .map(|guide| guide.lane)
            .collect::<BTreeSet<_>>();
        let parent = &mut self.nodes[parent_id];
        if let Some(edge) = parent
            .children
            .iter_mut()
            .find(|edge| edge.successor == successor)
        {
            self.used.duplicate_successor_edges =
                self.used.duplicate_successor_edges.saturating_add(1);
            if option
                .negative_log_policy()
                .total_cmp(&edge.negative_log_policy)
                .is_lt()
            {
                edge.actions = option.actions().to_vec();
                edge.negative_log_policy = option.negative_log_policy();
            }
            return;
        }
        parent.children.push(GraphEdge {
            successor,
            actions: option.actions().to_vec(),
            negative_log_policy: option.negative_log_policy(),
            visits: 0,
            anchor_visits: 0,
            guide_visits: BTreeMap::new(),
        });
        for lane in successor_lanes {
            let view = LocalServiceView::Guide(lane);
            if !parent.boundary_service_views.contains(&view) {
                parent.boundary_service_views.push(view);
            }
        }
        parent.exhausted = false;
        self.used.exact_edges = self.used.exact_edges.saturating_add(1);
    }

    fn path_actions(&self, path: &[(usize, usize)]) -> (Vec<TurnOptionAction>, f64) {
        let action_count = path
            .iter()
            .map(|(node_id, edge_index)| self.nodes[*node_id].children[*edge_index].actions.len())
            .sum();
        let mut actions = Vec::with_capacity(action_count);
        let mut negative_log_policy = 0.0;
        for (node_id, edge_index) in path {
            let edge = &self.nodes[*node_id].children[*edge_index];
            actions.extend_from_slice(&edge.actions);
            negative_log_policy += edge.negative_log_policy;
        }
        (actions, negative_log_policy)
    }

    fn diagnostic_actions_to_node(&self, mut node_id: usize) -> Vec<TurnOptionAction> {
        let mut path = Vec::new();
        while let Some(parent) = self.nodes[node_id].diagnostic_parent {
            path.push(parent);
            node_id = parent.0;
        }
        path.reverse();
        self.path_actions(&path).0
    }

    fn refresh_exhaustion(&mut self, node_id: usize) {
        if self.nodes[node_id].exhausted || !self.nodes[node_id].generator.is_finished() {
            return;
        }
        let all_children_exhausted = self.nodes[node_id]
            .children
            .iter()
            .all(|edge| self.nodes[edge.successor].exhausted);
        if all_children_exhausted {
            self.nodes[node_id].exhausted = true;
            self.used.exhausted_nodes = self.used.exhausted_nodes.saturating_add(1);
        }
    }

    fn snapshot(&self, status: LocalTurnGraphWitnessStatus) -> LocalTurnGraphWitnessReport {
        LocalTurnGraphWitnessReport {
            status,
            counters: self.used.clone(),
            root_visits: self.nodes[0].visits,
            root_generated_options: self.nodes[0].generated_options,
            root_children: self.nodes[0].children.len(),
            generation_gaps: self.generation_gaps.clone(),
            witness: self.witness.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum LocalWorkChoice {
    Widen {
        view: LocalServiceView,
    },
    Edge {
        edge_index: usize,
        view: LocalServiceView,
    },
}

fn select_local_work(
    node: &GraphNode,
    nodes: &[GraphNode],
    view: LocalServiceView,
) -> Option<LocalWorkChoice> {
    match view {
        LocalServiceView::Anchor => select_anchor_work(node, nodes),
        LocalServiceView::Guide(lane) => select_guide_work(node, nodes, lane),
    }
}

fn select_anchor_work(node: &GraphNode, nodes: &[GraphNode]) -> Option<LocalWorkChoice> {
    let widen = node.generator.best_retained_path_bound_snapshot().map(
        |(atomic_depth, negative_log_policy)| {
            (
                local_path_service_cost(
                    atomic_depth,
                    negative_log_policy,
                    node.widen_anchor_visits,
                ),
                LocalWorkChoice::Widen {
                    view: LocalServiceView::Anchor,
                },
            )
        },
    );
    let best_edge = node
        .children
        .iter()
        .enumerate()
        .filter(|(_, edge)| !nodes[edge.successor].exhausted)
        .map(|(edge_index, edge)| {
            (
                local_path_service_cost(
                    edge.actions.len(),
                    edge.negative_log_policy,
                    edge.anchor_visits,
                ),
                edge.visits,
                edge.successor,
                LocalWorkChoice::Edge {
                    edge_index,
                    view: LocalServiceView::Anchor,
                },
            )
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
    match (widen, best_edge) {
        (Some((widen_cost, widen)), Some((edge_cost, _, _, edge))) => {
            Some(if widen_cost.total_cmp(&edge_cost).is_le() {
                widen
            } else {
                edge
            })
        }
        (Some((_, widen)), None) => Some(widen),
        (None, Some((_, _, _, edge))) => Some(edge),
        (None, None) => None,
    }
}

fn select_guide_work(
    node: &GraphNode,
    nodes: &[GraphNode],
    lane: CombatGuideLaneId,
) -> Option<LocalWorkChoice> {
    let edge_ranks = node
        .children
        .iter()
        .map(|edge| {
            (!nodes[edge.successor].exhausted)
                .then(|| guide_rank(&nodes[edge.successor], lane).cloned())
                .flatten()
        })
        .collect::<Vec<_>>();
    let ranks = edge_ranks.iter().flatten().collect::<Vec<_>>();
    if ranks.is_empty() {
        return None;
    }

    edge_ranks
        .iter()
        .enumerate()
        .filter_map(|(edge_index, rank)| {
            let rank = rank.as_ref()?;
            let edge = &node.children[edge_index];
            Some((
                rank,
                local_path_base(edge.actions.len(), edge.negative_log_policy),
                edge.visits,
                edge.successor,
                LocalWorkChoice::Edge {
                    edge_index,
                    view: LocalServiceView::Guide(lane),
                },
            ))
        })
        .min_by(|left, right| {
            guide_choice_order(
                left.0, left.1, left.2, left.3, right.0, right.1, right.2, right.3,
            )
        })
        .map(|(_, _, _, _, edge)| edge)
}

fn guide_choice_order(
    left_rank: &CombatStateGuideRank,
    left_anchor: f64,
    left_visits: usize,
    left_successor: usize,
    right_rank: &CombatStateGuideRank,
    right_anchor: f64,
    right_visits: usize,
    right_successor: usize,
) -> std::cmp::Ordering {
    // The policy-only anchor already owns completeness and fair service. An
    // auxiliary guide must remain exploitative; charging it service debt at
    // every tree level makes a good multi-turn corridor lose a fresh fraction
    // of its budget at every parent.
    right_rank
        .cmp(left_rank)
        .then_with(|| left_anchor.total_cmp(&right_anchor))
        .then_with(|| left_visits.cmp(&right_visits))
        .then_with(|| left_successor.cmp(&right_successor))
}

fn local_path_base(atomic_depth: usize, negative_log_policy: f64) -> f64 {
    negative_log_policy + (atomic_depth.max(1) as f64).ln()
}

fn local_path_service_cost(atomic_depth: usize, negative_log_policy: f64, services: usize) -> f64 {
    local_path_base(atomic_depth, negative_log_policy) + (services.saturating_add(1) as f64).ln()
}

fn guide_rank(node: &GraphNode, lane: CombatGuideLaneId) -> Option<&CombatStateGuideRank> {
    node.guides
        .iter()
        .find(|guide| guide.lane == lane)
        .map(|guide| &guide.rank)
}

fn boundary_service_views(
    policy: &dyn super::policy::CombatActionPolicy,
    position: &CombatPosition,
) -> Vec<LocalServiceView> {
    let lanes = policy
        .state_guides(position)
        .into_iter()
        .map(|guide| guide.lane)
        .collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

fn generation_service_views(
    policy: &dyn super::policy::CombatActionPolicy,
    position: &CombatPosition,
) -> Vec<LocalServiceView> {
    let lanes = policy
        .turn_generation_guides(position)
        .into_iter()
        .map(|guide| guide.lane)
        .collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

fn replay_witness(
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    negative_log_policy: f64,
    stepper: &dyn CombatStepper,
) -> Result<OracleCombatWitness, OracleCombatWitnessReplayError> {
    let mut position = root.clone();
    let mut engine_steps = 0usize;
    for (action_index, action) in actions.iter().enumerate() {
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(OracleCombatWitnessReplayError::IllegalInput { action_index });
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: action.engine_steps.max(1),
                deadline: None,
            },
        );
        engine_steps = engine_steps.saturating_add(result.engine_steps);
        if result.truncated || result.timed_out {
            return Err(OracleCombatWitnessReplayError::TransitionStepLimit { action_index });
        }
        if exact_hash(&result.position) != action.expected_successor_hash {
            return Err(OracleCombatWitnessReplayError::SuccessorMismatch { action_index });
        }
        position = result.position;
    }
    if stepper.terminal(&position) != CombatTerminal::Win {
        return Err(OracleCombatWitnessReplayError::FinalStateIsNotWin);
    }
    Ok(OracleCombatWitness {
        actions: actions.to_vec(),
        final_position: position,
        negative_log_policy,
        replay_engine_steps: engine_steps,
        discovery_source: OracleCombatWitnessDiscoverySource::PlannerSearch,
    })
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn witness_better(left: &OracleCombatWitness, right: &OracleCombatWitness) -> bool {
    left.final_position
        .combat
        .entities
        .player
        .current_hp
        .cmp(&right.final_position.combat.entities.player.current_hp)
        .then_with(|| right.actions.len().cmp(&left.actions.len()))
        .then_with(|| {
            right
                .negative_log_policy
                .total_cmp(&left.negative_log_policy)
        })
        == std::cmp::Ordering::Greater
}

fn local_deep_state_snapshot(
    node: &GraphNode,
    path_atomic_depth: usize,
) -> OracleCombatDeepStateSnapshot {
    let combat = &node.generator.root().position().combat;
    let alive_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .collect::<Vec<_>>();
    OracleCombatDeepStateSnapshot {
        player_turn: combat.turn.turn_count,
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        alive_enemy_count: alive_monsters.len(),
        enemy_total_hp: alive_monsters
            .into_iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        hand_size: combat.zones.hand.len(),
        draw_pile_size: combat.zones.draw_pile.len(),
        discard_pile_size: combat.zones.discard_pile.len(),
        exhaust_pile_size: combat.zones.exhaust_pile.len(),
        path_atomic_depth,
    }
}

#[cfg(test)]
mod tests {
    use super::{guide_choice_order, local_path_service_cost, GraphEdge};
    use crate::policy::CombatStateGuideRank;

    fn edge(negative_log_policy: f64, visits: usize) -> GraphEdge {
        GraphEdge {
            successor: 0,
            actions: Vec::new(),
            negative_log_policy,
            visits,
            anchor_visits: visits,
            guide_visits: Default::default(),
        }
    }

    #[test]
    fn virtual_widen_and_materialized_child_share_one_local_service_currency() {
        let widen_before = local_path_service_cost(2, 0.5, 0);
        let child_before = local_path_service_cost(3, 0.7, 0);
        assert!(widen_before < child_before);

        let widen_after_service = local_path_service_cost(2, 0.5, 2);
        assert!(child_before < widen_after_service);
    }

    #[test]
    fn local_policy_service_cannot_permanently_starve_lower_prior_child() {
        let preferred = edge(0.0, 0);
        let alternate = edge(1.0, 0);
        let preferred_cost =
            preferred.negative_log_policy + (preferred.anchor_visits.saturating_add(1) as f64).ln();
        let alternate_cost =
            alternate.negative_log_policy + (alternate.anchor_visits.saturating_add(1) as f64).ln();
        assert!(preferred_cost < alternate_cost);

        let preferred_after_service = edge(0.0, 3);
        let preferred_after_cost = preferred_after_service.negative_log_policy
            + (preferred_after_service.anchor_visits.saturating_add(1) as f64).ln();
        assert!(alternate_cost < preferred_after_cost);
    }

    #[test]
    fn guide_exploits_its_best_child_while_anchor_owns_fairness() {
        let best = CombatStateGuideRank::new(vec![1, 0]);
        let alternate = CombatStateGuideRank::new(vec![0, 10_000]);

        assert!(
            guide_choice_order(&best, 100.0, usize::MAX, 9, &alternate, 0.0, 0, 1).is_lt(),
            "guide service debt must not overturn the guide's semantic ordering"
        );
    }
}
