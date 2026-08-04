use super::*;

impl LocalTurnGraphWitnessSession {
    pub fn set_satisfaction(&mut self, satisfaction: OracleCombatWitnessSatisfaction) {
        self.config.satisfaction = satisfaction;
    }

    /// Enables read-only plan facts on subsequently materialized exact edges.
    ///
    /// Enabling after graph construction would leave a mixture of annotated
    /// and unannotated edges, so the session rejects that ambiguous state.
    pub fn enable_plan_transition_annotations(
        &mut self,
    ) -> Result<(), LocalTurnGraphPlanAnnotationEnableError> {
        if self.used.exact_edges > 0 {
            return Err(LocalTurnGraphPlanAnnotationEnableError::EdgesAlreadyMaterialized);
        }
        self.collect_plan_transition_annotations = true;
        Ok(())
    }

    pub fn with_policy(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, None)
    }

    pub fn with_policy_and_lookahead(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
        lookahead_evaluator: SharedCombatLookaheadEvaluator,
    ) -> Self {
        Self::with_optional_lookahead(root, config, policy, Some(lookahead_evaluator))
    }

    fn with_optional_lookahead(
        root: CombatDecisionRoot,
        config: LocalTurnGraphWitnessConfig,
        policy: SharedCombatActionPolicy,
        lookahead_evaluator: Option<SharedCombatLookaheadEvaluator>,
    ) -> Self {
        let original_root = root.position().clone();
        let root_exact_key = root
            .exact_state_key()
            .expect("a newly constructed combat root retains its exact key")
            .clone();
        let (root_guides, root_lookahead_pending_lane) = guides_with_pending_lookahead(
            policy.as_ref(),
            lookahead_evaluator.as_deref(),
            root.position(),
        );
        let root_backed_guides = guide_rank_map(&root_guides);
        // Expensive lookahead evaluates exact player-turn boundaries. Atomic
        // partial states remain the generator's private proposal mechanism;
        // evaluating them here would reintroduce an independent inner search.
        let generator = turn_generator_for_potion_budget(
            root.clone(),
            config.generator,
            policy.clone(),
            config.max_potions_used,
            0,
        );
        let root_generation_service_views =
            generation_service_views_from_lanes(generator.retained_guide_lanes());
        let mut nodes_by_exact_key = HashMap::with_hasher(FxBuildHasher);
        nodes_by_exact_key.insert(
            ConstrainedExactStateKey::new(root_exact_key, config.max_potions_used, 0),
            0,
        );
        let root_node = GraphNode {
            generator,
            potion_expenditures: 0,
            diagnostic_parent: None,
            path_negative_log_policy: 0.0,
            path_atomic_depth: 0,
            relative_turn_depth: 0,
            visits: 0,
            first_service_selection: None,
            first_guide_service_selection: None,
            generated_options: 0,
            children: Vec::new(),
            guides: root_guides,
            generation_service_views: root_generation_service_views,
            next_generation_service_view: 0,
            widen_anchor_visits: 0,
            widen_guide_visits: BTreeMap::new(),
            lookahead_pending_lane: root_lookahead_pending_lane,
            backed_guides: root_backed_guides,
            backed_lookahead_rank: None,
            synced_gaps: 0,
            exhausted: false,
        };
        let mut shared_agenda =
            SharedBoundaryAgenda::new(lookahead_evaluator.is_some(), config.guide_service_bias);
        shared_agenda.publish_node(0, &root_node, root_lookahead_pending_lane);
        Self {
            original_root,
            config,
            policy,
            lookahead_evaluator,
            collect_plan_transition_annotations: false,
            lookahead_lane: root_lookahead_pending_lane,
            shared_agenda,
            nodes: vec![root_node],
            nodes_by_exact_key,
            used: LocalTurnGraphWitnessCounters {
                exact_nodes: 1,
                ..LocalTurnGraphWitnessCounters::default()
            },
            performance_timing: LocalTurnGraphPerformanceTiming::default(),
            granted_selections: 0,
            granted_generation_work: 0,
            granted_engine_steps: 0,
            generation_gaps: Vec::new(),
            root_action_families: Vec::new(),
            witness: None,
            witness_frontier: Vec::new(),
            replay_failure: None,
        }
    }

    pub fn witness(&self) -> Option<&OracleCombatWitness> {
        self.witness.as_ref()
    }

    pub fn witness_frontier(&self) -> &[OracleCombatWitness] {
        &self.witness_frontier
    }

    /// Offers one complete tactical line as an untrusted candidate.
    ///
    /// Policy code may discover a useful line cheaply, but it owns neither
    /// legality nor terminal truth. This session replays every action and
    /// expected exact successor from its unchanged root before installing a
    /// witness. Independent local-graph search remains available to improve
    /// or replace the candidate.
    pub fn offer_witness_proposal(
        &mut self,
        proposal: CombatPolicyWitnessProposal,
        stepper: &dyn CombatStepper,
    ) -> Result<bool, OracleCombatWitnessReplayError> {
        self.used.policy_witness_proposals = self.used.policy_witness_proposals.saturating_add(1);
        let witness = replay_witness(
            &self.original_root,
            &proposal.actions,
            proposal.actions.len() as f64,
            OracleCombatWitnessDiscoverySource::PolicyProposal,
            stepper,
        )?;
        self.used.policy_witness_replay_engine_steps = self
            .used
            .policy_witness_replay_engine_steps
            .saturating_add(witness.replay_engine_steps);
        self.used.engine_steps = self
            .used
            .engine_steps
            .saturating_add(witness.replay_engine_steps);
        Ok(self.remember_witness(witness).selected_changed)
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
        self.remember_witness(witness);
        Ok(())
    }

    pub(super) fn witness_satisfies(&self) -> bool {
        let Some(witness) = self.witness.as_ref() else {
            return false;
        };
        if !witness_within_potion_contract(
            &self.original_root,
            witness,
            self.config.max_potions_used,
            self.config.generator.allowed_potion_slots,
        ) {
            return false;
        }
        match self.config.satisfaction {
            OracleCombatWitnessSatisfaction::FirstWitness => true,
            OracleCombatWitnessSatisfaction::HpLossAtMost(limit) => {
                let initial_hp = self.original_root.combat.entities.player.current_hp;
                let final_hp = witness.final_position.combat.entities.player.current_hp;
                initial_hp.saturating_sub(final_hp).max(0) as u32 <= limit
            }
            OracleCombatWitnessSatisfaction::FinalHpAtLeast(minimum) => {
                witness.final_position.combat.entities.player.current_hp >= minimum
            }
            OracleCombatWitnessSatisfaction::BudgetOrExhaustion => false,
        }
    }

    pub(super) fn remember_witness(&mut self, witness: OracleCombatWitness) -> WitnessAdmission {
        if !witness_within_potion_contract(
            &self.original_root,
            &witness,
            self.config.max_potions_used,
            self.config.generator.allowed_potion_slots,
        ) {
            return WitnessAdmission::default();
        }
        let frontier_changed = remember_nondominated_witness(
            &self.original_root,
            &mut self.witness_frontier,
            &witness,
        );
        let selected = self.witness_frontier.iter().reduce(|best, candidate| {
            if witness_better_with_potion_budget(
                &self.original_root,
                candidate,
                best,
                self.config.max_potions_used,
            ) {
                candidate
            } else {
                best
            }
        });
        let selected_changed = match (self.witness.as_ref(), selected) {
            (None, Some(_)) => true,
            (Some(current), Some(next)) => current.actions != next.actions,
            _ => false,
        };
        self.witness = selected.cloned();
        WitnessAdmission {
            frontier_changed,
            selected_changed,
        }
    }
}
