use super::*;

impl LocalTurnGraphWitnessSession {
    /// Materializes one bounded, exact policy mainline as ordinary graph
    /// edges.
    ///
    /// At each stable action surface the existing action policy still selects
    /// the greedy action. An encounter plan may reject a preview only when
    /// exact before/after projections prove that it prematurely spends a
    /// reserved resource. The next policy-ranked compatible action is tried;
    /// EndTurn therefore wins naturally only when no better compatible play
    /// remains. Every rejected action remains available to normal search.
    ///
    /// This is intentionally a single line, not another scheduler, beam, or
    /// pruning rule. If the line loses or encounters an unsupported
    /// structured choice, already materialized turn boundaries remain useful
    /// and ordinary search continues unchanged.
    pub fn offer_plan_compatible_policy_line(
        &mut self,
        max_turns: usize,
        max_actions: usize,
        stepper: &dyn CombatStepper,
    ) -> Result<LocalTurnGraphPolicyLineReport, String> {
        self.offer_plan_compatible_policy_line_with_suffix_probes(
            max_turns,
            max_actions,
            0,
            stepper,
        )
    }

    /// Materializes the same exact policy line and, immediately before that
    /// line would cross a typed combat-plan milestone, gives the current
    /// exact state one bounded deterministic suffix search.
    ///
    /// This is a hierarchical laboratory control, not a second global
    /// scheduler. The policy line cheaply carries the combat through states
    /// where its plan stage is unchanged; exact branching is paid only at a
    /// semantic handoff. A suffix can become authoritative only after its
    /// actions are joined to the exact prefix and replayed from the unchanged
    /// combat root.
    pub fn offer_plan_compatible_policy_line_with_suffix_probes(
        &mut self,
        max_turns: usize,
        max_actions: usize,
        suffix_generation_work: usize,
        stepper: &dyn CombatStepper,
    ) -> Result<LocalTurnGraphPolicyLineReport, String> {
        let mut report = LocalTurnGraphPolicyLineReport::default();
        if max_turns == 0
            || max_actions == 0
            || combat_plan_projection_v1(&self.original_root).is_none()
        {
            return Ok(report);
        }

        let mut node_id = 0usize;
        let mut path = Vec::<(usize, usize)>::new();
        let mut total_actions = 0usize;

        'turns: for _ in 0..max_turns {
            if total_actions >= max_actions {
                break;
            }
            let segment_root = self.nodes[node_id].generator.root().position().clone();
            let segment_identity_started = Instant::now();
            let segment_root_hash = exact_hash(&segment_root);
            report.action_identity_elapsed_ns = report
                .action_identity_elapsed_ns
                .saturating_add(elapsed_nanos_u64(segment_identity_started));
            let root_turn = segment_root.combat.turn.turn_count;
            let mut position = segment_root.clone();
            let mut actions = Vec::<TurnOptionAction>::new();
            let mut negative_log_policy = 0.0f64;

            while total_actions < max_actions {
                if stepper.terminal(&position) != CombatTerminal::Unresolved {
                    break;
                }
                let legal_surface_started = Instant::now();
                let surface = stepper.legal_action_surface(&position);
                report.legal_surface_elapsed_ns = report
                    .legal_surface_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(legal_surface_started));
                let policy_ranking_started = Instant::now();
                let choices = surface
                    .atomic_actions
                    .iter()
                    .map(CombatPolicyChoice::Atomic)
                    .chain(
                        surface
                            .selection_families
                            .iter()
                            .map(CombatPolicyChoice::StructuredSelection),
                    )
                    .collect::<Vec<_>>();
                if choices.is_empty() {
                    break;
                }
                let weights = self.policy.weights(&position, &choices);
                let weights = (weights.len() == choices.len())
                    .then_some(weights)
                    .unwrap_or_else(|| vec![1.0; choices.len()]);
                let probabilities = normalized_probabilities(
                    weights,
                    self.config.generator.uniform_exploration_ppm,
                );
                let mut ranked_indices = (0..choices.len()).collect::<Vec<_>>();
                ranked_indices.sort_by(|left, right| {
                    probabilities[*right]
                        .total_cmp(&probabilities[*left])
                        .then_with(|| left.cmp(right))
                });
                let mut selected = None;
                let mut first_neutral = None;
                let seek_timed_preference = combat_plan_has_timed_action_preference_v1(&position);
                report.policy_ranking_elapsed_ns = report
                    .policy_ranking_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(policy_ranking_started));
                let mut blocked_by_structured_family = false;
                let transition_preview_started = Instant::now();
                for candidate_index in ranked_indices {
                    if candidate_index >= surface.atomic_actions.len() {
                        let family = &surface.selection_families
                            [candidate_index - surface.atomic_actions.len()];
                        // A mandatory singleton choice is still a small exact
                        // action surface. Reuse the policy's member ordering
                        // and materialize only its principal member. Variable
                        // subsets remain lazy generator work: eagerly choosing
                        // one here would hide the very combinatorial boundary
                        // this proposer is meant to avoid.
                        if family.declared_min != 1 || family.effective_max != 1 {
                            blocked_by_structured_family = true;
                            break;
                        }
                        let Ok(mut cursor) = SelectionTransactionCursor::new(family) else {
                            blocked_by_structured_family = true;
                            break;
                        };
                        let members =
                            std::iter::from_fn(|| cursor.next_input()).collect::<Vec<_>>();
                        if members.is_empty() {
                            blocked_by_structured_family = true;
                            break;
                        }
                        let member_weights = self
                            .policy
                            .structured_selection_member_weights(&position, family, &members);
                        let member_weights = (member_weights.len() == members.len())
                            .then_some(member_weights)
                            .unwrap_or_else(|| vec![1.0; members.len()]);
                        let member_probabilities = normalized_probabilities(
                            member_weights,
                            self.config.generator.uniform_exploration_ppm,
                        );
                        let mut ranked_member_indices = (0..members.len()).collect::<Vec<_>>();
                        ranked_member_indices.sort_by(|left, right| {
                            member_probabilities[*right]
                                .total_cmp(&member_probabilities[*left])
                                .then_with(|| left.cmp(right))
                        });
                        let member_index = ranked_member_indices
                            .iter()
                            .copied()
                            .find(|member_index| {
                                let timing = combat_plan_selection_member_timing_v1(
                                    &position,
                                    family,
                                    &members[*member_index],
                                );
                                if matches!(timing, CombatPlanActionTimingV1::Defer(_)) {
                                    report.rejected_preview_transitions =
                                        report.rejected_preview_transitions.saturating_add(1);
                                    report.deferred_actions =
                                        report.deferred_actions.saturating_add(1);
                                    false
                                } else {
                                    true
                                }
                            })
                            // A mandatory choice remains executable even if
                            // every member consumes a reserved plan asset.
                            .unwrap_or(ranked_member_indices[0]);
                        let input = members[member_index].clone();
                        let step = stepper.apply_to_stable(
                            &position,
                            input.clone(),
                            CombatStepLimits {
                                max_engine_steps: self
                                    .config
                                    .generator
                                    .max_engine_steps_per_transition
                                    .max(1),
                                deadline: None,
                            },
                        );
                        report.engine_steps = report.engine_steps.saturating_add(step.engine_steps);
                        self.used.applied_action_transitions =
                            self.used.applied_action_transitions.saturating_add(1);
                        self.used.engine_steps =
                            self.used.engine_steps.saturating_add(step.engine_steps);
                        if step.truncated || step.timed_out {
                            blocked_by_structured_family = true;
                            break;
                        }
                        let probability =
                            probabilities[candidate_index] * member_probabilities[member_index];
                        match combat_plan_action_timing_v1(&position, &step.position) {
                            CombatPlanActionTimingV1::PreferNow => {
                                selected = Some((input, probability, step));
                                break;
                            }
                            CombatPlanActionTimingV1::Neutral if seek_timed_preference => {
                                first_neutral.get_or_insert((input, probability, step));
                            }
                            CombatPlanActionTimingV1::Neutral => {
                                selected = Some((input, probability, step));
                                break;
                            }
                            CombatPlanActionTimingV1::Defer(_) => {
                                report.rejected_preview_transitions =
                                    report.rejected_preview_transitions.saturating_add(1);
                                report.deferred_actions = report.deferred_actions.saturating_add(1);
                            }
                        }
                        continue;
                    }
                    let input = surface.atomic_actions[candidate_index].clone();
                    let step = stepper.apply_to_stable(
                        &position,
                        input.clone(),
                        CombatStepLimits {
                            max_engine_steps: self
                                .config
                                .generator
                                .max_engine_steps_per_transition
                                .max(1),
                            deadline: None,
                        },
                    );
                    report.engine_steps = report.engine_steps.saturating_add(step.engine_steps);
                    self.used.applied_action_transitions =
                        self.used.applied_action_transitions.saturating_add(1);
                    self.used.engine_steps =
                        self.used.engine_steps.saturating_add(step.engine_steps);
                    if step.truncated || step.timed_out {
                        break;
                    }
                    match combat_plan_action_timing_v1(&position, &step.position) {
                        CombatPlanActionTimingV1::PreferNow => {
                            selected = Some((input, probabilities[candidate_index], step));
                            break;
                        }
                        CombatPlanActionTimingV1::Neutral if seek_timed_preference => {
                            first_neutral.get_or_insert((
                                input,
                                probabilities[candidate_index],
                                step,
                            ));
                        }
                        CombatPlanActionTimingV1::Neutral => {
                            selected = Some((input, probabilities[candidate_index], step));
                            break;
                        }
                        CombatPlanActionTimingV1::Defer(_) => {
                            report.rejected_preview_transitions =
                                report.rejected_preview_transitions.saturating_add(1);
                            report.deferred_actions = report.deferred_actions.saturating_add(1);
                        }
                    }
                }
                report.transition_preview_elapsed_ns = report
                    .transition_preview_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(transition_preview_started));
                if selected.is_none() && !blocked_by_structured_family {
                    selected = first_neutral;
                }
                let Some((selected_input, selected_probability, selected_step)) = selected else {
                    break;
                };
                report.chosen_action_transitions =
                    report.chosen_action_transitions.saturating_add(1);
                report.proposed_actions.push(selected_input.clone());

                negative_log_policy -= selected_probability.max(f64::MIN_POSITIVE).ln();
                let action_identity_started = Instant::now();
                actions.push(TurnOptionAction {
                    input: selected_input,
                    expected_successor_hash: exact_hash(&selected_step.position).into(),
                    engine_steps: selected_step.engine_steps,
                });
                report.action_identity_elapsed_ns = report
                    .action_identity_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(action_identity_started));
                total_actions = total_actions.saturating_add(1);
                position = selected_step.position;

                if stepper.terminal(&position) != CombatTerminal::Unresolved
                    || (matches!(
                        position.engine,
                        sts_core::state::core::EngineState::CombatPlayerTurn
                    ) && position.combat.turn.turn_count > root_turn)
                {
                    break;
                }
            }

            if actions.is_empty() {
                break;
            }
            let boundary = match stepper.terminal(&position) {
                CombatTerminal::Win => CompleteTurnOptionBoundary::TerminalWin,
                CombatTerminal::Loss => CompleteTurnOptionBoundary::TerminalLoss,
                CombatTerminal::Unresolved if position.combat.runtime.combat_smoked => {
                    CompleteTurnOptionBoundary::Escape
                }
                CombatTerminal::Unresolved
                    if matches!(
                        position.engine,
                        sts_core::state::core::EngineState::CombatPlayerTurn
                    ) && position.combat.turn.turn_count > root_turn =>
                {
                    CompleteTurnOptionBoundary::NextPlayerTurn
                }
                CombatTerminal::Unresolved => break,
            };
            let option = CompleteTurnOption::new(
                segment_root_hash,
                actions,
                boundary,
                position,
                negative_log_policy,
            );
            if node_id == 0 {
                self.record_root_option(&option);
            }
            self.nodes[node_id].generated_options =
                self.nodes[node_id].generated_options.saturating_add(1);
            self.used.completed_turn_options = self.used.completed_turn_options.saturating_add(1);
            report.proposed_turns = report.proposed_turns.saturating_add(1);

            match boundary {
                CompleteTurnOptionBoundary::TerminalWin => {
                    let (mut all_actions, prefix_negative_log_policy) = self.path_actions(&path);
                    all_actions.extend_from_slice(option.actions());
                    let witness = replay_witness(
                        &self.original_root,
                        &all_actions,
                        prefix_negative_log_policy + option.negative_log_policy(),
                        OracleCombatWitnessDiscoverySource::PlannerSearch,
                        stepper,
                    )
                    .map_err(|error| format!("policy-line terminal replay failed: {error:?}"))?;
                    self.remember_witness(witness);
                    report.reached_terminal_win = true;
                    break;
                }
                CompleteTurnOptionBoundary::TerminalLoss => {
                    self.used.terminal_losses = self.used.terminal_losses.saturating_add(1);
                    break;
                }
                CompleteTurnOptionBoundary::Escape => break,
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    let plan_annotation_started = Instant::now();
                    let crosses_plan_milestone = combat_plan_transition_annotation_v1(
                        &segment_root,
                        option.exact_successor(),
                    )
                    .is_some_and(|annotation| !annotation.completed_milestones().is_empty());
                    report.plan_annotation_elapsed_ns = report
                        .plan_annotation_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(plan_annotation_started));
                    if suffix_generation_work > 0
                        && crosses_plan_milestone
                        && self.offer_exact_suffix_probe(
                            node_id,
                            &path,
                            suffix_generation_work,
                            stepper,
                            &mut report,
                        )?
                    {
                        break 'turns;
                    }
                    let successor_admission_started = Instant::now();
                    let Some(successor_id) = self.accept_successor(node_id, &path, option) else {
                        return Err("policy-line successor was not admitted".to_owned());
                    };
                    let Some(edge_index) = self.nodes[node_id]
                        .children
                        .iter()
                        .position(|edge| edge.successor == successor_id)
                    else {
                        return Err("policy-line successor edge was not admitted".to_owned());
                    };
                    path.push((node_id, edge_index));
                    node_id = successor_id;
                    report.successor_admission_elapsed_ns = report
                        .successor_admission_elapsed_ns
                        .saturating_add(elapsed_nanos_u64(successor_admission_started));
                }
            }
        }

        Ok(report)
    }

    fn offer_exact_suffix_probe(
        &mut self,
        node_id: usize,
        path: &[(usize, usize)],
        suffix_generation_work: usize,
        stepper: &dyn CombatStepper,
        report: &mut LocalTurnGraphPolicyLineReport,
    ) -> Result<bool, String> {
        let suffix_setup_started = Instant::now();
        let root = CombatDecisionRoot::new(self.nodes[node_id].generator.root().position().clone())
            .map_err(|error| format!("suffix probe root is not a decision boundary: {error:?}"))?;
        let mut suffix = LocalTurnGraphWitnessSession::with_policy(
            root,
            LocalTurnGraphWitnessConfig {
                satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
                ..self.config
            },
            self.policy.clone(),
        );
        report.suffix_probe_setup_elapsed_ns = report
            .suffix_probe_setup_elapsed_ns
            .saturating_add(elapsed_nanos_u64(suffix_setup_started));
        let suffix_advance_started = Instant::now();
        let suffix_report = suffix.advance(
            LocalTurnGraphWitnessQuantum {
                additional_selections: suffix_generation_work.max(1),
                additional_generation_work: suffix_generation_work,
                additional_engine_steps: suffix_generation_work
                    .saturating_mul(self.config.generator.max_engine_steps_per_transition.max(1)),
                deadline: None,
            },
            stepper,
        );
        report.suffix_probe_advance_elapsed_ns = report
            .suffix_probe_advance_elapsed_ns
            .saturating_add(elapsed_nanos_u64(suffix_advance_started));
        report.suffix_probe_attempts = report.suffix_probe_attempts.saturating_add(1);
        report.suffix_probe_generation_work = report
            .suffix_probe_generation_work
            .saturating_add(suffix_report.counters.generation_work);
        report.suffix_probe_engine_steps = report
            .suffix_probe_engine_steps
            .saturating_add(suffix_report.counters.engine_steps);
        report.suffix_probe_completed_turn_options = report
            .suffix_probe_completed_turn_options
            .saturating_add(suffix_report.counters.completed_turn_options);
        report.suffix_probe_applied_action_transitions = report
            .suffix_probe_applied_action_transitions
            .saturating_add(suffix_report.counters.applied_action_transitions);
        report.suffix_probe_unique_successor_states = report
            .suffix_probe_unique_successor_states
            .saturating_add(suffix_report.counters.unique_successor_states);
        report.suffix_probe_exact_nodes = report
            .suffix_probe_exact_nodes
            .saturating_add(suffix_report.counters.exact_nodes);
        report.suffix_probe_exact_edges = report
            .suffix_probe_exact_edges
            .saturating_add(suffix_report.counters.exact_edges);
        report
            .suffix_probe_performance_timing
            .accumulate(suffix_report.performance_timing);
        let witness_found = suffix_report.witness.is_some();
        let final_hp = suffix_report
            .witness
            .as_ref()
            .map(|witness| witness.final_position.combat.entities.player.current_hp);
        report
            .suffix_probe_details
            .push(LocalTurnGraphSuffixProbeAttempt {
                exact_state_hash: exact_hash(self.nodes[node_id].generator.root().position()),
                player_turn: self.nodes[node_id]
                    .generator
                    .root()
                    .position()
                    .combat
                    .turn
                    .turn_count,
                plan_projection: combat_plan_projection_v1(
                    self.nodes[node_id].generator.root().position(),
                ),
                generation_work: suffix_report.counters.generation_work,
                engine_steps: suffix_report.counters.engine_steps,
                witness_found,
                final_hp,
            });

        let Some(suffix_witness) = suffix_report.witness else {
            return Ok(false);
        };
        let suffix_replay_started = Instant::now();
        let (mut actions, _) = self.path_actions(path);
        actions.extend(suffix_witness.actions);
        let final_hp_hint = suffix_witness
            .final_position
            .combat
            .entities
            .player
            .current_hp;
        let accepted = self
            .offer_witness_proposal(
                CombatPolicyWitnessProposal {
                    actions,
                    final_hp_hint,
                },
                stepper,
            )
            .map_err(|error| format!("combined suffix witness replay failed: {error:?}"))?;
        report.suffix_probe_replay_elapsed_ns = report
            .suffix_probe_replay_elapsed_ns
            .saturating_add(elapsed_nanos_u64(suffix_replay_started));
        report.suffix_probe_witness_found = accepted || self.witness.is_some();
        report.reached_terminal_win = report.suffix_probe_witness_found;
        Ok(report.suffix_probe_witness_found)
    }
}
