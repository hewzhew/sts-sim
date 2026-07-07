use super::*;

mod best_trajectories;
mod bootstrap;
mod child_dominance;
mod child_expansion;
mod child_frontier;
mod child_node;
mod child_preflight;
mod child_rollout;
mod child_step;
mod finalize;
mod finish_coverage;
mod finish_diagnostics;
mod finish_evidence;
mod finish_frontier;
mod finish_outcome;
mod finish_policy;
mod finish_trajectories;
mod loop_state;
mod node_action_ordering;
mod node_action_surface;
mod node_budget;
mod node_child_observers;
mod node_deferred_rollout;
mod node_expansion;
mod node_preflight;
mod node_pruning;
mod node_terminal;
mod rollout_timing;
mod turn_plan_seed_gate;
mod turn_plan_seeding;
mod win_acceptance;

use bootstrap::initialize_root_frontier;
use child_expansion::{expand_ordered_child, ChildExpansionInput, ChildExpansionOutcome};
use finalize::{finish_combat_search_report, SearchFinishInput};
use finish_diagnostics::finish_diagnostics_and_timing;
use loop_state::SearchLoopState;
use node_expansion::prepare_node_expansion;
use node_preflight::{prepare_node_for_expansion, NodePreflightInput, NodePreflightOutcome};
#[cfg(test)]
use turn_plan_seed_gate::{should_seed_turn_plan_at_node, tactical_enemy_turn_plan_seed_gate};

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
    let policy_evidence = combat_search_policy_evidence_for_combat(combat);
    let mut loop_state = SearchLoopState::new(&config);
    let root_for_turn_plan_diagnostics =
        initialize_root_frontier(&mut loop_state, engine, combat, stepper, &config, deadline);

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

    finish_diagnostics_and_timing(
        &mut loop_state,
        started,
        &root_for_turn_plan_diagnostics,
        stepper,
        &config,
    );
    finish_combat_search_report(SearchFinishInput {
        config,
        policy_evidence,
        loop_state,
    })
}

#[cfg(test)]
mod tests;
