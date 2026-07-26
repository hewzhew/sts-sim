//! Heavy offline and exact-search command frontend for the dedicated oracle runtime.

mod action_reanalysis_policy;
mod action_reanalysis_queue;
mod action_successor_reanalysis;
mod atomic_policy_searches;
mod boundary_successor_corpus;
mod boundary_successor_lookahead;
mod combat_case_atomic_turn_portfolio;
mod combat_case_fold_solved_suffix;
mod combat_case_layered;
mod combat_case_layered_window_race;
mod combat_case_legacy_global;
mod combat_case_local_graph;
mod combat_plan_diagnostics;
mod depth_beam_audits;
mod exact_combat_evidence;
mod policy_discrepancy_search;
mod run_witness_suite;
mod turn_audits;
mod turn_membership_audit;
mod v2_capability_audit;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use atomic_policy_searches::CombatCaseAtomicLevinArgs;
use blake2::{Blake2b512, Digest};
use clap::{Args, Parser, Subcommand, ValueEnum};
use combat_case_atomic_turn_portfolio::CombatCaseAtomicTurnPortfolioArgs;
use combat_case_fold_solved_suffix::CombatCaseFoldSolvedSuffixArgs;
use combat_case_layered::CombatCaseLayeredArgs;
use combat_case_layered_window_race::CombatCaseLayeredWindowRaceArgs;
use combat_case_legacy_global::CombatCaseLegacyGlobalArgs;
use combat_case_local_graph::CombatCaseLocalGraphArgs;
use combat_plan_diagnostics::{CombatCasePlanAnnotationsArgs, CombatCasePlanTraceArgs};
use depth_beam_audits::{DepthBeamAgendaAuditArgs, DepthBeamTurnAuditArgs};
use policy_discrepancy_search::CombatCasePolicyDiscrepancyArgs;
use run_witness_suite::RunWitnessSuiteArgs;
use serde::{Deserialize, Serialize};
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
    UniformCombatActionPolicy,
};
use sts_combat_strategy::{
    awakened_one_combat_plan_v1, awakened_one_plan_transition_v1, CombatPlanTransitionAnnotationV1,
    CombatPlanTransitionV1,
};
use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
};
use sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1;
use sts_simulator::content::{cards, monsters::EnemyId};
use sts_simulator::eval::combat_action_imitation::{
    audit_combat_action_imitation_v1, combat_action_imitation_policy_v1,
    root_player_turn_action_policy_v1,
    train_combat_action_imitation_from_demonstrations_with_base_v1,
    train_combat_action_imitation_v1, CombatActionImitationArtifactV1,
    CombatActionImitationDemonstrationV1, CombatActionImitationTrainingConfigV1,
};
use sts_simulator::eval::combat_case::{
    load_combat_case, save_combat_case, CombatCase, CombatCaseGap, CombatCasePathStep,
    CombatCaseRngSummary, CombatCaseRunSummary, CombatCaseSource,
};
use sts_simulator::eval::combat_guidance_bundle::{
    combat_value_prototype_policy_v1, combat_value_prototype_rank_v1,
    typed_combat_value_features_v1, CombatGuidanceBundleV1, CombatValuePrototypeArtifactV1,
    GUIDE_LEARNED_BOUNDARY_VALUE,
};
use sts_simulator::eval::combat_search_v2::{
    run_combat_root_proposal_probe_v1, CombatRootProposalProbeV1Report, CombatSearchV2LoadedStart,
    CombatSearchV2RunOptions,
};
use sts_simulator::eval::run_control::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
    ExistingCombatKnowledgeAdvisorAdvanceV1, ExistingCombatKnowledgeAdvisorV1,
    OracleAnalysisAdvanceRequestV1, OracleAnalysisNodeViewV1, RunProgressStepV1,
};
use sts_simulator::runtime::branch::{
    load_oracle_analysis_workspace_v1, load_oracle_run_continuation_v1,
    oracle_live_combat_diagnostic_v1, save_oracle_analysis_workspace_v1,
    save_oracle_run_continuation_v1, OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig,
    OracleRunContinuationV1,
};
use sts_simulator::sim::combat::{
    combat_terminal, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_simulator::sim::combat_action::{combat_action_key, target_label};
use sts_simulator::state::core::{ClientInput, EngineState};
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
enum ShadowCorridorGuide {
    #[default]
    Exact,
    TypedFeature,
}

const COMBAT_ACTION_IMITATION_CORPUS_SCHEMA_NAME: &str = "CombatActionImitationCorpusManifestV1";
const COMBAT_ACTION_IMITATION_CORPUS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CombatActionImitationCorpusManifestV1 {
    schema_name: String,
    schema_version: u32,
    demonstrations: Vec<CombatActionImitationCorpusEntryV1>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CombatActionImitationCorpusEntryV1 {
    id: String,
    case: PathBuf,
    actions: Vec<PathBuf>,
}

struct LoadedCombatActionImitationDemonstrationV1 {
    id: String,
    case_path: PathBuf,
    action_paths: Vec<PathBuf>,
    position: sts_simulator::sim::combat::CombatPosition,
    actions: Vec<ClientInput>,
}

#[derive(Clone, Debug)]
struct ExactTurnCorridor {
    rank_by_exact_hash: HashMap<String, i32>,
    atomic_rank_by_exact_hash: HashMap<String, i32>,
    typed_target_by_turn: HashMap<u32, (i32, Vec<i32>)>,
    positions_by_rank: Vec<sts_simulator::sim::combat::CombatPosition>,
    transition_actions: Vec<Vec<ClientInput>>,
    action_count: usize,
    terminal_final_hp: i32,
}

impl ExactTurnCorridor {
    fn membership_states(&self, search: &OracleCombatWitnessSession) -> Vec<Value> {
        let mut memberships = search.compact_state_memberships_by_exact_hashes(
            self.rank_by_exact_hash.keys().map(String::as_str),
        );
        let mut states = self
            .rank_by_exact_hash
            .iter()
            .map(|(exact_hash, rank)| {
                let membership = memberships
                    .remove(exact_hash)
                    .expect("bulk corridor membership includes every requested hash");
                (*rank, membership)
            })
            .collect::<Vec<_>>();
        states.sort_by_key(|(rank, _)| *rank);
        states
            .into_iter()
            .map(|(rank, membership)| {
                json!({
                    "corridor_rank": rank,
                    "membership": membership,
                })
            })
            .collect()
    }

    fn report(&self, search: &OracleCombatWitnessSession, guide: ShadowCorridorGuide) -> Value {
        json!({
            "kind": match guide {
                ShadowCorridorGuide::Exact => "exact_verified_turn_corridor_shadow",
                ShadowCorridorGuide::TypedFeature => "typed_feature_corridor_shadow",
            },
            "authority": "guide_only",
            "exact_turn_states": self.rank_by_exact_hash.len(),
            "exact_atomic_prefix_states": self.atomic_rank_by_exact_hash.len(),
            "typed_feature_targets": self.typed_target_by_turn.len(),
            "typed_feature_count": self.typed_target_by_turn.values().next().map(|(_, features)| features.len()).unwrap_or_default(),
            "action_count": self.action_count,
            "terminal": "Win",
            "terminal_final_hp": self.terminal_final_hp,
            "states": self.membership_states(search),
        })
    }

    fn diagnostic_report(&self, search: &OracleCombatWitnessSession) -> Value {
        json!({
            "kind": "exact_verified_turn_corridor_watch",
            "authority": "diagnostic_only",
            "changes_search_order": false,
            "exact_turn_states": self.rank_by_exact_hash.len(),
            "action_count": self.action_count,
            "terminal": "Win",
            "terminal_final_hp": self.terminal_final_hp,
            "states": self.membership_states(search),
        })
    }
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
        position: &sts_simulator::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        family: &sts_simulator::sim::combat_action_surface::CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guides(
        &self,
        _position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        Vec::new()
    }

    fn turn_generation_guides(
        &self,
        _position: &sts_simulator::sim::combat::CombatPosition,
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
        position: &sts_simulator::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        family: &sts_simulator::sim::combat_action_surface::CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guides(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        if position.combat.turn.turn_count == self.root_player_turn {
            Vec::new()
        } else {
            self.base.state_guides(position)
        }
    }

    fn turn_generation_guides(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
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
        position: &sts_simulator::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        family: &sts_simulator::sim::combat_action_surface::CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guides(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        let mut ranks = if self.shadow_only {
            Vec::new()
        } else {
            self.base.state_guides(position)
        };
        match self.guide {
            ShadowCorridorGuide::Exact => {
                let exact_hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
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
        position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuide> {
        let mut ranks = if self.shadow_only {
            Vec::new()
        } else {
            self.base.turn_generation_guides(position)
        };
        match self.guide {
            ShadowCorridorGuide::Exact => {
                let exact_hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
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
        position: &sts_simulator::sim::combat::CombatPosition,
        target_turn: u32,
    ) -> CombatStateGuideRank {
        let shadow_rank = match self.guide {
            ShadowCorridorGuide::Exact => {
                let exact_hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
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

fn load_value_prototype(path: &Path) -> Result<CombatValuePrototypeArtifactV1, String> {
    CombatValuePrototypeArtifactV1::load(path)
}

fn save_value_prototype(
    path: &PathBuf,
    artifact: &CombatValuePrototypeArtifactV1,
) -> Result<(), String> {
    artifact.save(path)
}

fn value_prototype_from_corridor(
    corridor: &ExactTurnCorridor,
) -> Result<CombatValuePrototypeArtifactV1, String> {
    CombatValuePrototypeArtifactV1::from_ranked_features(
        "exact_terminal_win_demonstration",
        corridor.action_count,
        corridor.terminal_final_hp,
        corridor
            .typed_target_by_turn
            .iter()
            .map(|(player_turn, (value_rank, features))| {
                (*player_turn, *value_rank, features.clone())
            }),
    )
}

fn value_prototype_from_corridors(
    corridors: &[ExactTurnCorridor],
) -> Result<CombatValuePrototypeArtifactV1, String> {
    CombatValuePrototypeArtifactV1::from_ranked_feature_trajectories(
        "exact_terminal_win_demonstration_corpus",
        corridors.iter().map(|corridor| {
            (
                corridor.action_count,
                corridor.terminal_final_hp,
                corridor
                    .typed_target_by_turn
                    .iter()
                    .map(|(player_turn, (value_rank, features))| {
                        (*player_turn, *value_rank, features.clone())
                    })
                    .collect(),
            )
        }),
    )
}

fn typed_combat_feature_components(
    position: &sts_simulator::sim::combat::CombatPosition,
) -> Vec<i32> {
    typed_combat_value_features_v1(position)
}

fn load_exact_turn_corridor(
    case_path: &PathBuf,
    action_paths: &[PathBuf],
    max_engine_steps_per_transition: usize,
) -> Result<ExactTurnCorridor, String> {
    let case = load_combat_case(case_path)?;
    let actions = load_combat_action_segments(action_paths)?;
    exact_turn_corridor_from_position_and_actions(
        case.position,
        actions,
        max_engine_steps_per_transition,
    )
}

fn exact_turn_corridor_from_position_and_actions(
    mut position: sts_simulator::sim::combat::CombatPosition,
    actions: Vec<ClientInput>,
    max_engine_steps_per_transition: usize,
) -> Result<ExactTurnCorridor, String> {
    let stepper = EngineCombatStepper;
    let mut rank_by_exact_hash = HashMap::new();
    let mut atomic_rank_by_exact_hash = HashMap::new();
    let mut typed_target_by_turn = HashMap::new();
    let initial_exact_hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
        &position.engine,
        &position.combat,
    );
    rank_by_exact_hash.insert(initial_exact_hash.clone(), 0);
    atomic_rank_by_exact_hash.insert(initial_exact_hash, 0);
    typed_target_by_turn.insert(
        position.combat.turn.turn_count,
        (0, typed_combat_feature_components(&position)),
    );
    let mut next_turn_rank = 1i32;
    let mut positions_by_rank = vec![position.clone()];
    let mut transition_actions = Vec::new();
    let mut current_transition_actions = Vec::new();
    for (action_index, input) in actions.iter().enumerate() {
        if stepper.choice_for_legal_input(&position, input).is_none() {
            return Err(format!(
                "shadow corridor action {action_index} is not legal at turn {}: {input:?}",
                position.combat.turn.turn_count
            ));
        }
        let previous_turn = position.combat.turn.turn_count;
        current_transition_actions.push(input.clone());
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
                "shadow corridor action {action_index} exceeded the engine-step limit"
            ));
        }
        position = step.position;
        atomic_rank_by_exact_hash.insert(
            sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                &position.engine,
                &position.combat,
            ),
            i32::try_from(action_index.saturating_add(1)).unwrap_or(i32::MAX),
        );
        if step.terminal == sts_simulator::sim::combat::CombatTerminal::Unresolved
            && position.combat.turn.turn_count != previous_turn
        {
            transition_actions.push(std::mem::take(&mut current_transition_actions));
            positions_by_rank.push(position.clone());
            rank_by_exact_hash.insert(
                sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                    &position.engine,
                    &position.combat,
                ),
                next_turn_rank,
            );
            typed_target_by_turn.insert(
                position.combat.turn.turn_count,
                (next_turn_rank, typed_combat_feature_components(&position)),
            );
            next_turn_rank = next_turn_rank.saturating_add(1);
        }
    }
    if stepper.terminal(&position) != sts_simulator::sim::combat::CombatTerminal::Win {
        return Err("shadow corridor action list is not an exact terminal win".to_string());
    }
    if !current_transition_actions.is_empty() {
        transition_actions.push(current_transition_actions);
    }
    if transition_actions.len() != positions_by_rank.len() {
        return Err(format!(
            "verified corridor has {} boundaries but {} outgoing turn segments",
            positions_by_rank.len(),
            transition_actions.len()
        ));
    }
    Ok(ExactTurnCorridor {
        rank_by_exact_hash,
        atomic_rank_by_exact_hash,
        typed_target_by_turn,
        positions_by_rank,
        transition_actions,
        action_count: actions.len(),
        terminal_final_hp: position.combat.entities.player.current_hp,
    })
}

fn load_combat_action_segments(action_paths: &[PathBuf]) -> Result<Vec<ClientInput>, String> {
    let mut actions = Vec::new();
    for path in action_paths {
        let mut segment = serde_json::from_slice::<Vec<ClientInput>>(
            &std::fs::read(path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("invalid combat action segment {}: {error}", path.display()))?;
        actions.append(&mut segment);
    }
    Ok(actions)
}

fn load_combat_action_imitation_corpus(
    manifest_path: &Path,
) -> Result<Vec<LoadedCombatActionImitationDemonstrationV1>, String> {
    let manifest = serde_json::from_slice::<CombatActionImitationCorpusManifestV1>(
        &std::fs::read(manifest_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("invalid action imitation corpus manifest: {error}"))?;
    if manifest.schema_name != COMBAT_ACTION_IMITATION_CORPUS_SCHEMA_NAME
        || manifest.schema_version != COMBAT_ACTION_IMITATION_CORPUS_SCHEMA_VERSION
    {
        return Err("unsupported action imitation corpus manifest schema".to_string());
    }
    if manifest.demonstrations.is_empty() {
        return Err("action imitation corpus manifest has no demonstrations".to_string());
    }
    let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut seen_ids = std::collections::HashSet::new();
    manifest
        .demonstrations
        .into_iter()
        .map(|entry| {
            if entry.id.trim().is_empty() || !seen_ids.insert(entry.id.clone()) {
                return Err(format!(
                    "action imitation corpus demonstration id is empty or duplicated: {:?}",
                    entry.id
                ));
            }
            if entry.actions.is_empty() {
                return Err(format!(
                    "action imitation corpus demonstration {:?} has no action segments",
                    entry.id
                ));
            }
            let case_path = resolve_manifest_relative_path(base, &entry.case);
            let action_paths = entry
                .actions
                .iter()
                .map(|path| resolve_manifest_relative_path(base, path))
                .collect::<Vec<_>>();
            let case = load_combat_case(&case_path)?;
            let actions = load_combat_action_segments(&action_paths)?;
            Ok(LoadedCombatActionImitationDemonstrationV1 {
                id: entry.id,
                case_path,
                action_paths,
                position: case.position,
                actions,
            })
        })
        .collect()
}

fn resolve_manifest_relative_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
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
    let corridor = load_exact_turn_corridor(
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
    validate_canonical_launch(cli.canonical_oracle)?;
    match cli.command {
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
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            let continuation = analysis.continuation(node)?;
            let journal_entries = continuation.journal.entries().len();
            save_oracle_run_continuation_v1(&output, &continuation)?;
            print_json(&json!({
                "schema_name": "OracleAnalysisContinuationExportV1",
                "workspace": workspace,
                "node_id": node,
                "output": output,
                "journal_entries": journal_entries,
            }))
        }
        Command::RecoverCombatCase {
            workspace,
            branch,
            output,
        } => {
            let case = sts_simulator::runtime::branch::recover_oracle_analysis_combat_case_v1(
                &workspace, branch,
            )?;
            save_combat_case(&output, &case)?;
            print_json(&json!({
                "schema_name": "OracleRecoveredCombatCaseV1",
                "workspace": workspace,
                "branch_id": branch,
                "output": output,
                "source": case.source,
                "run": case.run,
                "combat": case.combat,
                "path_steps": case.path.len(),
            }))
        }
        Command::VerifyRunWitness { workspace, node } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let continuation = analysis.continuation(node)?;
            let expected_final = continuation.session.into_session()?;
            let report = sts_simulator::eval::run_control::exact_replay_run_progress_journal_v1(
                analysis.seed,
                analysis.ascension,
                &continuation.journal,
                &expected_final,
            )?;
            print_json(&json!({
                "schema_name": "ExactOracleRunWitnessReplayV1",
                "schema_version": 1,
                "workspace": workspace,
                "node_id": node,
                "report": report,
            }))
        }
        Command::AuditRunWitnessPolicy {
            workspace,
            node,
            details,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let continuation = analysis.continuation(node)?;
            let expected_final = continuation.session.into_session()?;
            let report =
                sts_simulator::eval::run_control::exact_audit_run_progress_journal_policy_v1(
                    analysis.seed,
                    analysis.ascension,
                    &continuation.journal,
                    &expected_final,
                    sts_simulator::runtime::branch::current_oracle_candidate_order_v1,
                )?;
            let report = if details {
                serde_json::to_value(report)
                    .map_err(|error| format!("failed to encode witness policy audit: {error}"))?
            } else {
                json!({
                    "replay": report.replay,
                    "decisions_with_owner_preferences": report.decisions_with_owner_preferences,
                    "decisions_without_owner_preferences": report.decisions_without_owner_preferences,
                    "rank_zero_agreements": report.rank_zero_agreements,
                    "nonzero_rank_choices": report.nonzero_rank_choices,
                    "choices_absent_from_owner_preferences": report.choices_absent_from_owner_preferences,
                    "discrepancy_sum": report.discrepancy_sum,
                    "max_owner_rank": report.max_owner_rank,
                    "first_divergence": report.first_divergence,
                    "combat_sources": report.combat_sources,
                })
            };
            print_json(&json!({
                "schema_name": "ExactOracleRunWitnessPolicyAuditV1",
                "schema_version": 1,
                "workspace": workspace,
                "node_id": node,
                "report": report,
            }))
        }
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
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let continuation = analysis.continuation(node)?;
            let replacement_analysis = load_oracle_analysis_workspace_v1(&replacement_workspace)?;
            let replacement_continuation = replacement_analysis.continuation(replacement_node)?;
            if continuation.seed != replacement_continuation.seed
                || continuation.ascension != replacement_continuation.ascension
            {
                return Err("combat splice requires matching seed and ascension".to_string());
            }
            let replacement = replacement_continuation
                .journal
                .entries()
                .iter()
                .rev()
                .find_map(RunProgressStepV1::as_combat_resolution)
                .ok_or_else(|| {
                    "replacement witness contains no committed combat resolution".to_string()
                })?;
            let original_source = continuation
                .journal
                .entries()
                .get(journal_entry)
                .and_then(RunProgressStepV1::as_combat_resolution)
                .map(|record| record.trajectory.source.label())
                .ok_or_else(|| {
                    format!("journal entry {journal_entry} is not a combat resolution")
                })?;
            let expected_final = continuation.session.clone().into_session()?;
            let (journal, replay) =
                sts_simulator::eval::run_control::splice_exact_combat_resolution_v1(
                    continuation.seed,
                    continuation.ascension,
                    &continuation.journal,
                    &expected_final,
                    journal_entry,
                    replacement,
                )?;
            let replacement_source = replacement.trajectory.source.label();
            let output_continuation = OracleRunContinuationV1 {
                schema_name: continuation.schema_name,
                schema_version: continuation.schema_version,
                seed: continuation.seed,
                ascension: continuation.ascension,
                journal,
                session: continuation.session,
                explorer_frontier: None,
            };
            save_oracle_run_continuation_v1(&output, &output_continuation)?;
            print_json(&json!({
                "schema_name": "ExactOracleCombatWitnessSpliceV1",
                "schema_version": 1,
                "workspace": workspace,
                "node_id": node,
                "journal_entry": journal_entry,
                "original_source": original_source,
                "replacement_workspace": replacement_workspace,
                "replacement_node_id": replacement_node,
                "replacement_source": replacement_source,
                "output": output,
                "replay": replay,
            }))
        }
        Command::ExportHistoricalCombatWitness {
            workspace,
            node,
            journal_entry,
            case_output,
            actions_output,
            continuation_output,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let continuation = analysis.continuation(node)?;
            let resolution = continuation
                .journal
                .entries()
                .get(journal_entry)
                .and_then(RunProgressStepV1::as_combat_resolution)
                .cloned()
                .ok_or_else(|| {
                    format!("journal entry {journal_entry} is not a combat resolution")
                })?;
            let expected_final = continuation.session.clone().into_session()?;
            let historical =
                sts_simulator::eval::run_control::exact_replay_run_progress_journal_prefix_v1(
                    continuation.seed,
                    continuation.ascension,
                    &continuation.journal,
                    &expected_final,
                    journal_entry,
                )?;
            let active = historical.active_combat.as_ref().ok_or_else(|| {
                format!("journal entry {journal_entry} does not begin at an active combat")
            })?;
            let position = sts_simulator::sim::combat::CombatPosition::new(
                active.engine_state.clone(),
                active.combat_state.clone(),
            );
            let path = continuation
                .journal
                .entries()
                .iter()
                .take(journal_entry)
                .filter_map(RunProgressStepV1::as_decision)
                .map(|record| CombatCasePathStep {
                    key: Value::Null,
                    label: record.result.chosen_label.clone(),
                    state_before: Some(json!({
                        "title": record.before.title,
                        "location": record.before.location,
                    })),
                    decision_evidence: Some(json!({
                        "candidate_id": record.selection.candidate_id,
                        "source": record.selection.source,
                        "candidates": record.before.candidates.iter()
                            .map(|candidate| &candidate.label)
                            .collect::<Vec<_>>(),
                    })),
                })
                .collect::<Vec<_>>();
            let case = CombatCase::new(
                CombatCaseSource {
                    seed: continuation.seed,
                    ascension: continuation.ascension,
                    generation: path.len(),
                    branch_id: node,
                    parent_id: None,
                },
                CombatCaseGap {
                    boundary: format!(
                        "Act {} Floor {} historical combat",
                        historical.run_state.act_num, historical.run_state.floor_num
                    ),
                    reason: "verified_run_witness_extraction".to_string(),
                    search_nodes: 0,
                    search_ms: 0,
                    rescue_search_nodes: 0,
                    rescue_search_ms: 0,
                },
                CombatCaseRunSummary {
                    act: historical.run_state.act_num,
                    floor: historical.run_state.floor_num,
                    hp: historical.run_state.current_hp,
                    max_hp: historical.run_state.max_hp,
                    gold: historical.run_state.gold,
                    deck_size: historical.run_state.master_deck.len(),
                    relic_count: historical.run_state.relics.len(),
                    potion_slots: historical.run_state.potions.len(),
                },
                Vec::new(),
                None,
                path,
                CombatCaseRngSummary::from_pool(&historical.run_state.rng_pool),
                position,
            );
            save_combat_case(&case_output, &case)?;
            if let Some(parent) = actions_output.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let actions = resolution
                .trajectory
                .actions
                .iter()
                .map(|action| action.input.clone())
                .collect::<Vec<_>>();
            std::fs::write(
                &actions_output,
                serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            if let Some(output) = &continuation_output {
                let prefix_journal =
                    sts_simulator::eval::run_control::RunProgressJournalV1::from_committed_steps(
                        continuation.journal.entries()[..journal_entry].to_vec(),
                    )?;
                let prefix = OracleRunContinuationV1 {
                    schema_name: continuation.schema_name,
                    schema_version: continuation.schema_version,
                    seed: continuation.seed,
                    ascension: continuation.ascension,
                    journal: prefix_journal,
                    session: sts_simulator::eval::run_control::RunControlSessionCheckpointV1::
                        from_session(&historical),
                    explorer_frontier: None,
                };
                save_oracle_run_continuation_v1(output, &prefix)?;
            }
            print_json(&json!({
                "schema_name": "HistoricalCombatWitnessExportV1",
                "schema_version": 1,
                "workspace": workspace,
                "node_id": node,
                "journal_entry": journal_entry,
                "source": resolution.trajectory.source.label(),
                "case_output": case_output,
                "actions_output": actions_output,
                "continuation_output": continuation_output,
                "action_count": actions.len(),
                "combat": case.combat,
            }))
        }
        Command::BuildValuePrototype {
            case,
            actions,
            output,
            max_engine_steps_per_transition,
        } => {
            let corridor =
                load_exact_turn_corridor(&case, &actions, max_engine_steps_per_transition)?;
            let artifact = value_prototype_from_corridor(&corridor)?;
            save_value_prototype(&output, &artifact)?;
            print_json(&json!({
                "output": output,
                "artifact": artifact.report(),
            }))
        }
        Command::BuildValuePrototypeCorpus {
            manifest,
            output,
            max_engine_steps_per_transition,
        } => {
            let demonstrations = load_combat_action_imitation_corpus(&manifest)?;
            let ids = demonstrations
                .iter()
                .map(|demonstration| demonstration.id.clone())
                .collect::<Vec<_>>();
            let corridors = demonstrations
                .into_iter()
                .map(|demonstration| {
                    exact_turn_corridor_from_position_and_actions(
                        demonstration.position,
                        demonstration.actions,
                        max_engine_steps_per_transition,
                    )
                    .map_err(|error| format!("demonstration {:?}: {error}", demonstration.id))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let artifact = value_prototype_from_corridors(&corridors)?;
            save_value_prototype(&output, &artifact)?;
            print_json(&json!({
                "output": output,
                "manifest": manifest,
                "demonstration_ids": ids,
                "artifact": artifact.report(),
            }))
        }
        Command::BuildCombatGuidanceBundle {
            action_imitation_artifact,
            value_prototype_artifact,
            output,
        } => {
            let action = CombatActionImitationArtifactV1::load(&action_imitation_artifact)?;
            let value = CombatValuePrototypeArtifactV1::load(&value_prototype_artifact)?;
            let bundle = CombatGuidanceBundleV1::new(
                "verified_exact_combat_witness_distillation",
                action,
                value,
            )?;
            bundle.save(&output)?;
            print_json(&json!({
                "output": output,
                "schema_name": bundle.schema_name,
                "schema_version": bundle.schema_version,
                "training_authority": bundle.training_authority,
                "action_source_trajectory_count": bundle.action_imitation.source_trajectory_count,
                "action_source_action_count": bundle.action_imitation.source_action_count,
                "value_source_trajectory_count": bundle.boundary_value.source_trajectory_count,
                "value_source_action_count": bundle.boundary_value.source_action_count,
                "runtime_reads_exact_hashes": false,
                "runtime_reads_witness_actions": false,
            }))
        }
        Command::BuildActionImitation {
            case,
            actions,
            output,
            max_engine_steps_per_transition,
        } => {
            let loaded = load_combat_case(&case)?;
            let actions = load_combat_action_segments(&actions)?;
            let training_config = CombatActionImitationTrainingConfigV1 {
                max_engine_steps_per_transition,
                ..CombatActionImitationTrainingConfigV1::default()
            };
            let artifact =
                train_combat_action_imitation_v1(&loaded.position, &actions, training_config)?;
            let training_audit = audit_combat_action_imitation_v1(
                &loaded.position,
                &actions,
                &artifact,
                &UniformCombatActionPolicy,
                training_config.max_structured_alternatives,
                max_engine_steps_per_transition,
            )?;
            artifact.save(&output)?;
            print_json(&json!({
                "schema_name": "OracleCombatActionImitationBuildV1",
                "schema_version": 1,
                "case": case,
                "output": output,
                "artifact": artifact,
                "training_audit": training_audit,
            }))
        }
        Command::BuildActionImitationCorpus {
            manifest,
            output,
            residual_over_existing_policy,
            max_engine_steps_per_transition,
        } => {
            let demonstrations = load_combat_action_imitation_corpus(&manifest)?;
            let training_config = CombatActionImitationTrainingConfigV1 {
                max_engine_steps_per_transition,
                base_weight_exponent: if residual_over_existing_policy {
                    1.0
                } else {
                    0.0
                },
                ..CombatActionImitationTrainingConfigV1::default()
            };
            let base_policy: SharedCombatActionPolicy = if residual_over_existing_policy {
                existing_combat_knowledge_policy_v1()
            } else {
                Arc::new(UniformCombatActionPolicy)
            };
            let borrowed = demonstrations
                .iter()
                .map(|demonstration| CombatActionImitationDemonstrationV1 {
                    root: &demonstration.position,
                    actions: &demonstration.actions,
                })
                .collect::<Vec<_>>();
            let artifact = train_combat_action_imitation_from_demonstrations_with_base_v1(
                &borrowed,
                training_config,
                base_policy.clone(),
            )?;
            let audits = demonstrations
                .iter()
                .map(|demonstration| {
                    audit_combat_action_imitation_v1(
                        &demonstration.position,
                        &demonstration.actions,
                        &artifact,
                        base_policy.as_ref(),
                        training_config.max_structured_alternatives,
                        max_engine_steps_per_transition,
                    )
                    .map(|audit| {
                        json!({
                            "id": demonstration.id,
                            "source_action_count": audit.source_action_count,
                            "ranked_decision_count": audit.ranked_decision_count,
                            "skipped_forced_decision_count": audit.skipped_forced_decision_count,
                            "miss_count": audit.misses.len(),
                        })
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            artifact.save(&output)?;
            print_json(&json!({
                "schema_name": "OracleCombatActionImitationCorpusBuildV1",
                "schema_version": 1,
                "manifest": manifest,
                "output": output,
                "training_base": if residual_over_existing_policy {
                    "existing_combat_knowledge_v1"
                } else {
                    "uniform"
                },
                "artifact": {
                    "schema_name": artifact.schema_name,
                    "schema_version": artifact.schema_version,
                    "feature_schema": artifact.feature_schema,
                    "runtime_compatibility_id": artifact.runtime_compatibility_id,
                    "source_trajectory_count": artifact.source_trajectory_count,
                    "source_action_count": artifact.source_action_count,
                    "ranked_decision_count": artifact.ranked_decision_count,
                    "pairwise_comparison_count": artifact.pairwise_comparison_count,
                    "skipped_forced_decision_count": artifact.skipped_forced_decision_count,
                    "training_top1_correct": artifact.training_top1_correct,
                    "training_top1_total": artifact.training_top1_total,
                    "coefficient_count": artifact.coefficients.len(),
                },
                "demonstrations": audits,
            }))
        }
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
        } => {
            let loaded = load_combat_case(&case)?;
            let actions = load_combat_action_segments(&actions)?;
            let artifact_value = CombatActionImitationArtifactV1::load(&artifact)?;
            let base_policy = existing_combat_knowledge_policy_v1();
            let audit = audit_combat_action_imitation_v1(
                &loaded.position,
                &actions,
                &artifact_value,
                base_policy.as_ref(),
                CombatActionImitationTrainingConfigV1::default().max_structured_alternatives,
                max_engine_steps_per_transition,
            )?;
            print_json(&json!({
                "schema_name": "OracleCombatActionImitationAuditV1",
                "schema_version": 1,
                "case": case,
                "artifact": artifact,
                "artifact_source_trajectory_count": artifact_value.source_trajectory_count,
                "audit": audit,
            }))
        }
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
            let view = selected_analysis_view(&analysis, node)?;
            print_json(&compact_node_summary(&view, limit))
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
            print_json(&compact_node_summary(&view, 8))
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
                "status": compact_node_summary(&analysis.view()?, 8),
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
            print_json(&compact_timeline(&analysis, node, tail)?)
        }
        Command::ExportCombatCase {
            workspace,
            node,
            output,
        } => {
            let analysis = load_oracle_analysis_workspace_v1(&workspace)?;
            let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
            let case = analysis_combat_case(&analysis, node)?;
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

fn selected_analysis_view(
    analysis: &OracleAnalysisWorkspaceV1,
    node: Option<usize>,
) -> Result<OracleAnalysisNodeViewV1, String> {
    if let Some(node) = node {
        analysis.session.view_node(node)
    } else {
        analysis.view()
    }
}

fn compact_node_summary(view: &OracleAnalysisNodeViewV1, limit: usize) -> Value {
    let choices = view
        .choices
        .iter()
        .take(limit)
        .map(|choice| {
            json!({
                "choice_ref": choice.choice_ref,
                "kind": choice.kind,
                "candidate_id": choice.candidate_id,
                "label": choice.label,
                "owner_rank": choice.owner_rank,
                "path_discrepancy": choice.path_discrepancy,
            })
        })
        .collect::<Vec<_>>();
    let children = view
        .children
        .iter()
        .take(limit)
        .map(|child| {
            json!({
                "edge_id": child.edge_id,
                "child_node_id": child.child_node_id,
                "kind": child.kind,
                "label": child.label,
                "is_on_mainline": child.is_on_mainline,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "node": view.node_id,
        "parent": view.canonical_parent_node_id,
        "act": view.act,
        "floor": view.floor,
        "hp": view.current_hp,
        "max_hp": view.max_hp,
        "gold": view.gold,
        "boundary": view.boundary,
        "event": view.event,
        "choice_count": view.choices.len(),
        "choices_shown": choices.len(),
        "choices_truncated": view.choices.len() > choices.len(),
        "choices": choices,
        "child_count": view.children.len(),
        "children_shown": children.len(),
        "children_truncated": view.children.len() > children.len(),
        "children": children,
        "encounter": view.encounter,
        "combat": view.combat,
    })
}

fn compact_timeline(
    analysis: &OracleAnalysisWorkspaceV1,
    node: usize,
    tail: usize,
) -> Result<Value, String> {
    let entries = analysis.session.journal_entries(node)?;
    let start = entries.len().saturating_sub(tail);
    let compact = entries[start..]
        .iter()
        .enumerate()
        .map(|(offset, entry)| match entry {
            RunProgressStepV1::Decision(record) => json!({
                "journal_index": start + offset,
                "kind": "decision",
                "location": record.before.location,
                "title": record.before.title,
                "chosen": record.result.chosen_label,
                "candidates": record.before.candidates.iter().map(|candidate| &candidate.label).collect::<Vec<_>>(),
            }),
            RunProgressStepV1::ForcedTransition(record) => json!({
                "journal_index": start + offset,
                "kind": "forced_transition",
                "location": record.before.location,
                "title": record.before.title,
            }),
            RunProgressStepV1::CombatResolution(record) => json!({
                "journal_index": start + offset,
                "kind": "combat_resolution",
                "location": record.before.location,
                "title": record.before.title,
                "resolution": record.kind,
                "actions": record.trajectory.action_count,
                "changes": record.result.changes,
            }),
            RunProgressStepV1::Stop(record) => json!({
                "journal_index": start + offset,
                "kind": "stop",
                "stop_kind": record.kind,
                "reason": record.reason,
            }),
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "node": node,
        "total_entries": entries.len(),
        "returned_entries": compact.len(),
        "entries": compact,
    }))
}

fn analysis_combat_case(
    analysis: &OracleAnalysisWorkspaceV1,
    node: usize,
) -> Result<CombatCase, String> {
    let view = analysis.session.view_node(node)?;
    let (search_nodes, search_ms) = if view.encounter.as_ref().is_some_and(|it| it.is_boss) {
        (analysis.budget.boss_nodes, analysis.budget.boss_ms)
    } else if view.encounter.as_ref().is_some_and(|it| it.is_elite) {
        (analysis.budget.elite_nodes, analysis.budget.elite_ms)
    } else {
        (analysis.budget.hallway_nodes, analysis.budget.hallway_ms)
    };
    analysis.session.combat_case(
        node,
        analysis.seed,
        analysis.ascension,
        search_nodes,
        search_ms,
    )
}

fn validate_canonical_launch(canonical_oracle: bool) -> Result<(), String> {
    const REQUIRED_PROFILE: &str = "release";
    const BUILT_PROFILE: &str = env!("STS_CARGO_PROFILE");
    const REPOSITORY_ROOT: &str = env!("STS_REPOSITORY_ROOT");

    if !canonical_oracle {
        return Err(
            "oracle_lab refuses direct execution; run `cargo oracle-lab <command> ...`".to_string(),
        );
    }
    if BUILT_PROFILE != REQUIRED_PROFILE {
        return Err(format!(
            "oracle_lab was built with forbidden profile `{BUILT_PROFILE}`; \
             run `cargo oracle-lab <command> ...`"
        ));
    }
    let executable_name = if cfg!(windows) {
        "oracle_lab.exe"
    } else {
        "oracle_lab"
    };
    let expected = PathBuf::from(REPOSITORY_ROOT)
        .join("target")
        .join(REQUIRED_PROFILE)
        .join(executable_name);
    let current = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("failed to identify running oracle_lab: {error}"))?;
    let expected = expected.canonicalize().map_err(|error| {
        format!(
            "canonical oracle_lab artifact is missing at {}: {error}; \
             run `cargo oracle-lab <command> ...`",
            expected.display()
        )
    })?;
    if current != expected {
        return Err(format!(
            "oracle_lab refuses non-canonical artifact {}; expected {}; \
             run `cargo oracle-lab <command> ...`",
            current.display(),
            expected.display()
        ));
    }
    validate_source_freshness(&expected)?;
    Ok(())
}

fn validate_source_freshness(executable: &Path) -> Result<(), String> {
    let executable_modified = std::fs::metadata(executable)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "failed to inspect canonical oracle laboratory '{}': {error}",
                executable.display()
            )
        })?;
    let depfile = executable.with_extension("d");
    let depfile_text = std::fs::read_to_string(&depfile).map_err(|error| {
        format!(
            "canonical oracle dependency manifest is missing at '{}': {error}; rebuild with `cargo oracle-lab --help`",
            depfile.display()
        )
    })?;
    let repository = PathBuf::from(env!("STS_REPOSITORY_ROOT"));
    let mut dependencies = depfile_dependencies(&depfile_text);
    dependencies.extend([
        repository.join("Cargo.toml"),
        repository.join("Cargo.lock"),
        repository.join(".cargo/config.toml"),
        repository.join("crates/sts_combat_planner/Cargo.toml"),
        repository.join("crates/sts_simulator_control/Cargo.toml"),
    ]);
    let stale = dependencies.iter().find(|dependency| {
        std::fs::metadata(dependency)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified > executable_modified)
    });
    if let Some(stale) = stale {
        return Err(format!(
            "canonical oracle laboratory is stale: '{}' changed after '{}'; rebuild once with \
             `cargo oracle-lab --help`",
            stale.display(),
            executable.display()
        ));
    }
    Ok(())
}

fn source_content_fingerprint(
    repository: &Path,
    dependencies: &[PathBuf],
) -> Result<String, String> {
    let mut dependencies = dependencies
        .iter()
        .map(|dependency| {
            if dependency.is_absolute() {
                dependency.clone()
            } else {
                repository.join(dependency)
            }
        })
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    let mut digest = Blake2b512::new();
    for dependency in dependencies {
        let bytes = std::fs::read(&dependency).map_err(|error| {
            format!(
                "failed to fingerprint canonical dependency '{}': {error}",
                dependency.display()
            )
        })?;
        digest.update(dependency.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn depfile_dependencies(depfile: &str) -> Vec<PathBuf> {
    depfile
        .lines()
        .filter_map(|line| line.split_once(": ").map(|(_, dependencies)| dependencies))
        .flat_map(str::split_whitespace)
        .filter(|dependency| !dependency.ends_with(':'))
        .map(PathBuf::from)
        .collect()
}

fn combat_policy_surface(
    position: &sts_simulator::sim::combat::CombatPosition,
    limit: usize,
) -> Value {
    const UNIFORM_EXPLORATION: f64 = 0.05;

    let stepper = EngineCombatStepper;
    let actions = stepper.atomic_actions(position);
    let weights =
        sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_atomic_action_policy_weights(
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
    exported.combat = sts_simulator::eval::combat_case::combat_summary(&exported.position);
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
    root: sts_simulator::sim::combat::CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<Option<LocalTurnGraphStateSnapshot>, String> {
    let position = replay_descendant_position(root, actions, max_engine_steps_per_transition)?;
    let exact_state_hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
        &position.engine,
        &position.combat,
    );
    Ok(session.state_snapshot_by_exact_hash(&exact_state_hash))
}

fn replay_descendant_position(
    mut position: sts_simulator::sim::combat::CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<sts_simulator::sim::combat::CombatPosition, String> {
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
    mut position: sts_simulator::sim::combat::CombatPosition,
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
                sts_simulator::sim::combat::CombatTerminal::Unresolved
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
    position: &sts_simulator::sim::combat::CombatPosition,
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
    root: &sts_simulator::sim::combat::CombatPosition,
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
    initial: &sts_simulator::sim::combat::CombatPosition,
    target: &[ClientInput],
    max_engine_steps_per_transition: usize,
) -> Result<
    (
        Vec<Value>,
        String,
        Vec<sts_simulator::sim::combat::CombatPosition>,
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
            sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_atomic_action_policy_weights(
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
        sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
            &position.engine,
            &position.combat,
        ),
        prefix_positions,
    ))
}

fn compact_target_label(
    combat: &sts_simulator::runtime::combat::CombatState,
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
    combat: &sts_simulator::runtime::combat::CombatState,
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

fn combat_turn_snapshot(position: &sts_simulator::sim::combat::CombatPosition) -> Value {
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

fn combat_position_snapshot(position: &sts_simulator::sim::combat::CombatPosition) -> Value {
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
    combat: &sts_simulator::runtime::combat::CombatState,
    entity: sts_simulator::EntityId,
) -> Vec<String> {
    sts_simulator::content::powers::store::powers_for(combat, entity)
        .unwrap_or_default()
        .iter()
        .map(|power| format!("{:?}:{}", power.power_type, power.amount))
        .collect()
}

fn monster_state_label(
    combat: &sts_simulator::runtime::combat::CombatState,
    monster: &sts_simulator::runtime::combat::MonsterEntity,
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

fn card_label(card: &sts_simulator::runtime::combat::CombatCard) -> String {
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
}

impl CombatPlanTransitionServiceAggregateV1 {
    fn observe(&mut self, edge: &LocalTurnGraphPlanTransitionEdgeSnapshot) {
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
    position: &sts_simulator::sim::combat::CombatPosition,
) -> Value {
    json!({
        "progress": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(position),
        "survival": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(position),
        "horizon": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(position),
        "setup": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(position),
    })
}

fn oracle_lab_runtime_identity() -> Value {
    let repository = PathBuf::from(env!("STS_REPOSITORY_ROOT"));
    let executable = std::env::current_exe().ok();
    let metadata = executable
        .as_ref()
        .and_then(|path| std::fs::metadata(path).ok());
    let modified_unix_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| {
            modified
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .ok()
        })
        .map(|duration| duration.as_millis());
    let git_head = read_git_head_fast(&repository);
    json!({
        "profile": env!("STS_CARGO_PROFILE"),
        "executable": executable,
        "artifact_bytes": metadata.map(|metadata| metadata.len()),
        "artifact_modified_unix_ms": modified_unix_ms,
        "git_head": git_head,
        "git_dirty": Value::Null,
        "dirty_scan": "omitted_in_compact_mode",
    })
}

fn read_git_head_fast(repository: &std::path::Path) -> Option<String> {
    let dot_git = repository.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        let pointer = std::fs::read_to_string(dot_git).ok()?;
        let relative = pointer.trim().strip_prefix("gitdir:")?.trim();
        repository.join(relative)
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let revision = if let Some(reference) = head.trim().strip_prefix("ref: ") {
        std::fs::read_to_string(git_dir.join(reference))
            .ok()
            .or_else(|| {
                std::fs::read_to_string(git_dir.join("packed-refs"))
                    .ok()?
                    .lines()
                    .find_map(|line| {
                        let (hash, name) = line.split_once(' ')?;
                        (name == reference).then(|| hash.to_owned())
                    })
            })?
    } else {
        head
    };
    Some(revision.trim().chars().take(12).collect())
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
