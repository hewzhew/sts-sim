use super::*;

mod best_trajectories;
mod bootstrap;
mod child_dominance;
mod child_expansion;
mod child_frontier;
mod child_node;
mod child_preflight;
mod child_rollout;
mod child_step;
mod finalize;
mod finish_coverage;
mod finish_diagnostics;
mod finish_evidence;
mod finish_frontier;
mod finish_outcome;
mod finish_policy;
mod finish_trajectories;
mod loop_state;
mod node_action_ordering;
mod node_action_surface;
mod node_budget;
mod node_child_observers;
mod node_deferred_rollout;
mod node_expansion;
mod node_preflight;
mod node_pruning;
mod node_terminal;
mod pending_choice_expansion;
mod rollout_terminal_promotion;
mod rollout_timing;
mod root_evidence;
mod turn_boundary_expansion;
mod turn_plan_seed_gate;
mod turn_plan_seeding;
mod win_acceptance;

use bootstrap::{deferred_root_rollout_estimate, initialize_root_frontier};
use child_expansion::{expand_ordered_child, ChildExpansionInput, ChildExpansionOutcome};
use finalize::{finish_combat_search_report, SearchFinishInput};
use finish_diagnostics::finish_diagnostics_and_timing;
use loop_state::SearchLoopState;
use node_expansion::prepare_node_expansion;
use node_preflight::{prepare_node_for_expansion, NodePreflightInput, NodePreflightOutcome};
use pending_choice_expansion::{expand_pending_choice_prefix, PendingChoicePrefixOutcome};
use rollout_terminal_promotion::{promote_replayable_terminal_rollout, RolloutPromotionOutcome};
use root_evidence::root_evidence_snapshot;
#[cfg(test)]
use turn_plan_seed_gate::{should_seed_turn_plan_at_node, tactical_enemy_turn_plan_seed_gate};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatSearchV2WorkQuantum {
    pub additional_nodes: usize,
    pub soft_wall_time: Option<std::time::Duration>,
}

#[derive(Clone, Debug)]
pub struct CombatSearchV2DecisionSnapshot {
    pub candidate_frontier: Vec<CombatSearchV2TrajectoryReport>,
    pub candidate_frontier_revision: u64,
    pub best_win: Option<CombatSearchV2TrajectoryReport>,
    pub nodes_expanded: u64,
    pub frontier_remaining_states: usize,
    pub exact_state_keys: usize,
    pub rollout_cache_entries: usize,
    pub root_evidence: CombatSearchV2RootEvidenceSnapshot,
}

pub struct CombatSearchV2Session {
    config: CombatSearchV2Config,
    policy_evidence: CombatSearchV2PolicyEvidenceReport,
    loop_state: SearchLoopState,
    root_for_turn_plan_diagnostics: SearchNode,
    active_elapsed: std::time::Duration,
    complete: bool,
    quantum_history: Vec<CombatSearchV2QuantumEvidence>,
    authorized_nodes: usize,
    authorized_wall_time: Option<std::time::Duration>,
    root_rollout_pending: bool,
}

pub fn run_combat_search_v2(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
) -> CombatSearchV2Report {
    run_combat_search_v2_inner(engine, combat, config, &EngineCombatStepper)
}

pub fn run_combat_search_v2_with_stepper(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
    stepper: &impl CombatStepper,
) -> CombatSearchV2Report {
    run_combat_search_v2_inner(engine, combat, config, stepper)
}

fn run_combat_search_v2_inner(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
    stepper: &impl CombatStepper,
) -> CombatSearchV2Report {
    let quantum = CombatSearchV2WorkQuantum {
        additional_nodes: config.max_nodes,
        soft_wall_time: config.wall_time,
    };
    let mut session = CombatSearchV2Session::new_with_stepper(engine, combat, config, stepper);
    session.advance_with_stepper(quantum, stepper);
    session.finish_with_stepper(stepper)
}

impl CombatSearchV2Session {
    pub fn new(engine: &EngineState, combat: &CombatState, config: CombatSearchV2Config) -> Self {
        Self::new_with_stepper(engine, combat, config, &EngineCombatStepper)
    }

    fn new_with_stepper(
        engine: &EngineState,
        combat: &CombatState,
        config: CombatSearchV2Config,
        stepper: &impl CombatStepper,
    ) -> Self {
        let started = Instant::now();
        let policy_evidence = combat_search_policy_evidence_for_combat(combat);
        let mut loop_state = SearchLoopState::new(
            &config,
            stepper.supports_canonical_pending_choice_actions(),
            outcome_score::external_burden_count(combat),
        );
        let root_for_turn_plan_diagnostics =
            initialize_root_frontier(&mut loop_state, engine, combat, stepper, &config, None);
        let root_rollout_pending = terminal_label(engine, combat)
            == SearchTerminalLabel::Unresolved
            && !pending_choice_expansion::pending_choice_prefix_owned(&loop_state, engine);
        Self {
            config,
            policy_evidence,
            loop_state,
            root_for_turn_plan_diagnostics,
            active_elapsed: started.elapsed(),
            complete: false,
            quantum_history: Vec::new(),
            authorized_nodes: 0,
            authorized_wall_time: Some(std::time::Duration::ZERO),
            root_rollout_pending,
        }
    }

    pub fn advance(&mut self, quantum: CombatSearchV2WorkQuantum) -> CombatSearchV2AdvanceStop {
        self.advance_with_stepper(quantum, &EngineCombatStepper)
    }

    fn advance_with_stepper(
        &mut self,
        quantum: CombatSearchV2WorkQuantum,
        stepper: &impl CombatStepper,
    ) -> CombatSearchV2AdvanceStop {
        let before = self.quantum_counters();
        if self.complete {
            let stop = CombatSearchV2AdvanceStop::AlreadyComplete;
            self.record_quantum(quantum, stop, before);
            return stop;
        }
        self.authorized_nodes = self
            .authorized_nodes
            .saturating_add(quantum.additional_nodes);
        self.authorized_wall_time = match (self.authorized_wall_time, quantum.soft_wall_time) {
            (Some(total), Some(additional)) => Some(total.saturating_add(additional)),
            _ => None,
        };
        self.loop_state.begin_work_quantum();
        let started = Instant::now();
        let deadline = quantum.soft_wall_time.map(|duration| started + duration);
        let mut quantum_config = self.config.clone();
        quantum_config.max_nodes = (self.loop_state.stats.nodes_expanded as usize)
            .saturating_add(quantum.additional_nodes);
        let pending_choice_prefix_limit =
            (self.loop_state.performance.pending_choice_prefixes_expanded as usize)
                .saturating_add(quantum.additional_nodes);
        quantum_config.wall_time = quantum.soft_wall_time;
        if self.root_rollout_pending {
            let deadline_skips_before = self.loop_state.rollout_cache.deadline_budget_skips;
            let mut estimate = deferred_root_rollout_estimate(
                &mut self.loop_state,
                &self.root_for_turn_plan_diagnostics,
                stepper,
                &quantum_config,
                deadline,
            );
            let deadline_interrupted = estimate.stop_reason == RolloutStopReason::Deadline
                || self.loop_state.rollout_cache.deadline_budget_skips > deadline_skips_before;
            if estimate.stop_reason == RolloutStopReason::Deadline {
                let root_key = combat_exact_state_key(
                    &self.root_for_turn_plan_diagnostics.engine,
                    &self.root_for_turn_plan_diagnostics.combat,
                );
                self.loop_state.rollout_cache.cache.remove(&root_key);
                estimate = RolloutNodeEstimate::unevaluated();
            }
            self.root_for_turn_plan_diagnostics.rollout_estimate = estimate.clone();
            self.loop_state
                .frontier
                .replace_exact_state_rollout_estimate(
                    &self.root_for_turn_plan_diagnostics,
                    &estimate,
                );
            self.root_rollout_pending = deadline_interrupted
                && (self.loop_state.rollout_cache.evaluations as usize)
                    < self.loop_state.rollout_cache.max_evaluations
                && terminal_label(
                    &self.root_for_turn_plan_diagnostics.engine,
                    &self.root_for_turn_plan_diagnostics.combat,
                ) == SearchTerminalLabel::Unresolved;
        }
        let mut stop = run_search_quantum(
            &mut self.loop_state,
            stepper,
            &quantum_config,
            started,
            deadline,
            pending_choice_prefix_limit,
        );
        let promotion = promote_replayable_terminal_rollout(
            &mut self.loop_state,
            &self.root_for_turn_plan_diagnostics,
            stepper,
            &self.config,
            deadline,
        );
        if promotion == RolloutPromotionOutcome::ReplayInterrupted {
            self.loop_state.mark_deadline_hit();
            stop = CombatSearchV2AdvanceStop::QuantumWallTime;
        }
        if self.loop_state.accepted_complete_candidate {
            stop = CombatSearchV2AdvanceStop::CandidateSatisfied;
        }
        self.active_elapsed = self.active_elapsed.saturating_add(started.elapsed());
        if matches!(
            stop,
            CombatSearchV2AdvanceStop::CandidateSatisfied
                | CombatSearchV2AdvanceStop::FrontierExhausted
        ) {
            self.complete = true;
        }
        self.record_quantum(quantum, stop, before);
        stop
    }

    pub fn snapshot(&self) -> CombatSearchV2DecisionSnapshot {
        CombatSearchV2DecisionSnapshot {
            candidate_frontier: self
                .loop_state
                .trajectories
                .win_candidates
                .iter()
                .map(|node| trajectory_report(node, false))
                .collect(),
            candidate_frontier_revision: self.loop_state.trajectories.win_frontier_revision,
            best_win: self
                .loop_state
                .trajectories
                .best_win
                .as_ref()
                .map(|node| trajectory_report(node, false)),
            nodes_expanded: self.loop_state.stats.nodes_expanded,
            frontier_remaining_states: self.loop_state.frontier.concrete_state_count(),
            exact_state_keys: self.loop_state.exact_transpositions.len(),
            rollout_cache_entries: self.loop_state.rollout_cache.cache.len(),
            root_evidence: root_evidence_snapshot(&self.loop_state),
        }
    }

    pub fn finish(self) -> CombatSearchV2Report {
        self.finish_with_stepper(&EngineCombatStepper)
    }

    fn finish_with_stepper(mut self, stepper: &impl CombatStepper) -> CombatSearchV2Report {
        finish_diagnostics_and_timing(
            &mut self.loop_state,
            self.active_elapsed,
            &self.root_for_turn_plan_diagnostics,
            stepper,
            &self.config,
        );
        self.config.max_nodes = self.authorized_nodes;
        self.config.wall_time = self.authorized_wall_time;
        finish_combat_search_report(SearchFinishInput {
            config: self.config,
            policy_evidence: self.policy_evidence,
            loop_state: self.loop_state,
            quantum_history: self.quantum_history,
        })
    }

    fn quantum_counters(&self) -> CombatSearchV2QuantumCounters {
        CombatSearchV2QuantumCounters {
            nodes_expanded: self.loop_state.stats.nodes_expanded,
            nodes_generated: self.loop_state.stats.nodes_generated,
            pending_choice_prefixes_expanded: self
                .loop_state
                .performance
                .pending_choice_prefixes_expanded,
            rollout_promotion_actions_replayed: self
                .loop_state
                .performance
                .rollout_promotion_actions_replayed,
            frontier_work_items: self.loop_state.frontier.len(),
            exact_state_keys: self.loop_state.exact_transpositions.len(),
            rollout_cache_entries: self.loop_state.rollout_cache.cache.len(),
        }
    }

    fn record_quantum(
        &mut self,
        quantum: CombatSearchV2WorkQuantum,
        stop: CombatSearchV2AdvanceStop,
        before: CombatSearchV2QuantumCounters,
    ) {
        self.quantum_history.push(CombatSearchV2QuantumEvidence {
            quantum_index: self.quantum_history.len(),
            requested_additional_nodes: quantum.additional_nodes,
            requested_soft_wall_time_ms: quantum.soft_wall_time.map(|value| value.as_millis()),
            stop,
            before,
            after: self.quantum_counters(),
            root_evidence: root_evidence_snapshot(&self.loop_state),
        });
    }
}

fn run_search_quantum(
    loop_state: &mut SearchLoopState,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    started: Instant,
    deadline: Option<Instant>,
    pending_choice_prefix_limit: usize,
) -> CombatSearchV2AdvanceStop {
    loop {
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            loop_state.mark_deadline_hit();
            return CombatSearchV2AdvanceStop::QuantumWallTime;
        }
        let Some(entry) = loop_state.pop_frontier() else {
            return CombatSearchV2AdvanceStop::FrontierExhausted;
        };

        if let Some(work) = entry.pending_choice_work {
            if expand_pending_choice_prefix(
                loop_state,
                entry.node,
                work,
                stepper,
                config,
                pending_choice_prefix_limit,
                None,
            ) == PendingChoicePrefixOutcome::Stop
            {
                return quantum_stop_reason(loop_state);
            }
            continue;
        }

        let node = match prepare_node_for_expansion(
            loop_state,
            NodePreflightInput {
                node: entry.node,
                started,
                stepper,
                config,
                deadline: None,
            },
        ) {
            NodePreflightOutcome::Expand(node) => node,
            NodePreflightOutcome::Continue => continue,
            NodePreflightOutcome::Stop => return quantum_stop_reason(loop_state),
        };

        let Some(mut expansion) = prepare_node_expansion(loop_state, &node, stepper, config) else {
            continue;
        };

        for ordered_action in expansion.ordered_choices {
            let outcome = expand_ordered_child(
                loop_state,
                &mut expansion.turn_branching,
                &mut expansion.turn_local_dominance,
                ChildExpansionInput {
                    parent: &node,
                    position: &expansion.position,
                    ordered_choice: ordered_action.choice,
                    action_ordering_frontier_hint: ordered_action.action_ordering_frontier_hint,
                    action_prior_state_hash: expansion.action_prior_state_hash.as_deref(),
                    pending_choice: expansion.pending_choice.as_ref(),
                    stepper,
                    config,
                    deadline: None,
                },
            );
            debug_assert_ne!(outcome, ChildExpansionOutcome::DeadlineReached);
        }
        loop_state
            .diagnostics
            .observe_turn_branching(&expansion.turn_branching);
        loop_state
            .diagnostics
            .observe_turn_local_dominance(&expansion.turn_local_dominance);

        if loop_state.exhausted {
            return quantum_stop_reason(loop_state);
        }
    }
}

fn quantum_stop_reason(loop_state: &SearchLoopState) -> CombatSearchV2AdvanceStop {
    if loop_state.accepted_complete_candidate {
        CombatSearchV2AdvanceStop::CandidateSatisfied
    } else if loop_state.stats.deadline_hit {
        CombatSearchV2AdvanceStop::QuantumWallTime
    } else if loop_state.stats.node_budget_hit || loop_state.stats.action_prefix_budget_hit {
        CombatSearchV2AdvanceStop::QuantumNodeBudget
    } else if loop_state.frontier.is_empty() {
        CombatSearchV2AdvanceStop::FrontierExhausted
    } else {
        CombatSearchV2AdvanceStop::QuantumNodeBudget
    }
}

#[cfg(test)]
mod tests;
