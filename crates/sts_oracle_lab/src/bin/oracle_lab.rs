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
mod depth_beam_audits;
mod exact_combat_evidence;
mod exact_turn_corridor;
mod guidance_artifact_commands;
mod oracle_seed_panel;
mod policy_discrepancy_search;
mod run_witness_commands;
mod run_witness_suite;
mod turn_audits;
mod turn_membership_audit;
mod v2_capability_audit;
mod workspace_view;

use canonical_launch::{
    runtime_identity as oracle_lab_runtime_identity, source_content_fingerprint,
};

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use atomic_policy_searches::CombatCaseAtomicLevinArgs;
use clap::{Args, Parser, Subcommand};
use combat_case_atomic_turn_portfolio::CombatCaseAtomicTurnPortfolioArgs;
use combat_case_fold_solved_suffix::CombatCaseFoldSolvedSuffixArgs;
use combat_case_layered::CombatCaseLayeredArgs;
use combat_case_layered_window_race::CombatCaseLayeredWindowRaceArgs;
use combat_case_legacy_global::CombatCaseLegacyGlobalArgs;
use combat_case_local_graph::CombatCaseLocalGraphArgs;
use combat_plan_diagnostics::{CombatCasePlanAnnotationsArgs, CombatCasePlanTraceArgs};
use depth_beam_audits::{DepthBeamAgendaAuditArgs, DepthBeamTurnAuditArgs};
use exact_turn_corridor::{
    load as load_exact_turn_corridor, load_action_segments as load_combat_action_segments,
    load_corpus as load_combat_action_imitation_corpus,
    typed_feature_components as typed_combat_feature_components, ExactTurnCorridor,
    ShadowCorridorGuide,
};
use guidance_artifact_commands::{load_value_prototype, save_value_prototype};
use oracle_seed_panel::OracleSeedPanelArgs;
use policy_discrepancy_search::CombatCasePolicyDiscrepancyArgs;
use run_witness_suite::RunWitnessSuiteArgs;
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
    LayeredCombatFrontierState, LayeredCombatLineagePortfolioConfig,
    LayeredCombatLineagePortfolioEntryReport, LayeredCombatLineagePortfolioSession,
    LayeredCombatSolvedSuffixIndex, LayeredCombatWitnessConfig, LayeredCombatWitnessQuantum,
    LayeredCombatWitnessSession, LocalTurnGraphPlanTransitionEdgeSnapshot,
    LocalTurnGraphStateSnapshot, LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum,
    LocalTurnGraphWitnessSession, OracleCombatWitnessConfig, OracleCombatWitnessQuantum,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessSession, PolicyDiscrepancyConfig,
    PolicyDiscrepancyQuantum, PolicyDiscrepancySession, PolicyDiscrepancyTurnMacroConfig,
    SharedCombatActionPolicy, SolvedSuffixFoldConfig, SolvedSuffixFoldStatus, TurnOptionAction,
    TurnOptionGenerationStatus, TurnOptionGeneratorConfig, TurnOptionGeneratorSession,
};
use sts_combat_strategy::{
    awakened_one_combat_plan_v1, awakened_one_plan_transition_v1, CombatPlanTransitionAnnotationV1,
    CombatPlanTransitionV1,
};
use sts_oracle_runtime::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::content::{cards, monsters::EnemyId};
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
    OracleAnalysisAdvanceRequestV1,
};
use sts_oracle_runtime::runtime::branch::{
    load_oracle_analysis_workspace_v1, load_oracle_run_continuation_v1,
    oracle_live_combat_diagnostic_v1, save_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceV1,
    OracleRunBudget, OracleRunConfig,
};
use sts_oracle_runtime::sim::combat::{
    combat_terminal, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_oracle_runtime::sim::combat_action::{combat_action_key, target_label};
use sts_oracle_runtime::state::core::{ClientInput, EngineState};
use turn_audits::{TurnActionAuditArgs, TurnPlanAuditArgs};
use turn_membership_audit::TurnMembershipArgs;
use v2_capability_audit::V2CapabilityAuditArgs;

#[derive(Debug, Parser)]
#[command(
    name = "oracle_lab",
    about = "Inspect and steer exact oracle-run variations without editing checkpoints"
)]
struct Cli {
    /// Proves that Cargo's canonical `cargo oracle-lab` alias launched this
    /// process. Direct execution is intentionally rejected so that a stale or
    /// wrongly-profiled oracle laboratory cannot silently produce evidence.
    #[arg(long, hide = true, global = true)]
    canonical_oracle: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run consecutive A0 oracle seeds with durable per-seed reports and resumable stops.
    SeedPanel(OracleSeedPanelArgs),
    /// Start a new A0-style oracle analysis workspace at Neow.
    New {
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0)]
        ascension: u8,
        #[arg(long)]
        workspace: PathBuf,
        /// Embed one validated guidance bundle into the workspace so every
        /// later process restore uses the same immutable search policy.
        #[arg(long)]
        combat_guidance_bundle: Option<PathBuf>,
        #[command(flatten)]
        budget: BudgetArgs,
    },
    /// Import an exact state from an oracle_run continuation.
    Import {
        #[arg(long)]
        continuation: PathBuf,
        #[arg(long)]
        workspace: PathBuf,
        /// Import one retained frontier branch instead of the report-selected state.
        #[arg(long)]
        branch_id: Option<usize>,
        /// Embed one validated guidance bundle into the workspace so every
        /// later process restore uses the same immutable search policy.
        #[arg(long)]
        combat_guidance_bundle: Option<PathBuf>,
        #[command(flatten)]
        budget: BudgetArgs,
    },
    /// Export one exact analysis node as an oracle_run continuation.
    ExportContinuation {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Recover one exact combat branch from a stale analysis workspace without
    /// restoring or validating unrelated frontier branches.
    RecoverCombatCase {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        branch: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Replay the selected workspace node's entire committed journal from the
    /// canonical seed state and verify its exact final session fingerprint.
    VerifyRunWitness {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long, default_value_t = 0)]
        node: usize,
    },
    /// Replay a saved witness exactly and compare every committed non-combat
    /// choice with the current production owner ordering. No search runs.
    AuditRunWitnessPolicy {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long, default_value_t = 0)]
        node: usize,
        /// Include every owner/witness divergence instead of the compact
        /// completion summary.
        #[arg(long)]
        details: bool,
    },
    /// Replay a versioned set of exact F0-to-Act-3 witnesses in one process.
    ///
    /// The optional owner audit is diagnostic only: historical policy
    /// divergence cannot invalidate an otherwise exact terminal witness.
    VerifyRunWitnessSuite {
        #[command(flatten)]
        args: RunWitnessSuiteArgs,
    },
    /// Replace one historical combat trajectory with another exact trajectory
    /// and emit a continuation only if the full run still replays exactly.
    SpliceCombatWitness {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: usize,
        #[arg(long)]
        journal_entry: usize,
        #[arg(long)]
        replacement_workspace: PathBuf,
        #[arg(long)]
        replacement_node: usize,
        #[arg(long)]
        output: PathBuf,
    },
    /// Export an exact historical combat root and its verified action witness
    /// from a complete run. The run journal is replayed to the requested
    /// entry; no continuation JSON editing is involved.
    ExportHistoricalCombatWitness {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long, default_value_t = 0)]
        node: usize,
        #[arg(long)]
        journal_entry: usize,
        #[arg(long)]
        case_output: PathBuf,
        #[arg(long)]
        actions_output: PathBuf,
        /// Optionally emit the exact run prefix as an importable continuation
        /// at this combat root.
        #[arg(long)]
        continuation_output: Option<PathBuf>,
    },
    /// Inspect the retired global-agenda search on one exact case.
    #[command(name = "combat-case-legacy-global")]
    CombatCase(CombatCaseLegacyGlobalArgs),
    /// Run one pure atomic Levin policy-tree search on an exact combat case.
    /// This deliberately bypasses complete-turn generation, state guides,
    /// legacy donors, and every lane scheduler.
    CombatCaseAtomicLevin(CombatCaseAtomicLevinArgs),
    /// Annotate every finite atomic successor with read-only typed combat-plan
    /// facts. This command does not search, rank, prune, or modify a policy.
    CombatCasePlanAnnotations(CombatCasePlanAnnotationsArgs),
    /// Replay one exact action sequence and report typed combat-plan changes.
    /// This is a read-only trace: actions are supplied by the caller, never
    /// selected or ranked by this command.
    CombatCasePlanTrace(CombatCasePlanTraceArgs),
    /// Follow the action policy to terminal states and search complete
    /// trajectories by increasing weighted policy discrepancy.
    CombatCasePolicyDiscrepancy(CombatCasePolicyDiscrepancyArgs),
    /// Enumerate exact next-turn states under the base policy, while giving
    /// every state an independent resumable atomic suffix search.
    CombatCaseAtomicTurnPortfolio(CombatCaseAtomicTurnPortfolioArgs),
    /// Lab-only turn-synchronous beam control. It never invokes the legacy
    /// suffix donor or the production Widen/Deepen agenda.
    CombatCaseLayered(CombatCaseLayeredArgs),
    /// Isolated local-graph component with node-local lazy widening.
    #[command(name = "combat-case", visible_alias = "combat-case-local-graph")]
    CombatCaseLocalGraph(CombatCaseLocalGraphArgs),
    /// Generate one exact turn boundary, select one deferred beam window,
    /// then dovetail resumable layered continuations for its candidates.
    CombatCaseLayeredWindowRace(CombatCaseLayeredWindowRaceArgs),
    /// Compile one verified deep tactical suffix backwards through exact
    /// player-turn predecessors. The corridor supplies predecessor states
    /// only; each fold must naturally generate the already-proven successor.
    CombatCaseFoldSolvedSuffix(CombatCaseFoldSolvedSuffixArgs),
    /// Distill one exact terminal witness into a semantic action-order artifact.
    BuildActionImitation {
        #[arg(long)]
        case: PathBuf,
        /// One or more consecutive exact action segments. Repeat the flag to
        /// compose a witness without rewriting JSON by hand.
        #[arg(long, required = true)]
        actions: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Reanalyse every bounded legal action at one exact witness state.
    ///
    /// The offline corpus keeps exact wins, exact refutations, terminal
    /// non-wins, and budget-unknown successors as distinct evidence kinds.
    BuildActionSuccessorCorpus {
        #[command(flatten)]
        args: action_successor_reanalysis::ActionSuccessorReanalysisArgs,
    },
    /// Train a conservative residual policy from exact witnesses plus typed
    /// action-successor reanalysis. Budget-unknown actions retain base mass.
    BuildActionReanalysisPolicy {
        #[command(flatten)]
        args: action_reanalysis_policy::ActionReanalysisPolicyArgs,
    },
    /// Rank exact witness states for bounded action-successor reanalysis.
    ///
    /// This is a read-only compute-order tool. It does not treat policy
    /// disagreement as negative evidence and cannot alter production policy.
    BuildActionReanalysisQueue {
        #[command(flatten)]
        args: action_reanalysis_queue::ActionReanalysisQueueArgs,
    },
    /// Reanalyse the highest-priority states from a saved queue in one
    /// invocation, reusing the same verified manifest and policy identity.
    BuildActionReanalysisBatch {
        #[command(flatten)]
        args: action_reanalysis_queue::ActionReanalysisBatchArgs,
    },
    /// Build offline complete-turn successor evidence from verified witnesses.
    ///
    /// This command never changes a production policy. Exact wins,
    /// exhaustive refutations, and budget-unknown observations remain
    /// distinct in the exported corpus.
    BuildBoundarySuccessorCorpus {
        #[command(flatten)]
        args: boundary_successor_corpus::BoundarySuccessorCorpusArgs,
    },
    /// Compare bounded rollout guidance across exact complete-turn successors.
    ///
    /// This is a read-only teacher audit. It never changes the production
    /// action policy, successor scheduler, or exact witness contract.
    AuditBoundarySuccessorLookahead {
        #[command(flatten)]
        args: boundary_successor_lookahead::BoundarySuccessorLookaheadArgs,
    },
    /// Distill several exact terminal witnesses from one compact manifest.
    /// Relative case and action paths are resolved beside the manifest.
    BuildActionImitationCorpus {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
        /// Learn corrections to the mature combat action policy instead of
        /// replacing its action distribution. This mode is explicit because
        /// the resulting artifact must be paired with that same base policy.
        #[arg(long)]
        residual_over_existing_policy: bool,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Evaluate an existing semantic action policy against a verified witness
    /// without retraining it or changing the artifact.
    AuditActionImitation {
        #[arg(long)]
        case: PathBuf,
        #[arg(long, required = true)]
        actions: Vec<PathBuf>,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Distill one exact terminal witness into a typed-feature prototype
    /// artifact for lab-only state-value inference.
    BuildValuePrototype {
        #[arg(long)]
        case: PathBuf,
        /// One or more consecutive exact action segments. Repeat the flag to
        /// compose a witness without rewriting JSON by hand.
        #[arg(long, required = true)]
        actions: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Distill several exact terminal witnesses into one typed-feature value
    /// corpus. Uses the same compact manifest as action-imitation training.
    BuildValuePrototypeCorpus {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Package already-built action and value artifacts into one immutable,
    /// runtime-compatible guidance unit.
    BuildCombatGuidanceBundle {
        #[arg(long)]
        action_imitation_artifact: PathBuf,
        #[arg(long)]
        value_prototype_artifact: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Check when one exact complete-turn action sequence is generated.
    TurnMembership(TurnMembershipArgs),
    /// Compare the mature V2 search with and without rollout guidance on the
    /// same exact combat root. This is a compact capability ablation; it
    /// cannot seed or alter production search.
    V2CapabilityAudit(V2CapabilityAuditArgs),
    /// Audit action-policy order and exact one-step successor guides at one turn prefix.
    TurnActionAudit(TurnActionAuditArgs),
    /// Audit the mature V2 bounded complete-turn proposer on one exact case.
    /// This is read-only evidence: it does not seed either production search.
    TurnPlanAudit(TurnPlanAuditArgs),
    /// Generate complete-turn proposals with an independent action-depth beam.
    /// Finished short turns never displace still-live longer prefixes.
    DepthBeamTurnAudit(DepthBeamTurnAuditArgs),
    /// Lazily expand one exact player-turn boundary at a time using one
    /// explicitly selected guide lane. This lab control retains deferred
    /// exact variants instead of discarding them through a boundary beam.
    DepthBeamAgendaAudit(DepthBeamAgendaAuditArgs),
    /// View the current cursor or another exact analysis node.
    View {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Show a compact actionable summary of the current or selected node.
    Status {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    /// Choose one candidate by its owner rank at the current cursor.
    Choose {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        owner_rank: u64,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Apply the owner's first choice for a bounded number of decisions.
    Owner {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=64))]
        steps: u8,
    },
    /// Print a compact tail of the committed run journal.
    Timeline {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 30)]
        tail: usize,
    },
    /// Export the current or selected exact combat as a standalone case.
    ExportCombatCase {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Show the exact combat root, search progress, action families, and traces.
    Combat {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 512)]
        max_engine_steps_per_transition: usize,
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
    /// Run one bounded tactical attempt at the current cursor.
    ///
    /// Exact state, accounting, and any verified witness persist in the
    /// workspace. The in-memory tactical frontier does not survive a process
    /// exit, so repeated invocations restart search from the same combat root.
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
        /// Keep the verified incumbent resident and spend the full bounded
        /// request looking for a higher-HP witness. Use `accept-combat`
        /// afterwards to commit the best result.
        #[arg(long)]
        improve_incumbent: bool,
        /// Print the full tactical progress report and node view. The default
        /// output is intentionally compact; detailed traces remain opt-in.
        #[arg(long)]
        detailed: bool,
    },
    /// Accept the current combat's already verified incumbent.
    AcceptCombat {
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Replay and accept an explicit exact combat witness at the cursor.
    AcceptCombatActions {
        #[arg(long)]
        workspace: PathBuf,
        /// One or more action-list files, composed in flag order.
        #[arg(long)]
        actions: Vec<PathBuf>,
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
        /// Print the committed run journal, including history imported from an
        /// oracle_run continuation, instead of only oracle-lab variation edges.
        #[arg(long)]
        journal: bool,
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
    let cli = Cli::parse();
    canonical_launch::validate(cli.canonical_oracle)?;
    match cli.command {
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
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = if let Some(node) = node {
                analysis.session.view_node(node)?
            } else {
                analysis.view()?
            };
            print_json(&view)
        }
        Command::Status {
            workspace,
            node,
            limit,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = workspace_view::selected(&analysis, node)?;
            print_json(&workspace_view::compact_node(&view, limit))
        }
        Command::Choose {
            workspace,
            owner_rank,
            node,
        } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            if let Some(expected) = node {
                let actual = analysis.session.cursor_node_id();
                if expected != actual {
                    return Err(format!(
                        "oracle choose expected cursor node {expected}, but current cursor is {actual}"
                    ));
                }
            }
            let current = analysis.view()?;
            let matches = current
                .choices
                .iter()
                .filter(|choice| choice.owner_rank == owner_rank)
                .collect::<Vec<_>>();
            let [choice] = matches.as_slice() else {
                return Err(format!(
                    "oracle node {} has {} choices with owner rank {owner_rank}; expected exactly one",
                    current.node_id,
                    matches.len()
                ));
            };
            let view = analysis.try_choice(&choice.choice_ref.clone())?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&workspace_view::compact_node(&view, 8))
        }
        Command::Owner { workspace, steps } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let mut applied = Vec::new();
            let mut stopped = "step_limit";
            for _ in 0..steps {
                let current = analysis.view()?;
                let choices = current
                    .choices
                    .iter()
                    .filter(|choice| choice.owner_rank == 0)
                    .collect::<Vec<_>>();
                let [choice] = choices.as_slice() else {
                    stopped = if choices.is_empty() {
                        "no_owner_choice"
                    } else {
                        "ambiguous_owner_choice"
                    };
                    break;
                };
                let candidate_id = choice.candidate_id.clone();
                let label = choice.label.clone();
                let choice_ref = choice.choice_ref.clone();
                applied.push(json!({
                    "node": current.node_id,
                    "candidate_id": candidate_id,
                    "label": label,
                }));
                analysis.try_choice(&choice_ref)?;
            }
            if !applied.is_empty() {
                save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            }
            print_json(&json!({
                "requested_steps": steps,
                "applied_count": applied.len(),
                "applied": applied,
                "stopped": stopped,
                "status": workspace_view::compact_node(&analysis.view()?, 8),
            }))
        }
        Command::Timeline {
            workspace,
            node,
            tail,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            if tail == 0 || tail > 500 {
                return Err("timeline tail must be in 1..=500".to_string());
            }
            print_json(&workspace_view::compact_timeline(&analysis, node, tail)?)
        }
        Command::ExportCombatCase {
            workspace,
            node,
            output,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            let case = workspace_view::combat_case(&analysis, node)?;
            save_combat_case(&output, &case)?;
            print_json(&json!({
                "node": node,
                "output": output,
                "combat": case.combat,
            }))
        }
        Command::Combat {
            workspace,
            node,
            max_engine_steps_per_transition,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            print_json(&oracle_live_combat_diagnostic_v1(
                &analysis,
                node,
                max_engine_steps_per_transition,
            )?)
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
            improve_incumbent,
            detailed,
        } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let (report, view) = analysis.advance(OracleAnalysisAdvanceRequestV1 {
                max_quanta,
                quantum_nodes,
                quantum_ms: Some(quantum_ms),
                wall_ms,
                improve_incumbent,
            })?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            if detailed {
                print_json(&AdvanceOutput { report, view })
            } else {
                let combat = report.combat.as_ref().map(|combat| {
                    json!({
                        "generation_work": combat.generation_work,
                        "current_search_generation_work": combat.current_search_generation_work,
                        "exact_states": combat.exact_states,
                        "completed_turn_options": combat.completed_turn_options,
                        "retained_state_work": combat.retained_state_work,
                        "max_player_turn": combat.max_player_turn,
                        "incumbent_discovery_source": combat.incumbent_discovery_source,
                        "incumbent_final_hp": combat.incumbent_final_hp,
                        "incumbent_hp_loss": combat.incumbent_hp_loss,
                        "incumbent_action_count": combat.incumbent_action_count,
                        "last_status": combat.last_status,
                    })
                });
                print_json(&json!({
                    "schema_name": "OracleAnalysisAdvanceSummaryV1",
                    "schema_version": 1,
                    "source_node_id": report.source_node_id,
                    "status": report.status,
                    "quanta_served": report.quanta_served,
                    "elapsed_ms": report.elapsed_ms,
                    "combat": combat,
                    "result": {
                        "node": view.node_id,
                        "boundary": view.boundary,
                        "act": view.act,
                        "floor": view.floor,
                        "hp": view.current_hp,
                        "max_hp": view.max_hp,
                        "gold": view.gold,
                        "choice_count": view.choices.len(),
                        "child_count": view.children.len(),
                    },
                }))
            }
        }
        Command::AcceptCombat { workspace } => {
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = analysis.accept_combat_incumbent()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
        }
        Command::AcceptCombatActions { workspace, actions } => {
            let action_lists = actions
                .iter()
                .map(|path| {
                    serde_json::from_slice::<Vec<ClientInput>>(
                        &std::fs::read(path).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| {
                        format!(
                            "invalid combat witness action list '{}': {error}",
                            path.display()
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let actions = action_lists.into_iter().flatten().collect::<Vec<_>>();
            let mut analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let view = analysis.accept_combat_actions(&actions)?;
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
        Command::History {
            workspace,
            node,
            journal,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            if journal {
                print_json(&analysis.session.journal_entries(node)?)
            } else {
                print_json(&analysis.session.replay(node)?)
            }
        }
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
                    "action": combat_action_label(position, input),
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
        let action_key = combat_action_label(&position, &action.input);
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
                "end": combat_turn_snapshot(&position),
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
            "end": combat_turn_snapshot(&position),
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

fn combat_action_label(
    position: &sts_oracle_runtime::sim::combat::CombatPosition,
    input: &ClientInput,
) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => position
            .combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                let target = compact_target_label(&position.combat, *target);
                if target == "none" {
                    format!("play {}", card_label(card))
                } else {
                    format!("play {} -> {target}", card_label(card))
                }
            })
            .unwrap_or_else(|| combat_action_key(&position.combat, input)),
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            let potion = position
                .combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(Option::as_ref)
                .map(|potion| format!("{:?}", potion.id))
                .unwrap_or_else(|| format!("slot {potion_index}"));
            let target = compact_target_label(&position.combat, *target);
            if target == "none" {
                format!("use {potion}")
            } else {
                format!("use {potion} -> {target}")
            }
        }
        ClientInput::EndTurn => "end turn".to_string(),
        ClientInput::SubmitSelection(resolution) => {
            let selected = resolution
                .selected_card_uuids()
                .into_iter()
                .map(|uuid| combat_card_uuid_label(&position.combat, uuid))
                .collect::<Vec<_>>()
                .join(", ");
            format!("select {selected}")
        }
        _ => combat_action_key(&position.combat, input),
    }
}

fn readable_turn_option_action_labels(
    root: &sts_oracle_runtime::sim::combat::CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<Vec<String>, String> {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut labels = Vec::with_capacity(actions.len());
    for action in actions {
        labels.push(combat_action_label(&position, &action.input));
        let step = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return Err(
                "generated option action could not be replayed while formatting".to_string(),
            );
        }
        position = step.position;
    }
    Ok(labels)
}

fn target_atomic_policy_trace(
    initial: &sts_oracle_runtime::sim::combat::CombatPosition,
    target: &[ClientInput],
    max_engine_steps_per_transition: usize,
) -> Result<
    (
        Vec<Value>,
        String,
        Vec<sts_oracle_runtime::sim::combat::CombatPosition>,
    ),
    String,
> {
    const UNIFORM_EXPLORATION: f64 = 0.05;

    let stepper = EngineCombatStepper;
    let mut position = initial.clone();
    let mut trace = Vec::with_capacity(target.len());
    let mut prefix_positions = Vec::with_capacity(target.len());
    for (step_index, input) in target.iter().enumerate() {
        let legal = stepper.atomic_actions(&position);
        let weights =
            sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_atomic_action_policy_weights(
                &position,
                &legal,
            );
        let target_index = legal.iter().position(|candidate| candidate == input);
        let exact_input_is_legal =
            target_index.is_some() || stepper.choice_for_legal_input(&position, input).is_some();
        let (ordinal_rank, raw_weight, probability, negative_log_probability) = target_index
            .and_then(|index| weights.get(index).copied().map(|weight| (index, weight)))
            .map_or((None, None, None, None), |(_, weight)| {
                let rank = 1 + weights
                    .iter()
                    .filter(|candidate| **candidate > weight)
                    .count();
                let total = weights.iter().sum::<f64>();
                let uniform = 1.0 / weights.len().max(1) as f64;
                let probability = ((1.0 - UNIFORM_EXPLORATION) * (weight / total)
                    + UNIFORM_EXPLORATION * uniform)
                    .max(f64::MIN_POSITIVE);
                (
                    Some(rank),
                    Some(weight),
                    Some(probability),
                    Some(-probability.ln()),
                )
            });
        trace.push(json!({
            "step": step_index,
            "turn": position.combat.turn.turn_count,
            "action": combat_action_label(&position, input),
            "legal_action_count": legal.len(),
            "ordinal_rank": ordinal_rank,
            "raw_weight": raw_weight,
            "probability": probability,
            "negative_log_probability": negative_log_probability,
            "surface": if target_index.is_some() { "atomic" } else { "structured_selection" },
        }));
        if !exact_input_is_legal {
            return Err(format!(
                "target action {step_index} is not on the exact legal action surface: {input:?}"
            ));
        }
        let result = stepper.apply_to_stable(
            &position,
            input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if result.truncated {
            return Err(format!(
                "target action {step_index} exceeded the exact transition limit"
            ));
        }
        position = result.position;
        prefix_positions.push(position.clone());
    }
    Ok((
        trace,
        sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
            &position.engine,
            &position.combat,
        ),
        prefix_positions,
    ))
}

fn compact_target_label(
    combat: &sts_oracle_runtime::runtime::combat::CombatState,
    target: Option<usize>,
) -> String {
    let Some(target) = target else {
        return "none".to_string();
    };
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target)
        .map(|monster| {
            let label = EnemyId::from_id(monster.monster_type)
                .map(|enemy| enemy.get_name())
                .unwrap_or("Unknown");
            format!("{label}[{}]", monster.slot)
        })
        .unwrap_or_else(|| target_label(combat, Some(target)))
}

fn combat_card_uuid_label(
    combat: &sts_oracle_runtime::runtime::combat::CombatState,
    uuid: u32,
) -> String {
    combat
        .zones
        .hand
        .iter()
        .chain(&combat.zones.draw_pile)
        .chain(&combat.zones.discard_pile)
        .chain(&combat.zones.exhaust_pile)
        .find(|card| card.uuid == uuid)
        .map(card_label)
        .unwrap_or_else(|| format!("card#{uuid}"))
}

fn combat_turn_snapshot(position: &sts_oracle_runtime::sim::combat::CombatPosition) -> Value {
    let combat = &position.combat;
    let player = &combat.entities.player;
    json!({
        "hp": player.current_hp,
        "block": player.block,
        "energy": combat.turn.energy,
        "player_powers": combat_power_labels(combat, player.id),
        "hand": combat.zones.hand.iter().map(card_label).collect::<Vec<_>>().join(" | "),
        "piles": format!("draw {} / discard {} / exhaust {}", combat.zones.draw_pile.len(), combat.zones.discard_pile.len(), combat.zones.exhaust_pile.len()),
        "monsters": combat.entities.monsters.iter().map(|monster| monster_state_label(combat, monster)).collect::<Vec<_>>(),
    })
}

fn combat_position_snapshot(position: &sts_oracle_runtime::sim::combat::CombatPosition) -> Value {
    let combat = &position.combat;
    let player = &combat.entities.player;
    json!({
        "turn": combat.turn.turn_count,
        "phase": format!("{:?}", combat.turn.current_phase),
        "player": {
            "hp": player.current_hp,
            "max_hp": player.max_hp,
            "block": player.block,
            "energy": combat.turn.energy,
            "powers": combat_power_labels(combat, player.id),
        },
        "hand": combat.zones.hand.iter().map(card_label).collect::<Vec<_>>().join(" | "),
        "piles": format!("draw {} / discard {} / exhaust {}", combat.zones.draw_pile.len(), combat.zones.discard_pile.len(), combat.zones.exhaust_pile.len()),
        "monsters": combat.entities.monsters.iter().map(|monster| monster_state_label(combat, monster)).collect::<Vec<_>>(),
    })
}

fn combat_power_labels(
    combat: &sts_oracle_runtime::runtime::combat::CombatState,
    entity: sts_oracle_runtime::EntityId,
) -> Vec<String> {
    sts_oracle_runtime::content::powers::store::powers_for(combat, entity)
        .unwrap_or_default()
        .iter()
        .map(|power| format!("{:?}:{}", power.power_type, power.amount))
        .collect()
}

fn monster_state_label(
    combat: &sts_oracle_runtime::runtime::combat::CombatState,
    monster: &sts_oracle_runtime::runtime::combat::MonsterEntity,
) -> String {
    let label = EnemyId::from_id(monster.monster_type)
        .map(|enemy| enemy.get_name())
        .unwrap_or("Unknown");
    if !monster.is_alive_for_action() {
        return format!("{label}[{}] dead", monster.slot);
    }
    let intent = monster
        .move_state
        .planned_visible_spec
        .as_ref()
        .map(|intent| format!("{intent:?}"))
        .unwrap_or_else(|| format!("move:{}", monster.planned_move_id()));
    let powers = combat_power_labels(combat, monster.id);
    let powers = if powers.is_empty() {
        String::new()
    } else {
        format!(" powers=[{}]", powers.join(", "))
    };
    format!(
        "{label}[{}] {}/{} block={} intent={intent}{powers}",
        monster.slot, monster.current_hp, monster.max_hp, monster.block
    )
}

fn card_label(card: &sts_oracle_runtime::runtime::combat::CombatCard) -> String {
    let upgrade = if card.upgrades == 0 {
        String::new()
    } else {
        format!("+{}", card.upgrades)
    };
    format!("{}{}", cards::java_id(card.id), upgrade)
}

fn compact_corridor_report(report: Option<&Value>) -> Value {
    let Some(report) = report else {
        return Value::Null;
    };
    let states = report
        .get("states")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let reached = states
        .iter()
        .filter(|state| {
            state
                .get("membership")
                .and_then(|membership| membership.get("accepted"))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let first_missing = states.iter().find(|state| {
        let accepted = state
            .get("membership")
            .and_then(|membership| membership.get("accepted"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        !accepted
    });
    let furthest_accepted = states.iter().rev().find(|state| {
        state
            .get("membership")
            .and_then(|membership| membership.get("accepted"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    json!({
        "kind": report.get("kind"),
        "guide": report.get("guide"),
        "authority": report.get("authority"),
        "exact_turn_states": report.get("exact_turn_states"),
        "accepted_turn_states": reached,
        "first_missing_rank": first_missing
            .and_then(|state| state.get("corridor_rank")),
        "first_missing": first_missing,
        "furthest_accepted": furthest_accepted,
        "terminal": report.get("terminal"),
        "terminal_final_hp": report.get("terminal_final_hp"),
    })
}

fn compact_combat_trace(trace: Option<&Value>) -> Value {
    let Some(trace) = trace else {
        return Value::Null;
    };
    let turns = trace
        .get("turns")
        .and_then(Value::as_array)
        .map(|turns| {
            turns
                .iter()
                .map(|turn| {
                    let end = turn.get("end");
                    json!({
                        "turn": turn.get("turn"),
                        "action_range": turn.get("action_range"),
                        "start_hp": turn.get("start_hp"),
                        "actions": turn.get("actions"),
                        "end": {
                            "hp": end.and_then(|value| value.get("hp")),
                            "block": end.and_then(|value| value.get("block")),
                            "energy": end.and_then(|value| value.get("energy")),
                            "hand": end.and_then(|value| value.get("hand")),
                            "piles": end.and_then(|value| value.get("piles")),
                            "player_powers": end.and_then(|value| value.get("player_powers")),
                            "monsters": end.and_then(|value| value.get("monsters")),
                        },
                        "terminal": turn.get("terminal"),
                        "partial": turn.get("partial"),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "action_count": trace.get("action_count"),
        "turns": turns,
        "terminal": trace.get("terminal"),
    })
}

fn compact_local_corridor_report(report: Option<&Value>) -> Value {
    let Some(report) = report else {
        return Value::Null;
    };
    let states = report
        .get("states")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let reached = states
        .iter()
        .filter(|state| state.get("state").is_some_and(|value| !value.is_null()))
        .count();
    let first_missing = states
        .iter()
        .find(|state| state.get("state").is_none_or(Value::is_null));
    let furthest_reached_index = states
        .iter()
        .rposition(|state| state.get("state").is_some_and(|value| !value.is_null()));
    let furthest_reached = furthest_reached_index.and_then(|index| states.get(index));
    let incoming_to_furthest = furthest_reached_index
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| states.get(index))
        .and_then(|state| state.get("outgoing_to_next"))
        .filter(|value| !value.is_null());
    json!({
        "authority": report.get("authority"),
        "changes_search_order": report.get("changes_search_order"),
        "action_count": report.get("action_count"),
        "exact_turn_states": report.get("exact_turn_states"),
        "reached_turn_states": reached,
        "first_missing_rank": first_missing
            .and_then(|state| state.get("corridor_rank")),
        "first_missing": first_missing,
        "incoming_to_furthest": incoming_to_furthest,
        "furthest_reached": furthest_reached,
        "terminal_final_hp": report.get("terminal_final_hp"),
    })
}

#[derive(Clone, Debug, Default, Serialize)]
struct CombatPlanTransitionServiceAggregateV1 {
    generated_edges: usize,
    edge_served_edges: usize,
    unserved_edges: usize,
    successor_visited_edges: usize,
    root_generated_edges: usize,
    root_edge_served_edges: usize,
    total_edge_visits: usize,
    total_anchor_visits: usize,
    total_guide_visits: usize,
    total_backed_visits: usize,
    total_successor_visits: usize,
    minimum_negative_log_policy: Option<f64>,
    minimum_action_count: Option<usize>,
    maximum_player_hp_before: Option<i32>,
    maximum_player_hp_after: Option<i32>,
    maximum_visible_damage_margin_after: Option<i32>,
    maximum_player_intangible_before: Option<i32>,
    maximum_player_intangible_after: Option<i32>,
    maximum_strength_reduction_before: Option<u16>,
    maximum_strength_reduction_after: Option<u16>,
    maximum_intangible_sources_before: Option<u16>,
    maximum_intangible_sources_after: Option<u16>,
    minimum_priority_target_hp_after: Option<i32>,
    minimum_phase_transition_damage_after: Option<i32>,
}

impl CombatPlanTransitionServiceAggregateV1 {
    fn observe(&mut self, edge: &LocalTurnGraphPlanTransitionEdgeSnapshot) {
        let (_, transition) = plan_transition_parts(&edge.plan_transition_annotation);
        self.generated_edges = self.generated_edges.saturating_add(1);
        if edge.edge_visits > 0 {
            self.edge_served_edges = self.edge_served_edges.saturating_add(1);
        } else {
            self.unserved_edges = self.unserved_edges.saturating_add(1);
        }
        if edge.successor_visits > 0 {
            self.successor_visited_edges = self.successor_visited_edges.saturating_add(1);
        }
        if edge.parent_relative_turn_depth == 0 {
            self.root_generated_edges = self.root_generated_edges.saturating_add(1);
            if edge.edge_visits > 0 {
                self.root_edge_served_edges = self.root_edge_served_edges.saturating_add(1);
            }
        }
        self.total_edge_visits = self.total_edge_visits.saturating_add(edge.edge_visits);
        self.total_anchor_visits = self.total_anchor_visits.saturating_add(edge.anchor_visits);
        self.total_guide_visits = self.total_guide_visits.saturating_add(edge.guide_visits);
        self.total_backed_visits = self.total_backed_visits.saturating_add(edge.backed_visits);
        self.total_successor_visits = self
            .total_successor_visits
            .saturating_add(edge.successor_visits);
        self.minimum_negative_log_policy = Some(
            self.minimum_negative_log_policy
                .map_or(edge.negative_log_policy, |current| {
                    current.min(edge.negative_log_policy)
                }),
        );
        self.minimum_action_count = Some(
            self.minimum_action_count
                .map_or(edge.action_count, |current| current.min(edge.action_count)),
        );
        self.maximum_player_hp_before = Some(
            self.maximum_player_hp_before
                .map_or(transition.envelope_before.player_hp, |current| {
                    current.max(transition.envelope_before.player_hp)
                }),
        );
        self.maximum_player_intangible_before = Some(self.maximum_player_intangible_before.map_or(
            transition.envelope_before.player_intangible_turns,
            |current| current.max(transition.envelope_before.player_intangible_turns),
        ));
        self.maximum_strength_reduction_before =
            Some(self.maximum_strength_reduction_before.map_or(
                transition.resources_before.remaining_strength_reduction,
                |current| current.max(transition.resources_before.remaining_strength_reduction),
            ));
        self.maximum_intangible_sources_before =
            Some(self.maximum_intangible_sources_before.map_or(
                transition.resources_before.remaining_intangible_sources,
                |current| current.max(transition.resources_before.remaining_intangible_sources),
            ));
        if let Some(envelope) = transition.envelope_after {
            self.maximum_player_hp_after = Some(
                self.maximum_player_hp_after
                    .map_or(envelope.player_hp, |current| {
                        current.max(envelope.player_hp)
                    }),
            );
            self.maximum_visible_damage_margin_after = Some(
                self.maximum_visible_damage_margin_after
                    .map_or(envelope.visible_damage_margin, |current| {
                        current.max(envelope.visible_damage_margin)
                    }),
            );
            self.maximum_player_intangible_after = Some(
                self.maximum_player_intangible_after
                    .map_or(envelope.player_intangible_turns, |current| {
                        current.max(envelope.player_intangible_turns)
                    }),
            );
            if let Some(target_hp) = envelope.priority_target_hp_with_block {
                self.minimum_priority_target_hp_after = Some(
                    self.minimum_priority_target_hp_after
                        .map_or(target_hp, |current| current.min(target_hp)),
                );
            }
            if let Some(damage) = envelope.phase_transition_damage_remaining {
                self.minimum_phase_transition_damage_after = Some(
                    self.minimum_phase_transition_damage_after
                        .map_or(damage, |current| current.min(damage)),
                );
            }
        }
        if let Some(resources) = transition.resources_after {
            self.maximum_strength_reduction_after = Some(
                self.maximum_strength_reduction_after
                    .map_or(resources.remaining_strength_reduction, |current| {
                        current.max(resources.remaining_strength_reduction)
                    }),
            );
            self.maximum_intangible_sources_after = Some(
                self.maximum_intangible_sources_after
                    .map_or(resources.remaining_intangible_sources, |current| {
                        current.max(resources.remaining_intangible_sources)
                    }),
            );
        }
    }
}

fn serialized_plan_label<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value).expect("typed plan labels must serialize") {
        Value::String(label) => label,
        other => other.to_string(),
    }
}

fn plan_transition_parts(
    annotation: &CombatPlanTransitionAnnotationV1,
) -> (&'static str, &CombatPlanTransitionV1) {
    match annotation {
        CombatPlanTransitionAnnotationV1::AwakenedOnePhaseControl(transition) => {
            ("awakened_one_phase_control", transition)
        }
        CombatPlanTransitionAnnotationV1::ChampPhaseControl(transition) => {
            ("champ_phase_control", transition)
        }
        CombatPlanTransitionAnnotationV1::DonuAndDecaGrowthControl(transition) => {
            ("donu_and_deca_growth_control", transition)
        }
    }
}

fn combat_plan_transition_portfolio_v1(session: &LocalTurnGraphWitnessSession) -> Value {
    let edges = session.plan_transition_edge_snapshots();
    let mut overall = CombatPlanTransitionServiceAggregateV1::default();
    let mut plans = BTreeMap::<String, CombatPlanTransitionServiceAggregateV1>::new();
    let mut stage_transitions = BTreeMap::<String, CombatPlanTransitionServiceAggregateV1>::new();
    let mut completed_milestones =
        BTreeMap::<String, CombatPlanTransitionServiceAggregateV1>::new();
    let mut events = BTreeMap::<String, CombatPlanTransitionServiceAggregateV1>::new();

    for edge in &edges {
        overall.observe(edge);
        let (plan, transition) = plan_transition_parts(&edge.plan_transition_annotation);
        plans.entry(plan.to_string()).or_default().observe(edge);
        let before = serialized_plan_label(&transition.before_stage);
        let after = transition
            .after_stage
            .as_ref()
            .map(serialized_plan_label)
            .unwrap_or_else(|| "terminal_or_unowned".to_string());
        stage_transitions
            .entry(format!("{before}->{after}"))
            .or_default()
            .observe(edge);
        for milestone in &transition.completed_milestones {
            completed_milestones
                .entry(serialized_plan_label(milestone))
                .or_default()
                .observe(edge);
        }
        for event in &transition.events {
            let event = serde_json::to_value(event).expect("typed plan events must serialize");
            let kind = event
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown_event");
            events.entry(kind.to_string()).or_default().observe(edge);
        }
    }

    json!({
        "schema_name": "CombatPlanTransitionPortfolioV1",
        "schema_version": 1,
        "authority": "diagnostic_only",
        "changes_search_order": false,
        "overall": overall,
        "plans": plans,
        "stage_transitions": stage_transitions,
        "completed_milestones": completed_milestones,
        "events": events,
    })
}

fn oracle_lab_guide_lane_label(lane_id: u32) -> &'static str {
    match lane_id {
        1 => "progress",
        2 => "survival",
        3 => "horizon",
        4 => "setup",
        5 => "turn_depth",
        10_001 => "exact_corridor_control",
        10_002 => "typed_corridor_control",
        _ => "policy_defined",
    }
}

fn layered_candidate_view_ranks(
    candidates: &[LayeredCombatFrontierState],
    target_index: usize,
    policy: &dyn CombatActionPolicy,
) -> Value {
    let Some(target) = candidates.get(target_index) else {
        return Value::Null;
    };
    let policy_cost = |candidate: &LayeredCombatFrontierState| {
        candidate.negative_log_policy + (candidate.actions.len().max(1) as f64).ln()
    };
    let target_policy_cost = policy_cost(target);
    let anchor_rank = candidates
        .iter()
        .filter(|candidate| {
            policy_cost(candidate)
                .total_cmp(&target_policy_cost)
                .then_with(|| candidate.exact_state_hash.cmp(&target.exact_state_hash))
                .is_lt()
        })
        .count()
        .saturating_add(1);
    let target_guides = policy.state_guides(&target.position);
    let guide_ranks = target_guides
        .iter()
        .map(|target_guide| {
            let ordinal_rank = candidates
                .iter()
                .filter(|candidate| {
                    let candidate_guide = policy
                        .state_guides(&candidate.position)
                        .into_iter()
                        .find(|guide| guide.lane == target_guide.lane);
                    candidate_guide.is_some_and(|candidate_guide| {
                        candidate_guide
                            .rank
                            .cmp(&target_guide.rank)
                            .then_with(|| target_policy_cost.total_cmp(&policy_cost(candidate)))
                            .then_with(|| target.exact_state_hash.cmp(&candidate.exact_state_hash))
                            .is_gt()
                    })
                })
                .count()
                .saturating_add(1);
            json!({
                "lane_id": target_guide.lane.value(),
                "lane": oracle_lab_guide_lane_label(target_guide.lane.value()),
                "ordinal_rank": ordinal_rank,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "candidate_count": candidates.len(),
        "anchor_rank": anchor_rank,
        "guide_ranks": guide_ranks,
    })
}

fn existing_combat_guide_diagnostics(
    position: &sts_oracle_runtime::sim::combat::CombatPosition,
) -> Value {
    json!({
        "progress": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(position),
        "survival": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(position),
        "horizon": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(position),
        "setup": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(position),
    })
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize oracle_lab output: {error}"))?
    );
    Ok(())
}

fn lineage_portfolio_entries_json(
    entries: &[LayeredCombatLineagePortfolioEntryReport],
) -> Vec<Value> {
    entries
        .iter()
        .map(|entry| {
            json!({
                "parent_candidate_index": entry.parent_candidate_index,
                "parent_exact_state_hash": entry.parent_exact_state_hash,
                "parent_consensus_rank": entry.parent_consensus_rank,
                "source_window_index": entry.source_window_index,
                "window_discrepancy": entry.window_discrepancy,
                "generation_work": entry.generation_work,
                "engine_steps": entry.engine_steps,
                "recursive_splits_remaining": entry.recursive_splits_remaining,
                "terminal": entry.terminal,
                "found_witness": entry.found_witness,
                "child_entries": lineage_portfolio_entries_json(&entry.child_entries),
            })
        })
        .collect()
}
