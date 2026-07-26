//! Heavy offline and exact-search command frontend for the dedicated oracle runtime.

mod action_reanalysis_policy;
mod action_reanalysis_queue;
mod action_successor_reanalysis;
mod boundary_successor_corpus;
mod boundary_successor_lookahead;
mod exact_combat_evidence;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use blake2::{Blake2b512, Digest};
use clap::{Args, Parser, Subcommand, ValueEnum};
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
    /// Inspect the retired global-agenda search on one exact case. Production
    /// run combat uses `combat-case`; this command remains only for controlled
    /// historical comparisons and explicit V2-donor diagnostics.
    #[command(name = "combat-case-legacy-global")]
    CombatCase {
        #[arg(long)]
        case: PathBuf,
        /// Optional typed action residual. It changes proposal order only;
        /// the production agenda still owns search and exact replay.
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        #[arg(long, default_value_t = 250_000)]
        max_nodes: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        /// Lab-only control: keep the action policy but disable every state
        /// guide, leaving the single Levin/PHS-style anchor ordering.
        #[arg(long)]
        anchor_only: bool,
        /// Diagnostic capability boundary: disable the legacy CombatSearchV2
        /// complete-suffix donor while retaining the new planner's action
        /// priors and state guides.
        #[arg(long)]
        without_v2_donor: bool,
        /// Repeat to inspect membership for several exact corridor states in
        /// one search run.
        #[arg(long)]
        watch_state_hash: Vec<String>,
        /// Replay one complete verified witness and watch every exact player-
        /// turn boundary without adding corridor guidance or changing search.
        #[arg(long)]
        watch_corridor_actions: Option<PathBuf>,
        /// Start search after this many complete player turns from the watched
        /// witness. This reuses the verified action file and avoids hand-
        /// slicing JSON prefixes.
        #[arg(
            long,
            requires = "watch_corridor_actions",
            conflicts_with = "prefix_actions"
        )]
        corridor_prefix_turns: Option<usize>,
        /// Replay one or more exact legal input-prefix files in order before
        /// starting the planner. Repeat the flag to compose verified segments.
        #[arg(long)]
        prefix_actions: Vec<PathBuf>,
        /// Stop replay as soon as this exact player-turn boundary is reached.
        /// This avoids hand-slicing a saved action prefix to inspect or export
        /// an earlier turn.
        #[arg(long, requires = "prefix_actions")]
        prefix_stop_at_player_turn: Option<u32>,
        /// Print compact, card-labelled traces instead of raw action arrays.
        #[arg(long, conflicts_with = "full")]
        readable: bool,
        /// Print the legacy full probe including raw actions and replay traces.
        /// The default is the compact one-page diagnostic report.
        #[arg(long, conflicts_with = "readable")]
        full: bool,
        /// Replay the prefix and print its exact successor without starting search.
        #[arg(long)]
        replay_only: bool,
        /// Diagnostic-only replay counterfactual. Replace the combat root's
        /// current HP before applying --prefix-actions; the output remains
        /// explicitly non-authoritative for the original run.
        #[arg(long, requires = "replay_only")]
        counterfactual_hp: Option<i32>,
        /// Save the exact prefix successor as a standalone combat case.
        #[arg(long)]
        export_prefix_case: Option<PathBuf>,
        /// Lab-only perfect-information control: replay this verified combat
        /// witness and add its exact player-turn states as a fifth shadow
        /// guide. Requires --shadow-corridor-case.
        #[arg(long, requires = "shadow_corridor_case")]
        shadow_corridor_actions: Option<PathBuf>,
        /// Combat start corresponding to --shadow-corridor-actions.
        #[arg(long, requires = "shadow_corridor_actions")]
        shadow_corridor_case: Option<PathBuf>,
        /// How the lab-only corridor guide recognizes promising states.
        /// `typed-feature` never reads an exact state hash while ranking.
        #[arg(long, value_enum, default_value_t = ShadowCorridorGuide::Exact)]
        shadow_corridor_guide: ShadowCorridorGuide,
        /// Lab-only structural control: when an exact corridor is supplied,
        /// suppress the ordinary state guides and retain only the sparse
        /// exact-corridor lane plus the policy-only anchor. Actions are still
        /// generated and executed normally; no witness action is forced.
        #[arg(long, requires = "shadow_corridor_actions")]
        shadow_corridor_only: bool,
        /// Load a distilled typed-feature prototype model. Unlike the
        /// corridor controls, inference does not load witness actions, exact
        /// hashes, or the source combat case.
        #[arg(
            long,
            conflicts_with = "shadow_corridor_actions",
            conflicts_with = "shadow_corridor_case"
        )]
        shadow_value_prototype: Option<PathBuf>,
        /// If a replay-verified win is found, save its exact ClientInput list.
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
        /// Add newly proven one-turn loss prototypes to the loaded value
        /// artifact and save a new generation. Requires evidence collection.
        #[arg(long, requires = "shadow_value_prototype")]
        export_augmented_value_prototype: Option<PathBuf>,
        /// Retain at most this many gap-free states whose fully enumerated
        /// complete-turn language consists only of terminal losses.
        #[arg(long, default_value_t = 0)]
        one_turn_loss_evidence_limit: usize,
        /// Retain at most this many states with an exact complete option that
        /// reaches the next player turn or wins immediately.
        #[arg(long, default_value_t = 0)]
        one_turn_viability_evidence_limit: usize,
    },
    /// Run one pure atomic Levin policy-tree search on an exact combat case.
    /// This deliberately bypasses complete-turn generation, state guides,
    /// legacy donors, and every lane scheduler.
    CombatCaseAtomicLevin {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        #[arg(long, default_value_t = 250_000)]
        max_transitions: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long, default_value_t = 10_000)]
        uniform_exploration_ppm: u32,
        /// Use robust root-LTS with entry into each new player turn as a
        /// structural clue. The q-th observed boundary receives weight 1/q.
        #[arg(long)]
        reroot_player_turn_boundaries: bool,
        /// Diagnostic-only exact states to observe without changing search.
        #[arg(long)]
        watch_state_hash: Vec<String>,
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
    /// Annotate every finite atomic successor with read-only typed combat-plan
    /// facts. This command does not search, rank, prune, or modify a policy.
    CombatCasePlanAnnotations {
        #[arg(long)]
        case: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Replay one exact action sequence and report typed combat-plan changes.
    /// This is a read-only trace: actions are supplied by the caller, never
    /// selected or ranked by this command.
    CombatCasePlanTrace {
        #[arg(long)]
        case: PathBuf,
        /// Repeat to compose several exact action segments in order.
        #[arg(long, required = true)]
        actions: Vec<PathBuf>,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Follow the action policy to terminal states and search complete
    /// trajectories by increasing weighted policy discrepancy.
    CombatCasePolicyDiscrepancy {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        #[arg(long, default_value_t = 250_000)]
        max_transitions: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long, default_value_t = 10_000)]
        uniform_exploration_ppm: u32,
        #[arg(long, default_value_t = 128)]
        max_greedy_actions_per_dive: usize,
        /// Lazily generate bounded complete-turn alternatives at player-turn
        /// boundaries. Zero keeps the pure atomic discrepancy control.
        #[arg(long, default_value_t = 0)]
        turn_macro_transitions: usize,
        #[arg(long, default_value_t = 8)]
        turn_macro_proposals_per_view: usize,
        /// Read-only exact combat states to inspect after the search.
        #[arg(long)]
        watch_case: Vec<PathBuf>,
        /// Replay one or more exact action segments and report their weighted
        /// discrepancy under the same runtime policy surface as the search.
        #[arg(long)]
        audit_actions: Vec<PathBuf>,
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
    /// Enumerate exact next-turn states under the base policy, while giving
    /// every state an independent resumable atomic suffix search.
    CombatCaseAtomicTurnPortfolio {
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
    },
    /// Lab-only turn-synchronous beam control. It never invokes the legacy
    /// suffix donor or the production Widen/Deepen agenda.
    CombatCaseLayered {
        #[arg(long)]
        case: PathBuf,
        #[arg(long, conflicts_with = "guidance_bundle")]
        action_imitation_artifact: Option<PathBuf>,
        /// Optional immutable action-policy plus turn-boundary value package.
        /// This lab control lets the layered search test learned guidance
        /// without changing legality, exact-state ownership, or terminal truth.
        #[arg(long, conflicts_with = "action_imitation_artifact")]
        guidance_bundle: Option<PathBuf>,
        #[arg(long, default_value_t = 250_000)]
        max_nodes: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long, default_value_t = 32)]
        beam_width: usize,
        #[arg(long, default_value_t = 6)]
        retained_per_view: usize,
        #[arg(long, default_value_t = 8)]
        generation_quantum_work: usize,
        #[arg(long, default_value_t = 32)]
        max_turn_layers: usize,
        /// Report where these exact states reside in deferred beam windows
        /// without exporting the complete frontier.
        #[arg(long)]
        watch_exact_state_hash: Vec<String>,
        /// If a replay-verified win is found, save its exact ClientInput list.
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
    /// Isolated local-graph component with node-local lazy widening.
    ///
    /// Production also validates a cheap complete policy proposal and runs an
    /// independent global-agenda member; this command intentionally excludes
    /// both so local-graph behavior can be measured without portfolio effects.
    #[command(name = "combat-case", visible_alias = "combat-case-local-graph")]
    CombatCaseLocalGraph {
        #[arg(long)]
        case: PathBuf,
        /// Diagnostic control: preserve action-policy weights while removing
        /// every boundary and mid-turn state guide.
        #[arg(long, conflicts_with = "root_turn_anchor_only")]
        anchor_only: bool,
        /// Diagnostic control: use only action-policy anchor service during
        /// the root player turn, then restore all guides at later turns.
        #[arg(long, conflicts_with = "anchor_only")]
        root_turn_anchor_only: bool,
        /// Opt-in capability migration: lazily evaluate selected exact states
        /// with bounded rollout evidence. Rollout actions are never injected.
        #[arg(
            long,
            conflicts_with = "anchor_only",
            conflicts_with = "root_turn_anchor_only"
        )]
        rollout_lookahead: bool,
        /// Optional typed action-order policy distilled from exact witnesses.
        /// It changes guidance only; legality and terminal truth stay exact.
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        /// Optional lab-only turn-boundary value prototypes distilled from an
        /// exact witness. This is a teacher upper-bound control, not production.
        #[arg(long)]
        value_prototype_artifact: Option<PathBuf>,
        /// One immutable, compatibility-checked package containing both the
        /// typed action residual and cross-turn value prototypes.
        #[arg(
            long,
            conflicts_with = "action_imitation_artifact",
            conflicts_with = "value_prototype_artifact"
        )]
        guidance_bundle: Option<PathBuf>,
        /// Replay one verified witness and observe each exact player-turn
        /// boundary without changing policy, guides, or search order.
        #[arg(long)]
        watch_corridor_actions: Vec<PathBuf>,
        /// Attach encounter-owned, typed plan facts to newly materialized
        /// exact turn-boundary edges. Diagnostic only: annotations are not
        /// read by policy, scheduling, pruning, or witness authority.
        #[arg(long)]
        plan_transition_annotations: bool,
        /// Opt-in lab control: add the encounter-owned typed combat-plan
        /// state view as one independent guide lane. Action weights,
        /// legality, exact-state identity and terminal truth remain unchanged.
        #[arg(long, conflicts_with = "anchor_only")]
        typed_plan_guide: bool,
        /// Lab-only control: materialize one exact base-policy mainline at
        /// player-turn boundaries. A typed encounter plan may defer a
        /// prematurely resource-consuming action or prefer a precisely timed
        /// action; all rejected alternatives remain searchable.
        #[arg(long)]
        plan_compatible_policy_line: bool,
        /// Deterministic exact-search work granted immediately before the
        /// plan-compatible line would cross a typed combat-plan milestone.
        /// Zero disables suffix probes.
        #[arg(long, default_value_t = 0, requires = "plan_compatible_policy_line")]
        plan_compatible_suffix_work: usize,
        /// Contract assertion: return a non-zero exit status unless an exact,
        /// replay-verified combat witness is found.
        #[arg(long)]
        expect_witness: bool,
        /// Contract assertion: require the verified witness to finish with at
        /// least this much HP.
        #[arg(long, requires = "expect_witness")]
        expect_min_final_hp: Option<i32>,
        /// Contract assertion: fail if all plan-compatible suffix probes
        /// together consume more exact generation work than this allowance.
        #[arg(long, requires = "plan_compatible_policy_line")]
        expect_max_plan_suffix_work: Option<usize>,
        /// Print only the compact contract result after all requested
        /// assertions pass. This keeps repeat regression checks readable.
        #[arg(long, requires = "expect_witness")]
        contract_only: bool,
        #[arg(long, default_value_t = 250_000)]
        max_nodes: usize,
        #[arg(long, default_value_t = 1_000_000)]
        max_selections: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        /// Diagnostic-only quality mode: retain the first verified witness
        /// and keep searching until the explicit work/deadline allowance.
        #[arg(long)]
        improve_incumbent: bool,
        /// Stop at the first replay-verified witness whose HP loss is at most
        /// this non-negative bound. This exposes the planner's existing
        /// satisfaction contract without collapsing every combat to either
        /// first-win or best-HP search.
        #[arg(long, conflicts_with = "improve_incumbent")]
        max_hp_loss: Option<u32>,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long, default_value_t = 4)]
        generation_quantum_work: usize,
        #[arg(long, default_value_t = 32)]
        max_turn_depth: usize,
        /// Diagnostic counterfactual: keep the exact combat state, RNG,
        /// deck, relics and potions, but restore current HP to max HP before
        /// search. This classifies arrival debt; it is never a legal witness
        /// for the original run.
        #[arg(long)]
        full_health: bool,
        /// Include readable, exact replay traces for the deepest survival,
        /// deepest progress, and terminal witness paths.
        #[arg(long)]
        readable: bool,
        /// Print only compact per-turn traces for the deepest states and
        /// witness. Omits raw action hashes and full frontier diagnostics.
        #[arg(long, conflicts_with = "readable")]
        trace: bool,
        /// Report exact graph membership and local service for selected states.
        #[arg(long)]
        watch_exact_state_hash: Vec<String>,
        /// If a replay-verified win is found, save its exact ClientInput list.
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
        /// Save the exact deepest-survival state as a standalone diagnostic
        /// combat case. Inspect `deepest.survival_node.exhausted` before using
        /// it as a segmented-search continuation.
        #[arg(
            long,
            visible_alias = "export-deepest-case",
            conflicts_with = "export_deepest_progress_case"
        )]
        export_deepest_survival_case: Option<PathBuf>,
        /// Save the exact deepest-progress state as a new standalone combat
        /// case instead of the survival envelope.
        #[arg(long, conflicts_with = "export_deepest_survival_case")]
        export_deepest_progress_case: Option<PathBuf>,
    },
    /// Generate one exact turn boundary, select one deferred beam window,
    /// then dovetail resumable layered continuations for its candidates.
    CombatCaseLayeredWindowRace {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        source_window_index: usize,
        #[arg(long, default_value_t = 500_000)]
        max_nodes: usize,
        #[arg(long, default_value_t = 20_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long, default_value_t = 32)]
        beam_width: usize,
        #[arg(long, default_value_t = 6)]
        retained_per_view: usize,
        /// Total generator work available while acquiring the selected source
        /// window. Window publication itself is demand-driven.
        #[arg(long, default_value_t = 8_192)]
        source_generation_work: usize,
        #[arg(long, default_value_t = 8)]
        generation_quantum_work: usize,
        #[arg(long, default_value_t = 3)]
        continuation_turn_layers: usize,
        #[arg(long, default_value_t = 256)]
        continuation_service_quantum_work: usize,
        /// Resume all parents in the selected source window as one shared
        /// turn-synchronous cohort instead of multiplying a full continuation
        /// beam by every parent.
        #[arg(long)]
        shared_window_continuation: bool,
        /// Locate exact states inside parent-local continuation windows.
        #[arg(long)]
        watch_exact_state_hash: Vec<String>,
        /// Include one compact best-per-view summary for every parent-local
        /// continuation window.
        #[arg(long)]
        lineage_window_summaries: bool,
        /// After every source candidate exposes one exact layer, continue a
        /// bounded union of the strongest parents from each independent guide
        /// view. No scalar consensus winner receives exclusive authority.
        #[arg(long)]
        continue_parent_portfolio: bool,
        #[arg(long, default_value_t = 2)]
        portfolio_parents_per_view: usize,
        #[arg(long, default_value_t = 1)]
        portfolio_windows_per_parent: usize,
        #[arg(long, default_value_t = 2_048)]
        portfolio_service_quantum_work: usize,
        /// Repeat the parent-portfolio split this many additional turn
        /// boundaries before entering the final layered continuation.
        #[arg(long, default_value_t = 0)]
        portfolio_recursive_splits: usize,
        #[arg(long, default_value_t = 10)]
        nested_continuation_turn_layers: usize,
        #[arg(long)]
        solved_suffix_case: Option<PathBuf>,
        #[arg(long)]
        solved_suffix_actions: Option<PathBuf>,
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
    /// Compile one verified deep tactical suffix backwards through exact
    /// player-turn predecessors. The corridor supplies predecessor states
    /// only; each fold must naturally generate the already-proven successor.
    CombatCaseFoldSolvedSuffix {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        corridor_actions: PathBuf,
        #[arg(long)]
        solved_suffix_actions: PathBuf,
        #[arg(long)]
        solved_suffix_start_turn: usize,
        #[arg(long, default_value_t = 8_192)]
        max_generation_work_per_fold: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms_per_fold: u64,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long, default_value_t = 256)]
        beam_width: usize,
        #[arg(long, default_value_t = 32)]
        retained_per_view: usize,
        #[arg(long, default_value_t = 8)]
        generation_quantum_work: usize,
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
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
    TurnMembership {
        #[arg(long)]
        case: PathBuf,
        #[arg(
            long,
            required_unless_present = "corridor_actions",
            conflicts_with = "corridor_actions"
        )]
        actions: Option<PathBuf>,
        /// One or more consecutive exact action segments forming a complete
        /// verified witness. Repeat the flag instead of hand-splicing JSON.
        #[arg(long, required_unless_present = "actions", requires = "corridor_rank")]
        corridor_actions: Vec<PathBuf>,
        /// Zero-based player-turn boundary in --corridor-actions. The last
        /// boundary checks the terminal winning segment.
        #[arg(long, requires = "corridor_actions")]
        corridor_rank: Option<usize>,
        #[arg(long, default_value_t = 100_000)]
        max_work: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 8)]
        quantum_work: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        /// Lab-only control: keep action weights but disable all state guides.
        #[arg(long)]
        anchor_only: bool,
        /// Include every target-prefix queue snapshot. By default the report
        /// stays compact and includes only the last reached and first missing
        /// prefixes.
        #[arg(long)]
        full: bool,
    },
    /// Compare the mature V2 search with and without rollout guidance on the
    /// same exact combat root. This is a compact capability ablation; it
    /// cannot seed or alter production search.
    V2CapabilityAudit {
        #[arg(long)]
        case: PathBuf,
        /// Optional verified witness used only to identify the expected first
        /// turn successor in both runs.
        #[arg(long)]
        corridor_actions: Option<PathBuf>,
        #[arg(long, default_value_t = 250_000)]
        max_nodes: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 1_024)]
        quantum_nodes: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        /// Maximum atomic inputs allowed in the standalone deterministic
        /// rollout proposal. Exposed so production proposal bounds can be
        /// reproduced exactly.
        #[arg(long, default_value_t = 80)]
        root_rollout_max_actions: usize,
        /// Save the exact replayable winner found by the no-rollout control.
        /// The compact audit never embeds action arrays in its JSON report.
        #[arg(long)]
        export_without_rollout_witness_actions: Option<PathBuf>,
    },
    /// Audit action-policy order and exact one-step successor guides at one turn prefix.
    TurnActionAudit {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        /// Optional exact action list used only to reach the audited prefix.
        #[arg(long)]
        actions: Option<PathBuf>,
        /// Number of actions from --actions to replay before auditing.
        #[arg(long, default_value_t = 0, requires = "actions")]
        through: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
    },
    /// Audit the mature V2 bounded complete-turn proposer on one exact case.
    /// This is read-only evidence: it does not seed either production search.
    TurnPlanAudit {
        #[arg(long)]
        case: PathBuf,
        #[arg(long, default_value_t = 256)]
        max_inner_nodes: usize,
        #[arg(long, default_value_t = 24)]
        max_end_states: usize,
        #[arg(long, default_value_t = 24)]
        per_bucket_limit: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        /// Number of selected non-loss turn plans shown by the default compact
        /// report.
        #[arg(long, default_value_t = 8)]
        limit: usize,
        /// Include every selected plan and the complete preselection audit.
        #[arg(long)]
        full: bool,
        /// Export this zero-based rank among the displayed non-loss plans.
        #[arg(long)]
        export_rank: Option<usize>,
        /// Save the selected plan's exact next-turn state as a combat case.
        #[arg(long, requires = "export_rank")]
        export_case: Option<PathBuf>,
        /// Save the selected plan's exact ClientInput list.
        #[arg(long, requires = "export_rank")]
        export_actions: Option<PathBuf>,
    },
    /// Generate complete-turn proposals with an independent action-depth beam.
    /// Finished short turns never displace still-live longer prefixes.
    DepthBeamTurnAudit {
        #[arg(long)]
        case: PathBuf,
        /// Lab-only typed semantic action-order artifact. The artifact may
        /// reorder legal actions but cannot remove them or claim an outcome.
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        #[arg(long, default_value_t = 20_000)]
        max_applied_transitions: usize,
        #[arg(long, default_value_t = 5_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 32)]
        partial_beam_width: usize,
        #[arg(long, default_value_t = 6)]
        retained_per_view: usize,
        #[arg(long, default_value_t = 32)]
        max_atomic_depth: usize,
        #[arg(long, default_value_t = 256)]
        max_structured_members_per_family: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long)]
        watch_exact_state_hash: Vec<String>,
        #[arg(long, default_value_t = 64)]
        limit: usize,
    },
    /// Lazily expand one exact player-turn boundary at a time using one
    /// explicitly selected guide lane. This lab control retains deferred
    /// exact variants instead of discarding them through a boundary beam.
    DepthBeamAgendaAudit {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        action_imitation_artifact: Option<PathBuf>,
        /// Lab control: apply the state-conditioned learned action order at
        /// every simulated player turn instead of only the search root turn.
        #[arg(long, requires = "action_imitation_artifact")]
        action_imitation_all_turns: bool,
        #[arg(long)]
        value_prototype_artifact: Option<PathBuf>,
        #[arg(long, default_value_t = 500_000)]
        max_applied_transitions: usize,
        #[arg(long, default_value_t = 60_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 128)]
        partial_beam_width: usize,
        #[arg(long, default_value_t = 8)]
        partial_retained_per_view: usize,
        #[arg(long, default_value_t = 32)]
        max_atomic_depth: usize,
        #[arg(long, default_value_t = 4_096)]
        max_applied_transitions_per_parent: usize,
        #[arg(long, default_value_t = 256)]
        max_structured_members_per_family: usize,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
        #[arg(long)]
        watch_exact_state_hash: Vec<String>,
        /// Exact terminal witness segments used only to label known boundary
        /// membership in the report. They never affect generation or ranking.
        #[arg(long)]
        diagnostic_corridor_actions: Vec<PathBuf>,
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
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
        Command::CombatCaseLocalGraph {
            case,
            anchor_only,
            root_turn_anchor_only,
            rollout_lookahead,
            action_imitation_artifact,
            value_prototype_artifact,
            guidance_bundle,
            watch_corridor_actions,
            plan_transition_annotations,
            typed_plan_guide,
            plan_compatible_policy_line,
            plan_compatible_suffix_work,
            expect_witness,
            expect_min_final_hp,
            expect_max_plan_suffix_work,
            contract_only,
            max_nodes,
            max_selections,
            wall_ms,
            improve_incumbent,
            max_hp_loss,
            max_engine_steps_per_transition,
            generation_quantum_work,
            max_turn_depth,
            full_health,
            readable,
            trace,
            watch_exact_state_hash,
            export_witness_actions,
            export_deepest_survival_case,
            export_deepest_progress_case,
        } => {
            let command_started = Instant::now();
            let mut loaded = load_combat_case(&case)?;
            let original_hp = loaded.position.combat.entities.player.current_hp;
            if full_health {
                loaded.position.combat.entities.player.current_hp =
                    loaded.position.combat.entities.player.max_hp;
            }
            let initial_hp = loaded.position.combat.entities.player.current_hp;
            let root_player_turn = loaded.position.combat.turn.turn_count;
            let search_root_position = loaded.position.clone();
            let watched_corridor = if watch_corridor_actions.is_empty() {
                None
            } else {
                Some(load_exact_turn_corridor(
                    &case,
                    &watch_corridor_actions,
                    max_engine_steps_per_transition,
                )?)
            };
            let root = CombatDecisionRoot::new(loaded.position.clone())
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let satisfaction = if improve_incumbent {
                OracleCombatWitnessSatisfaction::BudgetOrExhaustion
            } else if let Some(limit) = max_hp_loss {
                OracleCombatWitnessSatisfaction::HpLossAtMost(limit)
            } else {
                OracleCombatWitnessSatisfaction::FirstWitness
            };
            let config = LocalTurnGraphWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                generation_quantum_work,
                backed_generation_quantum_work: 256,
                initial_expansion_work: 64,
                root_initial_expansion_work: 2_048,
                // Backed search charges every rollout to the same deterministic
                // work allowance as exact generation. The count guard merely
                // prevents more evaluations than that allowance can finance.
                lookahead_max_evaluations: max_nodes.saturating_div(24).max(1),
                lookahead_work_per_evaluation: 24,
                max_turn_depth,
                satisfaction,
            };
            let policy = if let Some(path) = guidance_bundle.as_deref() {
                CombatGuidanceBundleV1::load(path)?.policy(existing_combat_knowledge_policy_v1())?
            } else {
                let policy = action_imitation_artifact
                    .as_deref()
                    .map(|path| {
                        load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                    })
                    .transpose()?
                    .unwrap_or_else(existing_combat_knowledge_policy_v1);
                if let Some(path) = value_prototype_artifact.as_deref() {
                    let artifact = load_value_prototype(path)?;
                    combat_value_prototype_policy_v1(policy, &artifact)
                } else {
                    policy
                }
            };
            let policy = if anchor_only {
                anchor_only_policy(policy)
            } else if root_turn_anchor_only {
                root_turn_anchor_only_policy(root_player_turn, policy)
            } else {
                policy
            };
            let policy = if typed_plan_guide {
                combat_plan_state_guide_policy_v1(policy)
            } else {
                policy
            };
            let mut session = if rollout_lookahead {
                LocalTurnGraphWitnessSession::with_policy_and_lookahead(
                    root,
                    config,
                    policy,
                    existing_combat_rollout_lookahead_v1(),
                )
            } else {
                LocalTurnGraphWitnessSession::with_policy(root, config, policy)
            };
            if plan_transition_annotations {
                session
                    .enable_plan_transition_annotations()
                    .map_err(|error| {
                        format!(
                            "cannot enable plan transition annotations after graph construction: \
                             {error:?}"
                        )
                    })?;
            }
            let policy_line_report = plan_compatible_policy_line
                .then(|| {
                    session.offer_plan_compatible_policy_line_with_suffix_probes(
                        max_turn_depth,
                        256,
                        plan_compatible_suffix_work,
                        &EngineCombatStepper,
                    )
                })
                .transpose()?;
            let search_started = Instant::now();
            let report = session.advance(
                LocalTurnGraphWitnessQuantum {
                    additional_selections: max_selections,
                    additional_generation_work: max_nodes,
                    additional_engine_steps: max_nodes
                        .saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(Instant::now() + Duration::from_millis(wall_ms)),
                },
                &EngineCombatStepper,
            );
            let search_elapsed_ms = search_started.elapsed().as_millis();
            if expect_witness && report.witness.is_none() {
                return Err("combat-case contract failed: no replay-verified witness".to_owned());
            }
            if let Some(expected_minimum) = expect_min_final_hp {
                let actual = report
                    .witness
                    .as_ref()
                    .map(|witness| witness.final_position.combat.entities.player.current_hp)
                    .ok_or_else(|| {
                        "combat-case contract failed: final HP requires a verified witness"
                            .to_owned()
                    })?;
                if actual < expected_minimum {
                    return Err(format!(
                        "combat-case contract failed: final HP {actual} is below {expected_minimum}"
                    ));
                }
            }
            if let Some(expected_maximum) = expect_max_plan_suffix_work {
                let actual = policy_line_report
                    .as_ref()
                    .map(|policy_line| policy_line.suffix_probe_generation_work)
                    .unwrap_or_default();
                if actual > expected_maximum {
                    return Err(format!(
                        "combat-case contract failed: plan suffix work {actual} exceeds \
                         {expected_maximum}"
                    ));
                }
            }
            if contract_only {
                let witness = report
                    .witness
                    .as_ref()
                    .expect("clap requires --expect-witness");
                return print_json(&json!({
                    "schema_name": "CombatCaseContractResultV1",
                    "schema_version": 1,
                    "status": "passed",
                    "case": case,
                    "elapsed_ms": command_started.elapsed().as_millis(),
                    "final_hp": witness.final_position.combat.entities.player.current_hp,
                    "witness_actions": witness.actions.len(),
                    "plan_suffix": policy_line_report.as_ref().map(|policy_line| json!({
                        "attempts": policy_line.suffix_probe_attempts,
                        "generation_work": policy_line.suffix_probe_generation_work,
                        "engine_steps": policy_line.suffix_probe_engine_steps,
                    })),
                }));
            }
            let performance_timing = json!({
                "selection_elapsed_ns": report.performance_timing.selection_elapsed_ns,
                "generation_elapsed_ns": report.performance_timing.generation_elapsed_ns,
                "admission_elapsed_ns": report.performance_timing.admission_elapsed_ns,
                "atomic_expand_elapsed_ns": report.performance_timing.atomic_expand_elapsed_ns,
                "transition_simulation_elapsed_ns":
                    report.performance_timing.transition_simulation_elapsed_ns,
                "transition_identity_elapsed_ns":
                    report.performance_timing.transition_identity_elapsed_ns,
                "transition_admission_elapsed_ns":
                    report.performance_timing.transition_admission_elapsed_ns,
                "transition_trace_elapsed_ns":
                    report.performance_timing.transition_trace_elapsed_ns,
                "transition_seen_elapsed_ns":
                    report.performance_timing.transition_seen_elapsed_ns,
                "transition_publish_elapsed_ns":
                    report.performance_timing.transition_publish_elapsed_ns,
            });
            let progress = session.progress_snapshot();
            let root_action_families = session
                .root_action_families()
                .into_iter()
                .map(|family| {
                    json!({
                        "action": combat_action_label(
                            &search_root_position,
                            &family.first_action,
                        ),
                        "best_root_negative_log_policy":
                            family.best_root_negative_log_policy,
                        "completed_root_turn_options":
                            family.completed_root_turn_options,
                        "terminal_wins": family.terminal_wins,
                        "terminal_losses": family.terminal_losses,
                        "escapes": family.escapes,
                        "unique_next_turn_successors":
                            family.unique_next_turn_successors,
                        "retained_next_turn_successors":
                            family.retained_next_turn_successors,
                        "reachable_exact_states": family.reachable_exact_states,
                        "reachable_retained_states":
                            family.reachable_retained_states,
                        "reachable_generation_work":
                            family.reachable_generation_work,
                        "reachable_completed_turn_options":
                            family.reachable_completed_turn_options,
                        "max_player_turn": family.max_player_turn,
                        "best_hp_at_max_turn": family.best_hp_at_max_turn,
                        "lowest_enemy_hp_at_max_turn":
                            family.lowest_enemy_hp_at_max_turn,
                    })
                })
                .collect::<Vec<_>>();
            let include_trace = readable || trace;
            let deepest_survival_trace = include_trace
                .then(|| {
                    replay_combat_path(
                        search_root_position.clone(),
                        &progress.deepest_survival_actions,
                        max_engine_steps_per_transition,
                    )
                })
                .transpose()?;
            let deepest_progress_trace = include_trace
                .then(|| {
                    replay_combat_path(
                        search_root_position.clone(),
                        &progress.deepest_progress_actions,
                        max_engine_steps_per_transition,
                    )
                })
                .transpose()?;
            let deepest_survival_node = local_graph_state_snapshot_for_path(
                &session,
                search_root_position.clone(),
                &progress.deepest_survival_actions,
                max_engine_steps_per_transition,
            )?;
            let deepest_progress_node = local_graph_state_snapshot_for_path(
                &session,
                search_root_position.clone(),
                &progress.deepest_progress_actions,
                max_engine_steps_per_transition,
            )?;
            let witness_trace = if include_trace {
                report
                    .witness
                    .as_ref()
                    .map(|witness| {
                        replay_combat_path(
                            search_root_position.clone(),
                            &witness.actions,
                            max_engine_steps_per_transition,
                        )
                    })
                    .transpose()?
            } else {
                None
            };
            let watched_states = watch_exact_state_hash
                .iter()
                .map(|hash| {
                    json!({
                        "exact_state_hash": hash,
                        "state": session.state_snapshot_by_exact_hash(hash),
                        "incoming_from_root": session.edge_snapshot_by_exact_hashes(
                            &sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                                &search_root_position.engine,
                                &search_root_position.combat,
                            ),
                            hash,
                        ),
                    })
                })
                .collect::<Vec<_>>();
            let watched_corridor = watched_corridor.as_ref().map(|corridor| {
                let mut ranked_hashes = corridor
                    .rank_by_exact_hash
                    .iter()
                    .map(|(hash, rank)| (*rank, hash))
                    .collect::<Vec<_>>();
                ranked_hashes.sort_by_key(|(rank, _)| *rank);
                let states = ranked_hashes
                    .iter()
                    .enumerate()
                    .map(|(index, (rank, hash))| {
                        let outgoing_to_next =
                            ranked_hashes.get(index + 1).and_then(|(_, next_hash)| {
                                session.edge_snapshot_by_exact_hashes(hash, next_hash)
                            });
                        json!({
                            "corridor_rank": rank,
                            "exact_state_hash": hash,
                            "state": session.state_snapshot_by_exact_hash(hash),
                            "outgoing_to_next": outgoing_to_next,
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "authority": "diagnostic_only",
                    "changes_search_order": false,
                    "action_count": corridor.action_count,
                    "exact_turn_states": states.len(),
                    "terminal_final_hp": corridor.terminal_final_hp,
                    "states": states,
                })
            });
            if let (Some(path), Some(witness)) =
                (export_witness_actions.as_ref(), report.witness.as_ref())
            {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let inputs = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            let exported_deepest_survival_actions =
                if let Some(path) = export_deepest_survival_case.as_ref() {
                    Some(export_descendant_combat_case(
                        &loaded,
                        &progress.deepest_survival_actions,
                        path,
                        max_engine_steps_per_transition,
                        "local_turn_graph_deepest_survival",
                    )?)
                } else {
                    None
                };
            let exported_deepest_progress_actions =
                if let Some(path) = export_deepest_progress_case.as_ref() {
                    Some(export_descendant_combat_case(
                        &loaded,
                        &progress.deepest_progress_actions,
                        path,
                        max_engine_steps_per_transition,
                        "local_turn_graph_deepest_progress",
                    )?)
                } else {
                    None
                };
            let watched_corridor_output = if readable {
                watched_corridor.clone().unwrap_or(Value::Null)
            } else {
                compact_local_corridor_report(watched_corridor.as_ref())
            };
            if trace {
                let compact_survival_trace =
                    if progress.deepest_survival_actions == progress.deepest_progress_actions {
                        json!({"same_as": "deepest_progress_trace"})
                    } else {
                        compact_combat_trace(deepest_survival_trace.as_ref())
                    };
                return print_json(&json!({
                    "schema_name": "LocalTurnGraphCombatTraceV1",
                    "schema_version": 1,
                    "case": case,
                    "status": format!("{:?}", report.status),
                    "satisfaction": format!("{satisfaction:?}"),
                    "elapsed_ms": command_started.elapsed().as_millis(),
                    "counterfactual": {
                        "full_health": full_health,
                        "original_hp": original_hp,
                        "search_hp": initial_hp,
                    },
                    "work": {
                        "generation_work": report.counters.generation_work,
                        "exact_nodes": report.counters.exact_nodes,
                        "completed_turn_options": report.counters.completed_turn_options,
                        "applied_action_transitions": report.counters.applied_action_transitions,
                    },
                    "root_action_families": root_action_families,
                    "plan_compatible_policy_line": policy_line_report,
                    "deepest": {
                        "progress_state": progress.deepest_progress_state,
                        "progress_node": deepest_progress_node,
                        "progress_trace": compact_combat_trace(deepest_progress_trace.as_ref()),
                        "survival_state": progress.deepest_survival_state,
                        "survival_node": deepest_survival_node,
                        "survival_trace": compact_survival_trace,
                    },
                    "witness": report.witness.as_ref().map(|witness| json!({
                        "final_hp": witness.final_position.combat.entities.player.current_hp,
                        "action_count": witness.actions.len(),
                        "trace": compact_combat_trace(witness_trace.as_ref()),
                    })),
                    "exported_witness_actions": report.witness.is_some()
                        .then_some(export_witness_actions.as_ref())
                        .flatten(),
                    "exported_deepest_survival_case": export_deepest_survival_case,
                    "exported_deepest_survival_actions": exported_deepest_survival_actions,
                    "exported_deepest_progress_case": export_deepest_progress_case,
                    "exported_deepest_progress_actions": exported_deepest_progress_actions,
                }));
            }
            let mut output = json!({
                "schema_name": "LocalTurnGraphCombatSearchReportV1",
                "schema_version": 1,
                "case": case,
                "counterfactual": {
                    "full_health": full_health,
                    "original_hp": original_hp,
                    "search_hp": initial_hp,
                },
                "action_imitation_artifact": action_imitation_artifact,
                "value_prototype_artifact": value_prototype_artifact,
                "guidance_bundle": guidance_bundle,
                "watch_corridor_actions": watch_corridor_actions,
                "satisfaction": format!("{satisfaction:?}"),
                "scheduler": if anchor_only {
                    "anchor_only"
                } else if root_turn_anchor_only {
                    "root_turn_anchor_then_guides"
                } else if rollout_lookahead {
                    "anchor_guides_and_lazy_rollout_lookahead"
                } else {
                    "anchor_and_guides"
                },
                "status": format!("{:?}", report.status),
                "elapsed_ms": command_started.elapsed().as_millis(),
                "initial_hp": initial_hp,
                "final_hp": report.witness.as_ref().map(|witness| {
                    witness.final_position.combat.entities.player.current_hp
                }),
                "witness_actions": report.witness.as_ref().map(|witness| witness.actions.len()),
                "root": {
                    "visits": report.root_visits,
                    "generated_options": report.root_generated_options,
                    "children": report.root_children,
                },
                "root_action_families": root_action_families,
                "plan_compatible_policy_line": policy_line_report,
                "counters": {
                    "selections": report.counters.selections,
                    "node_visits": report.counters.node_visits,
                    "generation_work": report.counters.generation_work,
                    "lookahead_evaluations": report.counters.lookahead_evaluations,
                    "lookahead_work": report.counters.lookahead_work,
                    "atomic_lookahead_evaluations": report.counters.atomic_lookahead_evaluations,
                    "atomic_lookahead_work": report.counters.atomic_lookahead_work,
                    "boundary_lookahead_evaluations": report.counters.boundary_lookahead_evaluations,
                    "boundary_lookahead_work": report.counters.boundary_lookahead_work,
                    "engine_steps": report.counters.engine_steps,
                    "exact_nodes": report.counters.exact_nodes,
                    "exact_edges": report.counters.exact_edges,
                    "completed_turn_options": report.counters.completed_turn_options,
                    "applied_action_transitions": report.counters.applied_action_transitions,
                    "unique_successor_states": report.counters.unique_successor_states,
                    "duplicate_exact_successors": report.counters.duplicate_exact_successors,
                    "duplicate_successor_edges": report.counters.duplicate_successor_edges,
                    "terminal_losses": report.counters.terminal_losses,
                    "depth_limited_successors": report.counters.depth_limited_successors,
                    "exhausted_nodes": report.counters.exhausted_nodes,
                    "maximum_turn_depth": report.counters.maximum_turn_depth,
                },
                "progress": {
                    "retained_states": progress.retained_states,
                    "retained_state_work": session.retained_state_work(),
                    "max_player_turn": progress.max_player_turn,
                    "max_path_atomic_depth": progress.max_path_atomic_depth,
                    "deepest_survival_state": progress.deepest_survival_state,
                    "deepest_survival_node": deepest_survival_node,
                    "deepest_survival_actions": readable.then_some(&progress.deepest_survival_actions),
                    "deepest_survival_trace": deepest_survival_trace,
                    "deepest_progress_state": progress.deepest_progress_state,
                    "deepest_progress_node": deepest_progress_node,
                    "deepest_progress_actions": readable.then_some(&progress.deepest_progress_actions),
                    "deepest_progress_trace": deepest_progress_trace,
                    "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
                },
                "witness_trace": witness_trace,
                "generation_gap_count": report.generation_gaps.len(),
                "watched_states": watched_states,
                "watched_corridor": watched_corridor_output,
                "exported_witness_actions": report.witness.is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
                "exported_deepest_survival_case": export_deepest_survival_case,
                "exported_deepest_survival_actions": exported_deepest_survival_actions,
                "exported_deepest_progress_case": export_deepest_progress_case,
                "exported_deepest_progress_actions": exported_deepest_progress_actions,
            });
            let plan_transition_portfolio = plan_transition_annotations
                .then(|| combat_plan_transition_portfolio_v1(&session))
                .unwrap_or(Value::Null);
            output["counters"]["annotated_exact_edges"] =
                json!(report.counters.annotated_exact_edges);
            let output_object = output
                .as_object_mut()
                .expect("combat-case report must be a JSON object");
            output_object.insert(
                "plan_transition_annotations".to_string(),
                json!(plan_transition_annotations),
            );
            output_object.insert(
                "plan_transition_portfolio".to_string(),
                plan_transition_portfolio,
            );
            output_object.insert("search_elapsed_ms".to_string(), json!(search_elapsed_ms));
            output_object.insert("performance_timing".to_string(), performance_timing);
            print_json(&output)
        }
        Command::CombatCaseLayered {
            case,
            action_imitation_artifact,
            guidance_bundle,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            beam_width,
            retained_per_view,
            generation_quantum_work,
            max_turn_layers,
            watch_exact_state_hash,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let loaded = load_combat_case(&case)?;
            let initial_hp = loaded.position.combat.entities.player.current_hp;
            let root = CombatDecisionRoot::new(loaded.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let deadline = Instant::now() + Duration::from_millis(wall_ms);
            let config = LayeredCombatWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                beam_width,
                retained_per_view,
                generation_quantum_work,
                max_turn_layers,
            };
            let policy = if let Some(path) = guidance_bundle.as_deref() {
                CombatGuidanceBundleV1::load(path)?.policy(existing_combat_knowledge_policy_v1())?
            } else {
                action_imitation_artifact
                    .as_deref()
                    .map(|path| {
                        load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                    })
                    .transpose()?
                    .unwrap_or_else(existing_combat_knowledge_policy_v1)
            };
            let diagnostic_policy = policy.clone();
            let mut session = LayeredCombatWitnessSession::with_policy(root, config, policy);
            let report = session.advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: max_nodes,
                    additional_engine_steps: max_nodes
                        .saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(deadline),
                },
                &EngineCombatStepper,
            );
            let mut watched_states = Vec::new();
            for window in session.deferred_windows() {
                for (candidate_index, candidate) in window.candidates.iter().enumerate() {
                    if !watch_exact_state_hash.contains(&candidate.exact_state_hash) {
                        continue;
                    }
                    watched_states.push(json!({
                        "exact_state_hash": candidate.exact_state_hash,
                        "relative_turn_depth": window.relative_turn_depth,
                        "window_discrepancy": window.window_discrepancy,
                        "source_window_index": window.source_window_index,
                        "candidate_index": candidate_index,
                        "action_count": candidate.actions.len(),
                        "negative_log_policy": candidate.negative_log_policy,
                        "view_ranks": layered_candidate_view_ranks(
                            &window.candidates,
                            candidate_index,
                            diagnostic_policy.as_ref(),
                        ),
                        "guides": existing_combat_guide_diagnostics(&candidate.position),
                    }));
                }
            }
            if let (Some(path), Some(witness)) =
                (export_witness_actions.as_ref(), report.witness.as_ref())
            {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let inputs = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            let frontier = report
                .frontier
                .iter()
                .map(|state| {
                    json!({
                        "exact_state_hash": state.exact_state_hash,
                        "player_turn": state.position.combat.turn.turn_count,
                        "player_hp": state.position.combat.entities.player.current_hp,
                        "enemy_hp": state.position.combat.entities.monsters.iter()
                            .map(|monster| monster.current_hp.max(0))
                            .sum::<i32>(),
                        "path_action_count": state.actions.len(),
                        "negative_log_policy": state.negative_log_policy,
                        "guides": existing_combat_guide_diagnostics(&state.position),
                    })
                })
                .collect::<Vec<_>>();
            let layers = report
                .layers
                .iter()
                .map(|layer| {
                    json!({
                        "relative_turn_depth": layer.relative_turn_depth,
                        "window_discrepancy": layer.window_discrepancy,
                        "source_window_index": layer.source_window_index,
                        "player_turn": layer.player_turn,
                        "parent_states": layer.parent_states,
                        "parent_exact_state_hashes": layer.parent_exact_state_hashes,
                        "parent_work": layer.parent_work.iter().map(|parent| json!({
                            "exact_state_hash": parent.exact_state_hash,
                            "generation_work": parent.generation_work,
                            "completed_turn_options": parent.completed_turn_options,
                            "finished": parent.finished,
                        })).collect::<Vec<_>>(),
                        "expanded_parents": layer.expanded_parents,
                        "generation_work": layer.generation_work,
                        "completed_turn_options": layer.completed_turn_options,
                        "unique_next_turn_states": layer.unique_next_turn_states,
                        "duplicate_next_turn_states": layer.duplicate_next_turn_states,
                        "retained_next_turn_states": layer.retained_next_turn_states,
                        "retained_exact_state_hashes": layer.retained_exact_state_hashes,
                        "truncated_parents": layer.truncated_parents,
                        "emitted_windows": layer.emitted_windows,
                    })
                })
                .collect::<Vec<_>>();
            print_json(&json!({
                "schema_name": "OracleCombatCaseLayeredV1",
                "schema_version": 1,
                "case": case,
                "runtime": oracle_lab_runtime_identity(),
                "mode": {
                    "scheduler": "recoverable_turn_synchronous_multi_view_beam",
                    "v2_donor_enabled": false,
                    "action_imitation_artifact": action_imitation_artifact,
                    "guidance_bundle": guidance_bundle,
                },
                "status": format!("{:?}", report.status),
                "elapsed_ms": command_started.elapsed().as_millis(),
                "config": {
                    "beam_width": beam_width,
                    "retained_per_view": retained_per_view,
                    "generation_quantum_work": generation_quantum_work,
                    "max_turn_layers": max_turn_layers,
                },
                "budget": {
                    "generation_work": max_nodes,
                    "wall_ms": wall_ms,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                },
                "work": {
                    "generation_work": report.counters.generation_work,
                    "engine_steps": report.counters.engine_steps,
                    "expanded_parents": report.counters.expanded_parents,
                    "completed_turn_options": report.counters.completed_turn_options,
                    "unique_next_turn_states": report.counters.unique_next_turn_states,
                    "duplicate_next_turn_states": report.counters.duplicate_next_turn_states,
                    "truncated_parents": report.counters.truncated_parents,
                    "completed_layers": report.counters.completed_layers,
                    "deferred_windows": report.counters.deferred_windows,
                    "recovered_window_expansions": report.counters.recovered_window_expansions,
                    "maximum_window_discrepancy": report.counters.maximum_window_discrepancy,
                },
                "layers": layers,
                "frontier": frontier,
                "generation_gap_count": report.generation_gaps.len(),
                "watched_states": watched_states,
                "exported_witness_actions": report.witness.is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
                "witness": report.witness.as_ref().map(|witness| json!({
                    "discovery_source": witness.discovery_source,
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
        Command::CombatCaseFoldSolvedSuffix {
            case,
            corridor_actions,
            solved_suffix_actions,
            solved_suffix_start_turn,
            max_generation_work_per_fold,
            wall_ms_per_fold,
            max_engine_steps_per_transition,
            beam_width,
            retained_per_view,
            generation_quantum_work,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let corridor = load_exact_turn_corridor(
                &case,
                std::slice::from_ref(&corridor_actions),
                max_engine_steps_per_transition,
            )?;
            if solved_suffix_start_turn >= corridor.positions_by_rank.len() {
                return Err(format!(
                    "solved suffix starts at turn-boundary index {solved_suffix_start_turn}, but the corridor exposes only {} boundary states",
                    corridor.positions_by_rank.len()
                ));
            }
            let seed_inputs =
                load_combat_action_segments(std::slice::from_ref(&solved_suffix_actions))?;
            let policy = existing_combat_knowledge_policy_v1();
            let report = fold_verified_suffix_through_turn_predecessors(
                &corridor.positions_by_rank[..=solved_suffix_start_turn],
                seed_inputs,
                SolvedSuffixFoldConfig {
                    search: LayeredCombatWitnessConfig {
                        generator: TurnOptionGeneratorConfig {
                            max_engine_steps_per_transition,
                            ..TurnOptionGeneratorConfig::default()
                        },
                        beam_width: beam_width.max(1),
                        retained_per_view: retained_per_view.max(1),
                        generation_quantum_work: generation_quantum_work.max(1),
                        max_turn_layers: 1,
                    },
                    max_generation_work_per_fold: max_generation_work_per_fold.max(1),
                    max_engine_steps_per_transition: max_engine_steps_per_transition.max(1),
                    wall_time_per_fold: Some(Duration::from_millis(wall_ms_per_fold.max(1))),
                },
                policy,
                &EngineCombatStepper,
            )
            .map_err(|error| format!("solved suffix fold failed: {error:?}"))?;
            let root_witness_inputs = report.witness.as_ref().map(|witness| {
                witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>()
            });
            let root_final_hp = report
                .witness
                .as_ref()
                .map(|witness| witness.final_position.combat.entities.player.current_hp);
            if let (Some(path), Some(inputs)) = (
                export_witness_actions.as_ref(),
                root_witness_inputs.as_ref(),
            ) {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(inputs).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            let fold_reports = report
                .steps
                .iter()
                .map(|step| {
                    json!({
                        "predecessor_turn_index": step.predecessor_index,
                        "status": format!("{:?}", step.status),
                        "elapsed_ms": step.elapsed.as_millis(),
                        "generation_work": step.counters.generation_work,
                        "engine_steps": step.counters.engine_steps,
                        "solved_suffix_matches": step.counters.solved_suffix_matches,
                        "action_count": step.action_count,
                        "final_hp": step.final_hp,
                    })
                })
                .collect::<Vec<_>>();
            let overall_status = match report.status {
                SolvedSuffixFoldStatus::WitnessFound => "WitnessFound",
                SolvedSuffixFoldStatus::Partial { .. } => "Partial",
            };
            print_json(&json!({
                "schema_name": "OracleCombatSolvedSuffixFoldV1",
                "schema_version": 1,
                "case": case,
                "runtime": oracle_lab_runtime_identity(),
                "status": overall_status,
                "mode": {
                    "search": "exact_predecessor_proof_folding",
                    "corridor_is_search_guidance": false,
                    "v2_donor_enabled": false,
                },
                "budget": {
                    "max_generation_work_per_fold": max_generation_work_per_fold,
                    "wall_ms_per_fold": wall_ms_per_fold,
                    "solved_suffix_start_turn": solved_suffix_start_turn,
                },
                "folds": fold_reports,
                "solved_suffix_count": report.solved_suffix_count,
                "elapsed_ms": command_started.elapsed().as_millis(),
                "exported_witness_actions": export_witness_actions,
                "witness": root_witness_inputs.as_ref().map(|inputs| json!({
                    "action_count": inputs.len(),
                    "final_hp": root_final_hp,
                })),
            }))
        }
        Command::CombatCaseLayeredWindowRace {
            case,
            source_window_index,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            beam_width,
            retained_per_view,
            source_generation_work,
            generation_quantum_work,
            continuation_turn_layers,
            continuation_service_quantum_work,
            shared_window_continuation,
            watch_exact_state_hash,
            lineage_window_summaries,
            continue_parent_portfolio,
            portfolio_parents_per_view,
            portfolio_windows_per_parent,
            portfolio_service_quantum_work,
            portfolio_recursive_splits,
            nested_continuation_turn_layers,
            solved_suffix_case,
            solved_suffix_actions,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let loaded = load_combat_case(&case)?;
            let initial_hp = loaded.position.combat.entities.player.current_hp;
            let original_position = loaded.position.clone();
            let original_root = CombatDecisionRoot::new(loaded.position.clone())
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let source_root = CombatDecisionRoot::new(loaded.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let deadline = Instant::now() + Duration::from_millis(wall_ms);
            let policy = existing_combat_knowledge_policy_v1();
            let solved_suffixes = load_layered_solved_suffix_index(
                solved_suffix_case.as_ref(),
                solved_suffix_actions.as_ref(),
                max_engine_steps_per_transition,
            )?;
            let base_config = LayeredCombatWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                beam_width,
                retained_per_view,
                generation_quantum_work,
                max_turn_layers: 1,
            };
            let mut source = LayeredCombatWitnessSession::with_policy_and_solved_suffixes(
                source_root,
                base_config,
                policy.clone(),
                solved_suffixes.clone(),
            );
            let source_report = source.advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: source_generation_work.max(1),
                    additional_engine_steps: source_generation_work
                        .max(1)
                        .saturating_mul(max_engine_steps_per_transition.max(1)),
                    deadline: Some(deadline),
                },
                &EngineCombatStepper,
            );
            if let Some(witness) = source_report.witness.as_ref() {
                if let Some(path) = export_witness_actions.as_ref() {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                    }
                    let inputs = witness
                        .actions
                        .iter()
                        .map(|action| action.input.clone())
                        .collect::<Vec<_>>();
                    std::fs::write(
                        path,
                        serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
                }
                return print_json(&json!({
                    "schema_name": "OracleCombatCaseLayeredWindowRaceV1",
                    "schema_version": 1,
                    "case": case,
                    "runtime": oracle_lab_runtime_identity(),
                    "mode": {
                        "scheduler": "resumable_candidate_continuation_race",
                        "v2_donor_enabled": false,
                        "solved_suffix_count": solved_suffixes.len(),
                    },
                    "elapsed_ms": command_started.elapsed().as_millis(),
                    "source": {
                        "status": format!("{:?}", source_report.status),
                        "generation_work": source_report.counters.generation_work,
                        "solved_suffix_matches": source_report.counters.solved_suffix_matches,
                        "solved_suffix_replay_engine_steps": source_report.counters.solved_suffix_replay_engine_steps,
                    },
                    "race": null,
                    "lineage_portfolio": null,
                    "exported_witness_actions": export_witness_actions,
                    "witness": {
                        "final_hp": witness.final_position.combat.entities.player.current_hp,
                        "hp_loss": initial_hp.saturating_sub(
                            witness.final_position.combat.entities.player.current_hp,
                        ),
                        "action_count": witness.actions.len(),
                        "negative_log_policy": witness.negative_log_policy,
                        "replay_engine_steps": witness.replay_engine_steps,
                        "discovery_source": format!("{:?}", witness.discovery_source),
                    },
                }));
            }
            let window = source
                .deferred_windows()
                .into_iter()
                .find(|window| {
                    window.relative_turn_depth == 1
                        && window.source_window_index == source_window_index
                })
                .ok_or_else(|| {
                    format!(
                        "deferred window {source_window_index} was not generated; source status={:?}",
                        source_report.status
                    )
                })?;
            let candidate_count = window.candidates.len();
            let selected_window_discrepancy = window.window_discrepancy;
            let continuation = LayeredCombatWitnessConfig {
                max_turn_layers: if continue_parent_portfolio {
                    1
                } else {
                    continuation_turn_layers
                },
                ..base_config
            };
            if shared_window_continuation {
                let mut continuation_session =
                    LayeredCombatWitnessSession::from_deferred_window_with_solved_suffixes(
                        original_root,
                        window,
                        continuation,
                        policy,
                        solved_suffixes.clone(),
                    );
                let remaining_work =
                    max_nodes.saturating_sub(source_report.counters.generation_work);
                let continuation_report = continuation_session.advance(
                    LayeredCombatWitnessQuantum {
                        additional_generation_work: remaining_work,
                        additional_engine_steps: remaining_work
                            .saturating_mul(max_engine_steps_per_transition.max(1)),
                        deadline: Some(deadline),
                    },
                    &EngineCombatStepper,
                );
                if let (Some(path), Some(witness)) = (
                    export_witness_actions.as_ref(),
                    continuation_report.witness.as_ref(),
                ) {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                    }
                    let inputs = witness
                        .actions
                        .iter()
                        .map(|action| action.input.clone())
                        .collect::<Vec<_>>();
                    std::fs::write(
                        path,
                        serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
                }
                let watched_states = watch_exact_state_hash
                    .iter()
                    .map(|hash| {
                        let parent_work =
                            continuation_report
                                .layers
                                .iter()
                                .enumerate()
                                .filter_map(|(layer_index, layer)| {
                                    layer
                                    .parent_work
                                    .iter()
                                    .find(|parent| parent.exact_state_hash == *hash)
                                    .map(|parent| json!({
                                        "layer_index": layer_index,
                                        "generation_work": parent.generation_work,
                                        "completed_turn_options": parent.completed_turn_options,
                                        "finished": parent.finished,
                                    }))
                                })
                                .collect::<Vec<_>>();
                        let retained_layers = continuation_report
                            .layers
                            .iter()
                            .enumerate()
                            .filter_map(|(layer_index, layer)| {
                                layer
                                    .retained_exact_state_hashes
                                    .iter()
                                    .any(|candidate| candidate == hash)
                                    .then_some(layer_index)
                            })
                            .collect::<Vec<_>>();
                        let frontier = continuation_report
                            .frontier
                            .iter()
                            .any(|candidate| candidate.exact_state_hash == *hash);
                        json!({
                            "exact_state_hash": hash,
                            "parent_work": parent_work,
                            "retained_layers": retained_layers,
                            "frontier": frontier,
                        })
                    })
                    .collect::<Vec<_>>();
                return print_json(&json!({
                    "schema_name": "OracleCombatCaseLayeredSharedWindowV1",
                    "schema_version": 1,
                    "case": case,
                    "runtime": oracle_lab_runtime_identity(),
                    "mode": {
                        "scheduler": "shared_turn_synchronous_window",
                        "v2_donor_enabled": false,
                        "solved_suffix_count": solved_suffixes.len(),
                    },
                    "elapsed_ms": command_started.elapsed().as_millis(),
                    "source": {
                        "status": format!("{:?}", source_report.status),
                        "generation_work": source_report.counters.generation_work,
                        "candidate_count": candidate_count,
                        "source_window_index": source_window_index,
                        "window_discrepancy": selected_window_discrepancy,
                    },
                    "continuation": {
                        "status": format!("{:?}", continuation_report.status),
                        "counters": {
                            "generation_work": continuation_report.counters.generation_work,
                            "engine_steps": continuation_report.counters.engine_steps,
                            "expanded_parents": continuation_report.counters.expanded_parents,
                            "completed_turn_options": continuation_report.counters.completed_turn_options,
                            "unique_next_turn_states": continuation_report.counters.unique_next_turn_states,
                            "duplicate_next_turn_states": continuation_report.counters.duplicate_next_turn_states,
                            "completed_layers": continuation_report.counters.completed_layers,
                            "solved_suffix_matches": continuation_report.counters.solved_suffix_matches,
                        },
                        "layers": continuation_report.layers.iter().map(|layer| json!({
                            "relative_turn_depth": layer.relative_turn_depth,
                            "player_turn": layer.player_turn,
                            "parent_states": layer.parent_states,
                            "generation_work": layer.generation_work,
                            "completed_turn_options": layer.completed_turn_options,
                            "unique_next_turn_states": layer.unique_next_turn_states,
                            "retained_next_turn_states": layer.retained_next_turn_states,
                            "truncated_parents": layer.truncated_parents,
                            "emitted_windows": layer.emitted_windows,
                        })).collect::<Vec<_>>(),
                        "watched_states": watched_states,
                    },
                    "exported_witness_actions": export_witness_actions,
                    "witness": continuation_report.witness.as_ref().map(|witness| json!({
                        "final_hp": witness.final_position.combat.entities.player.current_hp,
                        "hp_loss": initial_hp.saturating_sub(
                            witness.final_position.combat.entities.player.current_hp,
                        ),
                        "action_count": witness.actions.len(),
                        "negative_log_policy": witness.negative_log_policy,
                        "replay_engine_steps": witness.replay_engine_steps,
                        "discovery_source": format!("{:?}", witness.discovery_source),
                    })),
                }));
            }
            let mut race = LayeredCombatCandidateRaceSession::from_window_with_solved_suffixes(
                original_root,
                window,
                LayeredCombatCandidateRaceConfig {
                    continuation,
                    service_quantum_work: continuation_service_quantum_work,
                },
                policy.clone(),
                solved_suffixes.clone(),
            );
            let remaining_work = max_nodes.saturating_sub(source_report.counters.generation_work);
            let race_report = race.advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: remaining_work,
                    additional_engine_steps: remaining_work
                        .saturating_mul(max_engine_steps_per_transition.max(1)),
                    deadline: Some(deadline),
                },
                &EngineCombatStepper,
            );
            let lineage_windows = race.deferred_lineage_windows();
            let lineage_parent_ranks =
                rank_layered_combat_lineage_parents(&lineage_windows, policy.as_ref());
            let mut portfolio_report = None;
            if continue_parent_portfolio && race_report.witness.is_none() {
                let portfolio_root = CombatDecisionRoot::new(original_position.clone())
                    .map_err(|error| format!("invalid portfolio combat root: {error:?}"))?;
                let nested_config = LayeredCombatWitnessConfig {
                    max_turn_layers: nested_continuation_turn_layers,
                    ..base_config
                };
                let mut portfolio =
                    LayeredCombatLineagePortfolioSession::from_lineage_windows_with_solved_suffixes(
                        portfolio_root,
                        lineage_windows.clone(),
                        LayeredCombatLineagePortfolioConfig {
                            candidate_race: LayeredCombatCandidateRaceConfig {
                                continuation: nested_config,
                                service_quantum_work: continuation_service_quantum_work,
                            },
                            parents_per_view: portfolio_parents_per_view,
                            windows_per_parent: portfolio_windows_per_parent,
                            service_quantum_work: portfolio_service_quantum_work,
                            recursive_splits: portfolio_recursive_splits,
                        },
                        policy.clone(),
                        solved_suffixes.clone(),
                    );
                let remaining_work = max_nodes
                    .saturating_sub(source_report.counters.generation_work)
                    .saturating_sub(race_report.counters.generation_work);
                portfolio_report = Some(portfolio.advance(
                    LayeredCombatWitnessQuantum {
                        additional_generation_work: remaining_work,
                        additional_engine_steps:
                            remaining_work.saturating_mul(max_engine_steps_per_transition.max(1)),
                        deadline: Some(deadline),
                    },
                    &EngineCombatStepper,
                ));
            }
            let watched_lineage_states =
                lineage_windows
                    .iter()
                    .flat_map(|lineage| {
                        lineage
                        .window
                        .candidates
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| {
                            watch_exact_state_hash.contains(&candidate.exact_state_hash)
                        })
                        .map(|(candidate_index, candidate)| json!({
                            "exact_state_hash": candidate.exact_state_hash,
                            "parent_candidate_index": lineage.parent_candidate_index,
                            "parent_exact_state_hash": lineage.parent_exact_state_hash,
                            "relative_turn_depth": lineage.window.relative_turn_depth,
                            "window_discrepancy": lineage.window.window_discrepancy,
                            "source_window_index": lineage.window.source_window_index,
                            "candidate_index": candidate_index,
                            "action_count": candidate.actions.len(),
                            "negative_log_policy": candidate.negative_log_policy,
                            "guides": existing_combat_guide_diagnostics(&candidate.position),
                        }))
                    })
                    .collect::<Vec<_>>();
            let lineage_window_summaries = lineage_window_summaries.then(|| {
                lineage_windows
                    .iter()
                    .map(|lineage| {
                        let best_policy = lineage
                            .window
                            .candidates
                            .iter()
                            .map(|candidate| candidate.negative_log_policy)
                            .min_by(f64::total_cmp);
                        let best_progress = lineage
                            .window
                            .candidates
                            .iter()
                            .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(&candidate.position))
                            .max();
                        let best_survival = lineage
                            .window
                            .candidates
                            .iter()
                            .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(&candidate.position))
                            .max();
                        let best_horizon = lineage
                            .window
                            .candidates
                            .iter()
                            .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(&candidate.position))
                            .max();
                        let best_setup = lineage
                            .window
                            .candidates
                            .iter()
                            .map(|candidate| sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(&candidate.position))
                            .max();
                        json!({
                            "parent_candidate_index": lineage.parent_candidate_index,
                            "parent_exact_state_hash": lineage.parent_exact_state_hash,
                            "source_window_index": lineage.window.source_window_index,
                            "window_discrepancy": lineage.window.window_discrepancy,
                            "candidate_count": lineage.window.candidates.len(),
                            "best_policy_negative_log": best_policy,
                            "best_progress": best_progress,
                            "best_survival": best_survival,
                            "best_horizon": best_horizon,
                            "best_setup": best_setup,
                        })
                    })
                    .collect::<Vec<_>>()
            });
            let final_witness = portfolio_report
                .as_ref()
                .and_then(|report| report.witness.as_ref())
                .or(race_report.witness.as_ref());
            if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), final_witness) {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let inputs = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            print_json(&json!({
                "schema_name": "OracleCombatCaseLayeredWindowRaceV1",
                "schema_version": 1,
                "case": case,
                "runtime": oracle_lab_runtime_identity(),
                "mode": {
                    "scheduler": "resumable_candidate_continuation_race",
                    "v2_donor_enabled": false,
                    "solved_suffix_count": solved_suffixes.len(),
                },
                "elapsed_ms": command_started.elapsed().as_millis(),
                "source": {
                    "status": format!("{:?}", source_report.status),
                    "generation_work": source_report.counters.generation_work,
                    "source_window_index": source_window_index,
                    "window_discrepancy": selected_window_discrepancy,
                    "candidate_count": candidate_count,
                },
                "race": {
                    "status": format!("{:?}", race_report.status),
                    "generation_work": race_report.counters.generation_work,
                    "engine_steps": race_report.counters.engine_steps,
                    "services": race_report.counters.services,
                    "candidates": race_report.candidates.iter().map(|candidate| json!({
                        "candidate_index": candidate.candidate_index,
                        "exact_state_hash": candidate.exact_state_hash,
                        "generation_work": candidate.generation_work,
                        "engine_steps": candidate.engine_steps,
                        "completed_layers": candidate.completed_layers,
                        "terminal": candidate.terminal,
                        "found_witness": candidate.found_witness,
                    })).collect::<Vec<_>>(),
                },
                "lineage_window_count": lineage_windows.len(),
                "lineage_parent_ranks": lineage_parent_ranks.iter().map(|parent| json!({
                    "parent_candidate_index": parent.parent_candidate_index,
                    "parent_exact_state_hash": parent.parent_exact_state_hash,
                    "consensus_rank": parent.consensus_rank,
                    "rank_sum": parent.rank_sum,
                    "anchor_rank": parent.anchor_rank,
                    "guide_ranks": parent.guide_ranks.iter().map(|(lane, rank)| json!({
                        "lane": lane.value(),
                        "rank": rank,
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "lineage_window_summaries": lineage_window_summaries,
                "watched_lineage_states": watched_lineage_states,
                "lineage_portfolio": portfolio_report.as_ref().map(|report| json!({
                    "status": format!("{:?}", report.status),
                    "generation_work": report.counters.generation_work,
                    "engine_steps": report.counters.engine_steps,
                    "services": report.counters.services,
                    "selected_parent_count": report.selected_parent_count,
                    "deferred_parent_count": report.deferred_parent_count,
                    "deferred_window_count": report.deferred_window_count,
                    "entries": lineage_portfolio_entries_json(&report.entries),
                })),
                "exported_witness_actions": final_witness.is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
                "witness": final_witness.map(|witness| json!({
                    "final_hp": witness.final_position.combat.entities.player.current_hp,
                    "hp_loss": initial_hp.saturating_sub(
                        witness.final_position.combat.entities.player.current_hp,
                    ),
                    "action_count": witness.actions.len(),
                    "negative_log_policy": witness.negative_log_policy,
                    "replay_engine_steps": witness.replay_engine_steps,
                    "discovery_source": format!("{:?}", witness.discovery_source),
                })),
            }))
        }
        Command::CombatCasePlanAnnotations {
            case,
            max_engine_steps_per_transition,
        } => {
            let case_path = case.clone();
            let loaded = load_combat_case(&case)?;
            let position = loaded.position;
            let stepper = EngineCombatStepper;
            let surface = stepper.legal_action_surface(&position);
            let root_plan = awakened_one_combat_plan_v1(&position);
            let annotations = surface
                .atomic_actions
                .iter()
                .map(|input| {
                    let step = stepper.apply_to_stable(
                        &position,
                        input.clone(),
                        CombatStepLimits {
                            max_engine_steps: max_engine_steps_per_transition,
                            deadline: None,
                        },
                    );
                    let exact_successor_hash = (!step.truncated).then(|| {
                        combat_exact_state_hash_v1(&step.position.engine, &step.position.combat)
                    });
                    let transition = (!step.truncated)
                        .then(|| awakened_one_plan_transition_v1(&position, &step.position))
                        .flatten();
                    let successor_plan = (!step.truncated)
                        .then(|| awakened_one_combat_plan_v1(&step.position))
                        .flatten();
                    json!({
                        "label": combat_action_label(&position, input),
                        "action_key": combat_action_key(&position.combat, input),
                        "input": input,
                        "engine_steps": step.engine_steps,
                        "truncated": step.truncated,
                        "timed_out": step.timed_out,
                        "terminal": format!("{:?}", step.terminal),
                        "exact_successor_hash": exact_successor_hash,
                        "plan_transition": transition,
                        "successor_plan": successor_plan,
                    })
                })
                .collect::<Vec<_>>();
            print_json(&json!({
                "schema_name": "OracleCombatCasePlanAnnotationsV1",
                "schema_version": 1,
                "case": case_path,
                "runtime": oracle_lab_runtime_identity(),
                "contract": {
                    "search": false,
                    "policy_mutation": false,
                    "ranking": false,
                    "pruning": false,
                    "terminal_truth": "exact_simulator_only",
                },
                "root_exact_state_hash": combat_exact_state_hash_v1(
                    &position.engine,
                    &position.combat,
                ),
                "root_plan": root_plan,
                "surface": {
                    "atomic_action_count": surface.atomic_actions.len(),
                    "structured_family_count": surface.selection_families.len(),
                    "complete": surface.selection_families.is_empty(),
                    "structured_families_unannotated": !surface.selection_families.is_empty(),
                },
                "max_engine_steps_per_transition": max_engine_steps_per_transition,
                "annotations": annotations,
            }))
        }
        Command::CombatCasePlanTrace {
            case,
            actions,
            max_engine_steps_per_transition,
        } => {
            let case_path = case.clone();
            let action_paths = actions.clone();
            let loaded = load_combat_case(&case)?;
            let inputs = load_combat_action_segments(&actions)?;
            let input_count = inputs.len();
            let stepper = EngineCombatStepper;
            let mut position = loaded.position;
            let root_exact_state_hash =
                combat_exact_state_hash_v1(&position.engine, &position.combat);
            let root_plan = awakened_one_combat_plan_v1(&position);
            let mut trace = Vec::new();
            let mut consumed_actions = 0_usize;

            for (index, input) in inputs.into_iter().enumerate() {
                let before_hash = combat_exact_state_hash_v1(&position.engine, &position.combat);
                let label = combat_action_label(&position, &input);
                let action_key = combat_action_key(&position.combat, &input);
                let step = stepper.apply_to_stable(
                    &position,
                    input.clone(),
                    CombatStepLimits {
                        max_engine_steps: max_engine_steps_per_transition,
                        deadline: None,
                    },
                );
                let transition = (!step.truncated)
                    .then(|| awakened_one_plan_transition_v1(&position, &step.position))
                    .flatten();
                let successor_plan = (!step.truncated)
                    .then(|| awakened_one_combat_plan_v1(&step.position))
                    .flatten();
                let after_hash = (!step.truncated).then(|| {
                    combat_exact_state_hash_v1(&step.position.engine, &step.position.combat)
                });
                trace.push(json!({
                    "action_index": index,
                    "label": label,
                    "action_key": action_key,
                    "input": input,
                    "before_exact_state_hash": before_hash,
                    "after_exact_state_hash": after_hash,
                    "engine_steps": step.engine_steps,
                    "truncated": step.truncated,
                    "timed_out": step.timed_out,
                    "terminal": format!("{:?}", step.terminal),
                    "plan_transition": transition,
                    "successor_plan": successor_plan,
                }));
                consumed_actions = consumed_actions.saturating_add(1);
                position = step.position;
                if step.truncated || step.terminal != CombatTerminal::Unresolved {
                    break;
                }
            }

            let final_terminal = combat_terminal(&position.engine, &position.combat);
            print_json(&json!({
                "schema_name": "OracleCombatCasePlanTraceV1",
                "schema_version": 1,
                "case": case_path,
                "actions": action_paths,
                "runtime": oracle_lab_runtime_identity(),
                "contract": {
                    "search": false,
                    "policy_mutation": false,
                    "ranking": false,
                    "pruning": false,
                    "caller_supplied_actions": true,
                    "terminal_truth": "exact_simulator_only",
                },
                "root_exact_state_hash": root_exact_state_hash,
                "root_plan": root_plan,
                "input_action_count": input_count,
                "consumed_action_count": consumed_actions,
                "unconsumed_action_count": input_count.saturating_sub(consumed_actions),
                "final_exact_state_hash": combat_exact_state_hash_v1(
                    &position.engine,
                    &position.combat,
                ),
                "final_terminal": format!("{final_terminal:?}"),
                "final_player_hp": position.combat.entities.player.current_hp,
                "final_plan": awakened_one_combat_plan_v1(&position),
                "max_engine_steps_per_transition": max_engine_steps_per_transition,
                "trace": trace,
            }))
        }
        Command::CombatCaseAtomicLevin {
            case,
            action_imitation_artifact,
            max_transitions,
            wall_ms,
            max_engine_steps_per_transition,
            uniform_exploration_ppm,
            reroot_player_turn_boundaries,
            watch_state_hash,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let case_path = case.clone();
            let case = load_combat_case(&case)?;
            let root = CombatDecisionRoot::new(case.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let initial_hp = root.position().combat.entities.player.current_hp;
            let policy = action_imitation_artifact
                .as_deref()
                .map(|path| {
                    load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                })
                .transpose()?
                .unwrap_or_else(existing_combat_knowledge_policy_v1);
            let mut search = AtomicLevinWitnessSession::with_policy(
                root,
                AtomicLevinWitnessConfig {
                    max_engine_steps_per_transition,
                    uniform_exploration_ppm,
                    rerooting: if reroot_player_turn_boundaries {
                        AtomicLevinRerooting::PlayerTurnBoundaries
                    } else {
                        AtomicLevinRerooting::Disabled
                    },
                    ..AtomicLevinWitnessConfig::default()
                },
                policy,
            );
            for exact_state_hash in &watch_state_hash {
                search.watch_exact_state_hash(exact_state_hash.clone());
            }
            let started = Instant::now();
            let report = search.advance(
                &EngineCombatStepper,
                AtomicLevinWitnessQuantum {
                    additional_applied_transitions: max_transitions,
                    additional_engine_steps: max_transitions
                        .saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(started + Duration::from_millis(wall_ms)),
                },
            );
            let elapsed = started.elapsed();
            if let (Some(path), Some(witness)) =
                (export_witness_actions.as_ref(), report.witness.as_ref())
            {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let actions = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                let bytes =
                    serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?;
                std::fs::write(path, bytes).map_err(|error| error.to_string())?;
            }
            print_json(&serde_json::json!({
                    "schema_name": "OracleCombatCaseAtomicLevinV1",
                    "schema_version": 1,
                    "case": case_path,
                    "runtime": oracle_lab_runtime_identity(),
                    "mode": {
                        "search": "atomic_levin_policy_tree",
                        "state_guides": false,
                        "complete_turn_generator": false,
                        "v2_donor": false,
                        "action_imitation_artifact": action_imitation_artifact,
                        "uniform_exploration_ppm": uniform_exploration_ppm,
                        "rerooting": if reroot_player_turn_boundaries {
                            "player_turn_boundaries"
                        } else {
                            "disabled"
                        },
                    },
                    "status": format!("{:?}", report.status),
                    "timing_ms": {
                        "setup": started.duration_since(command_started).as_millis(),
                        "search": elapsed.as_millis(),
                        "total_before_print": command_started.elapsed().as_millis(),
                    },
                    "budget": {
                        "max_transitions": max_transitions,
                        "wall_ms": wall_ms,
                        "max_engine_steps_per_transition": max_engine_steps_per_transition,
                    },
                    "work": {
                        "work_pops": report.after.work_pops,
                        "expanded_exact_states": report.after.expanded_exact_states,
                        "applied_action_transitions": report.after.applied_action_transitions,
                        "engine_steps": report.after.engine_steps,
                        "exact_states": report.after.exact_states,
                        "reopened_exact_states": report.after.reopened_exact_states,
                        "duplicate_or_dominated_successors": report.after.duplicate_or_dominated_successors,
                        "structured_inputs_materialized": report.after.structured_inputs_materialized,
                        "reroot_points_assigned": report.after.reroot_points_assigned,
                        "rerooted_action_transitions": report.after.rerooted_action_transitions,
                    },
                    "frontier": {
                        "entries": report.frontier_entries,
                        "max_atomic_depth": report.max_atomic_depth,
                        "max_player_turn": report.max_player_turn,
                        "unsupported_stable_boundaries": report.unsupported_stable_boundaries,
                        "transition_step_limit_gaps": report.transition_step_limit_gaps,
                    },
                    "watched_states": watch_state_hash.iter().map(|exact_state_hash| {
                        let state = search.watched_state(exact_state_hash);
                        json!({
                            "exact_state_hash": exact_state_hash,
                            "state": state.map(|state| json!({
                                "discovered": state.discovered,
                                "accepted": state.accepted,
                                "expanded": state.expanded,
                                "first_discovery_after_transitions": state.first_discovery_after_transitions,
                                "first_expansion_after_work_pops": state.first_expansion_after_work_pops,
                                "best_atomic_depth": state.best_atomic_depth,
                                "best_negative_log_policy": state.best_negative_log_policy,
                                "best_levin_log_priority": state.best_levin_log_priority,
                                "reroot_ordinal": state.reroot_ordinal,
                                "reroot_weight": state.reroot_weight,
                            })),
                        })
                    }).collect::<Vec<_>>(),
                    "exported_witness_actions": report.witness.is_some()
                        .then_some(export_witness_actions.as_ref())
                        .flatten(),
                    "witness": report.witness.as_ref().map(|witness| serde_json::json!({
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
        Command::CombatCasePolicyDiscrepancy {
            case,
            action_imitation_artifact,
            max_transitions,
            wall_ms,
            max_engine_steps_per_transition,
            uniform_exploration_ppm,
            max_greedy_actions_per_dive,
            turn_macro_transitions,
            turn_macro_proposals_per_view,
            watch_case,
            audit_actions,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let case_path = case.clone();
            let case = load_combat_case(&case)?;
            let root_position = case.position;
            let root = CombatDecisionRoot::new(root_position.clone())
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let initial_hp = root.position().combat.entities.player.current_hp;
            let watched_positions = watch_case
                .iter()
                .map(|path| load_combat_case(path).map(|case| (path.clone(), case.position)))
                .collect::<Result<Vec<_>, _>>()?;
            let policy = action_imitation_artifact
                .as_deref()
                .map(|path| {
                    load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                })
                .transpose()?
                .unwrap_or_else(existing_combat_knowledge_policy_v1);
            let search_config = PolicyDiscrepancyConfig {
                max_engine_steps_per_transition,
                uniform_exploration_ppm,
                max_greedy_actions_per_dive,
                turn_macro: (turn_macro_transitions > 0).then_some(
                    PolicyDiscrepancyTurnMacroConfig {
                        max_applied_transitions: turn_macro_transitions,
                        proposals_per_view: turn_macro_proposals_per_view,
                        ..PolicyDiscrepancyTurnMacroConfig::default()
                    },
                ),
            };
            let trajectory_audit = if audit_actions.is_empty() {
                None
            } else {
                let inputs = load_combat_action_segments(&audit_actions)?;
                let audit_root = CombatDecisionRoot::new(root_position.clone())
                    .map_err(|error| format!("invalid trajectory audit root: {error:?}"))?;
                let mut audit = PolicyDiscrepancySession::with_policy(
                    audit_root,
                    search_config,
                    policy.clone(),
                );
                Some(audit.audit_trajectory(&EngineCombatStepper, &inputs)?)
            };
            let mut search = PolicyDiscrepancySession::with_policy(root, search_config, policy);
            let started = Instant::now();
            let report = search.advance(
                &EngineCombatStepper,
                PolicyDiscrepancyQuantum {
                    additional_applied_transitions: max_transitions,
                    additional_engine_steps: max_transitions
                        .saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(started + Duration::from_millis(wall_ms)),
                },
            );
            let elapsed = started.elapsed();
            let watched = watched_positions
                .iter()
                .map(|(path, position)| {
                    let diagnostic = search.state_diagnostic(position);
                    json!({
                        "case": path,
                        "exact_state_hash": diagnostic.exact_state_hash,
                        "discovered": diagnostic.discovered,
                        "best_discrepancy": diagnostic.best_discrepancy,
                        "policy_dive_services": diagnostic.policy_dive_services,
                        "selected_by_turn_macro": diagnostic.selected_by_turn_macro,
                        "turn_macro_scheduled": diagnostic.turn_macro_scheduled,
                    })
                })
                .collect::<Vec<_>>();
            if let (Some(path), Some(witness)) =
                (export_witness_actions.as_ref(), report.witness.as_ref())
            {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let actions = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            print_json(&json!({
                "schema_name": "OracleCombatCasePolicyDiscrepancyV1",
                "schema_version": 1,
                "case": case_path,
                "runtime": oracle_lab_runtime_identity(),
                "mode": {
                    "search": "policy_discrepancy_complete_trajectories",
                    "state_guides": turn_macro_transitions > 0,
                    "complete_turn_generator": turn_macro_transitions > 0,
                    "lazy_turn_macro_proposals": turn_macro_transitions > 0,
                    "v2_donor": false,
                    "action_imitation_artifact": action_imitation_artifact,
                },
                "status": format!("{:?}", report.status),
                "timing_ms": {
                    "setup": started.duration_since(command_started).as_millis(),
                    "search": elapsed.as_millis(),
                    "total_before_print": command_started.elapsed().as_millis(),
                },
                "budget": {
                    "max_transitions": max_transitions,
                    "wall_ms": wall_ms,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                    "max_greedy_actions_per_dive": max_greedy_actions_per_dive,
                    "turn_macro_transitions": turn_macro_transitions,
                    "turn_macro_proposals_per_view": turn_macro_proposals_per_view,
                },
                "work": {
                    "policy_dives": report.after.policy_dives,
                    "applied_action_transitions": report.after.applied_action_transitions,
                    "engine_steps": report.after.engine_steps,
                    "exact_states": report.after.exact_states,
                    "queued_discrepancies": report.after.queued_discrepancies,
                    "structured_inputs_materialized": report.after.structured_inputs_materialized,
                    "duplicate_or_dominated_states": report.after.duplicate_or_dominated_states,
                    "unsupported_stable_boundaries": report.after.unsupported_stable_boundaries,
                    "transition_step_limit_gaps": report.after.transition_step_limit_gaps,
                    "greedy_depth_limit_hits": report.after.greedy_depth_limit_hits,
                    "turn_macro_generations": report.after.turn_macro_generations,
                    "turn_macro_partial_generations": report.after.turn_macro_partial_generations,
                    "turn_macro_applied_transitions": report.after.turn_macro_applied_transitions,
                    "turn_macro_options_generated": report.after.turn_macro_options_generated,
                    "turn_macro_options_enqueued": report.after.turn_macro_options_enqueued,
                },
                "frontier": {
                    "entries": report.frontier_entries,
                    "best_queued_priority": report.best_queued_priority,
                    "best_queued_discrepancy": report.best_queued_discrepancy,
                },
                "watched": watched,
                "trajectory_audit": trajectory_audit.as_ref().map(|audit| json!({
                    "source_action_count": audit.source_action_count,
                    "non_greedy_action_count": audit.non_greedy_action_count,
                    "total_weighted_discrepancy": audit.total_weighted_discrepancy,
                    "terminal": format!("{:?}", audit.terminal),
                    "deviations": audit.deviations.iter().map(|deviation| json!({
                        "action_index": deviation.action_index,
                        "player_turn": deviation.player_turn,
                        "demonstrated_input": deviation.demonstrated_input,
                        "greedy_input": deviation.greedy_input,
                        "demonstrated_probability": deviation.demonstrated_probability,
                        "greedy_probability": deviation.greedy_probability,
                        "discrepancy_increment": deviation.discrepancy_increment,
                        "cumulative_discrepancy": deviation.cumulative_discrepancy,
                        "demonstrated_was_lazy": deviation.demonstrated_was_lazy,
                    })).collect::<Vec<_>>(),
                })),
                "exported_witness_actions": report.witness.is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
                "witness": report.witness.as_ref().map(|witness| json!({
                    "final_hp": witness.final_position.combat.entities.player.current_hp,
                    "hp_loss": initial_hp.saturating_sub(
                        witness.final_position.combat.entities.player.current_hp,
                    ),
                    "action_count": witness.actions.len(),
                    "weighted_discrepancy": witness.negative_log_policy,
                    "replay_engine_steps": witness.replay_engine_steps,
            })),
            }))
        }
        Command::CombatCaseAtomicTurnPortfolio {
            case,
            action_imitation_artifact,
            max_search_work,
            wall_ms,
            max_engine_steps_per_transition,
            uniform_exploration_ppm,
            initial_boundary_work,
            boundary_service_work,
            suffix_service_work,
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
        } => {
            let command_started = Instant::now();
            let case_path = case.clone();
            let case = load_combat_case(&case)?;
            let root = CombatDecisionRoot::new(case.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let initial_hp = root.position().combat.entities.player.current_hp;
            let boundary_policy = existing_combat_knowledge_policy_v1();
            let suffix_policy = action_imitation_artifact
                .as_deref()
                .map(|path| {
                    load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                })
                .transpose()?
                .unwrap_or_else(existing_combat_knowledge_policy_v1);
            let boundary_config = TurnOptionGeneratorConfig {
                max_engine_steps_per_transition,
                uniform_exploration_ppm,
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
            if let (Some(path), Some(witness)) =
                (export_witness_actions.as_ref(), report.witness.as_ref())
            {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let actions = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
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
                                            && (task_anchor_key(other)
                                                .total_cmp(&anchor_key)
                                                .is_lt()
                                                || (task_anchor_key(other)
                                                    .total_cmp(&anchor_key)
                                                    .is_eq()
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
                "schema_name": "OracleCombatCaseAtomicTurnPortfolioV5",
                "schema_version": 5,
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
                    "suffix_turn_macro_transitions": policy_discrepancy_suffix
                        .then_some(suffix_turn_macro_transitions),
                    "boundary_layers": boundary_layers,
                    "terminal_work_per_boundary_batch": terminal_work_per_boundary_batch,
                },
                "work": {
                    "services": report.after.services,
                    "boundary_services": report.after.boundary_services,
                    "suffix_services": report.after.suffix_services,
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
        Command::CombatCase {
            case,
            action_imitation_artifact,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            anchor_only,
            without_v2_donor,
            watch_state_hash,
            watch_corridor_actions,
            corridor_prefix_turns,
            prefix_actions,
            prefix_stop_at_player_turn,
            readable,
            full,
            replay_only,
            counterfactual_hp,
            export_prefix_case,
            shadow_corridor_actions,
            shadow_corridor_case,
            shadow_corridor_guide,
            shadow_corridor_only,
            shadow_value_prototype,
            export_witness_actions,
            export_augmented_value_prototype,
            one_turn_loss_evidence_limit,
            one_turn_viability_evidence_limit,
        } => {
            let command_started = Instant::now();
            let case_path = case.clone();
            let watched_corridor = watch_corridor_actions
                .as_ref()
                .map(|actions| {
                    load_exact_turn_corridor(
                        &case,
                        std::slice::from_ref(actions),
                        max_engine_steps_per_transition,
                    )
                })
                .transpose()?;
            let mut case = load_combat_case(&case)?;
            let original_hp = case.position.combat.entities.player.current_hp;
            if let Some(hp) = counterfactual_hp {
                let max_hp = case.position.combat.entities.player.max_hp;
                if !(1..=max_hp).contains(&hp) {
                    return Err(format!(
                        "counterfactual HP must be within 1..={max_hp}, got {hp}"
                    ));
                }
                case.position.combat.entities.player.current_hp = hp;
                case.combat = sts_simulator::eval::combat_case::combat_summary(&case.position);
            }
            let stepper = EngineCombatStepper;
            let initial_position = case.position.clone();
            let mut position = initial_position.clone();
            let mut prefix = prefix_actions
                .iter()
                .map(|path| {
                    serde_json::from_slice::<Vec<ClientInput>>(
                        &std::fs::read(path).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| format!("invalid prefix action list: {error}"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            if let Some(turns) = corridor_prefix_turns {
                let actions_path = watch_corridor_actions
                    .as_ref()
                    .expect("clap requires watched corridor actions");
                let corridor_actions = serde_json::from_slice::<Vec<ClientInput>>(
                    &std::fs::read(actions_path).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("invalid corridor action list: {error}"))?;
                if turns > 0 {
                    let mut ended_turns = 0_usize;
                    for input in corridor_actions {
                        let ends_turn = matches!(input, ClientInput::EndTurn);
                        prefix.push(input);
                        if ends_turn {
                            ended_turns = ended_turns.saturating_add(1);
                            if ended_turns == turns {
                                break;
                            }
                        }
                    }
                    if ended_turns != turns {
                        return Err(format!(
                            "corridor contains only {ended_turns} completed player turns; requested prefix {turns}"
                        ));
                    }
                }
            }
            let mut applied_prefix = Vec::with_capacity(prefix.len());
            let mut prefix_replay_actions = Vec::with_capacity(prefix.len());
            for (action_index, input) in prefix.iter().enumerate() {
                if prefix_stop_at_player_turn.is_some_and(|target_turn| {
                    position.combat.turn.turn_count == target_turn
                        && matches!(position.engine, EngineState::CombatPlayerTurn)
                }) {
                    break;
                }
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
                prefix_replay_actions.push(TurnOptionAction {
                    input: input.clone(),
                    expected_successor_hash:
                        sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                            &step.position.engine,
                            &step.position.combat,
                        ),
                    engine_steps: step.engine_steps,
                });
                applied_prefix.push(input.clone());
                position = step.position;
            }
            if let Some(target_turn) = prefix_stop_at_player_turn {
                if position.combat.turn.turn_count != target_turn
                    || !matches!(position.engine, EngineState::CombatPlayerTurn)
                {
                    return Err(format!(
                        "prefix did not reach player turn {target_turn}; stopped at turn {} in {:?}",
                        position.combat.turn.turn_count, position.engine
                    ));
                }
            }
            prefix = applied_prefix;
            if let Some(path) = export_prefix_case.as_ref() {
                let mut focused_case = case.clone();
                focused_case.position = position.clone();
                focused_case.combat =
                    sts_simulator::eval::combat_case::combat_summary(&focused_case.position);
                focused_case.gap.boundary = format!(
                    "{} + {} exact prefix actions",
                    focused_case.gap.boundary,
                    prefix.len()
                );
                focused_case.gap.reason = "oracle_lab_prefix_successor".to_string();
                sts_simulator::eval::combat_case::save_combat_case(path, &focused_case)?;
            }
            if replay_only {
                let prefix_trace = replay_combat_path(
                    initial_position,
                    &prefix_replay_actions,
                    max_engine_steps_per_transition,
                )?;
                return print_json(&serde_json::json!({
                    "schema_name": "OracleCombatPrefixReplayV1",
                    "schema_version": 1,
                    "action_count": prefix.len(),
                    "counterfactual": {
                        "enabled": counterfactual_hp.is_some(),
                        "original_hp": original_hp,
                        "replay_hp": case.position.combat.entities.player.current_hp,
                    },
                    "exported_case": export_prefix_case,
                    "trace": prefix_trace,
                    "guide_components": {
                        "progress": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(&position),
                        "survival": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(&position),
                        "horizon": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(&position),
                        "setup": sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(&position),
                    },
                    "successor_exact_state_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                        &position.engine,
                        &position.combat,
                    ),
                    "successor": combat_position_snapshot(&position),
                }));
            }
            let search_root_position = position.clone();
            let root = CombatDecisionRoot::new(position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let initial_hp = root.position().combat.entities.player.current_hp;
            let base_policy = action_imitation_artifact
                .as_deref()
                .map(|path| {
                    load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                })
                .transpose()?
                .unwrap_or_else(existing_combat_knowledge_policy_v1);
            let (policy, shadow_corridor, mut shadow_value_artifact) =
                if let Some(model_path) = shadow_value_prototype.as_ref() {
                    let artifact = load_value_prototype(model_path)?;
                    let policy = value_prototype_shadow_policy(base_policy, &artifact);
                    (policy, None, Some(artifact))
                } else {
                    match (
                        shadow_corridor_case.as_ref(),
                        shadow_corridor_actions.as_ref(),
                    ) {
                        (Some(case_path), Some(actions_path)) => {
                            let corridor = load_exact_turn_corridor(
                                case_path,
                                std::slice::from_ref(actions_path),
                                max_engine_steps_per_transition,
                            )?;
                            let policy = exact_corridor_shadow_policy(
                                base_policy,
                                &corridor,
                                shadow_corridor_guide,
                                shadow_corridor_only,
                            );
                            (policy, Some(corridor), None)
                        }
                        (None, None) => (base_policy, None, None),
                        _ => unreachable!("clap requires both shadow corridor arguments"),
                    }
                };
            let policy = if anchor_only {
                anchor_only_policy(policy)
            } else {
                policy
            };
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
                policy,
            );
            search.set_one_turn_loss_evidence_limit(one_turn_loss_evidence_limit);
            search.set_one_turn_viability_evidence_limit(one_turn_viability_evidence_limit);
            let started = Instant::now();
            let deadline = started + Duration::from_millis(wall_ms);
            let mut advisor_nodes = 0u64;
            let mut advisor_elapsed_ms = 0u64;
            let mut advisor_status = "disabled";
            if !without_v2_donor {
                let mut advisor = ExistingCombatKnowledgeAdvisorV1::new(
                    &search_root_position,
                    max_engine_steps_per_transition,
                );
                let remaining = deadline.saturating_duration_since(Instant::now());
                match advisor.advance(Some(remaining), Some(remaining))? {
                    ExistingCombatKnowledgeAdvisorAdvanceV1::Pending => {
                        advisor_status = "pending";
                    }
                    ExistingCombatKnowledgeAdvisorAdvanceV1::Proposal(proposal) => {
                        search.offer_witness_proposal(proposal);
                        advisor_status = "proposal";
                    }
                    ExistingCombatKnowledgeAdvisorAdvanceV1::Exhausted => {
                        advisor_status = "exhausted";
                    }
                }
                advisor_nodes = advisor.total_nodes();
                advisor_elapsed_ms = advisor
                    .total_elapsed()
                    .as_millis()
                    .min(u128::from(u64::MAX)) as u64;
            }
            let report = search.advance(
                &EngineCombatStepper,
                OracleCombatWitnessQuantum {
                    additional_agenda_pops: max_nodes,
                    additional_generation_work: max_nodes,
                    additional_engine_steps: max_nodes
                        .saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(deadline),
                },
            );
            let search_elapsed = started.elapsed();
            let summary_started = Instant::now();
            let progress = search.progress_snapshot();
            if let (Some(path), Some(artifact)) = (
                export_augmented_value_prototype.as_ref(),
                shadow_value_artifact.as_mut(),
            ) {
                artifact.add_one_turn_viability_positions(
                    search
                        .one_turn_viability_evidence()
                        .iter()
                        .map(|sample| &sample.position),
                );
                artifact.add_one_turn_loss_positions(
                    search
                        .one_turn_loss_evidence()
                        .iter()
                        .map(|sample| &sample.position),
                );
                save_value_prototype(path, artifact)?;
            }
            let shadow_corridor_report = shadow_corridor
                .as_ref()
                .map(|corridor| corridor.report(&search, shadow_corridor_guide))
                .or_else(|| {
                    shadow_value_artifact
                        .as_ref()
                        .map(CombatValuePrototypeArtifactV1::report)
                });
            let one_turn_viability_evidence = search
                .one_turn_viability_evidence()
                .iter()
                .map(|evidence| {
                    json!({
                        "proof": "ExactWitness",
                        "horizon": "BeforeNextPlayerTurnOrWin",
                        "exact_state_hash": evidence.exact_state_hash,
                        "player_turn": evidence.position.combat.turn.turn_count,
                        "player_hp": evidence.position.combat.entities.player.current_hp,
                        "witness_boundary": format!("{:?}", evidence.witness_boundary),
                        "path_action_count": evidence.actions.len(),
                        "witness_turn_action_count": evidence.witness_turn_actions.len(),
                        "typed_features": typed_combat_feature_components(&evidence.position),
                    })
                })
                .collect::<Vec<_>>();
            let one_turn_loss_evidence = search
                .one_turn_loss_evidence()
                .iter()
                .map(|evidence| {
                    json!({
                        "proof": "ExhaustiveRefutation",
                        "horizon": "BeforeNextPlayerTurn",
                        "exact_state_hash": evidence.exact_state_hash,
                        "player_turn": evidence.position.combat.turn.turn_count,
                        "player_hp": evidence.position.combat.entities.player.current_hp,
                        "terminal_loss_turn_options": evidence.terminal_loss_turn_options,
                        "path_action_count": evidence.actions.len(),
                        "typed_features": typed_combat_feature_components(&evidence.position),
                    })
                })
                .collect::<Vec<_>>();
            let watched_states = watch_state_hash
                .iter()
                .map(|hash| search.state_membership_by_exact_hash(hash))
                .collect::<Vec<_>>();
            let watched_corridor_report = watched_corridor
                .as_ref()
                .map(|corridor| corridor.diagnostic_report(&search));
            let watched_state = (watched_states.len() == 1)
                .then(|| watched_states.first().cloned())
                .flatten();
            let witness = report.witness.as_ref();
            if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), witness) {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let actions = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                let bytes =
                    serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?;
                std::fs::write(path, bytes).map_err(|error| error.to_string())?;
            }
            if !full && !readable {
                let summary_elapsed = summary_started.elapsed();
                return print_json(&serde_json::json!({
                    "schema_name": "OracleCombatCaseCompactV1",
                    "schema_version": 1,
                    "case": case_path,
                    "runtime": oracle_lab_runtime_identity(),
                    "mode": {
                        "v2_donor_enabled": !without_v2_donor,
                        "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                        "action_imitation_artifact": action_imitation_artifact,
                    },
                    "status": format!("{:?}", report.status),
                    "timing_ms": {
                        "setup": started.duration_since(command_started).as_millis(),
                        "search": search_elapsed.as_millis(),
                        "summary": summary_elapsed.as_millis(),
                        "total_before_print": command_started.elapsed().as_millis(),
                    },
                    "budget": {
                        "generation_work": max_nodes,
                        "wall_ms": wall_ms,
                        "max_engine_steps_per_transition": max_engine_steps_per_transition,
                    },
                    "advisor": {
                        "status": advisor_status,
                        "nodes": advisor_nodes,
                        "elapsed_ms": advisor_elapsed_ms,
                    },
                    "work": {
                        "agenda_pops": report.after.agenda_pops,
                        "generation_work": report.after.generation_work,
                        "engine_steps": report.after.engine_steps,
                        "exact_states": report.after.exact_states,
                        "completed_turn_options": report.after.completed_turn_options,
                        "applied_action_transitions": report.after.applied_action_transitions,
                    },
                    "frontier": {
                        "retained_states": progress.retained_states,
                        "anchor_entries": progress.queued_anchor_entries,
                        "guide_queues": progress.guide_queues.iter().map(|queue| serde_json::json!({
                            "lane_id": queue.lane_id,
                            "lane": oracle_lab_guide_lane_label(queue.lane_id),
                            "entries": queue.entries,
                        })).collect::<Vec<_>>(),
                        "max_player_turn": progress.max_player_turn,
                        "max_path_atomic_depth": progress.max_path_atomic_depth,
                        "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
                        "generation_gap_count": progress.generation_gap_count,
                    },
                    "root": progress.root_state,
                    "deepest": {
                        "survival": progress.deepest_survival_state,
                        "progress": progress.deepest_progress_state,
                    },
                    "watched_state": watched_state,
                    "watched_states": (watched_states.len() != 1).then_some(watched_states),
                    "watched_corridor": compact_corridor_report(watched_corridor_report.as_ref()),
                    "shadow_corridor": compact_corridor_report(shadow_corridor_report.as_ref()),
                    "evidence": {
                        "one_turn_viable": one_turn_viability_evidence,
                        "one_turn_losses": one_turn_loss_evidence,
                    },
                    "exports": {
                        "witness_actions": witness.is_some().then_some(export_witness_actions.as_ref()).flatten(),
                        "augmented_value_prototype": export_augmented_value_prototype,
                    },
                    "witness": witness.map(|witness| serde_json::json!({
                        "discovery_source": witness.discovery_source,
                        "final_hp": witness.final_position.combat.entities.player.current_hp,
                        "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                        "action_count": witness.actions.len(),
                        "negative_log_policy": witness.negative_log_policy,
                        "replay_engine_steps": witness.replay_engine_steps,
                    })),
                }));
            }
            let prefix_trace = replay_combat_path(
                initial_position,
                &prefix_replay_actions,
                max_engine_steps_per_transition,
            )?;
            let deepest_progress_trace = replay_combat_path(
                search_root_position.clone(),
                &progress.deepest_progress_actions,
                max_engine_steps_per_transition,
            )?;
            let deepest_survival_trace =
                if progress.deepest_survival_actions == progress.deepest_progress_actions {
                    serde_json::json!({"same_as": "deepest_progress_trace"})
                } else {
                    replay_combat_path(
                        search_root_position.clone(),
                        &progress.deepest_survival_actions,
                        max_engine_steps_per_transition,
                    )?
                };
            let witness_trace = witness
                .map(|witness| {
                    replay_combat_path(
                        search_root_position.clone(),
                        &witness.actions,
                        max_engine_steps_per_transition,
                    )
                })
                .transpose()?;
            if readable {
                return print_json(&serde_json::json!({
                    "schema_name": "OracleCombatCaseReadableV1",
                    "schema_version": 1,
                    "v2_donor_enabled": !without_v2_donor,
                    "action_imitation_artifact": action_imitation_artifact,
                    "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                    "status": format!("{:?}", report.status),
                    "elapsed_ms": started.elapsed().as_millis(),
                    "budget": {
                        "max_nodes": max_nodes,
                        "wall_ms": wall_ms,
                    },
                    "advisor": {
                        "status": advisor_status,
                        "nodes": advisor_nodes,
                        "elapsed_ms": advisor_elapsed_ms,
                    },
                    "shadow_corridor": shadow_corridor_report,
                    "watched_corridor": watched_corridor_report,
                    "one_turn_viability_evidence": one_turn_viability_evidence,
                    "one_turn_loss_evidence": one_turn_loss_evidence,
                    "exported_augmented_value_prototype": export_augmented_value_prototype,
                    "exported_witness_actions": witness
                        .is_some()
                        .then_some(export_witness_actions.as_ref())
                        .flatten(),
                    "counters": {
                        "agenda_pops": report.after.agenda_pops,
                        "generation_work": report.after.generation_work,
                        "exact_states": report.after.exact_states,
                        "completed_turn_options": report.after.completed_turn_options,
                        "exact_one_turn_viable_states": report.after.exact_one_turn_viable_states,
                        "exhaustive_one_turn_losses": report.after.exhaustive_one_turn_losses,
                    },
                    "prefix": {
                        "trace": prefix_trace,
                        "successor_exact_state_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                            &search_root_position.engine,
                            &search_root_position.combat,
                        ),
                        "successor": combat_position_snapshot(&search_root_position),
                    },
                    "progress": {
                        "max_player_turn": progress.max_player_turn,
                        "deepest_survival_state": progress.deepest_survival_state,
                        "deepest_survival_trace": deepest_survival_trace,
                        "deepest_progress_state": progress.deepest_progress_state,
                        "deepest_progress_trace": deepest_progress_trace,
                        "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
                        "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
                        "generation_gap_count": progress.generation_gap_count,
                        "watched_state": watched_state,
                        "watched_states": watched_states,
                    },
                    "witness": witness.map(|witness| serde_json::json!({
                        "discovery_source": witness.discovery_source,
                        "final_hp": witness.final_position.combat.entities.player.current_hp,
                        "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                        "trace": witness_trace,
                    })),
                }));
            }
            print_json(&serde_json::json!({
                "schema_name": "OracleCombatCaseProbeV1",
                "schema_version": 1,
                "v2_donor_enabled": !without_v2_donor,
                "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                "status": format!("{:?}", report.status),
                "elapsed_ms": started.elapsed().as_millis(),
                "budget": {
                    "max_nodes": max_nodes,
                    "wall_ms": wall_ms,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                },
                "shadow_corridor": shadow_corridor_report,
                "watched_corridor": watched_corridor_report,
                "one_turn_viability_evidence": one_turn_viability_evidence,
                "one_turn_loss_evidence": one_turn_loss_evidence,
                "exported_augmented_value_prototype": export_augmented_value_prototype,
                "exported_witness_actions": witness
                    .is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
                "advisor": {
                    "status": advisor_status,
                    "nodes": advisor_nodes,
                    "elapsed_ms": advisor_elapsed_ms,
                },
                "prefix": {
                    "action_count": prefix.len(),
                    "actions": prefix,
                    "trace": prefix_trace,
                    "successor_exact_state_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                        &search_root_position.engine,
                        &search_root_position.combat,
                    ),
                    "successor": combat_position_snapshot(&search_root_position),
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
                    "exact_one_turn_viable_states": report.after.exact_one_turn_viable_states,
                    "exhaustive_one_turn_losses": report.after.exhaustive_one_turn_losses,
                },
                "progress": {
                    "retained_states": progress.retained_states,
                    "queued_anchor_entries": progress.queued_anchor_entries,
                    "queued_guided_entries": progress.queued_guided_entries,
                    "max_player_turn": progress.max_player_turn,
                    "deepest_survival_state": progress.deepest_survival_state,
                    "deepest_survival_actions": progress.deepest_survival_actions,
                    "deepest_survival_trace": deepest_survival_trace,
                    "deepest_progress_state": progress.deepest_progress_state,
                    "deepest_progress_actions": progress.deepest_progress_actions,
                    "deepest_progress_trace": deepest_progress_trace,
                    "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
                    "max_path_atomic_depth": progress.max_path_atomic_depth,
                    "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
                    "generation_gap_count": progress.generation_gap_count,
                    "root_state": progress.root_state,
                    "watched_state": watched_state,
                    "watched_states": watched_states,
                },
                "witness": witness.map(|witness| serde_json::json!({
                    "discovery_source": witness.discovery_source,
                    "final_hp": witness.final_position.combat.entities.player.current_hp,
                    "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                    "action_count": witness.actions.len(),
                    "negative_log_policy": witness.negative_log_policy,
                    "actions": witness.actions,
                })),
            }))
        }
        Command::TurnActionAudit {
            case,
            action_imitation_artifact,
            actions,
            through,
            max_engine_steps_per_transition,
        } => {
            let case = load_combat_case(&case)?;
            let mut position = case.position;
            if let Some(actions) = actions {
                let actions = serde_json::from_slice::<Vec<ClientInput>>(
                    &std::fs::read(actions).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("invalid prefix action list: {error}"))?;
                if through > actions.len() {
                    return Err(format!(
                        "--through {through} exceeds the {} available prefix actions",
                        actions.len()
                    ));
                }
                for (index, input) in actions.into_iter().take(through).enumerate() {
                    if EngineCombatStepper
                        .choice_for_legal_input(&position, &input)
                        .is_none()
                    {
                        return Err(format!("prefix action {index} is not legal"));
                    }
                    let result = EngineCombatStepper.apply_to_stable(
                        &position,
                        input,
                        CombatStepLimits {
                            max_engine_steps: max_engine_steps_per_transition,
                            deadline: None,
                        },
                    );
                    if result.truncated || result.timed_out {
                        return Err(format!(
                            "prefix action {index} did not reach a stable state"
                        ));
                    }
                    position = result.position;
                }
            }

            let policy = action_imitation_artifact
                .as_deref()
                .map(|path| {
                    load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                })
                .transpose()?
                .unwrap_or_else(existing_combat_knowledge_policy_v1);
            let surface = EngineCombatStepper.legal_action_surface(&position);
            let choices = surface
                .atomic_actions
                .iter()
                .map(CombatPolicyChoice::Atomic)
                .chain(
                    surface
                        .selection_families
                        .iter()
                        .map(CombatPolicyChoice::StructuredSelection),
                )
                .collect::<Vec<_>>();
            let raw_weights = policy.weights(&position, &choices);
            let raw_weights = (raw_weights.len() == choices.len())
                .then_some(raw_weights)
                .unwrap_or_else(|| vec![1.0; choices.len()]);
            let safe_weights = raw_weights
                .iter()
                .map(|weight| {
                    if weight.is_finite() && *weight > 0.0 {
                        *weight
                    } else {
                        1.0
                    }
                })
                .collect::<Vec<_>>();
            let total = safe_weights.iter().sum::<f64>();
            let uniform = 1.0 / safe_weights.len().max(1) as f64;
            let probabilities = safe_weights
                .iter()
                .map(|weight| 0.95 * (*weight / total) + 0.05 * uniform)
                .collect::<Vec<_>>();
            let atomic_priority_diagnostics =
                sts_simulator::ai::combat_search_v2::oracle_action_policy::
                    oracle_atomic_action_policy_priority_diagnostics_v1(
                        &position,
                        &surface.atomic_actions,
                    );
            let atomic = surface
                .atomic_actions
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    let result = EngineCombatStepper.apply_to_stable(
                        &position,
                        input.clone(),
                        CombatStepLimits {
                            max_engine_steps: max_engine_steps_per_transition,
                            deadline: None,
                        },
                    );
                    let raw_weight = safe_weights[index];
                    let rank = 1 + safe_weights
                        .iter()
                        .filter(|candidate| **candidate > raw_weight)
                        .count();
                    let successor_guides = (!result.truncated && !result.timed_out)
                        .then(|| {
                            policy
                                .turn_generation_guides(&result.position)
                                .into_iter()
                                .map(|guide| json!({
                                    "lane": guide.lane.value(),
                                    "components": guide.rank.components(),
                                }))
                                .collect::<Vec<_>>()
                        });
                    json!({
                        "canonical_index": index,
                        "label": combat_action_label(&position, input),
                        "key": combat_action_key(&position.combat, input),
                        "raw_weight": raw_weight,
                        "probability": probabilities[index],
                        "ordinal_rank": rank,
                        "priority": atomic_priority_diagnostics[index],
                        "transition": {
                            "truncated": result.truncated,
                            "timed_out": result.timed_out,
                            "engine_steps": result.engine_steps,
                            "terminal": format!("{:?}", result.terminal),
                            "exact_successor_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                                &result.position.engine,
                                &result.position.combat,
                            ),
                            "snapshot": combat_turn_snapshot(&result.position),
                            "generation_guides": successor_guides,
                        },
                    })
                })
                .collect::<Vec<_>>();
            let family_offset = surface.atomic_actions.len();
            let structured_families = surface
                .selection_families
                .iter()
                .enumerate()
                .map(|(index, family)| {
                    let weight_index = family_offset + index;
                    let raw_weight = safe_weights[weight_index];
                    let rank = 1 + safe_weights
                        .iter()
                        .filter(|candidate| **candidate > raw_weight)
                        .count();
                    json!({
                        "family_index": index,
                        "reason": format!("{:?}", family.reason),
                        "declared_min": family.declared_min,
                        "effective_max": family.effective_max,
                        "eligible_domain_count": family.eligible_domain_count,
                        "raw_weight": raw_weight,
                        "probability": probabilities[weight_index],
                        "ordinal_rank": rank,
                    })
                })
                .collect::<Vec<_>>();
            print_json(&json!({
                "schema_name": "OracleTurnActionAuditV1",
                "schema_version": 2,
                "through": through,
                "position_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                    &position.engine,
                    &position.combat,
                ),
                "position": combat_turn_snapshot(&position),
                "current_generation_guides": policy
                    .turn_generation_guides(&position)
                    .into_iter()
                    .map(|guide| json!({
                        "lane": guide.lane.value(),
                        "components": guide.rank.components(),
                    }))
                    .collect::<Vec<_>>(),
                "atomic_actions": atomic,
                "structured_families": structured_families,
            }))
        }
        Command::TurnPlanAudit {
            case,
            max_inner_nodes,
            max_end_states,
            per_bucket_limit,
            max_engine_steps_per_transition,
            limit,
            full,
            export_rank,
            export_case,
            export_actions,
        } => {
            let case = load_combat_case(&case)?;
            let mut config = sts_simulator::ai::combat_search_v2::CombatSearchV2Config::default();
            config.max_engine_steps_per_action = max_engine_steps_per_transition.max(1);
            config.turn_plan_probe_max_inner_nodes = Some(max_inner_nodes.max(1));
            config.turn_plan_probe_max_end_states = Some(max_end_states.max(1));
            config.turn_plan_probe_per_bucket_limit = Some(per_bucket_limit.max(1));
            config.input_label = Some("oracle_lab_turn_plan_audit".to_string());
            let audit = sts_simulator::ai::combat_search_v2::
                enumerate_combat_search_v2_turn_plan_probe_candidates_across_pending_choices(
                    &case.position.engine,
                    &case.position.combat,
                    &config,
                );
            let exported_plan = if let Some(rank) = export_rank {
                let candidate = audit
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.report.bucket != "terminal_loss")
                    .nth(rank)
                    .ok_or_else(|| format!("non-loss turn-plan rank {rank} is unavailable"))?;
                if let Some(path) = export_case.as_ref() {
                    let mut exported = case.clone();
                    exported.position = candidate.position.clone();
                    exported.combat =
                        sts_simulator::eval::combat_case::combat_summary(&exported.position);
                    exported.run.hp = exported.position.combat.entities.player.current_hp;
                    exported.run.max_hp = exported.position.combat.entities.player.max_hp;
                    exported.gap.boundary =
                        format!("{} + audited turn plan rank {rank}", exported.gap.boundary);
                    exported.gap.reason = "oracle_lab_turn_plan_audit_successor".to_string();
                    exported.combat_search_attempts.clear();
                    exported.failed_search = None;
                    save_combat_case(path, &exported)?;
                }
                if let Some(path) = export_actions.as_ref() {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                    }
                    let inputs = candidate
                        .report
                        .actions
                        .iter()
                        .map(|action| action.input.clone())
                        .collect::<Vec<_>>();
                    std::fs::write(
                        path,
                        serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
                }
                Some(json!({
                    "rank": rank,
                    "plan_index": candidate.report.plan_index,
                    "case": export_case,
                    "actions": export_actions,
                }))
            } else {
                None
            };
            let selected = audit
                .report
                .candidates
                .iter()
                .map(|candidate| {
                    json!({
                        "plan_index": candidate.plan_index,
                        "bucket": candidate.bucket,
                        "stop_reason": candidate.stop_reason,
                        "action_count": candidate.action_count,
                        "actions": candidate.actions.iter().map(|action| {
                            json!({
                                "key": action.action_key,
                                "debug": action.action_debug,
                            })
                        }).collect::<Vec<_>>(),
                        "end_exact_state_hash": candidate.steps.last().map(|step| {
                            step.state_after_exact_state_hash.as_str()
                        }),
                        "final_hp": candidate.eval_final_hp,
                        "risk_margin": candidate.eval_risk_margin,
                        "enemy_progress": candidate.eval_enemy_progress,
                    })
                })
                .collect::<Vec<_>>();
            let preselection = audit
                .report
                .selection_audit
                .candidates
                .iter()
                .map(|candidate| {
                    json!({
                        "preselection_rank": candidate.preselection_rank,
                        "selected_plan_index": candidate.selected_plan_index,
                        "outcome": candidate.outcome,
                        "drop_reason": candidate.drop_reason,
                        "bucket": candidate.bucket,
                        "action_keys": candidate.action_keys,
                    })
                })
                .collect::<Vec<_>>();
            if !full {
                let compact_selected = audit
                    .report
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.bucket != "terminal_loss")
                    .take(limit)
                    .map(|candidate| {
                        json!({
                            "plan_index": candidate.plan_index,
                            "bucket": candidate.bucket,
                            "stop_reason": candidate.stop_reason,
                            "action_count": candidate.action_count,
                            "actions": candidate.actions.iter().map(|action| {
                                action.action_key.as_str()
                            }).collect::<Vec<_>>(),
                            "end_exact_state_hash": candidate.steps.last().map(|step| {
                                step.state_after_exact_state_hash.as_str()
                            }),
                            "final_hp": candidate.eval_final_hp,
                            "risk_margin": candidate.eval_risk_margin,
                            "enemy_progress": candidate.eval_enemy_progress,
                        })
                    })
                    .collect::<Vec<_>>();
                return print_json(&json!({
                    "schema_name": "OracleTurnPlanAuditCompactV1",
                    "schema_version": 1,
                    "behavioral_scope": "read_only_no_search_seeding",
                    "config": audit.report.config,
                    "enumeration": audit.report.enumeration,
                    "exported_plan": exported_plan,
                    "selected_non_loss": compact_selected,
                }));
            }
            print_json(&json!({
                "schema_name": "OracleTurnPlanAuditV1",
                "schema_version": 1,
                "behavioral_scope": "read_only_no_search_seeding",
                "config": audit.report.config,
                "enumeration": audit.report.enumeration,
                "exported_plan": exported_plan,
                "preselection": preselection,
                "selected": selected,
            }))
        }
        Command::DepthBeamTurnAudit {
            case,
            action_imitation_artifact,
            max_applied_transitions,
            wall_ms,
            partial_beam_width,
            retained_per_view,
            max_atomic_depth,
            max_structured_members_per_family,
            max_engine_steps_per_transition,
            watch_exact_state_hash,
            limit,
        } => {
            let case = load_combat_case(&case)?;
            let root = CombatDecisionRoot::new(case.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let policy = action_imitation_artifact
                .as_deref()
                .map(|path| {
                    load_action_imitation_policy(path, existing_combat_knowledge_policy_v1())
                })
                .transpose()?
                .unwrap_or_else(existing_combat_knowledge_policy_v1);
            let report = generate_depth_beam_turn_options(
                root,
                DepthBeamTurnConfig {
                    generator: TurnOptionGeneratorConfig {
                        max_engine_steps_per_transition,
                        ..TurnOptionGeneratorConfig::default()
                    },
                    partial_beam_width,
                    retained_per_view,
                    max_atomic_depth,
                    max_structured_members_per_family,
                },
                DepthBeamTurnBudget {
                    max_applied_transitions,
                    max_engine_steps: max_applied_transitions
                        .saturating_mul(max_engine_steps_per_transition.max(1)),
                    deadline: Some(Instant::now() + Duration::from_millis(wall_ms)),
                },
                policy.clone(),
                &EngineCombatStepper,
            );
            let option_json = |option: &sts_combat_planner::CompleteTurnOption| {
                json!({
                    "exact_successor_hash": option.exact_successor_hash(),
                    "boundary": format!("{:?}", option.boundary()),
                    "action_count": option.actions().len(),
                    "negative_log_policy": option.negative_log_policy(),
                    "final_hp": option.exact_successor().combat.entities.player.current_hp,
                    "state_guides": policy.state_guides(option.exact_successor()).into_iter().map(|guide| json!({
                        "lane": guide.lane.value(),
                        "components": guide.rank.components(),
                    })).collect::<Vec<_>>(),
                    "actions": option.actions().iter().map(|action| json!({
                        "input": action.input,
                        "expected_successor_hash": action.expected_successor_hash,
                    })).collect::<Vec<_>>(),
                })
            };
            let watched = report
                .options
                .iter()
                .filter(|option| {
                    watch_exact_state_hash
                        .iter()
                        .any(|hash| hash == option.exact_successor_hash())
                })
                .map(option_json)
                .collect::<Vec<_>>();
            let options = report
                .options
                .iter()
                .take(limit)
                .map(option_json)
                .collect::<Vec<_>>();
            print_json(&json!({
                "schema_name": "OracleDepthBeamTurnAuditV1",
                "schema_version": 1,
                "behavioral_scope": "read_only_no_search_seeding",
                "status": format!("{:?}", report.status),
                "config": {
                    "max_applied_transitions": max_applied_transitions,
                    "wall_ms": wall_ms,
                    "partial_beam_width": partial_beam_width,
                    "retained_per_view": retained_per_view,
                    "max_atomic_depth": max_atomic_depth,
                    "max_structured_members_per_family": max_structured_members_per_family,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                    "action_imitation_artifact": action_imitation_artifact,
                },
                "counters": {
                    "expanded_partial_states": report.counters.expanded_partial_states,
                    "applied_transitions": report.counters.applied_transitions,
                    "engine_steps": report.counters.engine_steps,
                    "unique_partial_states": report.counters.unique_partial_states,
                    "duplicate_exact_successors": report.counters.duplicate_exact_successors,
                    "completed_turn_options": report.counters.completed_turn_options,
                    "retained_partial_states": report.counters.retained_partial_states,
                    "pruned_partial_states": report.counters.pruned_partial_states,
                    "maximum_atomic_depth": report.counters.maximum_atomic_depth,
                    "truncated_structured_families": report.counters.truncated_structured_families,
                },
                "gap_count": report.gaps.len(),
                "watched": watched,
                "layers": report.layers.iter().map(|layer| json!({
                    "atomic_depth": layer.atomic_depth,
                    "expanded_partial_states": layer.expanded_partial_states,
                    "generated_unique_partial_states": layer.generated_unique_partial_states,
                    "retained_partial_states": layer.retained_partial_states,
                    "retained_exact_state_hashes": layer.retained_exact_state_hashes,
                    "new_completed_turn_options": layer.new_completed_turn_options,
                })).collect::<Vec<_>>(),
                "options": options,
            }))
        }
        Command::DepthBeamAgendaAudit {
            case,
            action_imitation_artifact,
            action_imitation_all_turns,
            value_prototype_artifact,
            max_applied_transitions,
            wall_ms,
            partial_beam_width,
            partial_retained_per_view,
            max_atomic_depth,
            max_applied_transitions_per_parent,
            max_structured_members_per_family,
            max_engine_steps_per_transition,
            watch_exact_state_hash,
            diagnostic_corridor_actions,
            export_witness_actions,
        } => {
            let loaded = load_combat_case(&case)?;
            let diagnostic_corridor = if diagnostic_corridor_actions.is_empty() {
                None
            } else {
                Some(load_exact_turn_corridor(
                    &case,
                    &diagnostic_corridor_actions,
                    max_engine_steps_per_transition,
                )?)
            };
            let initial_hp = loaded.position.combat.entities.player.current_hp;
            let root_player_turn = loaded.position.combat.turn.turn_count;
            let root = CombatDecisionRoot::new(loaded.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let base_policy = existing_combat_knowledge_policy_v1();
            let policy = if let Some(path) = action_imitation_artifact.as_deref() {
                let learned = load_action_imitation_policy(path, base_policy.clone())?;
                if action_imitation_all_turns {
                    learned
                } else {
                    root_player_turn_action_policy_v1(root_player_turn, learned, base_policy)
                }
            } else {
                base_policy
            };
            let (policy, value_report, boundary_guide_lane) =
                if let Some(path) = value_prototype_artifact.as_deref() {
                    let artifact = load_value_prototype(path)?;
                    let report = artifact.report();
                    (
                        combat_value_prototype_policy_v1(policy, &artifact),
                        Some(report),
                        Some(GUIDE_LEARNED_BOUNDARY_VALUE),
                    )
                } else {
                    (policy, None, None)
                };
            let started = Instant::now();
            let report = search_depth_beam_agenda_witness(
                root,
                DepthBeamAgendaConfig {
                    turn: DepthBeamTurnConfig {
                        generator: TurnOptionGeneratorConfig {
                            max_engine_steps_per_transition,
                            ..TurnOptionGeneratorConfig::default()
                        },
                        partial_beam_width,
                        retained_per_view: partial_retained_per_view,
                        max_atomic_depth,
                        max_structured_members_per_family,
                    },
                    boundary_guide_lane,
                    max_applied_transitions_per_parent,
                },
                DepthBeamAgendaBudget {
                    max_applied_transitions,
                    max_engine_steps: max_applied_transitions
                        .saturating_mul(max_engine_steps_per_transition.max(1)),
                    deadline: Some(Instant::now() + Duration::from_millis(wall_ms)),
                },
                policy,
                &EngineCombatStepper,
            );
            if let (Some(path), Some(witness)) =
                (export_witness_actions.as_ref(), report.witness.as_ref())
            {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let inputs = witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect::<Vec<_>>();
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&inputs).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            let watched_frontier = report
                .frontier_exact_state_hashes
                .iter()
                .filter(|hash| watch_exact_state_hash.contains(hash))
                .cloned()
                .collect::<Vec<_>>();
            let expanded_hashes = report
                .expanded_parent_exact_state_hashes
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            let frontier_hashes = report
                .frontier_exact_state_hashes
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            let diagnostic_corridor_membership = diagnostic_corridor.as_ref().map(|corridor| {
                corridor
                    .positions_by_rank
                    .iter()
                    .enumerate()
                    .map(|(rank, position)| {
                        let hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                            &position.engine,
                            &position.combat,
                        );
                        json!({
                            "rank": rank,
                            "player_turn": position.combat.turn.turn_count,
                            "exact_state_hash": hash,
                            "membership": if expanded_hashes.contains(hash.as_str()) {
                                "expanded"
                            } else if frontier_hashes.contains(hash.as_str()) {
                                "frontier"
                            } else {
                                "missing"
                            },
                        })
                    })
                    .collect::<Vec<_>>()
            });
            print_json(&json!({
                "schema_name": "OracleDepthBeamAgendaAuditV1",
                "schema_version": 1,
                "behavioral_scope": "lab_only_no_v2_donor",
                "case": case,
                "runtime": oracle_lab_runtime_identity(),
                "elapsed_ms": started.elapsed().as_millis(),
                "status": format!("{:?}", report.status),
                "config": {
                    "action_imitation_artifact": action_imitation_artifact,
                    "action_imitation_scope": action_imitation_artifact.as_ref().map(|_| {
                        if action_imitation_all_turns {
                            "all_simulated_player_turns"
                        } else {
                            "root_player_turn_only"
                        }
                    }),
                    "value_prototype_artifact": value_prototype_artifact,
                    "value_prototype": value_report,
                    "boundary_guide_lane": boundary_guide_lane.map(CombatGuideLaneId::value),
                    "partial_beam_width": partial_beam_width,
                    "partial_retained_per_view": partial_retained_per_view,
                    "max_atomic_depth": max_atomic_depth,
                    "max_applied_transitions_per_parent": max_applied_transitions_per_parent,
                    "max_structured_members_per_family": max_structured_members_per_family,
                    "diagnostic_corridor_actions": diagnostic_corridor_actions,
                },
                "budget": {
                    "max_applied_transitions": max_applied_transitions,
                    "wall_ms": wall_ms,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                },
                "counters": {
                    "applied_transitions": report.counters.applied_transitions,
                    "engine_steps": report.counters.engine_steps,
                    "expanded_parents": report.counters.expanded_parents,
                    "partially_generated_parents": report.counters.partially_generated_parents,
                    "generated_complete_turn_options": report.counters.generated_complete_turn_options,
                    "unique_boundary_states": report.counters.unique_boundary_states,
                    "duplicate_boundary_states": report.counters.duplicate_boundary_states,
                    "peak_agenda_states": report.counters.peak_agenda_states,
                },
                "frontier_states": report.frontier_exact_state_hashes.len(),
                "expanded_parent_states": report.expanded_parent_exact_state_hashes.len(),
                "watched_frontier": watched_frontier,
                "diagnostic_corridor_membership": diagnostic_corridor_membership,
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
                })),
            }))
        }
        Command::TurnMembership {
            case,
            actions,
            corridor_actions,
            corridor_rank,
            max_work,
            wall_ms,
            quantum_work,
            max_engine_steps_per_transition,
            anchor_only,
            full,
        } => {
            let (root_position, target, selected_corridor_rank) =
                match (actions.as_ref(), corridor_actions.as_slice(), corridor_rank) {
                    (Some(actions), [], None) => {
                        let case = load_combat_case(&case)?;
                        let target = serde_json::from_slice::<Vec<ClientInput>>(
                            &std::fs::read(actions).map_err(|error| error.to_string())?,
                        )
                        .map_err(|error| format!("invalid target action list: {error}"))?;
                        (case.position, target, None)
                    }
                    (None, corridor_actions, Some(rank)) if !corridor_actions.is_empty() => {
                        let corridor = load_exact_turn_corridor(
                            &case,
                            corridor_actions,
                            max_engine_steps_per_transition,
                        )?;
                        let root_position = corridor
                            .positions_by_rank
                            .get(rank)
                            .cloned()
                            .ok_or_else(|| {
                                format!(
                                    "corridor rank {rank} is out of range 0..{}",
                                    corridor.positions_by_rank.len()
                                )
                            })?;
                        let target = corridor
                            .transition_actions
                            .get(rank)
                            .cloned()
                            .expect("verified corridor has one transition per boundary");
                        (root_position, target, Some(rank))
                    }
                    _ => unreachable!("clap selects either actions or corridor rank"),
                };
            let (target_policy_trace, target_successor_exact_state_hash, target_prefix_positions) =
                target_atomic_policy_trace(
                    &root_position,
                    &target,
                    max_engine_steps_per_transition,
                )?;
            let root = CombatDecisionRoot::new(root_position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let policy = existing_combat_knowledge_policy_v1();
            let policy = if anchor_only {
                anchor_only_policy(policy)
            } else {
                policy
            };
            let mut generator = TurnOptionGeneratorSession::with_policy(
                root,
                TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                policy,
            );
            let started = Instant::now();
            let deadline = started + Duration::from_millis(wall_ms);
            let mut scanned_options = 0usize;
            let mut matched = None;
            let mut prefix_insertions = vec![None; target_prefix_positions.len()];
            let mut transition_insertions = vec![None; target_prefix_positions.len()];
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
                for (index, position) in target_prefix_positions.iter().enumerate() {
                    if prefix_insertions[index].is_none()
                        && generator.has_seen_exact_position(position)
                    {
                        let anchor_rank = generator
                            .live_expand_queue_ranks_at_exact_position(position)
                            .map(|(anchor, _)| anchor);
                        prefix_insertions[index] = Some((
                            report.after.generation_work,
                            generator.anchor_work_pops(),
                            anchor_rank,
                        ));
                    }
                    if transition_insertions[index].is_none() {
                        transition_insertions[index] = target
                            .get(index + 1)
                            .and_then(|next| {
                                generator.live_action_transition_snapshot(position, next)
                            })
                            .map(|snapshot| {
                                serde_json::json!({
                                    "generation_work": report.after.generation_work,
                                    "candidate_ordinal": snapshot.candidate_ordinal,
                                    "remaining_candidate_count": snapshot.remaining_candidate_count,
                                    "conditional_probability": snapshot.conditional_probability,
                                    "candidate_negative_log_policy": snapshot.candidate_negative_log_policy,
                                    "cursor_negative_log_policy": snapshot.cursor_negative_log_policy,
                                    "anchor_queue_rank": snapshot.anchor_queue_rank,
                                    "guide_queue_ranks": snapshot.guide_queue_ranks,
                                })
                            });
                    }
                }
                for option in &generator.completed_options()[scanned_options..] {
                    let exact_action_match = option.actions().len() == target.len()
                        && option
                            .actions()
                            .iter()
                            .zip(&target)
                            .all(|(actual, expected)| actual.input == *expected);
                    let equivalent_successor_match =
                        option.exact_successor_hash() == target_successor_exact_state_hash;
                    if exact_action_match || equivalent_successor_match {
                        matched = Some(serde_json::json!({
                            "match_kind": if exact_action_match { "exact_actions" } else { "equivalent_exact_successor" },
                            "exact_action_match": exact_action_match,
                            "equivalent_successor_match": equivalent_successor_match,
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
            let target_prefix_membership = target_prefix_positions
                .iter()
                .enumerate()
                .map(|(index, position)| {
                    let insertion = prefix_insertions[index].map(
                        |(generation_work, anchor_pops, anchor_rank)| {
                            serde_json::json!({
                                "generation_work": generation_work,
                                "anchor_pops": anchor_pops,
                                "anchor_rank": anchor_rank,
                                "anchor_pops_since": generator
                                    .anchor_work_pops()
                                    .saturating_sub(anchor_pops),
                            })
                        },
                    );
                    let (live_expand, live_apply_action, live_structured_selection) =
                        generator.live_work_counts_at_exact_position(position);
                    let queue_ranks = generator
                        .live_expand_queue_ranks_at_exact_position(position)
                        .map(|(anchor, guides)| serde_json::json!({
                            "anchor": anchor,
                            "guides": guides,
                        }));
                    let next_target_transition = target.get(index + 1).and_then(|next| {
                        generator.live_action_transition_snapshot(position, next)
                    });
                    serde_json::json!({
                        "through_action": index + 1,
                        "exact_state_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                            &position.engine,
                            &position.combat,
                        ),
                        "seen": generator.has_seen_exact_position(position),
                        "first_observed": insertion,
                        "live_work": {
                            "expand": live_expand,
                            "apply_action": live_apply_action,
                            "structured_selection": live_structured_selection,
                        },
                        "live_expand_queue_ranks": queue_ranks,
                        "next_target_transition_live": next_target_transition.is_some(),
                        "next_target_transition_first_observed": transition_insertions[index],
                        "next_target_transition": next_target_transition.map(|snapshot| serde_json::json!({
                            "candidate_ordinal": snapshot.candidate_ordinal,
                            "remaining_candidate_count": snapshot.remaining_candidate_count,
                            "conditional_probability": snapshot.conditional_probability,
                            "candidate_negative_log_policy": snapshot.candidate_negative_log_policy,
                            "cursor_negative_log_policy": snapshot.cursor_negative_log_policy,
                            "anchor_queue_rank": snapshot.anchor_queue_rank,
                            "guide_queue_ranks": snapshot.guide_queue_ranks,
                        })),
                    })
                })
                .collect::<Vec<_>>();
            let last_reached_prefix = target_prefix_membership
                .iter()
                .rev()
                .find(|prefix| {
                    prefix.get("seen").and_then(serde_json::Value::as_bool) == Some(true)
                })
                .cloned();
            let first_missing_prefix = target_prefix_membership
                .iter()
                .find(|prefix| {
                    prefix.get("seen").and_then(serde_json::Value::as_bool) == Some(false)
                })
                .cloned();
            let mut output = serde_json::json!({
                "schema_name": "OracleTurnMembershipProbeV1",
                "schema_version": 1,
                "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                "matched": matched.is_some(),
                "match": matched,
                "target_action_count": target.len(),
                "corridor_rank": selected_corridor_rank,
                "target_successor_exact_state_hash": target_successor_exact_state_hash,
                "target_policy_trace": target_policy_trace,
                "last_reached_prefix": last_reached_prefix,
                "first_missing_prefix": first_missing_prefix,
                "status": format!("{:?}", last_status),
                "elapsed_ms": started.elapsed().as_millis(),
                "generation_work": counters.generation_work,
                "engine_steps": counters.engine_steps,
                "scheduler_counters": {
                    "atomic_state_expansions": generator.atomic_state_expansions(),
                    "anchor_work_pops": generator.anchor_work_pops(),
                    "guided_work_pops": generator.guided_work_pops(),
                    "applied_action_transitions": generator.diagnostics().applied_action_transitions,
                },
                "completed_turn_options": generator.completed_options().len(),
                "retained_work_items": generator.retained_work_items(),
                "finished": generator.is_finished(),
            });
            if full {
                output
                    .as_object_mut()
                    .expect("membership report must be an object")
                    .insert(
                        "target_prefix_membership".to_owned(),
                        serde_json::Value::Array(target_prefix_membership),
                    );
            }
            print_json(&output)
        }
        Command::V2CapabilityAudit {
            case,
            corridor_actions,
            max_nodes,
            wall_ms,
            quantum_nodes,
            max_engine_steps_per_transition,
            root_rollout_max_actions,
            export_without_rollout_witness_actions,
        } => {
            let loaded_case = load_combat_case(&case)?;
            let expected_first_turn_successor = corridor_actions
                .as_ref()
                .map(|actions| {
                    load_exact_turn_corridor(
                        &case,
                        std::slice::from_ref(actions),
                        max_engine_steps_per_transition,
                    )
                })
                .transpose()?
                .and_then(|corridor| corridor.positions_by_rank.get(1).cloned())
                .map(|position| {
                    sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                        &position.engine,
                        &position.combat,
                    )
                });
            let loaded = CombatSearchV2LoadedStart {
                label: format!("oracle_lab:{}", case.display()),
                position: loaded_case.position,
                artifact_trust_level: None,
                fingerprints: None,
            };
            let run = |rollout_policy| {
                run_combat_root_proposal_probe_v1(
                    &loaded,
                    CombatSearchV2RunOptions {
                        max_nodes: Some(max_nodes),
                        max_engine_steps_per_action: Some(max_engine_steps_per_transition),
                        wall_ms: Some(wall_ms),
                        potion_policy: Some(CombatSearchV2PotionPolicy::Never),
                        max_potions_used: Some(0),
                        rollout_policy: Some(rollout_policy),
                        ..CombatSearchV2RunOptions::default()
                    },
                    quantum_nodes,
                )
            };
            let baseline = run(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)?;
            let without_rollout = run(CombatSearchV2RolloutPolicy::Disabled)?;
            if let (Some(path), Some(actions)) = (
                export_without_rollout_witness_actions.as_ref(),
                without_rollout.final_best_actions.as_ref(),
            ) {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(actions).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
            }
            let root_rollout_started = Instant::now();
            let root_rollout =
                sts_simulator::ai::combat_search_v2::oracle_rollout_witness_proposal_v1(
                    &loaded.position,
                    root_rollout_max_actions,
                    Instant::now().checked_add(Duration::from_millis(wall_ms)),
                );
            let root_rollout_report = root_rollout.map(|proposal| {
                let stepper = EngineCombatStepper;
                let mut position = loaded.position.clone();
                let mut replay_valid = true;
                for input in &proposal.actions {
                    if stepper.choice_for_legal_input(&position, input).is_none() {
                        replay_valid = false;
                        break;
                    }
                    let step = stepper.apply_to_stable(
                        &position,
                        input.clone(),
                        CombatStepLimits {
                            max_engine_steps: max_engine_steps_per_transition,
                            deadline: None,
                        },
                    );
                    if step.truncated || step.timed_out {
                        replay_valid = false;
                        break;
                    }
                    position = step.position;
                }
                json!({
                    "elapsed_ms": root_rollout_started.elapsed().as_millis(),
                    "action_count": proposal.actions.len(),
                    "final_hp_hint": proposal.final_hp_hint,
                    "replay_valid": replay_valid,
                    "replay_terminal": format!("{:?}", stepper.terminal(&position)),
                    "replay_final_hp": position.combat.entities.player.current_hp,
                })
            });
            let compact = |report: &CombatRootProposalProbeV1Report| {
                let expected_observation =
                    expected_first_turn_successor.as_ref().and_then(|expected| {
                        report
                            .proposals
                            .iter()
                            .find(|proposal| proposal.successor_exact_state_hash == *expected)
                    });
                json!({
                    "rollout_policy": report.config.rollout_policy,
                    "proposal_count": report.proposals.len(),
                    "expected_first_turn_successor_seen": expected_observation.is_some(),
                    "expected_first_turn_successor": expected_observation,
                    "summary": report.summary,
                })
            };
            print_json(&json!({
                "schema_name": "OracleV2CapabilityAuditV1",
                "schema_version": 1,
                "authority": "diagnostic_only_no_production_seeding",
                "case": case,
                "expected_first_turn_successor_hash": expected_first_turn_successor,
                "root_rollout": root_rollout_report,
                "baseline": compact(&baseline),
                "without_rollout": compact(&without_rollout),
                "exported_without_rollout_witness_actions":
                    without_rollout.final_best_actions.is_some()
                        .then_some(export_without_rollout_witness_actions.as_ref())
                        .flatten(),
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
