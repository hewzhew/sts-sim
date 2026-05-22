use super::*;

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
    let mut exact_transpositions: HashMap<CombatExactStateKey, Vec<ResourceVector>> =
        HashMap::new();
    let mut dominance: HashMap<CombatDominanceKey, Vec<ResourceVector>> = HashMap::new();
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

        let resource = node.resource_vector();
        let exact_key = combat_exact_state_key(&node.engine, &node.combat);
        if is_resource_covered(&mut exact_transpositions, exact_key, resource) {
            stats.transposition_prunes = stats.transposition_prunes.saturating_add(1);
            continue;
        }

        let dominance_key = combat_dominance_key(&node.engine, &node.combat);
        if is_resource_covered(&mut dominance, dominance_key, resource) {
            stats.dominance_prunes = stats.dominance_prunes.saturating_add(1);
            continue;
        }

        stats.nodes_expanded = stats.nodes_expanded.saturating_add(1);
        let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
        let legal = filtered_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
        );
        if legal.is_empty() {
            unresolved_leaf_count = unresolved_leaf_count.saturating_add(1);
            continue;
        }

        for (action_id, choice) in legal.into_iter().enumerate() {
            if deadline.is_some_and(|limit| Instant::now() >= limit) {
                stats.deadline_hit = true;
                exhausted = true;
                break;
            }
            let step = stepper.apply_to_stable(
                &position,
                choice.input.clone(),
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
            child.note_input(&choice.input);
            child.actions.push(CombatSearchV2ActionTrace {
                step_index: node.actions.len(),
                action_id,
                action_key: choice.action_key,
                action_debug: choice.action_debug,
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
            transposition_table: "exact_runtime_state_key_with_resource_coverage",
            dominance_pruning: "dominance_bucket_excludes_player_hp_block_then_compares_resource_vector",
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
