use super::*;
use crate::generator::diagnostics::TurnOptionGeneratorTiming;

impl LocalTurnGraphWitnessSession {
    pub(super) fn evaluate_lookahead(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        deadline: Option<Instant>,
    ) -> bool {
        let Some(evaluator) = self.lookahead_evaluator.as_ref() else {
            self.nodes[node_id].lookahead_pending_lane = None;
            return true;
        };
        let Some(expected_lane) = self.nodes[node_id].lookahead_pending_lane else {
            return true;
        };
        let remaining_work = self.granted_generation_work.saturating_sub(
            self.used
                .generation_work
                .saturating_add(self.used.lookahead_work),
        );
        let max_work = self
            .config
            .lookahead_work_per_evaluation
            .max(1)
            .min(remaining_work);
        if max_work == 0 {
            return false;
        }
        let position = self.nodes[node_id].generator.root().position();
        let Some(evaluation) = evaluator.evaluate(position, max_work, deadline) else {
            return false;
        };
        if evaluation.guide.lane != expected_lane {
            self.nodes[node_id].lookahead_pending_lane = None;
            return true;
        }
        let backed_rank = evaluation.guide.rank.clone();
        if let Some(guide) = self.nodes[node_id]
            .guides
            .iter_mut()
            .find(|guide| guide.lane == expected_lane)
        {
            *guide = evaluation.guide;
        } else {
            self.nodes[node_id].guides.push(evaluation.guide);
        }
        update_max_rank(&mut self.nodes[node_id].backed_lookahead_rank, &backed_rank);
        for (parent_id, edge_index) in path {
            let edge = &mut self.nodes[*parent_id].children[*edge_index];
            update_max_rank(&mut edge.backed_lookahead_rank, &backed_rank);
        }
        self.nodes[node_id].lookahead_pending_lane = None;
        // The evaluated lane was deliberately absent while pending. Publish
        // only that newly grounded view: reinserting every guide here would
        // silently grant already-serviced one-shot views another expansion.
        self.shared_agenda
            .publish_guide_entry(node_id, &self.nodes[node_id], expected_lane);
        self.used.lookahead_evaluations = self.used.lookahead_evaluations.saturating_add(1);
        self.used.lookahead_work = self
            .used
            .lookahead_work
            .saturating_add(evaluation.work.max(1));
        self.used.boundary_lookahead_evaluations =
            self.used.boundary_lookahead_evaluations.saturating_add(1);
        self.used.boundary_lookahead_work = self
            .used
            .boundary_lookahead_work
            .saturating_add(evaluation.work.max(1));
        true
    }

    pub(super) fn widen(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        view: LocalServiceView,
        requested_work: usize,
        deadline: Option<Instant>,
        stepper: &dyn CombatStepper,
    ) -> bool {
        let remaining_work = self.granted_generation_work.saturating_sub(
            self.used
                .generation_work
                .saturating_add(self.used.lookahead_work),
        );
        let remaining_steps = self
            .granted_engine_steps
            .saturating_sub(self.used.engine_steps);
        let generator_work = self.nodes[node_id].generator.counters().generation_work;
        let requested_work = if node_id == 0 && generator_work == 0 {
            self.config.root_initial_expansion_work
        } else if generator_work == 0 {
            self.config.initial_expansion_work.max(requested_work)
        } else {
            requested_work
        };
        let work = requested_work.max(1).min(remaining_work);
        if work == 0 || remaining_steps == 0 {
            return false;
        }
        let remaining_lookahead_evaluations = self
            .config
            .lookahead_max_evaluations
            .saturating_sub(self.used.atomic_lookahead_evaluations);
        let remaining_lookahead_work = remaining_work.saturating_sub(work);

        let generation_started = Instant::now();
        let (
            before,
            after,
            before_lookahead_evaluations,
            after_lookahead_evaluations,
            before_lookahead_work,
            after_lookahead_work,
            before_diagnostics,
            after_diagnostics,
            before_timing,
            after_timing,
            options,
            new_gaps,
        ) = {
            let node = &mut self.nodes[node_id];
            node.generator.prefer_lane(match view {
                LocalServiceView::Anchor => TurnOptionGeneratorPreferredLane::Anchor,
                LocalServiceView::Guide(lane) => TurnOptionGeneratorPreferredLane::Guide(lane),
                LocalServiceView::LookaheadEvaluation => {
                    unreachable!("lookahead evaluation never widens a turn generator")
                }
            });
            let before = node.generator.counters();
            let before_lookahead_evaluations = node.generator.lookahead_evaluations();
            let before_lookahead_work = node.generator.lookahead_work();
            let before_diagnostics = node.generator.diagnostics();
            let before_timing = node.generator.timing();
            node.generator.advance_with_lookahead(
                stepper,
                CombatPlanningQuantum {
                    additional_generation_work: work,
                    additional_engine_steps: remaining_steps.min(work.saturating_mul(
                        self.config.generator.max_engine_steps_per_transition.max(1),
                    )),
                    deadline,
                },
                remaining_lookahead_evaluations,
                remaining_lookahead_work,
                self.config.lookahead_work_per_evaluation,
            );
            let after = node.generator.counters();
            let after_lookahead_evaluations = node.generator.lookahead_evaluations();
            let after_lookahead_work = node.generator.lookahead_work();
            for lane in node.generator.retained_guide_lanes() {
                let view = LocalServiceView::Guide(lane);
                if !node.generation_service_views.contains(&view) {
                    node.generation_service_views.push(view);
                }
            }
            let after_diagnostics = node.generator.diagnostics();
            let after_timing = node.generator.timing();
            let options = node.generator.take_completed_options();
            let gaps = node.generator.gaps()[node.synced_gaps..].to_vec();
            node.synced_gaps = node.generator.gaps().len();
            if node.generator.is_finished() {
                node.generator.retire_finished_search_storage();
            }
            (
                before,
                after,
                before_lookahead_evaluations,
                after_lookahead_evaluations,
                before_lookahead_work,
                after_lookahead_work,
                before_diagnostics,
                after_diagnostics,
                before_timing,
                after_timing,
                options,
                gaps,
            )
        };
        self.performance_timing.generation_elapsed_ns = self
            .performance_timing
            .generation_elapsed_ns
            .saturating_add(elapsed_nanos_u64(generation_started));

        let used_work = after.generation_work.saturating_sub(before.generation_work);
        let used_lookahead_evaluations =
            after_lookahead_evaluations.saturating_sub(before_lookahead_evaluations);
        let used_lookahead_work = after_lookahead_work.saturating_sub(before_lookahead_work);
        let used_steps = after.engine_steps.saturating_sub(before.engine_steps);
        if used_work == 0 && used_lookahead_work == 0 && used_steps == 0 {
            return false;
        }
        self.used.generation_work = self.used.generation_work.saturating_add(used_work);
        self.used.lookahead_evaluations = self
            .used
            .lookahead_evaluations
            .saturating_add(used_lookahead_evaluations);
        self.used.lookahead_work = self.used.lookahead_work.saturating_add(used_lookahead_work);
        self.used.atomic_lookahead_evaluations = self
            .used
            .atomic_lookahead_evaluations
            .saturating_add(used_lookahead_evaluations);
        self.used.atomic_lookahead_work = self
            .used
            .atomic_lookahead_work
            .saturating_add(used_lookahead_work);
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
        self.performance_timing
            .accumulate(generator_timing_delta(before_timing, after_timing));
        self.generation_gaps.extend(new_gaps);

        let admission_started = Instant::now();
        for option in options {
            let root_option_started = Instant::now();
            if node_id == 0 {
                self.record_root_option(&option);
            }
            self.nodes[node_id].generated_options =
                self.nodes[node_id].generated_options.saturating_add(1);
            self.used.completed_turn_options = self.used.completed_turn_options.saturating_add(1);
            self.performance_timing.admission_root_option_elapsed_ns = self
                .performance_timing
                .admission_root_option_elapsed_ns
                .saturating_add(elapsed_nanos_u64(root_option_started));
            match option.boundary() {
                CompleteTurnOptionBoundary::TerminalWin => {
                    self.used.terminal_win_options =
                        self.used.terminal_win_options.saturating_add(1);
                    let witness_filter_started = Instant::now();
                    let (mut actions, prefix_negative_log_policy) = self.path_actions(path);
                    actions.extend_from_slice(option.actions());
                    let negative_log_policy =
                        prefix_negative_log_policy + option.negative_log_policy();
                    let candidate_is_dominated = !terminal_candidate_could_improve_witness_frontier(
                        &self.original_root,
                        &self.witness_frontier,
                        option.exact_successor(),
                        &actions,
                        negative_log_policy,
                        self.config.max_potions_used,
                    );
                    self.performance_timing.admission_witness_filter_elapsed_ns = self
                        .performance_timing
                        .admission_witness_filter_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(witness_filter_started));
                    if candidate_is_dominated {
                        self.used.witness_replay_dominated_skips =
                            self.used.witness_replay_dominated_skips.saturating_add(1);
                        continue;
                    }
                    self.used.witness_replay_attempts =
                        self.used.witness_replay_attempts.saturating_add(1);
                    let witness_replay_started = Instant::now();
                    let admission = match replay_witness(
                        &self.original_root,
                        &actions,
                        negative_log_policy,
                        OracleCombatWitnessDiscoverySource::PlannerSearch,
                        stepper,
                    ) {
                        Ok(witness) => self.remember_witness(witness),
                        Err(error) => {
                            self.replay_failure = Some(error);
                            WitnessAdmission::default()
                        }
                    };
                    if admission.selected_changed {
                        self.used.witness_replay_improvements =
                            self.used.witness_replay_improvements.saturating_add(1);
                    }
                    if admission.frontier_changed {
                        self.used.witness_frontier_changes =
                            self.used.witness_frontier_changes.saturating_add(1);
                    }
                    self.performance_timing.admission_witness_replay_elapsed_ns = self
                        .performance_timing
                        .admission_witness_replay_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(witness_replay_started));
                    if self.witness_satisfies() {
                        self.performance_timing.admission_elapsed_ns = self
                            .performance_timing
                            .admission_elapsed_ns
                            .saturating_add(elapsed_nanos_u64(admission_started));
                        return true;
                    }
                }
                CompleteTurnOptionBoundary::TerminalLoss => {
                    self.used.terminal_losses = self.used.terminal_losses.saturating_add(1);
                }
                CompleteTurnOptionBoundary::Escape => {}
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    let _ = self.accept_successor(node_id, path, option);
                }
            }
        }
        let refresh_started = Instant::now();
        self.refresh_exhaustion(node_id);
        if self.nodes[node_id].generator.is_finished() || self.nodes[node_id].exhausted {
            self.shared_agenda.remove_guide_entries(
                node_id,
                &self.nodes[node_id],
                self.lookahead_lane,
            );
        }
        self.performance_timing.admission_refresh_elapsed_ns = self
            .performance_timing
            .admission_refresh_elapsed_ns
            .saturating_add(elapsed_nanos_u64(refresh_started));
        self.performance_timing.admission_elapsed_ns = self
            .performance_timing
            .admission_elapsed_ns
            .saturating_add(elapsed_nanos_u64(admission_started));
        true
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
}

fn generator_timing_delta(
    before: TurnOptionGeneratorTiming,
    after: TurnOptionGeneratorTiming,
) -> LocalTurnGraphPerformanceTiming {
    let TurnOptionGeneratorTiming {
        atomic_expand_elapsed_ns: before_atomic_expand_elapsed_ns,
        transition_simulation_elapsed_ns: before_transition_simulation_elapsed_ns,
        transition_identity_elapsed_ns: before_transition_identity_elapsed_ns,
        transition_key_build_elapsed_ns: before_transition_key_build_elapsed_ns,
        transition_key_index_elapsed_ns: before_transition_key_index_elapsed_ns,
        transition_admission_elapsed_ns: before_transition_admission_elapsed_ns,
        transition_trace_elapsed_ns: before_transition_trace_elapsed_ns,
        transition_seen_elapsed_ns: before_transition_seen_elapsed_ns,
        transition_publish_elapsed_ns: before_transition_publish_elapsed_ns,
        transition_publish_trace_node_elapsed_ns: before_transition_publish_trace_node_elapsed_ns,
        transition_publish_boundary_elapsed_ns: before_transition_publish_boundary_elapsed_ns,
        transition_publish_complete_elapsed_ns: before_transition_publish_complete_elapsed_ns,
        transition_publish_push_elapsed_ns: before_transition_publish_push_elapsed_ns,
        transition_publish_guide_elapsed_ns: before_transition_publish_guide_elapsed_ns,
        transition_publish_retain_elapsed_ns: before_transition_publish_retain_elapsed_ns,
        transition_publish_agenda_elapsed_ns: before_transition_publish_agenda_elapsed_ns,
    } = before;
    let TurnOptionGeneratorTiming {
        atomic_expand_elapsed_ns,
        transition_simulation_elapsed_ns,
        transition_identity_elapsed_ns,
        transition_key_build_elapsed_ns,
        transition_key_index_elapsed_ns,
        transition_admission_elapsed_ns,
        transition_trace_elapsed_ns,
        transition_seen_elapsed_ns,
        transition_publish_elapsed_ns,
        transition_publish_trace_node_elapsed_ns,
        transition_publish_boundary_elapsed_ns,
        transition_publish_complete_elapsed_ns,
        transition_publish_push_elapsed_ns,
        transition_publish_guide_elapsed_ns,
        transition_publish_retain_elapsed_ns,
        transition_publish_agenda_elapsed_ns,
    } = after;

    LocalTurnGraphPerformanceTiming {
        atomic_expand_elapsed_ns: atomic_expand_elapsed_ns
            .saturating_sub(before_atomic_expand_elapsed_ns),
        transition_simulation_elapsed_ns: transition_simulation_elapsed_ns
            .saturating_sub(before_transition_simulation_elapsed_ns),
        transition_identity_elapsed_ns: transition_identity_elapsed_ns
            .saturating_sub(before_transition_identity_elapsed_ns),
        transition_key_build_elapsed_ns: transition_key_build_elapsed_ns
            .saturating_sub(before_transition_key_build_elapsed_ns),
        transition_key_index_elapsed_ns: transition_key_index_elapsed_ns
            .saturating_sub(before_transition_key_index_elapsed_ns),
        transition_admission_elapsed_ns: transition_admission_elapsed_ns
            .saturating_sub(before_transition_admission_elapsed_ns),
        transition_trace_elapsed_ns: transition_trace_elapsed_ns
            .saturating_sub(before_transition_trace_elapsed_ns),
        transition_seen_elapsed_ns: transition_seen_elapsed_ns
            .saturating_sub(before_transition_seen_elapsed_ns),
        transition_publish_elapsed_ns: transition_publish_elapsed_ns
            .saturating_sub(before_transition_publish_elapsed_ns),
        transition_publish_trace_node_elapsed_ns: transition_publish_trace_node_elapsed_ns
            .saturating_sub(before_transition_publish_trace_node_elapsed_ns),
        transition_publish_boundary_elapsed_ns: transition_publish_boundary_elapsed_ns
            .saturating_sub(before_transition_publish_boundary_elapsed_ns),
        transition_publish_complete_elapsed_ns: transition_publish_complete_elapsed_ns
            .saturating_sub(before_transition_publish_complete_elapsed_ns),
        transition_publish_push_elapsed_ns: transition_publish_push_elapsed_ns
            .saturating_sub(before_transition_publish_push_elapsed_ns),
        transition_publish_guide_elapsed_ns: transition_publish_guide_elapsed_ns
            .saturating_sub(before_transition_publish_guide_elapsed_ns),
        transition_publish_retain_elapsed_ns: transition_publish_retain_elapsed_ns
            .saturating_sub(before_transition_publish_retain_elapsed_ns),
        transition_publish_agenda_elapsed_ns: transition_publish_agenda_elapsed_ns
            .saturating_sub(before_transition_publish_agenda_elapsed_ns),
        ..LocalTurnGraphPerformanceTiming::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generator_timing(value: u64) -> TurnOptionGeneratorTiming {
        TurnOptionGeneratorTiming {
            atomic_expand_elapsed_ns: value,
            transition_simulation_elapsed_ns: value,
            transition_identity_elapsed_ns: value,
            transition_key_build_elapsed_ns: value,
            transition_key_index_elapsed_ns: value,
            transition_admission_elapsed_ns: value,
            transition_trace_elapsed_ns: value,
            transition_seen_elapsed_ns: value,
            transition_publish_elapsed_ns: value,
            transition_publish_trace_node_elapsed_ns: value,
            transition_publish_boundary_elapsed_ns: value,
            transition_publish_complete_elapsed_ns: value,
            transition_publish_push_elapsed_ns: value,
            transition_publish_guide_elapsed_ns: value,
            transition_publish_retain_elapsed_ns: value,
            transition_publish_agenda_elapsed_ns: value,
        }
    }

    #[test]
    fn generator_timing_delta_maps_every_generator_field_only() {
        let delta = generator_timing_delta(generator_timing(5), generator_timing(8));
        assert_eq!(
            delta,
            LocalTurnGraphPerformanceTiming {
                atomic_expand_elapsed_ns: 3,
                transition_simulation_elapsed_ns: 3,
                transition_identity_elapsed_ns: 3,
                transition_key_build_elapsed_ns: 3,
                transition_key_index_elapsed_ns: 3,
                transition_admission_elapsed_ns: 3,
                transition_trace_elapsed_ns: 3,
                transition_seen_elapsed_ns: 3,
                transition_publish_elapsed_ns: 3,
                transition_publish_trace_node_elapsed_ns: 3,
                transition_publish_boundary_elapsed_ns: 3,
                transition_publish_complete_elapsed_ns: 3,
                transition_publish_push_elapsed_ns: 3,
                transition_publish_guide_elapsed_ns: 3,
                transition_publish_retain_elapsed_ns: 3,
                transition_publish_agenda_elapsed_ns: 3,
                ..LocalTurnGraphPerformanceTiming::default()
            }
        );
    }
}
