use super::*;

mod child_expansion;
mod finalize;
mod loop_state;
mod node_expansion;
mod node_preflight;
mod rollout_timing;

use child_expansion::{expand_ordered_child, ChildExpansionInput, ChildExpansionOutcome};
use finalize::{finish_combat_search_report, SearchFinishInput};
use loop_state::SearchLoopState;
use node_expansion::prepare_node_expansion;
use node_preflight::{prepare_node_for_expansion, NodePreflightInput, NodePreflightOutcome};
use rollout_timing::{timed_rollout_estimate, RolloutEstimateSource};

const TURN_PLAN_SEED_CRITICAL_SURVIVAL_MARGIN: i32 = 6;

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

        let node = match prepare_node_for_expansion(
            &mut loop_state,
            NodePreflightInput {
                node: entry.node,
                started,
                stepper,
                config: &config,
                deadline,
            },
        ) {
            NodePreflightOutcome::Expand(node) => node,
            NodePreflightOutcome::Continue => continue,
            NodePreflightOutcome::Stop => break,
        };

        let Some(mut expansion) = prepare_node_expansion(&mut loop_state, &node, stepper, &config)
        else {
            continue;
        };

        for ordered_choice in expansion.ordered_choices {
            let outcome = expand_ordered_child(
                &mut loop_state,
                &mut expansion.turn_branching,
                &mut expansion.turn_local_dominance,
                ChildExpansionInput {
                    parent: &node,
                    position: &expansion.position,
                    ordered_choice,
                    action_prior_state_hash: expansion.action_prior_state_hash.as_deref(),
                    pending_choice: expansion.pending_choice.as_ref(),
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
            .observe_turn_branching(&expansion.turn_branching);
        loop_state
            .diagnostics
            .observe_turn_local_dominance(&expansion.turn_local_dominance);

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
