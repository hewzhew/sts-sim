//! Breaking V2 control plane for bounded exact-combat contracts.
//!
//! Routine callers receive one compact typed result. Full graph evidence is
//! written to a fresh artifact directory and is never reparsed by summary or
//! rerun; those commands read only the stable V2 manifest.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sts_combat_planner::{LocalTurnGraphWitnessConfig, OracleCombatWitnessSatisfaction};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::run_control::existing_combat_knowledge_policy_v1;
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
mod classification;

use artifact::{load_artifact, reserve_artifact_directory, write_json_create_new};
use classification::{classify_contract, CombatContractResultV2};

const ARTIFACT_SCHEMA: &str = "OracleCombatContractArtifactV2";

#[derive(Debug, Args)]
pub(super) struct ContractCommandArgs {
    #[command(subcommand)]
    command: ContractCommand,
}

#[derive(Debug, Subcommand)]
enum ContractCommand {
    /// Run one bounded rollout-suffix contract from an exact combat root.
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
    /// Re-run the typed request stored in a V2 artifact.
    Rerun(ArtifactPathArgs),
}

#[derive(Debug, Args)]
struct ArtifactPathArgs {
    /// V2 artifact directory or its manifest.json.
    artifact: PathBuf,
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
    witness_actions: Option<PathBuf>,
    result: CombatContractResultV2,
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
        LocalGraphExecutionProfile::from_controls(false, false, true, false, false, None)?;
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
    let report = session.advance(search_spec.quantum(), &EngineCombatStepper);
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
        policy_line: None,
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
    let witness_actions_path =
        candidate_index.map(|_| reservation.final_path.join("witness.actions.json"));
    let mut result = assessment.result;
    result.artifact = manifest_path.clone();
    result.witness_actions = witness_actions_path.clone();
    let artifact = CombatContractArtifactV2 {
        schema_name: ARTIFACT_SCHEMA.to_owned(),
        schema_version: 2,
        request,
        root_exact_state_hash,
        source_content_fingerprint,
        runtime: runtime_identity(),
        report: report_path.clone(),
        witness_actions: witness_actions_path,
        result: result.clone(),
    };
    let persist_result = (|| {
        write_json_create_new(&reservation.staging_path.join("report.json"), &full_report)?;
        if let Some(index) = candidate_index {
            let actions = session.witness_frontier()[index]
                .actions
                .iter()
                .map(|action| &action.input)
                .collect::<Vec<_>>();
            write_json_create_new(
                &reservation.staging_path.join("witness.actions.json"),
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
