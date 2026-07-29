use super::*;

impl LocalTurnGraphWitnessSession {
    pub(super) fn accept_successor(
        &mut self,
        parent_id: usize,
        path: &[(usize, usize)],
        option: CompleteTurnOption,
    ) -> Option<usize> {
        let relative_turn_depth = self.nodes[parent_id].relative_turn_depth.saturating_add(1);
        if relative_turn_depth > self.config.max_turn_depth {
            self.used.depth_limited_successors =
                self.used.depth_limited_successors.saturating_add(1);
            return None;
        }

        let successor_identity_started = Instant::now();
        let (successor_identity, successor_position, option_actions, option_negative_log_policy) =
            option.into_successor_parts();
        let successor_path_negative_log_policy =
            self.nodes[parent_id].path_negative_log_policy + option_negative_log_policy;
        let successor_path_atomic_depth = self.nodes[parent_id]
            .path_atomic_depth
            .saturating_add(option_actions.len());
        let successor_exact_key = successor_identity.exact_key().cloned().unwrap_or_else(|| {
            Arc::new(combat_exact_state_key(
                &successor_position.engine,
                &successor_position.combat,
            ))
        });
        let successor_potion_expenditures = self.nodes[parent_id]
            .potion_expenditures
            .saturating_add(actions_potion_expenditures(&option_actions));
        self.performance_timing.successor_identity_elapsed_ns = self
            .performance_timing
            .successor_identity_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_identity_started));
        if self
            .config
            .max_potions_used
            .is_some_and(|limit| successor_potion_expenditures > limit)
        {
            return None;
        }
        let constrained_successor_key = ConstrainedExactStateKey::new(
            successor_exact_key,
            self.config.max_potions_used,
            successor_potion_expenditures,
        );
        let successor_lookup_started = Instant::now();
        let existing = self
            .nodes_by_exact_key
            .get(&constrained_successor_key)
            .copied();
        self.performance_timing.successor_lookup_elapsed_ns = self
            .performance_timing
            .successor_lookup_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_lookup_started));
        let successor = if let Some(existing) = existing {
            existing
        } else {
            let successor_node_build_started = Instant::now();
            let Ok(root) = CombatDecisionRoot::with_exact_state_identity(
                successor_position,
                successor_identity,
            ) else {
                self.performance_timing.successor_node_build_elapsed_ns = self
                    .performance_timing
                    .successor_node_build_elapsed_ns
                    .saturating_add(elapsed_nanos_u64(successor_node_build_started));
                return None;
            };
            let (guides, lookahead_pending_lane) = guides_with_pending_lookahead(
                self.policy.as_ref(),
                self.lookahead_evaluator.as_deref(),
                root.position(),
            );
            let backed_guides = guide_rank_map(&guides);
            let node_id = self.nodes.len();
            let generator = turn_generator_for_potion_budget(
                root,
                self.config.generator,
                self.policy.clone(),
                self.config.max_potions_used,
                successor_potion_expenditures,
            );
            let generation_service_views =
                generation_service_views_from_lanes(generator.retained_guide_lanes());
            self.nodes.push(GraphNode {
                generator,
                potion_expenditures: successor_potion_expenditures,
                diagnostic_parent: Some((parent_id, self.nodes[parent_id].children.len())),
                path_negative_log_policy: successor_path_negative_log_policy,
                path_atomic_depth: successor_path_atomic_depth,
                relative_turn_depth,
                visits: 0,
                generated_options: 0,
                children: Vec::new(),
                guides,
                generation_service_views,
                next_generation_service_view: 0,
                widen_anchor_visits: 0,
                widen_guide_visits: BTreeMap::new(),
                lookahead_pending_lane,
                backed_guides,
                backed_lookahead_rank: None,
                synced_gaps: 0,
                exhausted: false,
            });
            self.nodes_by_exact_key
                .insert(constrained_successor_key, node_id);
            self.used.exact_nodes = self.nodes.len();
            self.used.maximum_turn_depth = self.used.maximum_turn_depth.max(relative_turn_depth);
            self.performance_timing.successor_node_build_elapsed_ns = self
                .performance_timing
                .successor_node_build_elapsed_ns
                .saturating_add(elapsed_nanos_u64(successor_node_build_started));
            node_id
        };

        let successor_edge_started = Instant::now();
        let successor_backed_guides = self.nodes[successor].backed_guides.clone();
        let successor_backed_rank = self.nodes[successor].backed_lookahead_rank.clone();
        let existing_edge_index = self.nodes[parent_id]
            .children
            .iter()
            .position(|edge| edge.successor == successor);
        let edge_index = if let Some(edge_index) = existing_edge_index {
            self.used.duplicate_successor_edges =
                self.used.duplicate_successor_edges.saturating_add(1);
            let edge = &mut self.nodes[parent_id].children[edge_index];
            if option_negative_log_policy
                .total_cmp(&edge.negative_log_policy)
                .is_lt()
            {
                edge.actions = option_actions;
                edge.negative_log_policy = option_negative_log_policy;
            }
            edge_index
        } else {
            let plan_transition_annotation = self
                .collect_plan_transition_annotations
                .then(|| {
                    combat_plan_transition_annotation_v1(
                        self.nodes[parent_id].generator.root().position(),
                        self.nodes[successor].generator.root().position(),
                    )
                })
                .flatten();
            let parent = &mut self.nodes[parent_id];
            let edge_index = parent.children.len();
            parent.children.push(GraphEdge {
                successor,
                actions: option_actions,
                negative_log_policy: option_negative_log_policy,
                plan_transition_annotation: plan_transition_annotation.clone(),
                visits: 0,
                anchor_visits: 0,
                guide_visits: BTreeMap::new(),
                backed_guides: successor_backed_guides.clone(),
                backed_lookahead_rank: successor_backed_rank,
                backed_visits: 0,
            });
            parent.exhausted = false;
            self.used.exact_edges = self.used.exact_edges.saturating_add(1);
            if plan_transition_annotation.is_some() {
                self.used.annotated_exact_edges = self.used.annotated_exact_edges.saturating_add(1);
            }
            edge_index
        };
        let successor_path_improved = existing.is_some()
            && local_path_base(
                successor_path_atomic_depth,
                successor_path_negative_log_policy,
            )
            .total_cmp(&self.nodes[successor].path_cost())
            .is_lt();
        if successor_path_improved {
            self.shared_agenda.remove_guide_entries(
                successor,
                &self.nodes[successor],
                self.lookahead_lane,
            );
            let successor_node = &mut self.nodes[successor];
            successor_node.diagnostic_parent = Some((parent_id, edge_index));
            successor_node.path_negative_log_policy = successor_path_negative_log_policy;
            successor_node.path_atomic_depth = successor_path_atomic_depth;
            self.shared_agenda
                .publish_node(successor, &self.nodes[successor], self.lookahead_lane);
        }
        self.performance_timing.successor_edge_elapsed_ns = self
            .performance_timing
            .successor_edge_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_edge_started));
        let successor_backup_started = Instant::now();
        self.backup_guides_along_path(path, parent_id, edge_index, &successor_backed_guides);
        if existing.is_none() {
            self.shared_agenda
                .publish_node(successor, &self.nodes[successor], self.lookahead_lane);
        }
        self.performance_timing.successor_backup_elapsed_ns = self
            .performance_timing
            .successor_backup_elapsed_ns
            .saturating_add(elapsed_nanos_u64(successor_backup_started));
        Some(successor)
    }

    fn backup_guides_along_path(
        &mut self,
        path: &[(usize, usize)],
        parent_id: usize,
        edge_index: usize,
        guides: &GuideRankMap,
    ) {
        for (node_id, selected_edge) in path
            .iter()
            .copied()
            .chain(std::iter::once((parent_id, edge_index)))
        {
            for (lane, rank) in guides.iter() {
                update_max_guide(
                    &mut self.nodes[node_id].children[selected_edge].backed_guides,
                    *lane,
                    rank,
                );
                update_max_guide(&mut self.nodes[node_id].backed_guides, *lane, rank);
            }
        }
    }
}
