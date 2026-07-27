//! Heavy offline and exact-search command frontend for the dedicated oracle runtime.

mod action_reanalysis_policy;
mod action_reanalysis_queue;
mod action_successor_reanalysis;
mod atomic_policy_searches;
mod boundary_successor_corpus;
mod boundary_successor_lookahead;
mod canonical_launch;
mod combat_case_atomic_turn_portfolio;
mod combat_case_fold_solved_suffix;
mod combat_case_layered;
mod combat_case_layered_window_race;
mod combat_case_legacy_global;
mod combat_case_local_graph;
mod combat_case_performance;
mod combat_plan_diagnostics;
mod combat_planning_view;
mod combat_policy_controls;
mod combat_replay_tools;
mod combat_trace_view;
mod depth_beam_audits;
mod exact_combat_evidence;
mod exact_turn_corridor;
mod guidance_artifact_commands;
mod oracle_cli;
mod oracle_seed_panel;
mod policy_discrepancy_search;
mod run_witness_commands;
mod run_witness_suite;
mod turn_audits;
mod turn_membership_audit;
mod v2_capability_audit;
mod workspace_commands;
mod workspace_view;

use canonical_launch::{
    runtime_identity as oracle_lab_runtime_identity, source_content_fingerprint,
};

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use exact_turn_corridor::{
    load as load_exact_turn_corridor, load_action_segments as load_combat_action_segments,
    load_corpus as load_combat_action_imitation_corpus,
    typed_feature_components as typed_combat_feature_components, ShadowCorridorGuide,
};
use guidance_artifact_commands::{load_value_prototype, save_value_prototype};
use oracle_cli::Command;
use serde::Serialize;
use serde_json::{json, Value};
use sts_combat_planner::{
    combat_plan_state_guide_policy_v1, rank_layered_combat_lineage_parents, CombatDecisionRoot,
    LayeredCombatCandidateRaceConfig, LayeredCombatCandidateRaceSession,
    LayeredCombatLineagePortfolioConfig, LayeredCombatLineagePortfolioSession,
    LayeredCombatWitnessConfig, LayeredCombatWitnessQuantum, LayeredCombatWitnessSession,
    LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession,
    OracleCombatWitnessConfig, OracleCombatWitnessQuantum, OracleCombatWitnessSatisfaction,
    OracleCombatWitnessSession, TurnOptionAction, TurnOptionGeneratorConfig,
};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::combat_guidance_bundle::{
    combat_value_prototype_policy_v1, CombatGuidanceBundleV1, CombatValuePrototypeArtifactV1,
};
use sts_oracle_runtime::eval::run_control::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
    ExistingCombatKnowledgeAdvisorAdvanceV1, ExistingCombatKnowledgeAdvisorV1,
};
use sts_oracle_runtime::runtime::branch::{
    load_oracle_run_continuation_v1, save_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceV1,
    OracleRunConfig,
};
use sts_oracle_runtime::sim::combat::{CombatStepLimits, CombatStepper, EngineCombatStepper};
use sts_oracle_runtime::state::core::{ClientInput, EngineState};

fn main() -> Result<(), String> {
    let (canonical_oracle, command) = oracle_cli::parse();
    canonical_launch::validate(canonical_oracle)?;
    match command {
        Command::SeedPanel(args) => print_json(&oracle_seed_panel::run(args)?),
        Command::New {
            seed,
            ascension,
            workspace,
            combat_guidance_bundle,
            budget,
        } => {
            let guidance = combat_guidance_bundle
                .as_deref()
                .map(CombatGuidanceBundleV1::load)
                .transpose()?;
            let analysis = OracleAnalysisWorkspaceV1::new_with_combat_guidance(
                OracleRunConfig {
                    seed,
                    ascension,
                    budget: budget.into_budget(),
                },
                guidance,
            )?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::Import {
            continuation,
            workspace,
            branch_id,
            combat_guidance_bundle,
            budget,
        } => {
            let continuation = load_oracle_run_continuation_v1(&continuation)?;
            let guidance = combat_guidance_bundle
                .as_deref()
                .map(CombatGuidanceBundleV1::load)
                .transpose()?;
            let config = OracleRunConfig {
                seed: continuation.seed,
                ascension: continuation.ascension,
                budget: budget.into_budget(),
            };
            let analysis = match branch_id {
                Some(branch_id) => {
                    OracleAnalysisWorkspaceV1::from_continuation_branch_with_combat_guidance(
                        config,
                        continuation,
                        branch_id,
                        guidance,
                    )?
                }
                None => OracleAnalysisWorkspaceV1::from_continuation_with_combat_guidance(
                    config,
                    continuation,
                    guidance,
                )?,
            };
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::ExportContinuation {
            workspace,
            node,
            output,
        } => print_json(&run_witness_commands::export_continuation(
            &workspace, node, &output,
        )?),
        Command::RecoverCombatCase {
            workspace,
            branch,
            output,
        } => print_json(&run_witness_commands::recover_combat_case(
            &workspace, branch, &output,
        )?),
        Command::VerifyRunWitness { workspace, node } => {
            print_json(&run_witness_commands::verify(&workspace, node)?)
        }
        Command::AuditRunWitnessPolicy {
            workspace,
            node,
            details,
        } => print_json(&run_witness_commands::audit_policy(
            &workspace, node, details,
        )?),
        Command::VerifyRunWitnessSuite { args } => {
            print_json(&run_witness_suite::verify_run_witness_suite(args)?)
        }
        Command::SpliceCombatWitness {
            workspace,
            node,
            journal_entry,
            replacement_workspace,
            replacement_node,
            output,
        } => print_json(&run_witness_commands::splice_combat(
            &workspace,
            node,
            journal_entry,
            &replacement_workspace,
            replacement_node,
            &output,
        )?),
        Command::ExportHistoricalCombatWitness {
            workspace,
            node,
            journal_entry,
            case_output,
            actions_output,
            continuation_output,
        } => print_json(&run_witness_commands::export_historical_combat(
            &workspace,
            node,
            journal_entry,
            &case_output,
            &actions_output,
            continuation_output.as_deref(),
        )?),
        Command::BuildValuePrototype {
            case,
            actions,
            output,
            max_engine_steps_per_transition,
        } => print_json(&guidance_artifact_commands::build_value_prototype(
            &case,
            &actions,
            &output,
            max_engine_steps_per_transition,
        )?),
        Command::BuildValuePrototypeCorpus {
            manifest,
            output,
            max_engine_steps_per_transition,
        } => print_json(&guidance_artifact_commands::build_value_prototype_corpus(
            &manifest,
            &output,
            max_engine_steps_per_transition,
        )?),
        Command::BuildCombatGuidanceBundle {
            action_imitation_artifact,
            value_prototype_artifact,
            output,
        } => print_json(&guidance_artifact_commands::build_guidance_bundle(
            &action_imitation_artifact,
            &value_prototype_artifact,
            &output,
        )?),
        Command::BuildActionImitation {
            case,
            actions,
            output,
            max_engine_steps_per_transition,
        } => print_json(&guidance_artifact_commands::build_action_imitation(
            &case,
            &actions,
            &output,
            max_engine_steps_per_transition,
        )?),
        Command::BuildActionImitationCorpus {
            manifest,
            output,
            residual_over_existing_policy,
            max_engine_steps_per_transition,
        } => print_json(&guidance_artifact_commands::build_action_imitation_corpus(
            &manifest,
            &output,
            residual_over_existing_policy,
            max_engine_steps_per_transition,
        )?),
        Command::BuildActionSuccessorCorpus { args } => {
            let report = action_successor_reanalysis::build(args)?;
            print_json(&report)
        }
        Command::BuildActionReanalysisPolicy { args } => {
            let report = action_reanalysis_policy::build(args)?;
            print_json(&report)
        }
        Command::BuildActionReanalysisQueue { args } => {
            let report = action_reanalysis_queue::build(args)?;
            print_json(&report)
        }
        Command::BuildActionReanalysisBatch { args } => {
            let report = action_reanalysis_queue::build_batch(args)?;
            print_json(&report)
        }
        Command::AuditActionImitation {
            case,
            actions,
            artifact,
            max_engine_steps_per_transition,
        } => print_json(&guidance_artifact_commands::audit_action_imitation(
            &case,
            &actions,
            &artifact,
            max_engine_steps_per_transition,
        )?),
        Command::BuildBoundarySuccessorCorpus { args } => {
            let summary = boundary_successor_corpus::build(args)?;
            print_json(&summary)
        }
        Command::AuditBoundarySuccessorLookahead { args } => {
            let report = boundary_successor_lookahead::audit(args)?;
            print_json(&report)
        }
        Command::CombatCaseLocalGraph(args) => combat_case_local_graph::run(args),
        Command::CombatCaseLayered(args) => combat_case_layered::run(args),
        Command::CombatCaseFoldSolvedSuffix(args) => combat_case_fold_solved_suffix::run(args),
        Command::CombatCaseLayeredWindowRace(args) => combat_case_layered_window_race::run(args),
        Command::CombatCasePlanAnnotations(args) => combat_plan_diagnostics::run_annotations(args),
        Command::CombatCasePlanTrace(args) => combat_plan_diagnostics::run_trace(args),
        Command::CombatCaseAtomicLevin(args) => atomic_policy_searches::run_atomic_levin(args),
        Command::CombatCasePolicyDiscrepancy(args) => policy_discrepancy_search::run(args),
        Command::CombatCaseAtomicTurnPortfolio(args) => {
            combat_case_atomic_turn_portfolio::run(args)
        }
        Command::CombatCase(args) => combat_case_legacy_global::run(args),
        Command::TurnActionAudit(args) => turn_audits::run_action(args),
        Command::TurnPlanAudit(args) => turn_audits::run_plan(args),
        Command::DepthBeamTurnAudit(args) => depth_beam_audits::run_turn(args),
        Command::DepthBeamAgendaAudit(args) => depth_beam_audits::run_agenda(args),
        Command::TurnMembership(args) => turn_membership_audit::run(args),
        Command::V2CapabilityAudit(args) => v2_capability_audit::run(args),
        Command::View { workspace, node } => {
            print_json(&workspace_commands::view(&workspace, node)?)
        }
        Command::Status {
            workspace,
            node,
            limit,
        } => print_json(&workspace_commands::status(&workspace, node, limit)?),
        Command::Choose {
            workspace,
            owner_rank,
            node,
        } => print_json(&workspace_commands::choose(&workspace, owner_rank, node)?),
        Command::Owner { workspace, steps } => {
            print_json(&workspace_commands::owner(&workspace, steps)?)
        }
        Command::Timeline {
            workspace,
            node,
            tail,
        } => print_json(&workspace_commands::timeline(&workspace, node, tail)?),
        Command::ExportCombatCase {
            workspace,
            node,
            output,
        } => print_json(&workspace_commands::export_combat_case(
            &workspace, node, &output,
        )?),
        Command::Combat {
            workspace,
            node,
            max_engine_steps_per_transition,
        } => print_json(&workspace_commands::combat(
            &workspace,
            node,
            max_engine_steps_per_transition,
        )?),
        Command::Tree { workspace } => print_json(&workspace_commands::tree(&workspace)?),
        Command::Try {
            workspace,
            choice_ref,
        } => print_json(&workspace_commands::try_choice(&workspace, &choice_ref)?),
        Command::Focus { workspace, node } => {
            print_json(&workspace_commands::focus(&workspace, node)?)
        }
        Command::Follow { workspace, edge } => {
            print_json(&workspace_commands::follow(&workspace, edge)?)
        }
        Command::Back { workspace } => print_json(&workspace_commands::back(&workspace)?),
        Command::Promote { workspace } => print_json(&workspace_commands::promote(&workspace)?),
        Command::Advance {
            workspace,
            max_quanta,
            quantum_nodes,
            quantum_ms,
            wall_ms,
            improve_incumbent,
            detailed,
        } => print_json(&workspace_commands::advance(
            &workspace,
            max_quanta,
            quantum_nodes,
            quantum_ms,
            wall_ms,
            improve_incumbent,
            detailed,
        )?),
        Command::AcceptCombat { workspace } => {
            print_json(&workspace_commands::accept_combat(&workspace)?)
        }
        Command::AcceptCombatActions { workspace, actions } => print_json(
            &workspace_commands::accept_combat_actions(&workspace, &actions)?,
        ),
        Command::RestartCombat { workspace } => {
            print_json(&workspace_commands::restart_combat(&workspace)?)
        }
        Command::History {
            workspace,
            node,
            journal,
        } => print_json(&workspace_commands::history(&workspace, node, journal)?),
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
