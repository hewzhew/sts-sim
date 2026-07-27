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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Args;
use exact_turn_corridor::{
    load as load_exact_turn_corridor, load_action_segments as load_combat_action_segments,
    load_corpus as load_combat_action_imitation_corpus,
    typed_feature_components as typed_combat_feature_components, ExactTurnCorridor,
    ShadowCorridorGuide,
};
use guidance_artifact_commands::{load_value_prototype, save_value_prototype};
use oracle_cli::Command;
use serde::Serialize;
use serde_json::{json, Value};
use sts_combat_planner::{
    combat_plan_state_guide_policy_v1, fold_verified_suffix_through_turn_predecessors,
    generate_depth_beam_turn_options, rank_layered_combat_lineage_parents,
    search_depth_beam_agenda_witness, AtomicLevinRerooting, AtomicLevinWitnessConfig,
    AtomicLevinWitnessQuantum, AtomicLevinWitnessSession, AtomicTurnPortfolioConfig,
    AtomicTurnPortfolioEntryReport, AtomicTurnPortfolioQuantum, AtomicTurnPortfolioSession,
    CombatActionPolicy, CombatDecisionRoot, CombatGuideLaneId, CombatPlanningQuantum,
    CombatPolicyChoice, CombatStateGuide, CombatStateGuideRank, DepthBeamAgendaBudget,
    DepthBeamAgendaConfig, DepthBeamTurnBudget, DepthBeamTurnConfig,
    LayeredCombatCandidateRaceConfig, LayeredCombatCandidateRaceSession,
    LayeredCombatLineagePortfolioConfig, LayeredCombatLineagePortfolioSession,
    LayeredCombatSolvedSuffixIndex, LayeredCombatWitnessConfig, LayeredCombatWitnessQuantum,
    LayeredCombatWitnessSession, LocalTurnGraphStateSnapshot, LocalTurnGraphWitnessConfig,
    LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession, OracleCombatWitnessConfig,
    OracleCombatWitnessQuantum, OracleCombatWitnessSatisfaction, OracleCombatWitnessSession,
    PolicyDiscrepancyConfig, PolicyDiscrepancyQuantum, PolicyDiscrepancySession,
    PolicyDiscrepancyTurnMacroConfig, SharedCombatActionPolicy, SolvedSuffixFoldConfig,
    SolvedSuffixFoldStatus, TurnOptionAction, TurnOptionGenerationStatus,
    TurnOptionGeneratorConfig, TurnOptionGeneratorSession,
};
use sts_combat_strategy::{awakened_one_combat_plan_v1, awakened_one_plan_transition_v1};
use sts_oracle_runtime::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::eval::combat_action_imitation::{
    combat_action_imitation_policy_v1, root_player_turn_action_policy_v1,
    CombatActionImitationArtifactV1,
};
use sts_oracle_runtime::eval::combat_case::{load_combat_case, save_combat_case, CombatCase};
use sts_oracle_runtime::eval::combat_guidance_bundle::{
    combat_value_prototype_policy_v1, combat_value_prototype_rank_v1, CombatGuidanceBundleV1,
    CombatValuePrototypeArtifactV1, GUIDE_LEARNED_BOUNDARY_VALUE,
};
use sts_oracle_runtime::eval::combat_search_v2::{
    run_combat_root_proposal_probe_v1, CombatRootProposalProbeV1Report, CombatSearchV2LoadedStart,
    CombatSearchV2RunOptions,
};
use sts_oracle_runtime::eval::run_control::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
    ExistingCombatKnowledgeAdvisorAdvanceV1, ExistingCombatKnowledgeAdvisorV1,
};
use sts_oracle_runtime::runtime::branch::{
    load_oracle_run_continuation_v1, save_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceV1,
    OracleRunConfig,
};
use sts_oracle_runtime::sim::combat::{
    combat_terminal, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_oracle_runtime::sim::combat_action::combat_action_key;
use sts_oracle_runtime::state::core::{ClientInput, EngineState};

struct ExactCorridorShadowPolicy {
    base: SharedCombatActionPolicy,
    rank_by_exact_hash: Arc<HashMap<String, i32>>,
    atomic_rank_by_exact_hash: Arc<HashMap<String, i32>>,
    typed_target_by_turn: Arc<HashMap<u32, Vec<(i32, Vec<i32>)>>>,
    guide: ShadowCorridorGuide,
    shadow_only: bool,
}

struct AnchorOnlyPolicy {
    base: SharedCombatActionPolicy,
}

struct RootTurnAnchorOnlyPolicy {
    root_player_turn: u32,
    base: SharedCombatActionPolicy,
}

fn load_action_imitation_policy(
    path: &Path,
    base: SharedCombatActionPolicy,
) -> Result<SharedCombatActionPolicy, String> {
    let artifact = CombatActionImitationArtifactV1::load(path)?;
    combat_action_imitation_policy_v1(base, artifact)
}

const GUIDE_EXACT_CORRIDOR: CombatGuideLaneId = CombatGuideLaneId::new(10_001);
const GUIDE_TYPED_CORRIDOR: CombatGuideLaneId = CombatGuideLaneId::new(10_002);

impl CombatActionPolicy for AnchorOnlyPolicy {
    fn weights(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        family: &sts_oracle_runtime::sim::combat_action_surface::CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guides(
        &self,
        _position: &sts_oracle_runtime::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        Vec::new()
    }

    fn turn_generation_guides(
        &self,
        _position: &sts_oracle_runtime::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        Vec::new()
    }
}

fn anchor_only_policy(base: SharedCombatActionPolicy) -> SharedCombatActionPolicy {
    Arc::new(AnchorOnlyPolicy { base })
}

impl CombatActionPolicy for RootTurnAnchorOnlyPolicy {
    fn weights(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        family: &sts_oracle_runtime::sim::combat_action_surface::CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guides(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        if position.combat.turn.turn_count == self.root_player_turn {
            Vec::new()
        } else {
            self.base.state_guides(position)
        }
    }

    fn turn_generation_guides(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        if position.combat.turn.turn_count == self.root_player_turn {
            Vec::new()
        } else {
            self.base.turn_generation_guides(position)
        }
    }
}

fn root_turn_anchor_only_policy(
    root_player_turn: u32,
    base: SharedCombatActionPolicy,
) -> SharedCombatActionPolicy {
    Arc::new(RootTurnAnchorOnlyPolicy {
        root_player_turn,
        base,
    })
}

impl CombatActionPolicy for ExactCorridorShadowPolicy {
    fn weights(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        family: &sts_oracle_runtime::sim::combat_action_surface::CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guides(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        let mut ranks = if self.shadow_only {
            Vec::new()
        } else {
            self.base.state_guides(position)
        };
        match self.guide {
            ShadowCorridorGuide::Exact => {
                let exact_hash =
                    sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                        &position.engine,
                        &position.combat,
                    );
                if let Some(corridor_rank) = self.rank_by_exact_hash.get(&exact_hash).copied() {
                    // An exact-corridor control is a sparse oracle lane. Do
                    // not enqueue every non-corridor state with a low rank:
                    // the guide scheduler's service-sharing window would let
                    // those unrelated states dilute the perfect-information
                    // control and make its result uninterpretable.
                    ranks.push(CombatStateGuide::new(
                        GUIDE_EXACT_CORRIDOR,
                        vec![1, corridor_rank],
                    ));
                }
            }
            ShadowCorridorGuide::TypedFeature => {
                ranks.push(CombatStateGuide::from_rank(
                    GUIDE_TYPED_CORRIDOR,
                    self.shadow_rank(position, position.combat.turn.turn_count),
                ));
            }
        }
        ranks
    }

    fn turn_generation_guides(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        let mut ranks = if self.shadow_only {
            Vec::new()
        } else {
            self.base.turn_generation_guides(position)
        };
        match self.guide {
            ShadowCorridorGuide::Exact => {
                let exact_hash =
                    sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                        &position.engine,
                        &position.combat,
                    );
                if let Some(atomic_rank) = self.atomic_rank_by_exact_hash.get(&exact_hash).copied()
                {
                    ranks.push(CombatStateGuide::new(
                        GUIDE_EXACT_CORRIDOR,
                        vec![1, atomic_rank],
                    ));
                }
            }
            ShadowCorridorGuide::TypedFeature => {
                ranks.push(CombatStateGuide::from_rank(
                    GUIDE_TYPED_CORRIDOR,
                    self.shadow_rank(position, position.combat.turn.turn_count.saturating_add(1)),
                ));
            }
        }
        ranks
    }
}

impl ExactCorridorShadowPolicy {
    fn shadow_rank(
        &self,
        position: &sts_oracle_runtime::sim::combat::CombatPosition,
        target_turn: u32,
    ) -> CombatStateGuideRank {
        let shadow_rank = match self.guide {
            ShadowCorridorGuide::Exact => {
                let exact_hash =
                    sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                        &position.engine,
                        &position.combat,
                    );
                let corridor_rank = self.rank_by_exact_hash.get(&exact_hash).copied();
                vec![
                    i32::from(corridor_rank.is_some()),
                    corridor_rank.unwrap_or_default(),
                ]
            }
            ShadowCorridorGuide::TypedFeature => {
                combat_value_prototype_rank_v1(&self.typed_target_by_turn, position, target_turn)
            }
        };
        CombatStateGuideRank::new(shadow_rank)
    }
}

fn exact_corridor_shadow_policy(
    base: SharedCombatActionPolicy,
    corridor: &ExactTurnCorridor,
    guide: ShadowCorridorGuide,
    shadow_only: bool,
) -> SharedCombatActionPolicy {
    Arc::new(ExactCorridorShadowPolicy {
        base,
        rank_by_exact_hash: Arc::new(corridor.rank_by_exact_hash.clone()),
        atomic_rank_by_exact_hash: Arc::new(corridor.atomic_rank_by_exact_hash.clone()),
        typed_target_by_turn: Arc::new(
            corridor
                .typed_target_by_turn
                .iter()
                .map(|(turn, target)| (*turn, vec![target.clone()]))
                .collect(),
        ),
        guide,
        shadow_only,
    })
}

fn value_prototype_shadow_policy(
    base: SharedCombatActionPolicy,
    artifact: &CombatValuePrototypeArtifactV1,
) -> SharedCombatActionPolicy {
    Arc::new(ExactCorridorShadowPolicy {
        base,
        rank_by_exact_hash: Arc::new(HashMap::new()),
        atomic_rank_by_exact_hash: Arc::new(HashMap::new()),
        typed_target_by_turn: Arc::new(artifact.targets_by_turn()),
        guide: ShadowCorridorGuide::TypedFeature,
        shadow_only: false,
    })
}

fn load_layered_solved_suffix_index(
    case_path: Option<&PathBuf>,
    actions_path: Option<&PathBuf>,
    max_engine_steps_per_transition: usize,
) -> Result<Arc<LayeredCombatSolvedSuffixIndex>, String> {
    let (Some(case_path), Some(actions_path)) = (case_path, actions_path) else {
        if case_path.is_some() || actions_path.is_some() {
            return Err(
                "--solved-suffix-case and --solved-suffix-actions must be provided together"
                    .to_string(),
            );
        }
        return Ok(Arc::new(LayeredCombatSolvedSuffixIndex::default()));
    };
    let corridor = exact_turn_corridor::load(
        case_path,
        std::slice::from_ref(actions_path),
        max_engine_steps_per_transition,
    )?;
    let mut suffixes = LayeredCombatSolvedSuffixIndex::default();
    for (turn_index, position) in corridor.positions_by_rank.iter().enumerate() {
        let inputs = corridor.transition_actions[turn_index..]
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let root = CombatDecisionRoot::new(position.clone()).map_err(|error| {
            format!("invalid solved suffix root at turn segment {turn_index}: {error:?}")
        })?;
        suffixes
            .insert_verified_inputs(
                root,
                inputs,
                max_engine_steps_per_transition,
                &EngineCombatStepper,
            )
            .map_err(|error| {
                format!("solved suffix turn segment {turn_index} failed replay: {error:?}")
            })?;
    }
    Ok(Arc::new(suffixes))
}

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

fn combat_policy_surface(
    position: &sts_oracle_runtime::sim::combat::CombatPosition,
    limit: usize,
) -> Value {
    const UNIFORM_EXPLORATION: f64 = 0.05;

    let stepper = EngineCombatStepper;
    let actions = stepper.atomic_actions(position);
    let weights =
        sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_atomic_action_policy_weights(
            position,
            &actions,
        );
    let total = weights.iter().sum::<f64>();
    let uniform = 1.0 / actions.len().max(1) as f64;
    let mut ranked = actions
        .iter()
        .zip(&weights)
        .enumerate()
        .map(|(surface_index, (input, weight))| {
            let ordinal_rank = 1 + weights
                .iter()
                .filter(|candidate| **candidate > *weight)
                .count();
            let probability = if total > 0.0 {
                ((1.0 - UNIFORM_EXPLORATION) * (*weight / total) + UNIFORM_EXPLORATION * uniform)
                    .max(f64::MIN_POSITIVE)
            } else {
                uniform
            };
            (
                *weight,
                surface_index,
                json!({
                    "rank": ordinal_rank,
                    "surface_index": surface_index,
                    "action": combat_trace_view::combat_action_label(position, input),
                    "weight": weight,
                    "probability": probability,
                }),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    let shown = ranked.len().min(limit);
    json!({
        "action_count": ranked.len(),
        "shown": shown,
        "truncated": ranked.len() > shown,
        "actions": ranked
            .into_iter()
            .take(limit)
            .map(|(_, _, value)| value)
            .collect::<Vec<_>>(),
    })
}

fn export_descendant_combat_case(
    base: &CombatCase,
    actions: &[TurnOptionAction],
    output: &Path,
    max_engine_steps_per_transition: usize,
    reason: &str,
) -> Result<PathBuf, String> {
    let position = replay_descendant_position(
        base.position.clone(),
        actions,
        max_engine_steps_per_transition,
    )?;

    let mut exported = base.clone();
    exported.position = position;
    exported.combat = sts_oracle_runtime::eval::combat_case::combat_summary(&exported.position);
    exported.run.hp = exported.position.combat.entities.player.current_hp;
    exported.run.max_hp = exported.position.combat.entities.player.max_hp;
    exported.gap.boundary = format!(
        "{} + {} exact descendant actions",
        exported.gap.boundary,
        actions.len()
    );
    exported.gap.reason = reason.to_string();
    exported.combat_search_attempts.clear();
    exported.failed_search = None;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    save_combat_case(output, &exported)?;

    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("deepest");
    let action_output = output.with_file_name(format!("{stem}.prefix.actions.json"));
    let inputs = actions
        .iter()
        .map(|action| action.input.clone())
        .collect::<Vec<_>>();
    std::fs::write(
        &action_output,
        serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(action_output)
}

fn local_graph_state_snapshot_for_path(
    session: &LocalTurnGraphWitnessSession,
    root: sts_oracle_runtime::sim::combat::CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<Option<LocalTurnGraphStateSnapshot>, String> {
    let position = replay_descendant_position(root, actions, max_engine_steps_per_transition)?;
    let exact_state_hash = sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
        &position.engine,
        &position.combat,
    );
    Ok(session.state_snapshot_by_exact_hash(&exact_state_hash))
}

fn replay_descendant_position(
    mut position: sts_oracle_runtime::sim::combat::CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<sts_oracle_runtime::sim::combat::CombatPosition, String> {
    let stepper = EngineCombatStepper;
    for (index, action) in actions.iter().enumerate() {
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(format!(
                "deepest-case action {index} is not legal at turn {}: {:?}",
                position.combat.turn.turn_count, action.input
            ));
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if result.truncated {
            return Err(format!(
                "deepest-case action {index} exceeded {max_engine_steps_per_transition} engine steps"
            ));
        }
        position = result.position;
    }
    Ok(position)
}

fn replay_combat_path(
    mut position: sts_oracle_runtime::sim::combat::CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<Value, String> {
    let stepper = EngineCombatStepper;
    let mut turns = Vec::new();
    let mut turn_number = position.combat.turn.turn_count;
    let mut turn_start_hp = position.combat.entities.player.current_hp;
    let mut turn_start_policy = combat_policy_surface(&position, 12);
    let mut turn_start_action_index = 1usize;
    let mut turn_actions = Vec::new();
    let mut terminal = stepper.terminal(&position);

    for (index, action) in actions.iter().enumerate() {
        let action_key = combat_trace_view::combat_action_label(&position, &action.input);
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(format!(
                "diagnostic path action {index} is not legal at turn {}: {action_key}",
                position.combat.turn.turn_count
            ));
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if result.truncated {
            return Err(format!(
                "diagnostic path action {index} exceeded {max_engine_steps_per_transition} engine steps: {action_key}"
            ));
        }
        turn_actions.push(action_key);
        position = result.position;
        terminal = result.terminal;
        let next_turn = position.combat.turn.turn_count;
        if next_turn != turn_number
            || !matches!(
                terminal,
                sts_oracle_runtime::sim::combat::CombatTerminal::Unresolved
            )
        {
            turns.push(json!({
                "turn": turn_number,
                "action_range": {
                    "first": turn_start_action_index,
                    "last": index + 1,
                },
                "start_hp": turn_start_hp,
                "start_policy": turn_start_policy,
                "actions": turn_actions,
                "end": combat_trace_view::combat_turn_snapshot(&position),
                "terminal": format!("{terminal:?}"),
            }));
            turn_number = next_turn;
            turn_start_hp = position.combat.entities.player.current_hp;
            turn_start_policy = combat_policy_surface(&position, 12);
            turn_start_action_index = index + 2;
            turn_actions = Vec::new();
        }
    }
    if !turn_actions.is_empty() {
        turns.push(json!({
            "turn": turn_number,
            "action_range": {
                "first": turn_start_action_index,
                "last": actions.len(),
            },
            "start_hp": turn_start_hp,
            "start_policy": turn_start_policy,
            "actions": turn_actions,
            "end": combat_trace_view::combat_turn_snapshot(&position),
            "terminal": format!("{terminal:?}"),
            "partial": true,
        }));
    }

    Ok(json!({
        "action_count": actions.len(),
        "turns": turns,
        "terminal": format!("{terminal:?}"),
    }))
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize oracle_lab output: {error}"))?
    );
    Ok(())
}
