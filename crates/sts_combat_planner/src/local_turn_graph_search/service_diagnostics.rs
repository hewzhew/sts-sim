use super::*;

impl LocalTurnGraphWitnessSession {
    pub fn depth_service_snapshot(&self) -> Vec<LocalTurnGraphDepthServiceSnapshot> {
        let mut by_depth = BTreeMap::<usize, LocalTurnGraphDepthServiceSnapshot>::new();
        for node in &self.nodes {
            let counters = node.generator.counters();
            let depth = node.relative_turn_depth;
            let snapshot =
                by_depth
                    .entry(depth)
                    .or_insert_with(|| LocalTurnGraphDepthServiceSnapshot {
                        relative_turn_depth: depth,
                        ..LocalTurnGraphDepthServiceSnapshot::default()
                    });
            snapshot.exact_states = snapshot.exact_states.saturating_add(1);
            snapshot.serviced_states = snapshot
                .serviced_states
                .saturating_add(usize::from(counters.generation_work > 0));
            snapshot.generation_work = snapshot
                .generation_work
                .saturating_add(counters.generation_work);
            snapshot.generated_options = snapshot
                .generated_options
                .saturating_add(node.generated_options);
            snapshot.exact_children = snapshot.exact_children.saturating_add(node.children.len());
            snapshot.retained_generator_work_items = snapshot
                .retained_generator_work_items
                .saturating_add(node.generator.retained_work_items());
            snapshot.exhausted_states = snapshot
                .exhausted_states
                .saturating_add(usize::from(node.exhausted));
        }
        by_depth.into_values().collect()
    }

    pub fn serviced_state_samples(
        &self,
        max_states_per_depth: usize,
    ) -> Vec<LocalTurnGraphServicedStateSnapshot> {
        if max_states_per_depth == 0 {
            return Vec::new();
        }
        let mut by_depth = BTreeMap::<usize, Vec<usize>>::new();
        for (node_id, node) in self.nodes.iter().enumerate() {
            if node.generator.counters().generation_work > 0 {
                by_depth
                    .entry(node.relative_turn_depth)
                    .or_default()
                    .push(node_id);
            }
        }
        let mut samples = Vec::new();
        for node_ids in by_depth.values_mut() {
            node_ids.sort_by(|left, right| {
                let left_node = &self.nodes[*left];
                let right_node = &self.nodes[*right];
                right_node
                    .generator
                    .counters()
                    .generation_work
                    .cmp(&left_node.generator.counters().generation_work)
                    .then_with(|| {
                        right_node
                            .generated_options
                            .cmp(&left_node.generated_options)
                    })
                    .then_with(|| left.cmp(right))
            });
            for node_id in node_ids.iter().copied().take(max_states_per_depth) {
                samples.push(self.state_service_snapshot(node_id));
            }
        }
        samples
    }

    pub fn state_service_index(&self) -> Vec<LocalTurnGraphServicedStateSnapshot> {
        (0..self.nodes.len())
            .map(|node_id| self.state_service_snapshot(node_id))
            .collect()
    }

    fn state_service_snapshot(&self, node_id: usize) -> LocalTurnGraphServicedStateSnapshot {
        let node = &self.nodes[node_id];
        let position = node.generator.root().position();
        let combat = &position.combat;
        let counters = node.generator.counters();
        let state = local_deep_state_snapshot(node, self.diagnostic_actions_to_node(node_id).len());
        let persistent =
            sts_core::ai::combat_persistent_outcome_v1::CombatPersistentOutcomeV1::from_combat(
                combat,
            );
        let anchor_position = self.shared_agenda.anchor_position(node_id, &self.nodes);
        let proposal_root_position = self
            .shared_agenda
            .proposal_root_position(node_id, &self.nodes);
        let proposal_continuation_position = self
            .shared_agenda
            .proposal_continuation_position(node_id, &self.nodes);
        LocalTurnGraphServicedStateSnapshot {
            exact_state_hash: exact_hash(position),
            relative_turn_depth: node.relative_turn_depth,
            player_turn: state.player_turn,
            player_hp: state.player_hp,
            alive_enemy_count: state.alive_enemy_count,
            enemy_total_hp: state.enemy_total_hp,
            recoverable_stolen_gold:
                sts_core::ai::combat_persistent_outcome_v1::recoverable_stolen_gold(combat),
            unrecovered_stolen_gold: persistent.unrecovered_stolen_gold,
            generation_work: counters.generation_work,
            generated_options: node.generated_options,
            exact_children: node.children.len(),
            retained_generator_work_items: node.generator.retained_work_items(),
            path_action_count: state.path_atomic_depth,
            anchor_ordinal_rank: anchor_position.agenda.ordinal_rank,
            anchor_candidate_count: anchor_position.agenda.candidate_count,
            proposal_root_ordinal_rank: proposal_root_position.ordinal_rank,
            proposal_root_candidate_count: proposal_root_position.candidate_count,
            proposal_root_services: node.widen_proposal_root_visits,
            proposal_continuation_ordinal_rank: proposal_continuation_position.ordinal_rank,
            proposal_continuation_candidate_count: proposal_continuation_position.candidate_count,
            proposal_continuation_services: node.widen_proposal_continuation_visits,
        }
    }
}
