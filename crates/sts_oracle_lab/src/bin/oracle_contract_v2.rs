//! Breaking V2 control plane for bounded exact-combat contracts.
//!
//! Routine callers receive one compact typed result. Full graph evidence is
//! written to a fresh artifact directory and is never reparsed by summary or
//! rerun; those commands read only the stable V2 manifest.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sts_combat_planner::{
    LocalTurnGraphDepthServiceSnapshot, LocalTurnGraphServicedStateSnapshot,
    LocalTurnGraphWitnessConfig, OracleCombatWitnessSatisfaction,
};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::run_control::{
    existing_combat_guide_service_bias_v1, existing_combat_knowledge_policy_v1,
};
use sts_oracle_runtime::sim::combat::EngineCombatStepper;

use super::canonical_launch::{runtime_identity, runtime_source_content_fingerprint};
use super::combat_case_performance;
use super::combat_graph_diagnostics::{
    materialize_local_graph_diagnostics, LocalGraphDiagnosticPaths,
};
use super::combat_graph_execution::LocalGraphExecutionProfile;
use super::combat_graph_exports::{
    export_local_graph_paths, LocalGraphExportActions, LocalGraphExportPaths,
};
use super::combat_graph_observation::capture_local_graph_observation;
use super::combat_graph_report::{
    local_graph_full_report, LocalGraphCounterfactual, LocalGraphFullReportOptions,
    LocalGraphReportData, LocalGraphRunIdentity,
};
use super::combat_graph_search_spec::LocalGraphSearchSpec;
use super::oracle_case_catalog_v2::{register_case, resolve_case};
use super::print_json;

mod artifact;
mod artifact_compare;
mod artifact_trace;
mod artifact_turn;
mod classification;

use artifact::{load_artifact, reserve_artifact_directory, write_json_create_new};
use classification::{
    classify_contract, outcome_satisfies_contract, outcome_satisfies_resource_contract,
    CombatContractResultV2,
};

const ARTIFACT_SCHEMA: &str = "OracleCombatContractArtifactV2";
const ARTIFACT_SCHEMA_VERSION: u32 = 7;

#[derive(Debug, Args)]
pub(super) struct ContractCommandArgs {
    #[command(subcommand)]
    command: ContractCommand,
}

#[derive(Debug, Subcommand)]
enum ContractCommand {
    /// Run one bounded exact-frontier contract from an exact combat root.
    Combat(CombatContractRunArgs),
}

#[derive(Debug, Args)]
pub(super) struct ArtifactCommandArgs {
    #[command(subcommand)]
    command: ArtifactCommand,
}

#[derive(Debug, Subcommand)]
enum ArtifactCommand {
    /// Print only the stable compact result from a V2 artifact.
    Summary(ArtifactPathArgs),
    /// Inspect compact exact-search service accounting without parsing report.json.
    Search(ArtifactSearchArgs),
    /// Replay and inspect the selected witness without reading the full report.
    Trace(ArtifactPathArgs),
    /// Replay-compare the contract-aligned and local-HP terminal candidates.
    Compare(ArtifactPathArgs),
    /// Enumerate one exact complete-turn surface along a retained candidate.
    Turn(ArtifactTurnArgs),
    /// Re-run the typed request stored in a V2 artifact.
    Rerun(ArtifactPathArgs),
}

#[derive(Debug, Args)]
struct ArtifactPathArgs {
    /// V2 artifact directory or its manifest.json.
    artifact: PathBuf,
}

#[derive(Debug, Args)]
struct ArtifactSearchArgs {
    /// V2 artifact directory or its manifest.json.
    artifact: PathBuf,
    /// Include a few highest-service exact-state samples per turn depth.
    #[arg(long)]
    states: bool,
    /// Query one retained exact state by its full hash or a unique prefix.
    #[arg(long, conflicts_with = "states")]
    state: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ArtifactCandidateRole {
    Contract,
    LocalHp,
}

#[derive(Debug, Args)]
struct ArtifactTurnArgs {
    /// V2 artifact directory or its manifest.json.
    artifact: PathBuf,
    /// Retained terminal candidate whose exact prefix owns the turn.
    #[arg(long, value_enum, default_value_t = ArtifactCandidateRole::Contract)]
    candidate: ArtifactCandidateRole,
    /// Exact player-turn number shown by `artifact compare`.
    #[arg(long)]
    turn: u32,
    /// Follow one displayed plan index to its exact successor. Repeat to walk
    /// several complete-turn branches without exporting cases or action files.
    #[arg(long)]
    follow_plan: Vec<usize>,
    /// From every candidate on the reached surface, enumerate exactly one more
    /// complete turn and aggregate terminal HP and stolen-gold outcomes.
    #[arg(long)]
    scan_next_terminal: bool,
    #[arg(long, default_value_t = 1_024)]
    max_inner_nodes: usize,
    #[arg(long, default_value_t = 96)]
    max_end_states: usize,
    #[arg(long, default_value_t = 96)]
    per_bucket_limit: usize,
    #[arg(long, default_value_t = 16)]
    limit: usize,
}

#[derive(Clone, Debug, Args, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct CombatContractRunArgs {
    /// Exact CombatCase path. The case is admitted to the V2 catalog.
    #[arg(long, conflicts_with = "case_id", required_unless_present = "case_id")]
    case: Option<PathBuf>,
    /// Exact-root id or one unique prefix from `case list`.
    #[arg(long, conflicts_with = "case", required_unless_present = "case")]
    case_id: Option<String>,
    #[arg(long)]
    min_final_hp: Option<i32>,
    #[arg(long, default_value_t = 0)]
    max_potions_used: u32,
    #[arg(long)]
    require_recovered_stolen_gold: bool,
    #[arg(long, default_value_t = 4_096)]
    generation_work: usize,
    #[arg(long, default_value_t = 2_000)]
    wall_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CombatContractRequestV2 {
    case_id: String,
    case: PathBuf,
    min_final_hp: Option<i32>,
    max_potions_used: u32,
    require_recovered_stolen_gold: bool,
    generation_work: usize,
    wall_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CombatContractArtifactV2 {
    schema_name: String,
    schema_version: u32,
    request: CombatContractRequestV2,
    root_exact_state_hash: String,
    source_content_fingerprint: String,
    runtime: Value,
    report: PathBuf,
    terminal_candidates: Vec<CombatContractTerminalCandidateV2>,
    search: CombatContractSearchDiagnosticsV2,
    result: CombatContractResultV2,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CombatContractTerminalCandidateV2 {
    candidate_id: String,
    frontier_index: usize,
    terminal_exact_state_hash: String,
    selected_by_contract_view: bool,
    satisfies_contract: bool,
    satisfies_resource_contract: bool,
    selected_by_local_hp_view: bool,
    final_hp: i32,
    potions_used: u32,
    unrecovered_stolen_gold: i32,
    action_count: usize,
    negative_log_policy: f64,
    discovery_source: sts_combat_planner::OracleCombatWitnessDiscoverySource,
    actions: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CombatContractSearchDiagnosticsV2 {
    summary: CombatContractSearchSummaryV2,
    serviced_state_samples: Vec<CombatContractSearchStateV2>,
    states: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CombatContractSearchSummaryV2 {
    generation_work: usize,
    plan_prefix_attempts: usize,
    plan_prefix_completed: usize,
    plan_prefix_rejections: usize,
    plan_prefix_root_enqueues: usize,
    plan_prefix_root_services: usize,
    plan_prefix_continuation_enqueues: usize,
    plan_prefix_continuation_services: usize,
    exact_nodes: usize,
    exact_edges: usize,
    completed_turn_options: usize,
    terminal_win_options: usize,
    root_generation_work: usize,
    root_completed_turn_options: usize,
    root_children: usize,
    max_player_turn: u32,
    retained_states: usize,
    depths: Vec<CombatContractDepthServiceV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CombatContractDepthServiceV2 {
    relative_turn_depth: usize,
    exact_states: usize,
    serviced_states: usize,
    generation_work: usize,
    generated_options: usize,
    exact_children: usize,
    retained_generator_work_items: usize,
    exhausted_states: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CombatContractSearchStateV2 {
    exact_state_hash: String,
    relative_turn_depth: usize,
    player_turn: u32,
    player_hp: i32,
    alive_enemy_count: usize,
    enemy_total_hp: i32,
    recoverable_stolen_gold: i32,
    unrecovered_stolen_gold: i32,
    generation_work: usize,
    generated_options: usize,
    exact_children: usize,
    retained_generator_work_items: usize,
    path_action_count: usize,
    anchor_ordinal_rank: Option<usize>,
    anchor_candidate_count: usize,
    proposal_root_ordinal_rank: Option<usize>,
    proposal_root_candidate_count: usize,
    proposal_root_services: usize,
    proposal_continuation_ordinal_rank: Option<usize>,
    proposal_continuation_candidate_count: usize,
    proposal_continuation_services: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CombatContractSearchStateIndexV2 {
    schema_name: String,
    schema_version: u32,
    root_exact_state_hash: String,
    states: Vec<CombatContractSearchStateV2>,
}

pub(super) fn run_contract_command(args: ContractCommandArgs) -> Result<(), String> {
    match args.command {
        ContractCommand::Combat(args) => {
            let case = resolve_case(args.case.as_deref(), args.case_id.as_deref())?;
            let request = CombatContractRequestV2 {
                case_id: case.id,
                case: case.path,
                min_final_hp: args.min_final_hp,
                max_potions_used: args.max_potions_used,
                require_recovered_stolen_gold: args.require_recovered_stolen_gold,
                generation_work: args.generation_work,
                wall_ms: args.wall_ms,
            };
            let result = run_combat_contract(request)?;
            print_json(&result)
        }
    }
}

pub(super) fn run_artifact_command(args: ArtifactCommandArgs) -> Result<(), String> {
    match args.command {
        ArtifactCommand::Summary(args) => {
            let artifact = load_artifact(&args.artifact)?;
            print_json(&artifact.result)
        }
        ArtifactCommand::Search(args) => {
            let artifact = load_artifact(&args.artifact)?;
            if let Some(query) = args.state.as_deref() {
                query_search_state(&artifact, query)
            } else if args.states {
                print_json(&artifact.search)
            } else {
                print_json(&artifact.search.summary)
            }
        }
        ArtifactCommand::Trace(args) => {
            let artifact = load_artifact(&args.artifact)?;
            artifact_trace::run(&args.artifact, &artifact)
        }
        ArtifactCommand::Compare(args) => {
            let artifact = load_artifact(&args.artifact)?;
            artifact_compare::run(&args.artifact, &artifact)
        }
        ArtifactCommand::Turn(args) => {
            let artifact = load_artifact(&args.artifact)?;
            artifact_turn::run(&args, &artifact)
        }
        ArtifactCommand::Rerun(args) => {
            let artifact = load_artifact(&args.artifact)?;
            let current = register_case(&artifact.request.case)?;
            if current.id != artifact.request.case_id {
                return Err(format!(
                    "combat case root drifted: artifact expects {}, current case is {}",
                    artifact.request.case_id, current.id
                ));
            }
            let result = run_combat_contract(artifact.request)?;
            print_json(&result)
        }
    }
}

fn run_combat_contract(request: CombatContractRequestV2) -> Result<CombatContractResultV2, String> {
    if request.generation_work == 0 {
        return Err("--generation-work must be positive".to_owned());
    }
    if request.wall_ms == 0 {
        return Err("--wall-ms must be positive".to_owned());
    }
    let started = Instant::now();
    let loaded = load_combat_case(&request.case)?;
    let root_exact_state_hash =
        combat_exact_state_hash_v2(&loaded.position.engine, &loaded.position.combat);
    if root_exact_state_hash != request.case_id {
        return Err(format!(
            "combat case root drifted: request expects {}, loaded case is {}",
            request.case_id, root_exact_state_hash
        ));
    }
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let root_player_turn = loaded.position.combat.turn.turn_count;
    let satisfaction = request
        .min_final_hp
        .map(OracleCombatWitnessSatisfaction::FinalHpAtLeast)
        .unwrap_or(OracleCombatWitnessSatisfaction::FirstWitness);
    let execution_profile =
        LocalGraphExecutionProfile::from_controls(false, false, false, false, None)?;
    let search_spec = LocalGraphSearchSpec::from_controls(
        request.generation_work,
        1_000_000,
        request.wall_ms,
        250,
        50_000,
        4,
        32,
        Some(request.max_potions_used),
        false,
        None,
        None,
        None,
    );
    let mut config: LocalTurnGraphWitnessConfig = search_spec.planner_config(satisfaction);
    config.guide_service_bias = existing_combat_guide_service_bias_v1(&loaded.position);
    config.require_no_unrecovered_stolen_gold = request.require_recovered_stolen_gold;
    let root = sts_combat_planner::CombatDecisionRoot::new(loaded.position.clone())
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let mut session = execution_profile.prepare_session(
        root,
        root_player_turn,
        config,
        existing_combat_knowledge_policy_v1(),
    );

    let search_started = Instant::now();
    let policy_line_report = session
        .has_supported_initial_plan_prefix()
        .then(|| {
            session.offer_plan_compatible_policy_line(
                6,
                request.generation_work.min(64),
                &EngineCombatStepper,
            )
        })
        .transpose()?;
    let mut search_quantum = search_spec.quantum();
    search_quantum.deadline = Some(search_started + Duration::from_millis(request.wall_ms));
    if let Some(policy_line) = &policy_line_report {
        search_quantum.additional_generation_work = search_quantum
            .additional_generation_work
            .saturating_sub(policy_line.proposed_actions.len());
        search_quantum.additional_engine_steps = search_quantum
            .additional_engine_steps
            .saturating_sub(policy_line.engine_steps);
    }
    let report = session.advance(search_quantum, &EngineCombatStepper);
    let search_elapsed = search_started.elapsed();
    let progress = session.progress_snapshot();
    let diagnostics = materialize_local_graph_diagnostics(
        &session,
        &loaded.position,
        LocalGraphDiagnosticPaths {
            deepest_survival: &progress.deepest_survival_actions,
            deepest_progress: &progress.deepest_progress_actions,
            witness: report
                .witness
                .as_ref()
                .map(|witness| witness.actions.as_slice()),
        },
        false,
        250,
    )?;
    let observation = capture_local_graph_observation(&session, &loaded.position, &[], None);
    let exports = export_local_graph_paths(
        &loaded,
        Some(&request.case),
        LocalGraphExportPaths {
            witness_actions: None,
            deepest_survival_case: None,
            deepest_progress_case: None,
        },
        LocalGraphExportActions {
            witness: None,
            witness_final_position: None,
            deepest_survival: &progress.deepest_survival_actions,
            deepest_progress: &progress.deepest_progress_actions,
        },
        250,
    )?;
    let performance_profile = combat_case_performance::local_graph_performance_report(
        search_elapsed,
        &request.case,
        &report,
    );
    let performance_timing = combat_case_performance::local_graph_performance_timing(&report);
    let mut search = CombatContractSearchDiagnosticsV2 {
        summary: CombatContractSearchSummaryV2 {
            generation_work: report.counters.generation_work,
            plan_prefix_attempts: report.counters.plan_prefix_attempts,
            plan_prefix_completed: report.counters.plan_prefix_completed,
            plan_prefix_rejections: report.counters.plan_prefix_rejections,
            plan_prefix_root_enqueues: report.counters.plan_prefix_root_enqueues,
            plan_prefix_root_services: report.counters.plan_prefix_root_services,
            plan_prefix_continuation_enqueues: report.counters.plan_prefix_continuation_enqueues,
            plan_prefix_continuation_services: report.counters.plan_prefix_continuation_services,
            exact_nodes: report.counters.exact_nodes,
            exact_edges: report.counters.exact_edges,
            completed_turn_options: report.counters.completed_turn_options,
            terminal_win_options: report.counters.terminal_win_options,
            root_generation_work: progress
                .root_state
                .as_ref()
                .map_or(0, |root| root.generator_work),
            root_completed_turn_options: report.root_generated_options,
            root_children: report.root_children,
            max_player_turn: progress.max_player_turn,
            retained_states: progress.retained_states,
            depths: session
                .depth_service_snapshot()
                .into_iter()
                .map(CombatContractDepthServiceV2::from)
                .collect(),
        },
        serviced_state_samples: session
            .serviced_state_samples(4)
            .into_iter()
            .map(CombatContractSearchStateV2::from)
            .collect(),
        states: PathBuf::new(),
    };
    let plan_transition_portfolio = Value::Null;
    let report_data = LocalGraphReportData {
        run: LocalGraphRunIdentity {
            case: &request.case,
            elapsed: started.elapsed(),
            satisfaction,
            execution_profile,
            search_spec,
            counterfactual: LocalGraphCounterfactual {
                full_health: false,
                original_hp: initial_hp,
                search_hp: initial_hp,
            },
        },
        report: &report,
        progress: &progress,
        retained_state_work: session.retained_state_work(),
        storage: session.storage_snapshot(),
        policy_line: policy_line_report.as_ref(),
        plan_transition_annotations: false,
        plan_transition_portfolio: &plan_transition_portfolio,
        diagnostics: &diagnostics,
        observation: &observation,
        exports: &exports,
    };
    let full_report = local_graph_full_report(
        &report_data,
        LocalGraphFullReportOptions {
            action_imitation_artifact: None,
            value_prototype_artifact: None,
            guidance_bundle: None,
            watch_corridor_actions: &[],
            readable: false,
            search_elapsed,
            performance_timing: &performance_timing,
            performance_profile: &performance_profile,
        },
    );

    let source_content_fingerprint = runtime_source_content_fingerprint()?;
    let reservation = reserve_artifact_directory(&root_exact_state_hash)?;
    search.states = reservation.final_path.join("search-states.json");
    let search_state_index = CombatContractSearchStateIndexV2 {
        schema_name: "OracleCombatContractSearchStateIndexV2".to_owned(),
        schema_version: 3,
        root_exact_state_hash: root_exact_state_hash.clone(),
        states: session
            .state_service_index()
            .into_iter()
            .map(CombatContractSearchStateV2::from)
            .collect(),
    };
    let report_path = reservation.final_path.join("report.json");
    let manifest_path = reservation.final_path.join("manifest.json");
    let assessment = classify_contract(
        &request,
        &report,
        session.witness_frontier(),
        &manifest_path,
        started.elapsed(),
    );
    let candidate_index = assessment.selected_witness_index;
    let terminal_candidates = report
        .witness_frontier
        .iter()
        .zip(session.witness_frontier())
        .enumerate()
        .map(|(index, (outcome, witness))| {
            let terminal_exact_state_hash = combat_exact_state_hash_v2(
                &witness.final_position.engine,
                &witness.final_position.combat,
            );
            CombatContractTerminalCandidateV2 {
                candidate_id: format!("frontier-{index:03}"),
                frontier_index: index,
                terminal_exact_state_hash,
                selected_by_contract_view: candidate_index == Some(index),
                satisfies_contract: outcome_satisfies_contract(&request, outcome),
                satisfies_resource_contract: outcome_satisfies_resource_contract(&request, outcome),
                selected_by_local_hp_view: assessment.local_hp_witness_index == Some(index),
                final_hp: outcome.final_hp,
                potions_used: outcome.potion_expenditures,
                unrecovered_stolen_gold: outcome.unrecovered_stolen_gold,
                action_count: outcome.action_count,
                negative_log_policy: outcome.negative_log_policy,
                discovery_source: witness.discovery_source,
                actions: reservation
                    .final_path
                    .join("candidates")
                    .join(format!("frontier-{index:03}.actions.json")),
            }
        })
        .collect::<Vec<_>>();
    let witness_actions_path = candidate_index
        .and_then(|index| terminal_candidates.get(index))
        .map(|candidate| candidate.actions.clone());
    let mut result = assessment.result;
    result.artifact = manifest_path.clone();
    result.witness_actions = witness_actions_path.clone();
    let artifact = CombatContractArtifactV2 {
        schema_name: ARTIFACT_SCHEMA.to_owned(),
        schema_version: ARTIFACT_SCHEMA_VERSION,
        request,
        root_exact_state_hash,
        source_content_fingerprint,
        runtime: runtime_identity(),
        report: report_path.clone(),
        terminal_candidates,
        search,
        result: result.clone(),
    };
    let persist_result = (|| {
        write_json_create_new(&reservation.staging_path.join("report.json"), &full_report)?;
        write_json_create_new(
            &reservation.staging_path.join("search-states.json"),
            &search_state_index,
        )?;
        if !artifact.terminal_candidates.is_empty() {
            fs::create_dir(reservation.staging_path.join("candidates")).map_err(|error| {
                format!(
                    "failed to create candidate sidecar directory '{}': {error}",
                    reservation.staging_path.join("candidates").display()
                )
            })?;
        }
        for candidate in &artifact.terminal_candidates {
            let actions = session.witness_frontier()[candidate.frontier_index]
                .actions
                .iter()
                .map(|action| &action.input)
                .collect::<Vec<_>>();
            write_json_create_new(
                &reservation
                    .staging_path
                    .join("candidates")
                    .join(format!("{}.actions.json", candidate.candidate_id)),
                &actions,
            )?;
        }
        write_json_create_new(&reservation.staging_path.join("manifest.json"), &artifact)?;
        fs::rename(&reservation.staging_path, &reservation.final_path).map_err(|error| {
            format!(
                "failed to publish V2 artifact '{}' atomically: {error}",
                reservation.final_path.display()
            )
        })
    })();
    if let Err(error) = persist_result {
        let _ = fs::remove_dir_all(&reservation.staging_path);
        return Err(error);
    }
    Ok(result)
}

impl From<LocalTurnGraphDepthServiceSnapshot> for CombatContractDepthServiceV2 {
    fn from(snapshot: LocalTurnGraphDepthServiceSnapshot) -> Self {
        Self {
            relative_turn_depth: snapshot.relative_turn_depth,
            exact_states: snapshot.exact_states,
            serviced_states: snapshot.serviced_states,
            generation_work: snapshot.generation_work,
            generated_options: snapshot.generated_options,
            exact_children: snapshot.exact_children,
            retained_generator_work_items: snapshot.retained_generator_work_items,
            exhausted_states: snapshot.exhausted_states,
        }
    }
}

impl From<LocalTurnGraphServicedStateSnapshot> for CombatContractSearchStateV2 {
    fn from(snapshot: LocalTurnGraphServicedStateSnapshot) -> Self {
        Self {
            exact_state_hash: snapshot.exact_state_hash,
            relative_turn_depth: snapshot.relative_turn_depth,
            player_turn: snapshot.player_turn,
            player_hp: snapshot.player_hp,
            alive_enemy_count: snapshot.alive_enemy_count,
            enemy_total_hp: snapshot.enemy_total_hp,
            recoverable_stolen_gold: snapshot.recoverable_stolen_gold,
            unrecovered_stolen_gold: snapshot.unrecovered_stolen_gold,
            generation_work: snapshot.generation_work,
            generated_options: snapshot.generated_options,
            exact_children: snapshot.exact_children,
            retained_generator_work_items: snapshot.retained_generator_work_items,
            path_action_count: snapshot.path_action_count,
            anchor_ordinal_rank: snapshot.anchor_ordinal_rank,
            anchor_candidate_count: snapshot.anchor_candidate_count,
            proposal_root_ordinal_rank: snapshot.proposal_root_ordinal_rank,
            proposal_root_candidate_count: snapshot.proposal_root_candidate_count,
            proposal_root_services: snapshot.proposal_root_services,
            proposal_continuation_ordinal_rank: snapshot.proposal_continuation_ordinal_rank,
            proposal_continuation_candidate_count: snapshot.proposal_continuation_candidate_count,
            proposal_continuation_services: snapshot.proposal_continuation_services,
        }
    }
}

fn query_search_state(artifact: &CombatContractArtifactV2, query: &str) -> Result<(), String> {
    if query.is_empty() {
        return Err("--state must not be empty".to_owned());
    }
    let bytes = fs::read(&artifact.search.states).map_err(|error| {
        format!(
            "failed to read V2 search-state index '{}': {error}",
            artifact.search.states.display()
        )
    })?;
    let index: CombatContractSearchStateIndexV2 =
        serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "invalid V2 search-state index '{}': {error}",
                artifact.search.states.display()
            )
        })?;
    if index.schema_name != "OracleCombatContractSearchStateIndexV2"
        || index.schema_version != 3
        || index.root_exact_state_hash != artifact.root_exact_state_hash
    {
        return Err(format!(
            "search-state index '{}' does not match its V2 artifact",
            artifact.search.states.display()
        ));
    }
    let matches = index
        .states
        .iter()
        .filter(|state| state.exact_state_hash.starts_with(query))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!(
            "search-state prefix '{query}' is ambiguous across {} retained states",
            matches.len()
        ));
    }
    let state = matches.first().copied();
    print_json(&serde_json::json!({
        "schema_name": "OracleCombatContractSearchStateQueryV2",
        "schema_version": 3,
        "root_exact_state_hash": artifact.root_exact_state_hash,
        "query": query,
        "retained": state.is_some(),
        "serviced": state.is_some_and(|state| state.generation_work > 0),
        "state": state,
    }))
}
