use super::*;

pub(super) fn diagnosis_tags(
    proof_status: SearchProofStatus,
    stats: &CombatSearchV2Stats,
    branching: &CombatSearchV2DiagnosticsBranching,
    expansion: &CombatSearchV2DiagnosticsExpansion,
    target_fanout: &CombatSearchV2DiagnosticsTargetFanout,
    equivalence: &CombatSearchV2DiagnosticsEquivalence,
    ordering: &CombatSearchV2DiagnosticsOrdering,
    turn_branching: &CombatSearchV2DiagnosticsTurnBranching,
    pending_choice: &CombatSearchV2DiagnosticsPendingChoice,
    turn_prefix: &CombatSearchV2DiagnosticsTurnPrefix,
    turn_sequence: &CombatSearchV2DiagnosticsTurnSequence,
    card_identity: &CombatSearchV2DiagnosticsCardIdentity,
    turn_local_dominance: &CombatSearchV2DiagnosticsTurnLocalDominance,
    pruning: &CombatSearchV2DiagnosticsPruning,
    frontier_remaining_states: usize,
) -> Vec<&'static str> {
    let mut tags = Vec::new();

    match proof_status {
        SearchProofStatus::Exhaustive => tags.push("frontier_exhausted"),
        SearchProofStatus::BudgetExhausted => {
            if frontier_remaining_states > 0 {
                tags.push("budget_exhausted_with_unresolved_frontier");
            } else {
                tags.push("budget_exhausted");
            }
        }
        SearchProofStatus::DeadlineHit => {
            if frontier_remaining_states > 0 {
                tags.push("deadline_hit_with_unresolved_frontier");
            } else {
                tags.push("deadline_hit");
            }
        }
        SearchProofStatus::FrontierUnresolved => tags.push("frontier_unresolved"),
    }

    if stats.terminal_wins > 0 {
        tags.push("terminal_wins_found");
    } else {
        tags.push("no_terminal_wins_found");
    }
    if stats.terminal_losses > 0 {
        tags.push("terminal_losses_found");
    }
    if stats.transposition_prunes > 0 {
        tags.push("transposition_pruning_active");
    } else {
        tags.push("transposition_pruning_inactive");
    }
    if stats.dominance_prunes > 0 {
        tags.push("dominance_pruning_active");
    } else {
        tags.push("dominance_pruning_inactive");
    }
    if pruning.engine_step_limit_count > 0 {
        tags.push("engine_step_limit_truncated_children");
    }
    if pruning.potion_budget_cut_count > 0 {
        tags.push("potion_budget_cutoffs");
    }
    if pruning.turn_local_dominance_prunes > 0 {
        tags.push("turn_local_dominance_pruning_active");
    }
    if pruning.max_actions_cut_count > 0 {
        tags.push("max_actions_per_line_cutoffs");
    }
    if pruning.unresolved_leaf_count > 0 {
        tags.push("unresolved_leaf_states");
    }
    if branching.states_queried > 0 && branching.legal_actions_max == 0 {
        tags.push("no_legal_actions_observed");
    }
    if expansion.states_observed > 0 {
        tags.push("action_expansion_diagnostics_active");
    }
    if expansion.max_group_size > 1 {
        tags.push("action_fanout_groups_observed");
    }
    if target_fanout.states_observed > 0 {
        tags.push("target_fanout_diagnostics_active");
    }
    if target_fanout.multi_target_fanout_groups > 0 {
        tags.push("multi_target_fanout_observed");
    }
    if target_fanout.lethal_target_groups > 0 {
        tags.push("lethal_target_fanout_observed");
    }
    if equivalence.states_observed > 0 {
        tags.push("action_equivalence_diagnostics_active");
    }
    if equivalence.actions_removed > 0 {
        tags.push("equivalence_pruning_active");
        tags.push("duplicate_actions_compressed");
    }
    if ordering.states_observed > 0 {
        tags.push("action_ordering_diagnostics_active");
    }
    if ordering.states_reordered > 0 {
        tags.push("action_ordering_reordered_legal_actions");
    }
    if turn_branching.states_observed > 0 {
        tags.push("turn_branching_diagnostics_active");
    }
    if turn_branching.same_turn_children > 0 {
        tags.push("same_turn_children_observed");
    }
    if turn_branching.next_turn_children > 0 {
        tags.push("next_turn_children_observed");
    }
    if turn_branching.pending_choice_children > 0 {
        tags.push("pending_choice_children_observed");
    }
    if pending_choice.states_observed > 0 {
        tags.push("pending_choice_profile_diagnostics_active");
    }
    if pending_choice.pending_choice_states > 0 {
        tags.push("pending_choice_states_observed");
    }
    if pending_choice.high_fanout_states > 0 {
        tags.push("high_fanout_pending_choices_observed");
    }
    if pending_choice.expanded_pending_choice_states > 0 {
        tags.push("pending_choice_contract_observed");
    }
    if pending_choice.resolved_children > 0 {
        tags.push("pending_choice_children_resolved");
    }
    if pending_choice.still_pending_children > 0 {
        tags.push("pending_choice_children_remained_pending");
    }
    if pending_choice.truncated_children > 0 {
        tags.push("pending_choice_children_truncated");
    }
    if turn_prefix.states_observed > 0 {
        tags.push("turn_prefix_diagnostics_active");
    }
    if turn_prefix.non_empty_prefix_states > 0 {
        tags.push("non_empty_turn_prefix_observed");
    }
    if turn_prefix.max_prefix_length >= 3 {
        tags.push("long_turn_prefix_observed");
    }
    if turn_sequence.states_observed > 0 {
        tags.push("turn_sequence_diagnostics_active");
    }
    if turn_sequence.groups_with_order_variants > 0 {
        tags.push("turn_sequence_order_variants_observed");
    }
    if turn_sequence.same_effect_order_variant_groups > 0 {
        tags.push("turn_sequence_same_effect_candidates_observed");
    }
    if turn_sequence.order_sensitive_groups > 0 {
        tags.push("turn_sequence_order_sensitive_groups_observed");
    }
    if turn_sequence.discard_order_shadow_audit.candidate_groups > 0 {
        tags.push("discard_order_shadow_audit_candidates_observed");
    }
    if card_identity.states_observed > 0 {
        tags.push("card_identity_diagnostics_active");
    }
    if card_identity.states_with_duplicate_active_uuid > 0 {
        tags.push("duplicate_active_card_uuid_observed");
    }
    if card_identity.states_with_uuid_card_id_conflict > 0 {
        tags.push("card_uuid_id_conflict_observed");
    }
    if card_identity.action_payload_placeholder_cards > 0 {
        tags.push("card_payload_placeholders_observed");
    }
    if turn_local_dominance.parent_states_observed > 0 {
        tags.push("turn_local_dominance_diagnostics_active");
    }
    if turn_local_dominance.pruned_child_states > 0 {
        tags.push("turn_local_dominance_pruned_children");
    }
    if frontier_remaining_states > 0 {
        tags.push("frontier_remaining");
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::combat_search_v2::state_abstraction::StateAbstractionRevealGate;

    #[test]
    fn tags_surface_budget_and_active_diagnostics() {
        let stats = CombatSearchV2Stats {
            terminal_wins: 1,
            transposition_prunes: 2,
            ..CombatSearchV2Stats::default()
        };
        let mut branching = branching();
        branching.states_with_legal_actions = 0;
        branching.legal_actions_total = 0;
        branching.legal_actions_avg = 0.0;
        branching.legal_actions_max = 0;
        let mut expansion = expansion();
        expansion.states_observed = 1;
        expansion.max_group_size = 2;
        let mut target_fanout = target_fanout();
        target_fanout.states_observed = 1;
        target_fanout.multi_target_fanout_groups = 1;
        target_fanout.lethal_target_groups = 1;
        let mut equivalence = equivalence();
        equivalence.states_observed = 1;
        equivalence.actions_removed = 1;
        let mut ordering = ordering();
        ordering.states_observed = 1;
        ordering.states_reordered = 1;
        let mut turn_branching = turn_branching();
        turn_branching.states_observed = 1;
        turn_branching.same_turn_children = 1;
        turn_branching.next_turn_children = 1;
        turn_branching.pending_choice_children = 1;
        let mut pending_choice = pending_choice();
        pending_choice.states_observed = 3;
        pending_choice.pending_choice_states = 2;
        pending_choice.high_fanout_states = 1;
        pending_choice.expanded_pending_choice_states = 1;
        pending_choice.resolved_children = 1;
        pending_choice.still_pending_children = 1;
        pending_choice.truncated_children = 1;
        let mut turn_prefix = turn_prefix();
        turn_prefix.states_observed = 1;
        turn_prefix.non_empty_prefix_states = 1;
        turn_prefix.max_prefix_length = 3;
        let mut turn_sequence = turn_sequence();
        turn_sequence.states_observed = 1;
        turn_sequence.groups_with_order_variants = 1;
        turn_sequence.same_effect_order_variant_groups = 1;
        turn_sequence.order_sensitive_groups = 1;
        turn_sequence.discard_order_shadow_audit.candidate_groups = 1;
        let mut card_identity = card_identity();
        card_identity.states_observed = 1;
        card_identity.states_with_duplicate_active_uuid = 1;
        card_identity.states_with_uuid_card_id_conflict = 1;
        card_identity.action_payload_placeholder_cards = 1;
        let mut turn_local_dominance = turn_local_dominance();
        turn_local_dominance.parent_states_observed = 1;
        turn_local_dominance.pruned_child_states = 1;
        let mut pruning = pruning();
        pruning.turn_local_dominance_prunes = 1;
        pruning.unresolved_leaf_count = 1;

        let tags = diagnosis_tags(
            SearchProofStatus::BudgetExhausted,
            &stats,
            &branching,
            &expansion,
            &target_fanout,
            &equivalence,
            &ordering,
            &turn_branching,
            &pending_choice,
            &turn_prefix,
            &turn_sequence,
            &card_identity,
            &turn_local_dominance,
            &pruning,
            4,
        );

        assert!(tags.contains(&"budget_exhausted_with_unresolved_frontier"));
        assert!(tags.contains(&"terminal_wins_found"));
        assert!(tags.contains(&"transposition_pruning_active"));
        assert!(tags.contains(&"dominance_pruning_inactive"));
        assert!(tags.contains(&"turn_local_dominance_pruning_active"));
        assert!(tags.contains(&"unresolved_leaf_states"));
        assert!(tags.contains(&"no_legal_actions_observed"));
        assert!(tags.contains(&"action_expansion_diagnostics_active"));
        assert!(tags.contains(&"action_fanout_groups_observed"));
        assert!(tags.contains(&"target_fanout_diagnostics_active"));
        assert!(tags.contains(&"multi_target_fanout_observed"));
        assert!(tags.contains(&"lethal_target_fanout_observed"));
        assert!(tags.contains(&"action_equivalence_diagnostics_active"));
        assert!(tags.contains(&"equivalence_pruning_active"));
        assert!(tags.contains(&"duplicate_actions_compressed"));
        assert!(tags.contains(&"action_ordering_diagnostics_active"));
        assert!(tags.contains(&"action_ordering_reordered_legal_actions"));
        assert!(tags.contains(&"turn_branching_diagnostics_active"));
        assert!(tags.contains(&"same_turn_children_observed"));
        assert!(tags.contains(&"next_turn_children_observed"));
        assert!(tags.contains(&"pending_choice_children_observed"));
        assert!(tags.contains(&"pending_choice_profile_diagnostics_active"));
        assert!(tags.contains(&"pending_choice_states_observed"));
        assert!(tags.contains(&"high_fanout_pending_choices_observed"));
        assert!(tags.contains(&"pending_choice_contract_observed"));
        assert!(tags.contains(&"pending_choice_children_resolved"));
        assert!(tags.contains(&"pending_choice_children_remained_pending"));
        assert!(tags.contains(&"pending_choice_children_truncated"));
        assert!(tags.contains(&"turn_prefix_diagnostics_active"));
        assert!(tags.contains(&"non_empty_turn_prefix_observed"));
        assert!(tags.contains(&"long_turn_prefix_observed"));
        assert!(tags.contains(&"turn_sequence_diagnostics_active"));
        assert!(tags.contains(&"turn_sequence_order_variants_observed"));
        assert!(tags.contains(&"turn_sequence_same_effect_candidates_observed"));
        assert!(tags.contains(&"turn_sequence_order_sensitive_groups_observed"));
        assert!(tags.contains(&"discard_order_shadow_audit_candidates_observed"));
        assert!(tags.contains(&"card_identity_diagnostics_active"));
        assert!(tags.contains(&"duplicate_active_card_uuid_observed"));
        assert!(tags.contains(&"card_uuid_id_conflict_observed"));
        assert!(tags.contains(&"card_payload_placeholders_observed"));
        assert!(tags.contains(&"turn_local_dominance_diagnostics_active"));
        assert!(tags.contains(&"turn_local_dominance_pruned_children"));
        assert!(tags.contains(&"frontier_remaining"));
    }

    fn branching() -> CombatSearchV2DiagnosticsBranching {
        CombatSearchV2DiagnosticsBranching {
            states_queried: 1,
            states_with_legal_actions: 1,
            legal_actions_total: 2,
            legal_actions_avg: 2.0,
            legal_actions_max: 2,
            nodes_generated_per_expanded: 0.0,
        }
    }

    fn expansion() -> CombatSearchV2DiagnosticsExpansion {
        CombatSearchV2DiagnosticsExpansion {
            grouping_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            total_atomic_actions: 0,
            total_fanout_groups: 0,
            fanout_groups_avg: 0.0,
            fanout_groups_max: 0,
            max_group_size: 0,
            action_kind_counts: Vec::new(),
            largest_groups: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn target_fanout() -> CombatSearchV2DiagnosticsTargetFanout {
        CombatSearchV2DiagnosticsTargetFanout {
            grouping_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            targeted_actions_total: 0,
            target_fanout_groups_total: 0,
            multi_target_fanout_groups: 0,
            avg_targets_per_group: 0.0,
            max_targets_per_group: 0,
            lethal_target_groups: 0,
            unique_lethal_target_groups: 0,
            uniform_damage_groups: 0,
            max_target_hp_span: 0,
            group_kind_counts: Vec::new(),
            largest_target_fanouts: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn equivalence() -> CombatSearchV2DiagnosticsEquivalence {
        CombatSearchV2DiagnosticsEquivalence {
            equivalence_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            states_compressed: 0,
            atomic_actions_in: 0,
            representative_actions_out: 0,
            actions_removed: 0,
            removed_action_ratio: 0.0,
            max_group_size: 0,
            group_kind_counts: Vec::new(),
            largest_groups: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn ordering() -> CombatSearchV2DiagnosticsOrdering {
        CombatSearchV2DiagnosticsOrdering {
            ordering_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            states_reordered: 0,
            reordered_state_ratio: 0.0,
            total_actions_observed: 0,
            action_effect_actions: 0,
            phase_action_hint_actions: 0,
            max_position_shift: 0,
            avg_position_shift: 0.0,
            action_role_counts: Vec::new(),
            largest_reorders: Vec::new(),
            action_effect_samples: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn turn_branching() -> CombatSearchV2DiagnosticsTurnBranching {
        CombatSearchV2DiagnosticsTurnBranching {
            organization_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            total_legal_actions: 0,
            total_generated_children: 0,
            generated_children_per_state: 0.0,
            same_turn_children: 0,
            next_turn_children: 0,
            pending_choice_children: 0,
            terminal_children: 0,
            other_children: 0,
            end_turn_children: 0,
            same_turn_child_ratio: 0.0,
            next_turn_child_ratio: 0.0,
            transition_counts: Vec::new(),
            largest_turn_fanouts: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn pending_choice() -> CombatSearchV2DiagnosticsPendingChoice {
        CombatSearchV2DiagnosticsPendingChoice {
            profiling_policy: "test",
            behavioral_effect: "diagnostic_only",
            rollout_contract_policy: "test",
            rollout_contract_behavioral_effect: "diagnostic_only",
            states_observed: 0,
            pending_choice_states: 0,
            expanded_pending_choice_states: 0,
            high_fanout_states: 0,
            max_candidate_count: 0,
            legal_actions_from_pending_choice: 0,
            max_legal_actions_from_pending_choice: 0,
            resolved_children: 0,
            still_pending_children: 0,
            truncated_children: 0,
            kind_counts: Vec::new(),
            ordering_role_counts: Vec::new(),
            largest_pending_choices: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn turn_prefix() -> CombatSearchV2DiagnosticsTurnPrefix {
        CombatSearchV2DiagnosticsTurnPrefix {
            tracking_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            non_empty_prefix_states: 0,
            empty_prefix_states: 0,
            avg_prefix_length: 0.0,
            max_prefix_length: 0,
            max_legal_actions_after_non_empty_prefix: 0,
            total_cards_played_in_prefix: 0,
            total_potions_used_in_prefix: 0,
            total_potions_discarded_in_prefix: 0,
            total_other_actions_in_prefix: 0,
            prefix_length_counts: Vec::new(),
            prefix_kind_counts: Vec::new(),
            largest_prefix_fanouts: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn turn_sequence() -> CombatSearchV2DiagnosticsTurnSequence {
        CombatSearchV2DiagnosticsTurnSequence {
            grouping_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            non_empty_prefix_states: 0,
            grouped_prefix_states: 0,
            unordered_sequence_groups: 0,
            groups_with_order_variants: 0,
            same_effect_order_variant_groups: 0,
            order_sensitive_groups: 0,
            max_ordered_variants_per_group: 0,
            max_effect_variants_per_group: 0,
            max_prefix_length: 0,
            max_legal_actions_after_prefix: 0,
            order_sensitive_divergence_histogram: Vec::new(),
            discard_order_shadow_audit: CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
                audit_policy: "test",
                behavioral_effect: "diagnostic_only",
                candidate_groups: 0,
                candidate_states: 0,
                static_immediate_safe_groups: 0,
                static_immediate_safe_states: 0,
                exact_rollout_verified_groups: 0,
                proof_pruning_enabled: false,
                reveal_gate: StateAbstractionRevealGate::NextShuffle,
                one_step_exact_policy: "test",
                one_step_exact_stored_group_limit: 0,
                one_step_exact_sample_limit_groups: 0,
                one_step_exact_sample_limit_actions_per_group: 0,
                one_step_exact_checked_groups: 0,
                one_step_exact_sample_verified_groups: 0,
                one_step_exact_blocked_groups: 0,
                one_step_exact_checked_actions: 0,
                one_step_exact_verified_actions: 0,
                one_step_exact_blocked_actions: 0,
                sample_limit: 0,
                samples: Vec::new(),
                notes: Vec::new(),
            },
            largest_groups: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn card_identity() -> CombatSearchV2DiagnosticsCardIdentity {
        CombatSearchV2DiagnosticsCardIdentity {
            audit_policy: "test",
            behavioral_effect: "diagnostic_only",
            states_observed: 0,
            active_cards_observed: 0,
            action_payload_cards_observed: 0,
            action_payload_placeholder_cards: 0,
            states_with_duplicate_active_uuid: 0,
            duplicate_active_uuid_observations: 0,
            states_with_uuid_card_id_conflict: 0,
            uuid_card_id_conflict_observations: 0,
            max_duplicate_group_size: 0,
            largest_duplicate_groups: Vec::new(),
            largest_conflict_groups: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn turn_local_dominance() -> CombatSearchV2DiagnosticsTurnLocalDominance {
        CombatSearchV2DiagnosticsTurnLocalDominance {
            pruning_policy: "test",
            behavioral_effect: "diagnostic_only",
            parent_states_observed: 0,
            enabled_parent_states: 0,
            eligible_child_states: 0,
            accepted_child_states: 0,
            pruned_child_states: 0,
            prune_ratio: 0.0,
            max_parent_dominance_buckets: 0,
            max_parent_resource_vectors: 0,
            max_bucket_width: 0,
            largest_parent_samples: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn pruning() -> CombatSearchV2DiagnosticsPruning {
        CombatSearchV2DiagnosticsPruning {
            transposition_prunes: 2,
            dominance_prunes: 0,
            turn_local_dominance_prunes: 0,
            terminal_wins: 1,
            terminal_losses: 0,
            unresolved_leaf_count: 0,
            max_actions_cut_count: 0,
            engine_step_limit_count: 0,
            potion_budget_cut_count: 0,
        }
    }
}
