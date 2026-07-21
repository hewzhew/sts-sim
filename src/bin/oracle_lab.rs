use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_combat_planner::{
    search_layered_combat_witness, CombatActionPolicy, CombatDecisionRoot, CombatGuideLaneId,
    CombatPlanningQuantum, CombatPolicyChoice, CombatStateGuide, CombatStateGuideRank,
    LayeredCombatCandidateRaceConfig, LayeredCombatCandidateRaceSession,
    LayeredCombatWitnessBudget, LayeredCombatWitnessConfig, LayeredCombatWitnessQuantum,
    LayeredCombatWitnessSession, OracleCombatOneTurnLossEvidence,
    OracleCombatOneTurnViabilityEvidence, OracleCombatWitnessConfig, OracleCombatWitnessQuantum,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessSession, SharedCombatActionPolicy,
    TurnOptionAction, TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
    TurnOptionGeneratorSession,
};
use sts_simulator::content::{cards, monsters::EnemyId};
use sts_simulator::eval::combat_case::{load_combat_case, save_combat_case, CombatCase};
use sts_simulator::eval::run_control::{
    existing_combat_knowledge_policy_v1, ExistingCombatKnowledgeAdvisorAdvanceV1,
    ExistingCombatKnowledgeAdvisorV1, OracleAnalysisAdvanceRequestV1, OracleAnalysisNodeViewV1,
    RunProgressStepV1,
};
use sts_simulator::runtime::branch::{
    load_oracle_analysis_workspace_v1, load_oracle_run_continuation_v1,
    oracle_live_combat_diagnostic_v1, save_oracle_analysis_workspace_v1,
    save_oracle_run_continuation_v1, OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig,
};
use sts_simulator::sim::combat::{CombatStepLimits, CombatStepper, EngineCombatStepper};
use sts_simulator::sim::combat_action::{combat_action_key, target_label};
use sts_simulator::state::core::ClientInput;

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
    canonical_fast_run: bool,
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
    /// Import an exact state from an oracle_run continuation.
    Import {
        #[arg(long)]
        continuation: PathBuf,
        #[arg(long)]
        workspace: PathBuf,
        /// Import one retained frontier branch instead of the report-selected state.
        #[arg(long)]
        branch_id: Option<usize>,
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
    /// Lab-only turn-synchronous beam control. It never invokes the legacy
    /// suffix donor or the production Widen/Deepen agenda.
    CombatCaseLayered {
        #[arg(long)]
        case: PathBuf,
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
        /// Minimum shared generator work before one turn layer may close.
        #[arg(long, default_value_t = 640)]
        minimum_generation_work_per_layer: usize,
        /// Hard shared generator-work ceiling for one turn layer.
        #[arg(long, default_value_t = 8_192)]
        maximum_generation_work_per_layer: usize,
        /// Close a sufficiently worked layer when it has this many beam-widths
        /// of exact next-turn candidates.
        #[arg(long, default_value_t = 8)]
        candidate_pool_multiplier: usize,
        #[arg(long, default_value_t = 8)]
        generation_quantum_work: usize,
        #[arg(long, default_value_t = 32)]
        max_turn_layers: usize,
        /// If a replay-verified win is found, save its exact ClientInput list.
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
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
        #[arg(long, default_value_t = 640)]
        minimum_generation_work_per_layer: usize,
        #[arg(long, default_value_t = 8_192)]
        maximum_generation_work_per_layer: usize,
        #[arg(long, default_value_t = 8)]
        candidate_pool_multiplier: usize,
        #[arg(long, default_value_t = 8)]
        generation_quantum_work: usize,
        #[arg(long, default_value_t = 3)]
        continuation_turn_layers: usize,
        #[arg(long, default_value_t = 256)]
        continuation_service_quantum_work: usize,
        #[arg(long)]
        export_witness_actions: Option<PathBuf>,
    },
    /// Distill one exact terminal witness into a typed-feature prototype
    /// artifact for lab-only state-value inference.
    BuildValuePrototype {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        actions: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 250)]
        max_engine_steps_per_transition: usize,
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
        /// Complete verified witness from which one turn transition is
        /// selected without hand-slicing action JSON.
        #[arg(long, required_unless_present = "actions", requires = "corridor_rank")]
        corridor_actions: Option<PathBuf>,
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

const COMBAT_VALUE_PROTOTYPE_SCHEMA_NAME: &str = "CombatValuePrototypeArtifactV1";
const COMBAT_VALUE_PROTOTYPE_SCHEMA_VERSION: u32 = 1;
const COMBAT_TYPED_FEATURE_SCHEMA: &str = "existing-combat-guides/concatenated-v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CombatValuePrototypeArtifactV1 {
    schema_name: String,
    schema_version: u32,
    feature_schema: String,
    training_authority: String,
    source_action_count: usize,
    source_terminal_final_hp: i32,
    feature_count: usize,
    prototypes: Vec<CombatValuePrototypeV1>,
    #[serde(default)]
    one_turn_viability_prototypes: Vec<CombatValueStatePrototypeV1>,
    #[serde(default)]
    one_turn_loss_prototypes: Vec<CombatValueStatePrototypeV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CombatValuePrototypeV1 {
    player_turn: u32,
    value_rank: i32,
    features: Vec<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CombatValueStatePrototypeV1 {
    player_turn: u32,
    features: Vec<i32>,
}

impl CombatValuePrototypeArtifactV1 {
    fn from_corridor(corridor: &ExactTurnCorridor) -> Self {
        let mut prototypes = corridor
            .typed_target_by_turn
            .iter()
            .map(
                |(player_turn, (value_rank, features))| CombatValuePrototypeV1 {
                    player_turn: *player_turn,
                    value_rank: *value_rank,
                    features: features.clone(),
                },
            )
            .collect::<Vec<_>>();
        prototypes.sort_by_key(|prototype| prototype.player_turn);
        Self {
            schema_name: COMBAT_VALUE_PROTOTYPE_SCHEMA_NAME.to_string(),
            schema_version: COMBAT_VALUE_PROTOTYPE_SCHEMA_VERSION,
            feature_schema: COMBAT_TYPED_FEATURE_SCHEMA.to_string(),
            training_authority: "exact_terminal_win_demonstration".to_string(),
            source_action_count: corridor.action_count,
            source_terminal_final_hp: corridor.terminal_final_hp,
            feature_count: prototypes
                .first()
                .map(|prototype| prototype.features.len())
                .unwrap_or_default(),
            prototypes,
            one_turn_viability_prototypes: Vec::new(),
            one_turn_loss_prototypes: Vec::new(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_name != COMBAT_VALUE_PROTOTYPE_SCHEMA_NAME
            || self.schema_version != COMBAT_VALUE_PROTOTYPE_SCHEMA_VERSION
            || self.feature_schema != COMBAT_TYPED_FEATURE_SCHEMA
        {
            return Err("unsupported combat value prototype schema".to_string());
        }
        if self.prototypes.is_empty() || self.feature_count == 0 {
            return Err("combat value prototype artifact is empty".to_string());
        }
        if self
            .prototypes
            .iter()
            .any(|prototype| prototype.features.len() != self.feature_count)
            || self
                .one_turn_loss_prototypes
                .iter()
                .any(|prototype| prototype.features.len() != self.feature_count)
            || self
                .one_turn_viability_prototypes
                .iter()
                .any(|prototype| prototype.features.len() != self.feature_count)
        {
            return Err("combat value prototype feature widths disagree".to_string());
        }
        if self
            .prototypes
            .windows(2)
            .any(|pair| pair[0].player_turn >= pair[1].player_turn)
        {
            return Err("combat value prototypes must have unique ascending turns".to_string());
        }
        Ok(())
    }

    fn targets_by_turn(&self) -> HashMap<u32, (i32, Vec<i32>)> {
        self.prototypes
            .iter()
            .map(|prototype| {
                (
                    prototype.player_turn,
                    (prototype.value_rank, prototype.features.clone()),
                )
            })
            .collect()
    }

    fn add_one_turn_viability_evidence(
        &mut self,
        evidence: &[OracleCombatOneTurnViabilityEvidence],
    ) {
        let mut known = self
            .one_turn_viability_prototypes
            .iter()
            .map(|prototype| (prototype.player_turn, prototype.features.clone()))
            .collect::<std::collections::HashSet<_>>();
        for sample in evidence {
            let player_turn = sample.position.combat.turn.turn_count;
            let features = typed_combat_feature_components(&sample.position);
            if known.insert((player_turn, features.clone())) {
                self.one_turn_viability_prototypes
                    .push(CombatValueStatePrototypeV1 {
                        player_turn,
                        features,
                    });
            }
        }
        self.one_turn_viability_prototypes
            .sort_by_key(|prototype| prototype.player_turn);
    }

    fn add_one_turn_loss_evidence(&mut self, evidence: &[OracleCombatOneTurnLossEvidence]) {
        let mut known = self
            .one_turn_loss_prototypes
            .iter()
            .map(|prototype| (prototype.player_turn, prototype.features.clone()))
            .collect::<std::collections::HashSet<_>>();
        for sample in evidence {
            let player_turn = sample.position.combat.turn.turn_count;
            let features = typed_combat_feature_components(&sample.position);
            if known.insert((player_turn, features.clone())) {
                self.one_turn_loss_prototypes
                    .push(CombatValueStatePrototypeV1 {
                        player_turn,
                        features,
                    });
            }
        }
        self.one_turn_loss_prototypes
            .sort_by_key(|prototype| prototype.player_turn);
    }

    fn report(&self) -> Value {
        json!({
            "kind": "typed_feature_value_prototype",
            "authority": "guide_only",
            "feature_schema": self.feature_schema,
            "training_authority": self.training_authority,
            "feature_count": self.feature_count,
            "prototype_count": self.prototypes.len(),
            "one_turn_viability_prototype_count": self.one_turn_viability_prototypes.len(),
            "one_turn_viability_prototype_authority": "training_evidence_only",
            "one_turn_loss_prototype_count": self.one_turn_loss_prototypes.len(),
            "one_turn_loss_prototype_authority": "training_evidence_only",
            "source_action_count": self.source_action_count,
            "source_terminal_final_hp": self.source_terminal_final_hp,
            "runtime_reads_exact_hashes": false,
            "runtime_reads_witness_actions": false,
        })
    }
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
    typed_target_by_turn: Arc<HashMap<u32, (i32, Vec<i32>)>>,
    guide: ShadowCorridorGuide,
    shadow_only: bool,
}

struct AnchorOnlyPolicy {
    base: SharedCombatActionPolicy,
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

impl CombatActionPolicy for ExactCorridorShadowPolicy {
    fn weights(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
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
                self.typed_target_by_turn.get(&target_turn).map_or_else(
                    || vec![0, i32::MIN / 4, 0],
                    |(corridor_rank, target)| {
                        let candidate = typed_combat_feature_components(position);
                        let distance = normalized_feature_distance(target, &candidate);
                        vec![i32::from(distance == 0), -distance, *corridor_rank]
                    },
                )
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
        typed_target_by_turn: Arc::new(corridor.typed_target_by_turn.clone()),
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

fn load_value_prototype(path: &PathBuf) -> Result<CombatValuePrototypeArtifactV1, String> {
    let artifact = serde_json::from_slice::<CombatValuePrototypeArtifactV1>(
        &std::fs::read(path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("invalid combat value prototype artifact: {error}"))?;
    artifact.validate()?;
    Ok(artifact)
}

fn save_value_prototype(
    path: &PathBuf,
    artifact: &CombatValuePrototypeArtifactV1,
) -> Result<(), String> {
    artifact.validate()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(artifact).map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}

fn typed_combat_feature_components(
    position: &sts_simulator::sim::combat::CombatPosition,
) -> Vec<i32> {
    let mut features =
        sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(
            position,
        );
    features.extend(
        sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(
            position,
        ),
    );
    features.extend(
        sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(
            position,
        ),
    );
    features.extend(
        sts_simulator::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(
            position,
        ),
    );
    features
}

fn normalized_feature_distance(target: &[i32], candidate: &[i32]) -> i32 {
    let distance = target
        .iter()
        .zip(candidate)
        .map(|(target, candidate)| {
            let difference = i64::from(*target).abs_diff(i64::from(*candidate)) as i64;
            let scale = i64::from(*target)
                .abs()
                .max(i64::from(*candidate).abs())
                .max(1);
            difference.saturating_mul(1_024) / scale
        })
        .fold(0_i64, i64::saturating_add);
    i32::try_from(distance).unwrap_or(i32::MAX)
}

fn load_exact_turn_corridor(
    case_path: &PathBuf,
    actions_path: &PathBuf,
    max_engine_steps_per_transition: usize,
) -> Result<ExactTurnCorridor, String> {
    let case = load_combat_case(case_path)?;
    let actions = serde_json::from_slice::<Vec<ClientInput>>(
        &std::fs::read(actions_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("invalid shadow corridor action list: {error}"))?;
    let stepper = EngineCombatStepper;
    let mut position = case.position;
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

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    validate_canonical_launch(cli.canonical_fast_run)?;
    match cli.command {
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
            branch_id,
            budget,
        } => {
            let continuation = load_oracle_run_continuation_v1(&continuation)?;
            let config = OracleRunConfig {
                seed: continuation.seed,
                ascension: continuation.ascension,
                budget: budget.into_budget(),
            };
            let analysis = match branch_id {
                Some(branch_id) => OracleAnalysisWorkspaceV1::from_continuation_branch(
                    config,
                    continuation,
                    branch_id,
                )?,
                None => OracleAnalysisWorkspaceV1::from_continuation(config, continuation)?,
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
        Command::BuildValuePrototype {
            case,
            actions,
            output,
            max_engine_steps_per_transition,
        } => {
            let corridor =
                load_exact_turn_corridor(&case, &actions, max_engine_steps_per_transition)?;
            let artifact = CombatValuePrototypeArtifactV1::from_corridor(&corridor);
            save_value_prototype(&output, &artifact)?;
            print_json(&json!({
                "output": output,
                "artifact": artifact.report(),
            }))
        }
        Command::CombatCaseLayered {
            case,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            beam_width,
            retained_per_view,
            minimum_generation_work_per_layer,
            maximum_generation_work_per_layer,
            candidate_pool_multiplier,
            generation_quantum_work,
            max_turn_layers,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let loaded = load_combat_case(&case)?;
            let initial_hp = loaded.position.combat.entities.player.current_hp;
            let root = CombatDecisionRoot::new(loaded.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let deadline = Instant::now() + Duration::from_millis(wall_ms);
            let report = search_layered_combat_witness(
                root,
                LayeredCombatWitnessConfig {
                    generator: TurnOptionGeneratorConfig {
                        max_engine_steps_per_transition,
                        ..TurnOptionGeneratorConfig::default()
                    },
                    beam_width,
                    retained_per_view,
                    minimum_generation_work_per_layer,
                    maximum_generation_work_per_layer,
                    candidate_pool_multiplier,
                    generation_quantum_work,
                    max_turn_layers,
                },
                LayeredCombatWitnessBudget {
                    max_generation_work: max_nodes,
                    max_engine_steps: max_nodes.saturating_mul(max_engine_steps_per_transition),
                    deadline: Some(deadline),
                },
                existing_combat_knowledge_policy_v1(),
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
                },
                "status": format!("{:?}", report.status),
                "elapsed_ms": command_started.elapsed().as_millis(),
                "config": {
                    "beam_width": beam_width,
                    "retained_per_view": retained_per_view,
                    "minimum_generation_work_per_layer": minimum_generation_work_per_layer,
                    "maximum_generation_work_per_layer": maximum_generation_work_per_layer,
                    "candidate_pool_multiplier": candidate_pool_multiplier,
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
        Command::CombatCaseLayeredWindowRace {
            case,
            source_window_index,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            beam_width,
            retained_per_view,
            minimum_generation_work_per_layer,
            maximum_generation_work_per_layer,
            candidate_pool_multiplier,
            generation_quantum_work,
            continuation_turn_layers,
            continuation_service_quantum_work,
            export_witness_actions,
        } => {
            let command_started = Instant::now();
            let loaded = load_combat_case(&case)?;
            let initial_hp = loaded.position.combat.entities.player.current_hp;
            let original_root = CombatDecisionRoot::new(loaded.position.clone())
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let source_root = CombatDecisionRoot::new(loaded.position)
                .map_err(|error| format!("invalid combat case root: {error:?}"))?;
            let deadline = Instant::now() + Duration::from_millis(wall_ms);
            let policy = existing_combat_knowledge_policy_v1();
            let base_config = LayeredCombatWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                beam_width,
                retained_per_view,
                minimum_generation_work_per_layer,
                maximum_generation_work_per_layer,
                candidate_pool_multiplier,
                generation_quantum_work,
                max_turn_layers: 1,
            };
            let mut source =
                LayeredCombatWitnessSession::with_policy(source_root, base_config, policy.clone());
            let source_report = source.advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: maximum_generation_work_per_layer.max(1),
                    additional_engine_steps: maximum_generation_work_per_layer
                        .max(1)
                        .saturating_mul(max_engine_steps_per_transition.max(1)),
                    deadline: Some(deadline),
                },
                &EngineCombatStepper,
            );
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
                max_turn_layers: continuation_turn_layers,
                ..base_config
            };
            let mut race = LayeredCombatCandidateRaceSession::from_window(
                original_root,
                window,
                LayeredCombatCandidateRaceConfig {
                    continuation,
                    service_quantum_work: continuation_service_quantum_work,
                },
                policy,
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
            if let (Some(path), Some(witness)) = (
                export_witness_actions.as_ref(),
                race_report.witness.as_ref(),
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
            print_json(&json!({
                "schema_name": "OracleCombatCaseLayeredWindowRaceV1",
                "schema_version": 1,
                "case": case,
                "runtime": oracle_lab_runtime_identity(),
                "mode": {
                    "scheduler": "resumable_candidate_continuation_race",
                    "v2_donor_enabled": false,
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
                "exported_witness_actions": race_report.witness.is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
                "witness": race_report.witness.as_ref().map(|witness| json!({
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
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            anchor_only,
            without_v2_donor,
            watch_state_hash,
            watch_corridor_actions,
            corridor_prefix_turns,
            prefix_actions,
            readable,
            full,
            replay_only,
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
                    load_exact_turn_corridor(&case, actions, max_engine_steps_per_transition)
                })
                .transpose()?;
            let case = load_combat_case(&case)?;
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
            let mut prefix_replay_actions = Vec::with_capacity(prefix.len());
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
                prefix_replay_actions.push(TurnOptionAction {
                    input: input.clone(),
                    expected_successor_hash:
                        sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                            &step.position.engine,
                            &step.position.combat,
                        ),
                    engine_steps: step.engine_steps,
                });
                position = step.position;
            }
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
            let base_policy = existing_combat_knowledge_policy_v1();
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
                                actions_path,
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
                artifact.add_one_turn_viability_evidence(search.one_turn_viability_evidence());
                artifact.add_one_turn_loss_evidence(search.one_turn_loss_evidence());
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
        } => {
            let (root_position, target, selected_corridor_rank) =
                match (actions.as_ref(), corridor_actions.as_ref(), corridor_rank) {
                    (Some(actions), None, None) => {
                        let case = load_combat_case(&case)?;
                        let target = serde_json::from_slice::<Vec<ClientInput>>(
                            &std::fs::read(actions).map_err(|error| error.to_string())?,
                        )
                        .map_err(|error| format!("invalid target action list: {error}"))?;
                        (case.position, target, None)
                    }
                    (None, Some(corridor_actions), Some(rank)) => {
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
                        "next_target_transition_live": target.get(index + 1).map(|next| {
                            generator.has_live_action_transition(position, next)
                        }),
                    })
                })
                .collect::<Vec<_>>();
            print_json(&serde_json::json!({
                "schema_name": "OracleTurnMembershipProbeV1",
                "schema_version": 1,
                "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                "matched": matched.is_some(),
                "match": matched,
                "target_action_count": target.len(),
                "corridor_rank": selected_corridor_rank,
                "target_successor_exact_state_hash": target_successor_exact_state_hash,
                "target_policy_trace": target_policy_trace,
                "target_prefix_membership": target_prefix_membership,
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

fn validate_canonical_launch(canonical_fast_run: bool) -> Result<(), String> {
    const REQUIRED_PROFILE: &str = "fast-run";
    const BUILT_PROFILE: &str = env!("STS_CARGO_PROFILE");
    const REPOSITORY_ROOT: &str = env!("STS_REPOSITORY_ROOT");

    if !canonical_fast_run {
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
    if let Some(stale) = dependencies.into_iter().find(|dependency| {
        std::fs::metadata(dependency)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified > executable_modified)
    }) {
        return Err(format!(
            "canonical oracle laboratory is stale: '{}' is newer than '{}'; rebuild once with `cargo oracle-lab --help`",
            stale.display(),
            executable.display()
        ));
    }
    Ok(())
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
    let first_missing = states.iter().find_map(|state| {
        let accepted = state
            .get("membership")
            .and_then(|membership| membership.get("accepted"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        (!accepted)
            .then(|| state.get("corridor_rank").cloned())
            .flatten()
    });
    json!({
        "kind": report.get("kind"),
        "guide": report.get("guide"),
        "authority": report.get("authority"),
        "exact_turn_states": report.get("exact_turn_states"),
        "accepted_turn_states": reached,
        "first_missing_rank": first_missing,
        "terminal": report.get("terminal"),
        "terminal_final_hp": report.get("terminal_final_hp"),
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
