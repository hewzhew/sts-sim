use super::*;
use std::collections::HashSet;

mod finalize;

use finalize::{finish_combat_search_report, SearchFinishInput};

const TURN_PLAN_SEED_CRITICAL_SURVIVAL_MARGIN: i32 = 6;

#[derive(Clone, Copy, Debug)]
enum RolloutEstimateSource {
    Root,
    Child,
    DeferredChild,
    TurnPlanSeed,
}

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
    let policy_evidence = combat_search_policy_evidence_for_combat(combat);
    let mut exact_transpositions: HashMap<CombatExactStateKey, Vec<ResourceVector>> =
        HashMap::new();
    let mut dominance: HashMap<CombatDominanceKey, Vec<ResourceVector>> = HashMap::new();
    let mut rollout_cache = RolloutCache::new(
        config.rollout_policy,
        config.rollout_max_evaluations,
        config.rollout_max_actions,
        config.rollout_beam_width,
    );
    let mut performance = CombatSearchV2PerformanceReport::default();
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
        action_prior_score: None,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    };
    root.rollout_estimate = timed_rollout_estimate(
        &mut rollout_cache,
        &root,
        stepper,
        &config,
        deadline,
        &mut performance,
        RolloutEstimateSource::Root,
    );
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
            &mut performance,
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
    let mut accepted_complete_candidate = false;

    loop {
        let frontier_pop_started = Instant::now();
        let Some(entry) = frontier.pop() else {
            break;
        };
        performance.frontier_pop_calls = performance.frontier_pop_calls.saturating_add(1);
        performance.frontier_pop_elapsed_us = performance
            .frontier_pop_elapsed_us
            .saturating_add(frontier_pop_started.elapsed().as_micros());

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

        let mut node = entry.node;
        if should_complete_deferred_child_rollout(&node, &config) {
            node.rollout_estimate = timed_rollout_estimate(
                &mut rollout_cache,
                &node,
                stepper,
                &config,
                deadline,
                &mut performance,
                RolloutEstimateSource::DeferredChild,
            );
            if node.rollout_estimate.is_evaluated() {
                performance.deferred_child_rollout_requeues = performance
                    .deferred_child_rollout_requeues
                    .saturating_add(1);
                push_frontier(&mut frontier, node, &mut next_sequence_id);
                continue;
            }
        }

        let pre_expand_started = Instant::now();
        remember_best_frontier(&mut best_frontier, &node);

        match terminal_label(&node.engine, &node.combat) {
            SearchTerminalLabel::Win => {
                stats.terminal_wins = stats.terminal_wins.saturating_add(1);
                if stats.nodes_to_first_win.is_none() {
                    stats.nodes_to_first_win = Some(stats.nodes_generated);
                }
                remember_best_complete(&mut best_complete, node);
                if best_complete
                    .as_ref()
                    .is_some_and(|best| accepted_complete_win(best, &config))
                {
                    performance.pre_expand_elapsed_us = performance
                        .pre_expand_elapsed_us
                        .saturating_add(pre_expand_started.elapsed().as_micros());
                    accepted_complete_candidate = true;
                    break;
                }
                performance.pre_expand_elapsed_us = performance
                    .pre_expand_elapsed_us
                    .saturating_add(pre_expand_started.elapsed().as_micros());
                continue;
            }
            SearchTerminalLabel::Loss => {
                stats.terminal_losses = stats.terminal_losses.saturating_add(1);
                remember_best_complete(&mut best_complete, node);
                performance.pre_expand_elapsed_us = performance
                    .pre_expand_elapsed_us
                    .saturating_add(pre_expand_started.elapsed().as_micros());
                continue;
            }
            SearchTerminalLabel::Unresolved => {}
        }

        if node.actions.len() >= config.max_actions_per_line {
            max_actions_cut_count = max_actions_cut_count.saturating_add(1);
            performance.pre_expand_elapsed_us = performance
                .pre_expand_elapsed_us
                .saturating_add(pre_expand_started.elapsed().as_micros());
            continue;
        }

        let resource = node.resource_vector();
        let exact_key = combat_exact_state_key(&node.engine, &node.combat);
        if is_resource_covered(&mut exact_transpositions, exact_key, resource) {
            stats.transposition_prunes = stats.transposition_prunes.saturating_add(1);
            performance.pre_expand_elapsed_us = performance
                .pre_expand_elapsed_us
                .saturating_add(pre_expand_started.elapsed().as_micros());
            continue;
        }

        let dominance_key = combat_dominance_key(&node.engine, &node.combat);
        if is_resource_covered(&mut dominance, dominance_key, resource) {
            stats.dominance_prunes = stats.dominance_prunes.saturating_add(1);
            performance.pre_expand_elapsed_us = performance
                .pre_expand_elapsed_us
                .saturating_add(pre_expand_started.elapsed().as_micros());
            continue;
        }

        if should_seed_turn_plan_at_node(&node, &config) {
            seed_turn_plan_frontier(
                &node,
                stepper,
                &config,
                deadline,
                &mut rollout_cache,
                &mut performance,
                &mut stats,
                &mut diagnostics,
                &mut frontier,
                &mut next_sequence_id,
                &mut turn_plan_seeded_sources,
            );
        }
        performance.pre_expand_elapsed_us = performance
            .pre_expand_elapsed_us
            .saturating_add(pre_expand_started.elapsed().as_micros());

        stats.nodes_expanded = stats.nodes_expanded.saturating_add(1);
        let expansion_started = Instant::now();
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
            performance.expansion_elapsed_us = performance
                .expansion_elapsed_us
                .saturating_add(expansion_started.elapsed().as_micros());
            continue;
        }
        let equivalence = compress_equivalent_actions(&node.engine, &node.combat, legal);
        diagnostics.observe_action_equivalence(&equivalence.summary);
        let action_prior_state_hash = config
            .root_action_prior
            .as_ref()
            .filter(|prior| !prior.is_empty())
            .map(|_| combat_exact_state_hash_v1(&node.engine, &node.combat));
        let ordered = order_indexed_action_choices_with_prior(
            &node.engine,
            &node.combat,
            equivalence.choices,
            config.root_action_prior.as_ref(),
        );
        diagnostics.observe_action_ordering(&ordered.summary);
        diagnostics.observe_pending_choice_ordering(pending_choice.as_ref(), &ordered.summary);
        let mut turn_branching =
            TurnBranchingStateObservation::new(&node.combat, ordered.choices.len());
        let mut turn_local_dominance = TurnLocalDominanceStateObservation::new(
            &node.engine,
            &node.combat,
            ordered.choices.len(),
        );
        performance.expansion_elapsed_us = performance
            .expansion_elapsed_us
            .saturating_add(expansion_started.elapsed().as_micros());

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
            let step_started = Instant::now();
            let step = stepper.apply_to_stable(
                &position,
                choice.input.clone(),
                CombatStepLimits {
                    max_engine_steps: config.max_engine_steps_per_action,
                    deadline,
                },
            );
            performance.engine_step_calls = performance.engine_step_calls.saturating_add(1);
            performance.engine_step_elapsed_us = performance
                .engine_step_elapsed_us
                .saturating_add(step_started.elapsed().as_micros());
            if step.truncated && !step.timed_out {
                engine_step_limit_count = engine_step_limit_count.saturating_add(1);
            }
            if step.timed_out {
                stats.deadline_hit = true;
                exhausted = true;
            }

            let child_bookkeeping_started = Instant::now();
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
            child.note_action_prior_score(action_prior_state_hash.as_ref().and_then(
                |state_hash| {
                    config
                        .root_action_prior
                        .as_ref()
                        .and_then(|prior| prior.score(state_hash, &choice.action_key))
                },
            ));
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
            performance.child_bookkeeping_elapsed_us = performance
                .child_bookkeeping_elapsed_us
                .saturating_add(child_bookkeeping_started.elapsed().as_micros());

            let child_bookkeeping_started = Instant::now();
            if !step.truncated && turn_local_dominance.observe_child(&child) {
                stats.turn_local_dominance_prunes =
                    stats.turn_local_dominance_prunes.saturating_add(1);
                performance.turn_local_dominance_rollout_skips = performance
                    .turn_local_dominance_rollout_skips
                    .saturating_add(1);
                performance.child_bookkeeping_elapsed_us = performance
                    .child_bookkeeping_elapsed_us
                    .saturating_add(child_bookkeeping_started.elapsed().as_micros());
                continue;
            }

            child.rollout_estimate = if terminal_label(&child.engine, &child.combat)
                != SearchTerminalLabel::Unresolved
            {
                performance.terminal_child_rollout_skips =
                    performance.terminal_child_rollout_skips.saturating_add(1);
                RolloutNodeEstimate::from_node(
                    &child,
                    0,
                    RolloutStopReason::TerminalState,
                    Some("terminal_child_no_rollout"),
                    super::rollout_pending_choice::RolloutPendingChoiceProgress::default(),
                )
            } else if config.child_rollout_policy == CombatSearchV2ChildRolloutPolicy::LazyOnPop
                && config.rollout_policy != CombatSearchV2RolloutPolicy::Disabled
            {
                performance.deferred_child_rollout_nodes =
                    performance.deferred_child_rollout_nodes.saturating_add(1);
                RolloutNodeEstimate::unevaluated()
            } else {
                timed_rollout_estimate(
                    &mut rollout_cache,
                    &child,
                    stepper,
                    &config,
                    deadline,
                    &mut performance,
                    RolloutEstimateSource::Child,
                )
            };

            let child_bookkeeping_started = Instant::now();
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
            performance.child_bookkeeping_elapsed_us = performance
                .child_bookkeeping_elapsed_us
                .saturating_add(child_bookkeeping_started.elapsed().as_micros());
        }
        diagnostics.observe_turn_branching(&turn_branching);
        diagnostics.observe_turn_local_dominance(&turn_local_dominance);

        if exhausted {
            break;
        }
    }

    let shadow_audit_started = Instant::now();
    diagnostics.run_discard_order_exact_shadow_audit(stepper, &config);
    performance.shadow_audit_elapsed_us = shadow_audit_started.elapsed().as_micros();
    let root_turn_plan_diagnostics_started = Instant::now();
    diagnostics.observe_root_turn_plan(&root_for_turn_plan_diagnostics, stepper);
    performance.root_turn_plan_diagnostics_elapsed_us =
        root_turn_plan_diagnostics_started.elapsed().as_micros();
    let total_elapsed = started.elapsed();
    stats.elapsed_ms = total_elapsed.as_millis();
    performance.total_elapsed_us = total_elapsed.as_micros();
    performance.unattributed_elapsed_us = performance.total_elapsed_us.saturating_sub(
        performance
            .engine_step_elapsed_us
            .saturating_add(performance.rollout_estimate_elapsed_us)
            .saturating_add(performance.frontier_pop_elapsed_us)
            .saturating_add(performance.pre_expand_elapsed_us)
            .saturating_add(performance.expansion_elapsed_us)
            .saturating_add(performance.child_bookkeeping_elapsed_us)
            .saturating_add(performance.turn_plan_frontier_seed_elapsed_us)
            .saturating_add(performance.shadow_audit_elapsed_us)
            .saturating_add(performance.root_turn_plan_diagnostics_elapsed_us),
    );
    finish_combat_search_report(SearchFinishInput {
        config,
        policy_evidence,
        stats,
        diagnostics,
        exact_transpositions,
        dominance,
        frontier,
        best_complete,
        best_frontier,
        rollout_cache,
        performance,
        unresolved_leaf_count,
        max_actions_cut_count,
        engine_step_limit_count,
        potion_budget_cut_count,
        exhausted,
        accepted_complete_candidate,
    })
}

fn timed_rollout_estimate(
    rollout_cache: &mut RolloutCache,
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    performance: &mut CombatSearchV2PerformanceReport,
    source: RolloutEstimateSource,
) -> RolloutNodeEstimate {
    let started = Instant::now();
    let estimate = rollout_cache.estimate(node, stepper, config, deadline);
    performance.rollout_estimate_calls = performance.rollout_estimate_calls.saturating_add(1);
    match source {
        RolloutEstimateSource::Root => {
            performance.root_rollout_estimate_calls =
                performance.root_rollout_estimate_calls.saturating_add(1);
        }
        RolloutEstimateSource::Child => {
            performance.child_rollout_estimate_calls =
                performance.child_rollout_estimate_calls.saturating_add(1);
        }
        RolloutEstimateSource::DeferredChild => {
            performance.deferred_child_rollout_estimate_calls = performance
                .deferred_child_rollout_estimate_calls
                .saturating_add(1);
        }
        RolloutEstimateSource::TurnPlanSeed => {
            performance.turn_plan_seed_rollout_estimate_calls = performance
                .turn_plan_seed_rollout_estimate_calls
                .saturating_add(1);
        }
    }
    performance.rollout_estimate_elapsed_us = performance
        .rollout_estimate_elapsed_us
        .saturating_add(started.elapsed().as_micros());
    estimate
}

fn should_complete_deferred_child_rollout(
    node: &SearchNode,
    config: &CombatSearchV2Config,
) -> bool {
    config.child_rollout_policy == CombatSearchV2ChildRolloutPolicy::LazyOnPop
        && config.rollout_policy != CombatSearchV2RolloutPolicy::Disabled
        && !node.rollout_estimate.is_evaluated()
        && terminal_label(&node.engine, &node.combat) == SearchTerminalLabel::Unresolved
}

fn accepted_complete_win(node: &SearchNode, config: &CombatSearchV2Config) -> bool {
    if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Win {
        return false;
    }
    let hp_loss = (node.initial_hp - node.combat.entities.player.current_hp).max(0) as u32;
    if hp_loss == 0 && !super::external_payoff::has_external_payoff_opportunity(&node.combat) {
        return true;
    }
    let Some(limit) = config.stop_on_win_hp_loss_at_most else {
        return false;
    };
    hp_loss <= limit
}

fn seed_turn_plan_frontier(
    source: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    rollout_cache: &mut RolloutCache,
    performance: &mut CombatSearchV2PerformanceReport,
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

    let seed_started = Instant::now();
    let mut seeded_nodes = turn_plan_frontier_seed(source, stepper, config, deadline);
    performance.turn_plan_frontier_seed_calls =
        performance.turn_plan_frontier_seed_calls.saturating_add(1);
    performance.turn_plan_frontier_seed_elapsed_us = performance
        .turn_plan_frontier_seed_elapsed_us
        .saturating_add(seed_started.elapsed().as_micros());
    diagnostics.observe_turn_plan_frontier_seeded_nodes(seeded_nodes.nodes.len());
    diagnostics.observe_turn_plan_prior_scored_plans(seeded_nodes.turn_plan_prior_scored_plans);
    for mut seed in seeded_nodes.nodes.drain(..) {
        seed.rollout_estimate =
            if terminal_label(&seed.engine, &seed.combat) == SearchTerminalLabel::Unresolved {
                timed_rollout_estimate(
                    rollout_cache,
                    &seed,
                    stepper,
                    config,
                    deadline,
                    performance,
                    RolloutEstimateSource::TurnPlanSeed,
                )
            } else {
                performance.terminal_turn_plan_seed_rollout_skips = performance
                    .terminal_turn_plan_seed_rollout_skips
                    .saturating_add(1);
                RolloutNodeEstimate::from_node(
                    &seed,
                    0,
                    RolloutStopReason::TerminalState,
                    Some("terminal_turn_plan_seed_no_rollout"),
                    super::rollout_pending_choice::RolloutPendingChoiceProgress::default(),
                )
            };
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

    if turn_plan_prior_has_current_state(node, config) {
        return true;
    }

    if config.turn_plan_policy.requires_tactical_enemy_gate() {
        return tactical_enemy_turn_plan_seed_gate(node);
    }

    true
}

fn turn_plan_prior_has_current_state(node: &SearchNode, config: &CombatSearchV2Config) -> bool {
    let Some(prior) = config
        .turn_plan_prior
        .as_ref()
        .filter(|prior| !prior.is_empty())
    else {
        return false;
    };
    let state_hash = combat_exact_state_hash_v1(&node.engine, &node.combat);
    prior.has_hints_for_state(&state_hash)
}

fn tactical_enemy_turn_plan_seed_gate(node: &SearchNode) -> bool {
    if node.combat.meta.is_boss_fight || node.combat.meta.is_elite_fight {
        return true;
    }

    if visible_high_pressure_turn_plan_seed_gate(&node.combat) {
        return true;
    }

    let profile = combat_search_phase_profile(&node.engine, &node.combat);
    (profile.enemy_mechanics.healer_support_count > 0 && living_enemy_count(&node.combat) >= 2)
        || profile.enemy_mechanics.fungi_beast_count >= 3
}

fn visible_high_pressure_turn_plan_seed_gate(combat: &CombatState) -> bool {
    let incoming = visible_incoming_damage(combat);
    if incoming <= 0 {
        return false;
    }
    let survival_margin = combat
        .entities
        .player
        .current_hp
        .saturating_add(combat.entities.player.block)
        .saturating_sub(incoming);
    survival_margin <= TURN_PLAN_SEED_CRITICAL_SURVIVAL_MARGIN
}

#[cfg(test)]
mod tests;
