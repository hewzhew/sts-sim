use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use serde_json::json;
use sts_combat_planner::{
    AtomicLevinRerooting, AtomicLevinWitnessConfig, AtomicTurnPortfolioConfig,
    AtomicTurnPortfolioEntryReport, AtomicTurnPortfolioQuantum, AtomicTurnPortfolioSession,
    CombatDecisionRoot, LocalTurnGraphWitnessConfig, OracleCombatWitnessSatisfaction,
    PolicyDiscrepancyConfig, PolicyDiscrepancyTurnMacroConfig, TurnOptionGeneratorConfig,
};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::run_control::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
};
use sts_oracle_runtime::sim::combat::EngineCombatStepper;

use super::combat_policy_controls::load_action_imitation_policy;
use super::combat_replay_tools::save_combat_inputs;
use super::{oracle_lab_runtime_identity, print_json};

#[derive(Debug, Args)]
pub(super) struct CombatCaseAtomicTurnPortfolioArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    #[arg(long, alias = "max-transitions", default_value_t = 250_000)]
    max_search_work: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 10_000)]
    uniform_exploration_ppm: u32,
    #[arg(long, default_value_t = 512)]
    initial_boundary_work: usize,
    #[arg(long, default_value_t = 64)]
    boundary_service_work: usize,
    #[arg(long, alias = "suffix-service-transitions", default_value_t = 8_192)]
    suffix_service_work: usize,
    /// One-time equal service granted to every newly exposed exact suffix
    /// before evidence-guided deepening may revisit any suffix.
    #[arg(long, default_value_t = 64)]
    initial_suffix_work: usize,
    /// Reroot an independent policy-discrepancy search at every terminal
    /// portfolio boundary instead of using the atomic Levin suffix.
    #[arg(long)]
    policy_discrepancy_suffix: bool,
    /// Give every exact next-turn successor an independent resumable
    /// local-turn graph. This is mutually exclusive with discrepancy
    /// suffixes and is the coherent root-successor service control.
    #[arg(long, conflicts_with = "policy_discrepancy_suffix")]
    local_turn_graph_suffix: bool,
    /// Add the existing bounded rollout evaluator to local-turn suffixes.
    #[arg(long, requires = "local_turn_graph_suffix")]
    suffix_rollout_lookahead: bool,
    /// Complete-turn work reserved by each independent discrepancy suffix.
    #[arg(long, default_value_t = 4_096)]
    suffix_turn_macro_transitions: usize,
    #[arg(long, default_value_t = 1)]
    boundary_layers: usize,
    #[arg(long, default_value_t = 65_536)]
    terminal_work_per_boundary_batch: usize,
    #[arg(long)]
    suffix_reroot_player_turn_boundaries: bool,
    /// Include every live task in the JSON report. Off by default because
    /// the task table grows with each exposed turn layer.
    #[arg(long)]
    include_task_entries: bool,
    /// Include full opaque guide vectors in the live task table.
    #[arg(long)]
    include_task_guides: bool,
    /// Report exact service and scheduler ranks only for these state
    /// hashes, without materializing the complete task table.
    #[arg(long)]
    watch_state_hash: Vec<String>,
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
}

pub(super) fn run(args: CombatCaseAtomicTurnPortfolioArgs) -> Result<(), String> {
    let CombatCaseAtomicTurnPortfolioArgs {
        case,
        action_imitation_artifact,
        max_search_work,
        wall_ms,
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        initial_boundary_work,
        boundary_service_work,
        suffix_service_work,
        initial_suffix_work,
        policy_discrepancy_suffix,
        local_turn_graph_suffix,
        suffix_rollout_lookahead,
        suffix_turn_macro_transitions,
        boundary_layers,
        terminal_work_per_boundary_batch,
        suffix_reroot_player_turn_boundaries,
        include_task_entries,
        include_task_guides,
        watch_state_hash,
        export_witness_actions,
    } = args;
    let command_started = Instant::now();
    let case_path = case.clone();
    let case = load_combat_case(&case)?;
    let root = CombatDecisionRoot::new(case.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let initial_hp = root.position().combat.entities.player.current_hp;
    let boundary_policy = existing_combat_knowledge_policy_v1();
    let suffix_policy = action_imitation_artifact
        .as_deref()
        .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
        .transpose()?
        .unwrap_or_else(existing_combat_knowledge_policy_v1);
    let boundary_config = TurnOptionGeneratorConfig {
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        ..TurnOptionGeneratorConfig::default()
    };
    let suffix_config = AtomicLevinWitnessConfig {
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        ..AtomicLevinWitnessConfig::default()
    };
    let portfolio_config = AtomicTurnPortfolioConfig {
        boundary_search: boundary_config,
        suffix_search: AtomicLevinWitnessConfig {
            rerooting: if suffix_reroot_player_turn_boundaries {
                AtomicLevinRerooting::PlayerTurnBoundaries
            } else {
                AtomicLevinRerooting::Disabled
            },
            ..suffix_config
        },
        initial_boundary_work,
        boundary_service_work,
        suffix_service_work,
        initial_suffix_work,
        boundary_layers,
        terminal_work_per_boundary_batch,
    };
    let mut portfolio = if policy_discrepancy_suffix {
        AtomicTurnPortfolioSession::with_policy_discrepancy_suffix(
            root,
            portfolio_config,
            PolicyDiscrepancyConfig {
                max_engine_steps_per_transition,
                uniform_exploration_ppm,
                turn_macro: (suffix_turn_macro_transitions > 0).then_some(
                    PolicyDiscrepancyTurnMacroConfig {
                        max_applied_transitions: suffix_turn_macro_transitions,
                        ..PolicyDiscrepancyTurnMacroConfig::default()
                    },
                ),
                ..PolicyDiscrepancyConfig::default()
            },
            boundary_policy,
            suffix_policy,
        )
    } else if local_turn_graph_suffix {
        AtomicTurnPortfolioSession::with_local_turn_graph_suffix(
            root,
            portfolio_config,
            LocalTurnGraphWitnessConfig {
                generator: boundary_config,
                satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
                ..LocalTurnGraphWitnessConfig::default()
            },
            boundary_policy,
            suffix_policy,
            suffix_rollout_lookahead.then(existing_combat_rollout_lookahead_v1),
        )
    } else {
        AtomicTurnPortfolioSession::with_policies(
            root,
            portfolio_config,
            boundary_policy,
            suffix_policy,
        )
    };
    let started = Instant::now();
    let report = portfolio.advance(
        &EngineCombatStepper,
        AtomicTurnPortfolioQuantum {
            additional_search_work: max_search_work,
            additional_engine_steps: max_search_work
                .saturating_mul(max_engine_steps_per_transition),
            deadline: Some(started + Duration::from_millis(wall_ms)),
        },
    );
    let elapsed = started.elapsed();
    if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), report.witness.as_ref())
    {
        save_combat_inputs(
            path,
            witness.actions.iter().map(|action| action.input.clone()),
        )?;
    }
    let task_anchor_key = |entry: &AtomicTurnPortfolioEntryReport| {
        let next_quantum = if entry.remaining_boundary_layers > 0 {
            boundary_service_work.min(entry.boundary_guides.len().saturating_add(1))
        } else {
            suffix_service_work
        }
        .max(1);
        entry.prefix_negative_log_policy
            + (entry.scheduler_work.saturating_add(next_quantum).max(1) as f64).ln()
    };
    let task_entries = include_task_entries.then(|| {
        report
            .suffix_entries
            .iter()
            .map(|entry| {
                let mut value = json!({
                    "boundary_id": entry.boundary_id,
                    "exact_state_hash": entry.exact_state_hash,
                    "prefix_action_count": entry.prefix_action_count,
                    "prefix_negative_log_policy": entry.prefix_negative_log_policy,
                    "scheduler_work": entry.scheduler_work,
                    "services": entry.services,
                    "boundary_generation_work": entry.boundary_generation_work,
                    "terminal_search_work": entry.terminal_search_work,
                    "applied_action_transitions": entry.applied_action_transitions,
                    "engine_steps": entry.engine_steps,
                    "remaining_boundary_layers": entry.remaining_boundary_layers,
                    "task_kind": format!("{:?}", entry.task_kind),
                    "recursive_active_tasks": entry.recursive_active_tasks,
                    "recursive_unique_exact_states": entry.recursive_unique_exact_states,
                    "recursive_duplicate_exact_states": entry.recursive_duplicate_exact_states,
                    "maximum_portfolio_depth": entry.maximum_portfolio_depth,
                });
                if include_task_guides {
                    let object = value.as_object_mut().expect("task entry is an object");
                    object.insert(
                        "boundary_guides".to_string(),
                        json!(entry
                            .boundary_guides
                            .iter()
                            .map(|guide| json!({
                                "lane": guide.lane,
                                "components": guide.components,
                            }))
                            .collect::<Vec<_>>()),
                    );
                }
                value
            })
            .collect::<Vec<_>>()
    });
    let watched_tasks = report
        .suffix_entries
        .iter()
        .filter(|entry| watch_state_hash.contains(&entry.exact_state_hash))
        .map(|entry| {
            let boundary_class = entry.remaining_boundary_layers > 0;
            let anchor_key = task_anchor_key(entry);
            let anchor_rank = 1 + report
                .suffix_entries
                .iter()
                .filter(|other| {
                    (other.remaining_boundary_layers > 0) == boundary_class
                        && (task_anchor_key(other).total_cmp(&anchor_key).is_lt()
                            || (task_anchor_key(other).total_cmp(&anchor_key).is_eq()
                                && other.boundary_id < entry.boundary_id))
                })
                .count();
            let guide_ranks = entry
                .boundary_guides
                .iter()
                .map(|guide| {
                    let rank = 1 + report
                        .suffix_entries
                        .iter()
                        .filter(|other| {
                            if (other.remaining_boundary_layers > 0) != boundary_class {
                                return false;
                            }
                            let Some(other_guide) = other
                                .boundary_guides
                                .iter()
                                .find(|other_guide| other_guide.lane == guide.lane)
                            else {
                                return false;
                            };
                            other_guide.components > guide.components
                                || (other_guide.components == guide.components
                                    && (task_anchor_key(other).total_cmp(&anchor_key).is_lt()
                                        || (task_anchor_key(other).total_cmp(&anchor_key).is_eq()
                                            && other.boundary_id < entry.boundary_id)))
                        })
                        .count();
                    json!({
                        "lane": guide.lane,
                        "rank": rank,
                    })
                })
                .collect::<Vec<_>>();
            let mut value = json!({
                "boundary_id": entry.boundary_id,
                "exact_state_hash": entry.exact_state_hash,
                "prefix_action_count": entry.prefix_action_count,
                "prefix_negative_log_policy": entry.prefix_negative_log_policy,
                "scheduler_work": entry.scheduler_work,
                "services": entry.services,
                "boundary_generation_work": entry.boundary_generation_work,
                "terminal_search_work": entry.terminal_search_work,
                "applied_action_transitions": entry.applied_action_transitions,
                "engine_steps": entry.engine_steps,
                "remaining_boundary_layers": entry.remaining_boundary_layers,
                "task_kind": format!("{:?}", entry.task_kind),
                "recursive_active_tasks": entry.recursive_active_tasks,
                "recursive_unique_exact_states": entry.recursive_unique_exact_states,
                "recursive_duplicate_exact_states": entry.recursive_duplicate_exact_states,
                "maximum_portfolio_depth": entry.maximum_portfolio_depth,
                "anchor_rank": anchor_rank,
                "guide_ranks": guide_ranks,
            });
            if include_task_guides {
                let object = value.as_object_mut().expect("task entry is an object");
                object.insert(
                    "boundary_guides".to_string(),
                    json!(entry
                        .boundary_guides
                        .iter()
                        .map(|guide| json!({
                            "lane": guide.lane,
                            "components": guide.components,
                        }))
                        .collect::<Vec<_>>()),
                );
            }
            value
        })
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema_name": "OracleCombatCaseAtomicTurnPortfolioV6",
        "schema_version": 6,
        "case": case_path,
        "runtime": oracle_lab_runtime_identity(),
        "mode": {
            "search": "turn_boundary_atomic_suffix_portfolio",
            "boundary_worker": "exact_multi_guide_turn_generator",
            "boundary_policy": "existing_combat_knowledge_v1",
            "suffix_action_imitation_artifact": action_imitation_artifact,
            "suffix_search": if policy_discrepancy_suffix {
                "policy_discrepancy"
            } else if local_turn_graph_suffix && suffix_rollout_lookahead {
                "local_turn_graph_with_rollout_lookahead"
            } else if local_turn_graph_suffix {
                "local_turn_graph"
            } else {
                "atomic_levin"
            },
            "suffix_rerooting": suffix_reroot_player_turn_boundaries,
            "v2_rollout_lookahead": suffix_rollout_lookahead,
            "task_entries_included": include_task_entries,
            "task_guides_included": include_task_guides,
            "v2_donor": false,
        },
        "status": format!("{:?}", report.status),
        "timing_ms": {
            "setup": started.duration_since(command_started).as_millis(),
            "search": elapsed.as_millis(),
            "total_before_print": command_started.elapsed().as_millis(),
        },
        "budget": {
            "max_search_work": max_search_work,
            "wall_ms": wall_ms,
            "boundary_service_work": boundary_service_work,
            "initial_boundary_work": initial_boundary_work,
            "suffix_service_work": suffix_service_work,
            "initial_suffix_work": initial_suffix_work,
            "suffix_turn_macro_transitions": policy_discrepancy_suffix
                .then_some(suffix_turn_macro_transitions),
            "boundary_layers": boundary_layers,
            "terminal_work_per_boundary_batch": terminal_work_per_boundary_batch,
        },
        "work": {
            "services": report.after.services,
            "boundary_services": report.after.boundary_services,
            "suffix_services": report.after.suffix_services,
            "suffix_initial_services": report.after.suffix_initial_services,
            "boundary_generation_work": report.after.boundary_generation_work,
            "terminal_search_work": report.after.terminal_search_work,
            "charged_search_work": report.after.charged_search_work,
            "applied_action_transitions": report.after.applied_action_transitions,
            "engine_steps": report.after.engine_steps,
            "turn_boundaries_found": report.after.turn_boundaries_found,
            "suffix_sessions_started": report.after.suffix_sessions_started,
            "suffix_sessions_exhausted": report.after.suffix_sessions_exhausted,
            "suffix_sessions_mechanics_gap": report.after.suffix_sessions_mechanics_gap,
            "invalid_boundary_roots": report.after.invalid_boundary_roots,
            "duplicate_boundary_successors": report.after.duplicate_boundary_successors,
            "anchor_view_services": report.after.anchor_view_services,
            "guide_view_services": report.after.guide_view_services,
            "active_suffix_sessions": report.active_suffix_sessions,
            "active_boundary_tasks": report.active_boundary_tasks,
            "active_terminal_tasks": report.active_terminal_tasks,
            "recursive_active_tasks": report.recursive_active_tasks,
            "recursive_unique_exact_states": report.recursive_unique_exact_states,
            "recursive_duplicate_exact_states": report.recursive_duplicate_exact_states,
            "recursive_boundary_tasks": report.recursive_boundary_tasks,
            "recursive_terminal_tasks": report.recursive_terminal_tasks,
            "maximum_portfolio_depth": report.maximum_portfolio_depth,
            "boundary_generator_active": report.boundary_generator_active,
            "root_exact_state_hash": report.root_exact_state_hash,
            "winning_boundary_id": report.winning_boundary_id,
            "winning_boundary_exact_state_hash": report.winning_boundary_exact_state_hash,
            "suffix_entries": task_entries,
            "watched_tasks": watched_tasks,
        },
        "exported_witness_actions": report.witness.is_some()
            .then_some(export_witness_actions.as_ref())
            .flatten(),
        "witness": report.witness.as_ref().map(|witness| json!({
            "final_hp": witness.final_position.combat.entities.player.current_hp,
            "hp_loss": initial_hp.saturating_sub(
                witness.final_position.combat.entities.player.current_hp,
            ),
            "action_count": witness.actions.len(),
            "negative_log_policy": witness.negative_log_policy,
            "replay_engine_steps": witness.replay_engine_steps,
        })),
    }))
}
