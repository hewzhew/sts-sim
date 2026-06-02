use super::*;
use std::collections::HashSet;

mod finalize;

use finalize::{finish_combat_search_report, SearchFinishInput};

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
    let mut diagnostics = SearchDiagnosticsCollector::default();
    let initial_hp = combat.entities.player.current_hp;
    let mut exact_transpositions: HashMap<CombatExactStateKey, Vec<ResourceVector>> =
        HashMap::new();
    let mut dominance: HashMap<CombatDominanceKey, Vec<ResourceVector>> = HashMap::new();
    let mut rollout_cache = RolloutCache::new(
        config.rollout_policy,
        config.rollout_max_evaluations,
        config.rollout_max_actions,
        config.rollout_beam_width,
    );
    let mut frontier = FrontierQueue::new(config.frontier_policy);
    let mut turn_plan_seeded_sources = HashSet::new();
    let mut next_sequence_id = 0u64;
    let mut root = SearchNode {
        engine: engine.clone(),
        combat: combat.clone(),
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    };
    root.rollout_estimate = rollout_cache.estimate(&root, stepper, &config, deadline);
    if terminal_label(&root.engine, &root.combat) == SearchTerminalLabel::Win {
        stats.nodes_to_first_win = Some(0);
    }
    let root_for_turn_plan_diagnostics = root.clone();
    push_frontier(&mut frontier, root, &mut next_sequence_id);
    if config.turn_plan_policy.seeds_root_frontier() {
        seed_turn_plan_frontier(
            &root_for_turn_plan_diagnostics,
            stepper,
            &config,
            deadline,
            &mut rollout_cache,
            &mut stats,
            &mut diagnostics,
            &mut frontier,
            &mut next_sequence_id,
            &mut turn_plan_seeded_sources,
        );
    }

    let mut best_complete: Option<SearchNode> = None;
    let mut best_frontier: Option<SearchNode> = None;
    let mut unresolved_leaf_count = 0u64;
    let mut max_actions_cut_count = 0u64;
    let mut engine_step_limit_count = 0u64;
    let mut potion_budget_cut_count = 0u64;
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

        if should_seed_turn_plan_at_node(&node, &config) {
            seed_turn_plan_frontier(
                &node,
                stepper,
                &config,
                deadline,
                &mut rollout_cache,
                &mut stats,
                &mut diagnostics,
                &mut frontier,
                &mut next_sequence_id,
                &mut turn_plan_seeded_sources,
            );
        }

        stats.nodes_expanded = stats.nodes_expanded.saturating_add(1);
        let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
        let legal = filtered_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
            &node.combat,
        );
        let pending_choice = summarize_pending_choice(&node.engine);
        diagnostics.observe_pending_choice(pending_choice.as_ref());
        let expansion = summarize_action_expansion(&node.engine, &node.combat, &legal);
        diagnostics.observe_legal_actions(&expansion);
        let turn_prefix = summarize_turn_prefix(&node.turn_prefix, legal.len());
        diagnostics.observe_turn_prefix(&turn_prefix);
        let turn_sequence = summarize_turn_sequence(&node, legal.len());
        diagnostics.observe_turn_sequence(&turn_sequence, &node);
        let card_identity = summarize_card_identity(&node.combat);
        diagnostics.observe_card_identity(&card_identity);
        let target_fanout = summarize_target_fanout(&node.combat, &legal);
        diagnostics.observe_target_fanout(&target_fanout);
        if legal.is_empty() {
            unresolved_leaf_count = unresolved_leaf_count.saturating_add(1);
            continue;
        }
        let equivalence = compress_equivalent_actions(&node.engine, &node.combat, legal);
        diagnostics.observe_action_equivalence(&equivalence.summary);
        let ordered = order_indexed_action_choices(&node.engine, &node.combat, equivalence.choices);
        diagnostics.observe_action_ordering(&ordered.summary);
        diagnostics.observe_pending_choice_ordering(pending_choice.as_ref(), &ordered.summary);
        let mut turn_branching =
            TurnBranchingStateObservation::new(&node.combat, ordered.choices.len());
        let mut turn_local_dominance = TurnLocalDominanceStateObservation::new(
            &node.engine,
            &node.combat,
            ordered.choices.len(),
        );

        for ordered_choice in ordered.choices {
            let action_id = ordered_choice.original_action_id;
            let choice = ordered_choice.choice;
            let potion_tactical_priority =
                potions::semantic_potion_tactical_priority(&node.combat, &choice.input);
            if config
                .max_potions_used
                .is_some_and(|max| node.potions_used >= max && is_use_potion_input(&choice.input))
            {
                potion_budget_cut_count = potion_budget_cut_count.saturating_add(1);
                continue;
            }
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
            diagnostics.observe_pending_choice_child_transition(
                pending_choice.as_ref(),
                step.truncated,
                &child.engine,
            );
            let turn_transition = classify_turn_branch_transition(
                &node.engine,
                &node.combat,
                &choice.input,
                &child.engine,
                &child.combat,
            );
            child.note_turn_prefix(&node.combat, &choice.input, turn_transition);
            child.note_input(&choice.input);
            child.note_potion_tactical_priority(potion_tactical_priority);
            child.note_turn_branch_priority(turn_transition.frontier_priority_hint());
            turn_branching.observe_child(turn_transition);
            child.actions.push(CombatSearchV2ActionTrace {
                step_index: node.actions.len(),
                action_id,
                action_key: choice.action_key,
                action_debug: choice.action_debug,
                input: choice.input,
            });
            stats.nodes_generated = stats.nodes_generated.saturating_add(1);
            child.rollout_estimate = rollout_cache.estimate(&child, stepper, &config, deadline);

            if !step.truncated && turn_local_dominance.observe_child(&child) {
                stats.turn_local_dominance_prunes =
                    stats.turn_local_dominance_prunes.saturating_add(1);
                continue;
            }

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
        diagnostics.observe_turn_branching(&turn_branching);
        diagnostics.observe_turn_local_dominance(&turn_local_dominance);

        if exhausted {
            break;
        }
    }

    diagnostics.run_discard_order_exact_shadow_audit(stepper, &config);
    stats.elapsed_ms = started.elapsed().as_millis();
    diagnostics.observe_root_turn_plan(&root_for_turn_plan_diagnostics, stepper);
    finish_combat_search_report(SearchFinishInput {
        config,
        stats,
        diagnostics,
        exact_transpositions,
        dominance,
        frontier,
        best_complete,
        best_frontier,
        rollout_cache,
        unresolved_leaf_count,
        max_actions_cut_count,
        engine_step_limit_count,
        potion_budget_cut_count,
        exhausted,
    })
}

fn seed_turn_plan_frontier(
    source: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    rollout_cache: &mut RolloutCache,
    stats: &mut CombatSearchV2Stats,
    diagnostics: &mut SearchDiagnosticsCollector,
    frontier: &mut FrontierQueue,
    next_sequence_id: &mut u64,
    seeded_sources: &mut HashSet<CombatExactStateKey>,
) {
    let source_key = combat_exact_state_key(&source.engine, &source.combat);
    if !seeded_sources.insert(source_key) {
        return;
    }

    let mut seeded_nodes = turn_plan_frontier_seed(source, stepper, config, deadline);
    diagnostics.observe_turn_plan_frontier_seeded_nodes(seeded_nodes.nodes.len());
    for mut seed in seeded_nodes.nodes.drain(..) {
        seed.rollout_estimate = rollout_cache.estimate(&seed, stepper, config, deadline);
        stats.nodes_generated = stats.nodes_generated.saturating_add(1);
        if stats.nodes_to_first_win.is_none()
            && terminal_label(&seed.engine, &seed.combat) == SearchTerminalLabel::Win
        {
            stats.nodes_to_first_win = Some(stats.nodes_generated);
        }
        push_frontier(frontier, seed, next_sequence_id);
    }
}

fn should_seed_turn_plan_at_node(node: &SearchNode, config: &CombatSearchV2Config) -> bool {
    if !config.turn_plan_policy.seeds_turn_boundary_frontier()
        || !matches!(node.engine, EngineState::CombatPlayerTurn)
        || node.turn_prefix.prefix_length() != 0
        || terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Unresolved
    {
        return false;
    }

    if config.turn_plan_policy.requires_support_enemy_gate() {
        return support_enemy_turn_plan_seed_gate(node);
    }

    true
}

fn support_enemy_turn_plan_seed_gate(node: &SearchNode) -> bool {
    let profile = combat_search_phase_profile(&node.engine, &node.combat);
    profile.enemy_mechanics.healer_support_count > 0 && living_enemy_count(&node.combat) >= 2
}

#[cfg(test)]
mod tests;
