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
        let original_root = root.position().clone();
        let root_exact_key = root
            .exact_state_key()
            .expect("a newly constructed combat root retains its exact key")
            .clone();
        let root_guides = policy.state_guides(root.position());
        let root_backed_guides = guide_rank_map(&root_guides);
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
            widen_proposal_root_visits: 0,
            widen_proposal_continuation_visits: 0,
            widen_guide_visits: BTreeMap::new(),
            boundary_anchor_services: 0,
            boundary_proposal_root_services: 0,
            boundary_proposal_continuation_services: 0,
            boundary_guide_services: 0,
            generation_anchor_services: 0,
            generation_guide_services: 0,
            backed_guides: root_backed_guides,
            synced_gaps: 0,
            exhausted: false,
        };
        let mut shared_agenda = SharedBoundaryAgenda::new(config.guide_service_bias);
        shared_agenda.publish_node(0, &root_node);
        let root_proposal_enqueued =
            plan_prefix_root_eligible(root_node.generator.root().position())
                && shared_agenda.publish_proposal_root(0, &root_node);
        Self {
            original_root,
            config,
            policy,
            collect_plan_transition_annotations: false,
            shared_agenda,
            nodes: vec![root_node],
            nodes_by_exact_key,
            used: LocalTurnGraphWitnessCounters {
                exact_nodes: 1,
                plan_prefix_root_enqueues: usize::from(root_proposal_enqueued),
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
        self.witness_frontier
            .iter()
            .any(|witness| self.witness_satisfies_config(witness))
    }

    fn witness_satisfies_config(&self, witness: &OracleCombatWitness) -> bool {
        if !witness_within_potion_contract(
            &self.original_root,
            witness,
            self.config.max_potions_used,
            self.config.generator.allowed_potion_slots,
        ) {
            return false;
        }
        if self.config.require_no_unrecovered_stolen_gold
            && sts_core::ai::combat_persistent_outcome_v1::unrecovered_stolen_gold(
                &witness.final_position.combat,
            ) > 0
        {
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
