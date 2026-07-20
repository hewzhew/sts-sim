use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use sts_combat_planner::{
    CombatDecisionRoot, CombatPlanningQuantum, OracleCombatWitnessConfig,
    OracleCombatWitnessQuantum, OracleCombatWitnessSatisfaction, OracleCombatWitnessSession,
    TurnOptionGenerationStatus, TurnOptionGeneratorConfig, TurnOptionGeneratorSession,
};
use sts_simulator::eval::combat_case::load_combat_case;
use sts_simulator::eval::run_control::{
    existing_combat_knowledge_policy_v1, OracleAnalysisAdvanceRequestV1,
};
use sts_simulator::runtime::branch::{
    call_oracle_analysis_tcp_v1, load_oracle_analysis_workspace_v1,
    load_oracle_run_continuation_v1, save_oracle_analysis_workspace_v1,
    serve_oracle_analysis_jsonl_v1, serve_oracle_analysis_tcp_v1, OracleAnalysisWorkspaceV1,
    OracleRunBudget, OracleRunConfig,
};
use sts_simulator::sim::combat::{CombatStepLimits, CombatStepper, EngineCombatStepper};
use sts_simulator::state::core::ClientInput;

#[derive(Debug, Parser)]
#[command(
    name = "oracle_lab",
    about = "Inspect and steer exact oracle-run variations without editing checkpoints"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start a new A0-style oracle analysis workspace at Neow.
    New {
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0)]
        ascension: u8,
        #[arg(long)]
        workspace: PathBuf,
        #[command(flatten)]
        budget: BudgetArgs,
    },
    /// Import the exact selected state from an oracle_run continuation.
    Import {
        #[arg(long)]
        continuation: PathBuf,
        #[arg(long)]
        workspace: PathBuf,
        #[command(flatten)]
        budget: BudgetArgs,
    },
    /// Run the production oracle combat planner directly on one exact case.
    CombatCase {
        #[arg(long)]
        case: PathBuf,
        #[arg(long, default_value_t = 250_000)]
        max_nodes: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long)]
        watch_state_hash: Option<String>,
        /// Replay this exact legal input prefix before starting the planner.
        #[arg(long)]
        prefix_actions: Option<PathBuf>,
    },
    /// Check when one exact complete-turn action sequence is generated.
    TurnMembership {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        actions: PathBuf,
        #[arg(long, default_value_t = 100_000)]
        max_work: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 8)]
        quantum_work: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// View the current cursor or another exact analysis node.
    View {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
    },
    /// List every materialized variation and its edges.
    Tree {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Create a child variation from an exact choice reference returned by view.
    Try {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        choice_ref: String,
    },
    /// Move the analysis cursor to an existing node.
    Focus {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: usize,
    },
    /// Follow one already materialized child edge from the current cursor.
    Follow {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        edge: u64,
    },
    /// Return to the parent variation used to reach the current cursor.
    Back {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Mark the current variation as the preferred mainline without deleting siblings.
    Promote {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Incrementally search the combat at the current cursor.
    Advance {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long, default_value_t = 32)]
        max_quanta: usize,
        #[arg(long, default_value_t = 50_000)]
        quantum_nodes: usize,
        #[arg(long, default_value_t = 1_000)]
        quantum_ms: u64,
        #[arg(long)]
        wall_ms: Option<u64>,
    },
    /// Accept the current combat's already verified incumbent.
    AcceptCombat {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Restart tactical search from the cursor's unchanged exact combat state.
    RestartCombat {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Print the strategic replay attached to a node.
    History {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Keep one analysis workspace resident and accept JSONL commands on stdin.
    Serve {
        #[arg(long)]
        workspace: PathBuf,
        /// Bind a persistent loopback endpoint instead of reading stdin.
        #[arg(long)]
        listen: Option<SocketAddr>,
        /// Write connection metadata for `oracle_lab call`.
        #[arg(long, requires = "listen")]
        endpoint: Option<PathBuf>,
    },
    /// Send one JSON command to a resident loopback service.
    Call {
        #[arg(long)]
        endpoint: PathBuf,
        #[arg(long)]
        request: String,
    },
}

#[derive(Clone, Copy, Debug, Args)]
struct BudgetArgs {
    #[arg(long, default_value_t = 250_000)]
    hallway_nodes: usize,
    #[arg(long, default_value_t = 5_000)]
    hallway_ms: u64,
    #[arg(long, default_value_t = 750_000)]
    elite_nodes: usize,
    #[arg(long, default_value_t = 15_000)]
    elite_ms: u64,
    #[arg(long, default_value_t = 2_000_000)]
    boss_nodes: usize,
    #[arg(long, default_value_t = 30_000)]
    boss_ms: u64,
}

impl BudgetArgs {
    fn into_budget(self) -> OracleRunBudget {
        OracleRunBudget {
            hallway_nodes: self.hallway_nodes,
            hallway_ms: self.hallway_ms,
            elite_nodes: self.elite_nodes,
            elite_ms: self.elite_ms,
            boss_nodes: self.boss_nodes,
            boss_ms: self.boss_ms,
            ..OracleRunBudget::default()
        }
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AdvanceOutput<T, U> {
    report: T,
    view: U,
}

fn main() -> Result<(), String> {
    match Cli::parse().command {
        Command::New {
            seed,
            ascension,
            workspace,
            budget,
        } => {
            let analysis = OracleAnalysisWorkspaceV1::new(OracleRunConfig {
                seed,
                ascension,
                budget: budget.into_budget(),
            })?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Import {
            continuation,
            workspace,
            budget,
        } => {
            let continuation = load_oracle_run_continuation_v1(&continuation)?;
            let config = OracleRunConfig {
                seed: continuation.seed,
                ascension: continuation.ascension,
                budget: budget.into_budget(),
            };
            let analysis = OracleAnalysisWorkspaceV1::from_continuation(config, continuation)?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::CombatCase {
            case,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            watch_state_hash,
            prefix_actions,
        } => {
            let case = load_combat_case(&case)?;
            let stepper = EngineCombatStepper;
            let mut position = case.position;
            let prefix = prefix_actions
                .as_ref()
                .map(|path| {
                    serde_json::from_slice::<Vec<ClientInput>>(
                        &std::fs::read(path).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| format!("invalid prefix action list: {error}"))
                })
                .transpose()?
                .unwrap_or_default();
            for (action_index, input) in prefix.iter().enumerate() {
                if stepper.choice_for_legal_input(&position, input).is_none() {
                    return Err(format!(
                        "combat prefix action {action_index} is not legal at its exact state: {input:?}"
                    ));
                }
                let step = stepper.apply_to_stable(
                    &position,
                    input.clone(),
                    CombatStepLimits {
                        max_engine_steps: max_engine_steps_per_transition,
                        deadline: None,
                    },
                );
                if step.truncated {
                    return Err(format!(
                        "combat prefix action {action_index} exceeded the engine-step limit"
                    ));
                }
                position = step.position;
            }
            let root = CombatDecisionRoot::new(position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let initial_hp = root.position().combat.entities.player.current_hp;
            let mut search = OracleCombatWitnessSession::with_policy(
                root,
                OracleCombatWitnessConfig {
                    generator: TurnOptionGeneratorConfig {
                        max_engine_steps_per_transition,
                        ..TurnOptionGeneratorConfig::default()
                    },
                    generation_work_per_agenda_pop: 4,
                    satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
                },
                existing_combat_knowledge_policy_v1(),
            );
            let started = Instant::now();
            let report = search.advance(
                &EngineCombatStepper,
                OracleCombatWitnessQuantum {
                    additional_agenda_pops: max_nodes,
                    additional_generation_work: max_nodes,
                    additional_engine_steps: max_nodes
                        .saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(started + Duration::from_millis(wall_ms)),
                },
            );
            let progress = search.progress_snapshot();
            let watched_state = watch_state_hash
                .as_deref()
                .and_then(|hash| search.state_progress_by_exact_hash(hash));
            let witness = report.witness.as_ref();
            print_json(&serde_json::json!({
                "schema_name": "OracleCombatCaseProbeV1",
                "schema_version": 1,
                "status": format!("{:?}", report.status),
                "elapsed_ms": started.elapsed().as_millis(),
                "budget": {
                    "max_nodes": max_nodes,
                    "wall_ms": wall_ms,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                },
                "prefix": {
                    "action_count": prefix.len(),
                    "actions": prefix,
                },
                "counters": {
                    "agenda_pops": report.after.agenda_pops,
                    "generation_work": report.after.generation_work,
                    "engine_steps": report.after.engine_steps,
                    "exact_states": report.after.exact_states,
                    "applied_action_transitions": report.after.applied_action_transitions,
                    "unique_successor_states": report.after.unique_successor_states,
                    "duplicate_exact_successors": report.after.duplicate_exact_successors,
                    "completed_turn_options": report.after.completed_turn_options,
                    "policy_witness_proposals": report.after.policy_witness_proposals,
                },
                "progress": {
                    "retained_states": progress.retained_states,
                    "queued_anchor_entries": progress.queued_anchor_entries,
                    "queued_guided_entries": progress.queued_guided_entries,
                    "max_player_turn": progress.max_player_turn,
                    "deepest_survival_state": progress.deepest_survival_state,
                    "deepest_survival_actions": progress.deepest_survival_actions,
                    "deepest_progress_state": progress.deepest_progress_state,
                    "deepest_progress_actions": progress.deepest_progress_actions,
                    "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
                    "max_path_atomic_depth": progress.max_path_atomic_depth,
                    "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
                    "generation_gap_count": progress.generation_gap_count,
                    "root_state": progress.root_state,
                    "watched_state": watched_state,
                },
                "witness": witness.map(|witness| serde_json::json!({
                    "final_hp": witness.final_position.combat.entities.player.current_hp,
                    "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                    "action_count": witness.actions.len(),
                    "negative_log_policy": witness.negative_log_policy,
                    "actions": witness.actions,
                })),
            }))
        }
        Command::TurnMembership {
            case,
            actions,
            max_work,
            wall_ms,
            quantum_work,
            max_engine_steps_per_transition,
        } => {
            let case = load_combat_case(&case)?;
            let target: Vec<ClientInput> = serde_json::from_slice(
                &std::fs::read(&actions).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("invalid target action list: {error}"))?;
            let root = CombatDecisionRoot::new(case.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let mut generator = TurnOptionGeneratorSession::with_policy(
                root,
                TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                existing_combat_knowledge_policy_v1(),
            );
            let started = Instant::now();
            let deadline = started + Duration::from_millis(wall_ms);
            let mut scanned_options = 0usize;
            let mut matched = None;
            let mut last_status = TurnOptionGenerationStatus::Partial(
                sts_combat_planner::GenerationInterruption::GenerationWorkBudget,
            );
            while generator.counters().generation_work < max_work
                && !generator.is_finished()
                && Instant::now() < deadline
            {
                let remaining = max_work.saturating_sub(generator.counters().generation_work);
                let work = quantum_work.max(1).min(remaining);
                let report = generator.advance(
                    &EngineCombatStepper,
                    CombatPlanningQuantum {
                        additional_generation_work: work,
                        additional_engine_steps: work
                            .saturating_mul(max_engine_steps_per_transition),
                        deadline: Some(deadline),
                    },
                );
                last_status = report.status;
                for option in &generator.completed_options()[scanned_options..] {
                    if option.actions().len() == target.len()
                        && option
                            .actions()
                            .iter()
                            .zip(&target)
                            .all(|(actual, expected)| actual.input == *expected)
                    {
                        matched = Some(serde_json::json!({
                            "generation_work": report.after.generation_work,
                            "engine_steps": report.after.engine_steps,
                            "elapsed_ms": started.elapsed().as_millis(),
                            "boundary": format!("{:?}", option.boundary()),
                            "successor_exact_state_hash": option.exact_successor_hash(),
                            "negative_log_policy": option.negative_log_policy(),
                        }));
                        break;
                    }
                }
                scanned_options = generator.completed_options().len();
                if matched.is_some() {
                    break;
                }
            }
            let counters = generator.counters();
            print_json(&serde_json::json!({
                "schema_name": "OracleTurnMembershipProbeV1",
                "schema_version": 1,
                "matched": matched.is_some(),
                "match": matched,
                "target_action_count": target.len(),
                "status": format!("{:?}", last_status),
                "elapsed_ms": started.elapsed().as_millis(),
                "generation_work": counters.generation_work,
                "engine_steps": counters.engine_steps,
                "completed_turn_options": generator.completed_options().len(),
                "retained_work_items": generator.retained_work_items(),
                "finished": generator.is_finished(),
            }))
        }
        Command::View { workspace, node } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = if let Some(node) = node {
                analysis.session.view_node(node)?
            } else {
                analysis.view()?
            };
            print_json(&view)
        }
        Command::Tree { workspace } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            print_json(&analysis.session.tree())
        }
        Command::Try {
            workspace,
            choice_ref,
        } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = analysis.try_choice(&choice_ref)?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Focus { workspace, node } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            analysis.session.focus_node(node)?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Follow { workspace, edge } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            analysis.session.follow_edge(edge)?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Back { workspace } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            analysis.session.back()?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Promote { workspace } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            analysis.session.promote_cursor();
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Advance {
            workspace,
            max_quanta,
            quantum_nodes,
            quantum_ms,
            wall_ms,
        } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let (report, view) = analysis.advance(OracleAnalysisAdvanceRequestV1 {
                max_quanta,
                quantum_nodes,
                quantum_ms: Some(quantum_ms),
                wall_ms,
            })?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&AdvanceOutput { report, view })
        }
        Command::AcceptCombat { workspace } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = analysis.accept_combat_incumbent()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::RestartCombat { workspace } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            analysis.session.restart_cursor_combat_search()?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::History { workspace, node } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            print_json(&analysis.session.replay(node)?)
        }
        Command::Serve {
            workspace,
            listen,
            endpoint,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            if let Some(listen) = listen {
                let endpoint = endpoint
                    .ok_or_else(|| "oracle_lab serve --listen requires --endpoint".to_string())?;
                serve_oracle_analysis_tcp_v1(&workspace, analysis, listen, &endpoint)?;
            } else {
                if endpoint.is_some() {
                    return Err("oracle_lab serve --endpoint requires --listen".to_string());
                }
                let stdin = io::stdin();
                let stdout = io::stdout();
                serve_oracle_analysis_jsonl_v1(&workspace, analysis, stdin.lock(), stdout.lock())?;
            }
            Ok(())
        }
        Command::Call { endpoint, request } => {
            print_json(&call_oracle_analysis_tcp_v1(&endpoint, &request)?)
        }
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize oracle_lab output: {error}"))?
    );
    Ok(())
}
