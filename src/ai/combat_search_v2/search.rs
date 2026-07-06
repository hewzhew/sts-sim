use super::rollout_scheduler::{deferred_child_rollout_admission, DeferredRolloutAdmission};
use super::*;

mod child_expansion;
mod finalize;
mod loop_state;

use child_expansion::{expand_ordered_child, ChildExpansionInput, ChildExpansionOutcome};
use finalize::{finish_combat_search_report, SearchFinishInput};
use loop_state::SearchLoopState;

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
    let initial_hp = combat.entities.player.current_hp;
    let policy_evidence = combat_search_policy_evidence_for_combat(combat);
    let mut loop_state = SearchLoopState::new(&config);
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
        &mut loop_state.rollout_cache,
        &root,
        stepper,
        &config,
        deadline,
        &mut loop_state.performance,
        RolloutEstimateSource::Root,
    );
    if terminal_label(&root.engine, &root.combat) == SearchTerminalLabel::Win {
        loop_state.stats.nodes_to_first_win = Some(0);
    }
    let root_for_turn_plan_diagnostics = root.clone();
    loop_state.push_frontier(root);
    if config.turn_plan_policy.seeds_root_frontier() {
        loop_state.seed_turn_plan_frontier(
            &root_for_turn_plan_diagnostics,
            stepper,
            &config,
            deadline,
        );
    }

    loop {
        let Some(entry) = loop_state.pop_frontier() else {
            break;
        };

        if loop_state.stats.nodes_expanded as usize >= config.max_nodes {
            loop_state.stats.node_budget_hit = true;
            loop_state.exhausted = true;
            loop_state.push_frontier(entry.node);
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            loop_state.stats.deadline_hit = true;
            loop_state.exhausted = true;
            loop_state.push_frontier(entry.node);
            break;
        }

        let mut node = entry.node;
        let admission = deferred_child_rollout_admission(
            &node,
            &config,
            &loop_state.stats,
            &loop_state.performance,
            started,
        );
        observe_deferred_rollout_admission(admission, &mut loop_state.performance);
        if admission.admitted() {
            node.rollout_estimate = timed_rollout_estimate(
                &mut loop_state.rollout_cache,
                &node,
                stepper,
                &config,
                deadline,
                &mut loop_state.performance,
                RolloutEstimateSource::DeferredChild,
            );
            if node.rollout_estimate.is_evaluated() {
                loop_state.performance.deferred_child_rollout_requeues = loop_state
                    .performance
                    .deferred_child_rollout_requeues
                    .saturating_add(1);
                loop_state.push_frontier(node);
                continue;
            }
        }

        let pre_expand_started = Instant::now();
        loop_state.remember_best_frontier(&node);

        match terminal_label(&node.engine, &node.combat) {
            SearchTerminalLabel::Win => {
                if loop_state.remember_win(node, &config) {
                    loop_state.performance.pre_expand_elapsed_us = loop_state
                        .performance
                        .pre_expand_elapsed_us
                        .saturating_add(pre_expand_started.elapsed().as_micros());
                    loop_state.accepted_complete_candidate = true;
                    break;
                }
                loop_state.performance.pre_expand_elapsed_us = loop_state
                    .performance
                    .pre_expand_elapsed_us
                    .saturating_add(pre_expand_started.elapsed().as_micros());
                continue;
            }
            SearchTerminalLabel::Loss => {
                loop_state.remember_loss(node);
                loop_state.performance.pre_expand_elapsed_us = loop_state
                    .performance
                    .pre_expand_elapsed_us
                    .saturating_add(pre_expand_started.elapsed().as_micros());
                continue;
            }
            SearchTerminalLabel::Unresolved => {}
        }

        if node.actions.len() >= config.max_actions_per_line {
            loop_state.max_actions_cut_count = loop_state.max_actions_cut_count.saturating_add(1);
            loop_state.performance.pre_expand_elapsed_us = loop_state
                .performance
                .pre_expand_elapsed_us
                .saturating_add(pre_expand_started.elapsed().as_micros());
            continue;
        }

        let resource = node.resource_vector();
        let exact_key = combat_exact_state_key(&node.engine, &node.combat);
        if is_resource_covered(&mut loop_state.exact_transpositions, exact_key, resource) {
            loop_state.stats.transposition_prunes =
                loop_state.stats.transposition_prunes.saturating_add(1);
            loop_state.performance.pre_expand_elapsed_us = loop_state
                .performance
                .pre_expand_elapsed_us
                .saturating_add(pre_expand_started.elapsed().as_micros());
            continue;
        }

        let dominance_key = combat_dominance_key(&node.engine, &node.combat);
        if is_resource_covered(&mut loop_state.dominance, dominance_key, resource) {
            loop_state.stats.dominance_prunes = loop_state.stats.dominance_prunes.saturating_add(1);
            loop_state.performance.pre_expand_elapsed_us = loop_state
                .performance
                .pre_expand_elapsed_us
                .saturating_add(pre_expand_started.elapsed().as_micros());
            continue;
        }

        if should_seed_turn_plan_at_node(&node, &config) {
            loop_state.seed_turn_plan_frontier(&node, stepper, &config, deadline);
        }
        loop_state.performance.pre_expand_elapsed_us = loop_state
            .performance
            .pre_expand_elapsed_us
            .saturating_add(pre_expand_started.elapsed().as_micros());

        loop_state.stats.nodes_expanded = loop_state.stats.nodes_expanded.saturating_add(1);
        let expansion_started = Instant::now();
        let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
        let legal = filtered_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
            &node.combat,
        );
        let pending_choice = summarize_pending_choice(&node.engine);
        loop_state
            .diagnostics
            .observe_pending_choice(pending_choice.as_ref());
        let expansion = summarize_action_expansion(&node.engine, &node.combat, &legal);
        loop_state.diagnostics.observe_legal_actions(&expansion);
        let turn_prefix = summarize_turn_prefix(&node.turn_prefix, legal.len());
        loop_state.diagnostics.observe_turn_prefix(&turn_prefix);
        let turn_sequence = summarize_turn_sequence(&node, legal.len());
        loop_state
            .diagnostics
            .observe_turn_sequence(&turn_sequence, &node);
        let card_identity = summarize_card_identity(&node.combat);
        loop_state.diagnostics.observe_card_identity(&card_identity);
        let target_fanout = summarize_target_fanout(&node.combat, &legal);
        loop_state.diagnostics.observe_target_fanout(&target_fanout);
        if legal.is_empty() {
            loop_state.unresolved_leaf_count = loop_state.unresolved_leaf_count.saturating_add(1);
            loop_state.performance.expansion_elapsed_us = loop_state
                .performance
                .expansion_elapsed_us
                .saturating_add(expansion_started.elapsed().as_micros());
            continue;
        }
        let equivalence = compress_equivalent_actions(&node.engine, &node.combat, legal);
        loop_state
            .diagnostics
            .observe_action_equivalence(&equivalence.summary);
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
            config.phase_guard_policy,
            config.setup_bias_policy,
        );
        loop_state
            .diagnostics
            .observe_action_ordering(&ordered.summary);
        loop_state
            .diagnostics
            .observe_pending_choice_ordering(pending_choice.as_ref(), &ordered.summary);
        let mut turn_branching =
            TurnBranchingStateObservation::new(&node.combat, ordered.choices.len());
        let mut turn_local_dominance = TurnLocalDominanceStateObservation::new(
            &node.engine,
            &node.combat,
            ordered.choices.len(),
        );
        loop_state.performance.expansion_elapsed_us = loop_state
            .performance
            .expansion_elapsed_us
            .saturating_add(expansion_started.elapsed().as_micros());

        for ordered_choice in ordered.choices {
            let outcome = expand_ordered_child(
                &mut loop_state,
                &mut turn_branching,
                &mut turn_local_dominance,
                ChildExpansionInput {
                    parent: &node,
                    position: &position,
                    ordered_choice,
                    action_prior_state_hash: action_prior_state_hash.as_deref(),
                    pending_choice: pending_choice.as_ref(),
                    stepper,
                    config: &config,
                    deadline,
                },
            );
            if outcome == ChildExpansionOutcome::DeadlineReached {
                break;
            }
        }
        loop_state
            .diagnostics
            .observe_turn_branching(&turn_branching);
        loop_state
            .diagnostics
            .observe_turn_local_dominance(&turn_local_dominance);

        if loop_state.exhausted {
            break;
        }
    }

    loop_state.finish_diagnostics_and_timing(
        started,
        &root_for_turn_plan_diagnostics,
        stepper,
        &config,
    );
    finish_combat_search_report(SearchFinishInput {
        config,
        policy_evidence,
        stats: loop_state.stats,
        diagnostics: loop_state.diagnostics,
        exact_transpositions: loop_state.exact_transpositions,
        dominance: loop_state.dominance,
        frontier: loop_state.frontier,
        best_complete: loop_state.best_complete,
        best_win: loop_state.best_win,
        win_candidates: loop_state.win_candidates,
        best_frontier: loop_state.best_frontier,
        rollout_cache: loop_state.rollout_cache,
        performance: loop_state.performance,
        unresolved_leaf_count: loop_state.unresolved_leaf_count,
        max_actions_cut_count: loop_state.max_actions_cut_count,
        engine_step_limit_count: loop_state.engine_step_limit_count,
        potion_budget_cut_count: loop_state.potion_budget_cut_count,
        exhausted: loop_state.exhausted,
        accepted_complete_candidate: loop_state.accepted_complete_candidate,
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

fn observe_deferred_rollout_admission(
    admission: DeferredRolloutAdmission,
    performance: &mut CombatSearchV2PerformanceReport,
) {
    match admission {
        DeferredRolloutAdmission::AdmitSignal => {
            performance.deferred_child_rollout_admitted_signal = performance
                .deferred_child_rollout_admitted_signal
                .saturating_add(1);
        }
        DeferredRolloutAdmission::AdmitPeriodic => {
            performance.deferred_child_rollout_admitted_periodic = performance
                .deferred_child_rollout_admitted_periodic
                .saturating_add(1);
        }
        DeferredRolloutAdmission::SkipLowSignal => {
            performance.deferred_child_rollout_skipped_low_signal = performance
                .deferred_child_rollout_skipped_low_signal
                .saturating_add(1);
        }
        DeferredRolloutAdmission::SkipBudgetShare => {
            performance.deferred_child_rollout_skipped_budget_share = performance
                .deferred_child_rollout_skipped_budget_share
                .saturating_add(1);
        }
    }
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
