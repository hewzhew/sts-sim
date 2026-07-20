use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_combat_planner::{
    CombatActionPolicy, CombatDecisionRoot, CombatPlanningQuantum, CombatPolicyChoice,
    CombatPolicyWitnessProposal, CombatStateGuideRank, OracleCombatOneTurnLossEvidence,
    OracleCombatOneTurnViabilityEvidence, OracleCombatWitnessConfig, OracleCombatWitnessQuantum,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessSession, SharedCombatActionPolicy,
    TurnOptionAction, TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
    TurnOptionGeneratorSession,
};
use sts_simulator::content::{cards, monsters::EnemyId};
use sts_simulator::eval::combat_case::load_combat_case;
use sts_simulator::eval::run_control::{
    existing_combat_knowledge_policy_v1, OracleAnalysisAdvanceRequestV1,
};
use sts_simulator::runtime::branch::{
    call_oracle_analysis_tcp_v1, load_oracle_analysis_workspace_v1,
    load_oracle_run_continuation_v1, save_oracle_analysis_workspace_v1,
    serve_oracle_analysis_jsonl_v1, serve_oracle_analysis_tcp_v1, OracleAnalysisServiceCommandV1,
    OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig,
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
        /// Repeat to inspect membership for several exact corridor states in
        /// one search run.
        #[arg(long)]
        watch_state_hash: Vec<String>,
        /// Replay one or more exact legal input-prefix files in order before
        /// starting the planner. Repeat the flag to compose verified segments.
        #[arg(long)]
        prefix_actions: Vec<PathBuf>,
        /// Print compact, card-labelled traces instead of raw action arrays.
        #[arg(long)]
        readable: bool,
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
    /// Use typed commands against a resident oracle workspace.
    Live {
        #[arg(long)]
        endpoint: PathBuf,
        #[command(subcommand)]
        command: LiveCommand,
    },
}

#[derive(Debug, Subcommand)]
enum LiveCommand {
    /// Show the current node, choices, and tactical progress.
    Status {
        #[arg(long)]
        node: Option<usize>,
    },
    /// Continue the current tactical search and return its new status.
    Advance {
        #[arg(long, default_value_t = 100_000)]
        max_quanta: usize,
        #[arg(long, default_value_t = 4_096)]
        quantum_nodes: usize,
        #[arg(long, default_value_t = 100)]
        quantum_ms: u64,
        #[arg(long, default_value_t = 10_000)]
        wall_ms: u64,
    },
    /// Choose an owner-ranked decision at the current node.
    Choose {
        #[arg(long)]
        owner_rank: u64,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Accept the current combat incumbent and materialize its child node.
    Accept,
    /// Restart tactical search at the unchanged exact combat state.
    Restart,
    /// Print a compact timeline for the current or selected node.
    Timeline {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 30)]
        tail: usize,
    },
    /// Export the current or selected exact combat case.
    ExportCase {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Show the exact combat root plus replayed deepest search trajectories.
    Combat {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 512)]
        max_engine_steps_per_transition: usize,
    },
    /// Save the resident workspace immediately.
    Save,
    /// Save and stop the resident workspace service.
    Shutdown,
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
    typed_target_by_turn: HashMap<u32, (i32, Vec<i32>)>,
    action_count: usize,
    terminal_final_hp: i32,
}

impl ExactTurnCorridor {
    fn report(&self, search: &OracleCombatWitnessSession, guide: ShadowCorridorGuide) -> Value {
        let mut states = self
            .rank_by_exact_hash
            .iter()
            .map(|(exact_hash, rank)| (*rank, search.state_membership_by_exact_hash(exact_hash)))
            .collect::<Vec<_>>();
        states.sort_by_key(|(rank, _)| *rank);
        json!({
            "kind": match guide {
                ShadowCorridorGuide::Exact => "exact_verified_turn_corridor_shadow",
                ShadowCorridorGuide::TypedFeature => "typed_feature_corridor_shadow",
            },
            "authority": "guide_only",
            "exact_turn_states": self.rank_by_exact_hash.len(),
            "typed_feature_targets": self.typed_target_by_turn.len(),
            "typed_feature_count": self.typed_target_by_turn.values().next().map(|(_, features)| features.len()).unwrap_or_default(),
            "action_count": self.action_count,
            "terminal": "Win",
            "terminal_final_hp": self.terminal_final_hp,
            "states": states.into_iter().map(|(rank, membership)| json!({
                "corridor_rank": rank,
                "membership": membership,
            })).collect::<Vec<_>>(),
        })
    }
}

struct ExactCorridorShadowPolicy {
    base: SharedCombatActionPolicy,
    rank_by_exact_hash: Arc<HashMap<String, i32>>,
    typed_target_by_turn: Arc<HashMap<u32, (i32, Vec<i32>)>>,
    guide: ShadowCorridorGuide,
}

struct AnchorOnlyPolicy {
    base: SharedCombatActionPolicy,
}

impl CombatActionPolicy for AnchorOnlyPolicy {
    fn weights(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        choices: &[CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn state_guide_ranks(
        &self,
        _position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuideRank> {
        Vec::new()
    }

    fn turn_generation_guide_ranks(
        &self,
        _position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuideRank> {
        Vec::new()
    }

    fn witness_proposal(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        deadline: Option<Instant>,
    ) -> Option<CombatPolicyWitnessProposal> {
        self.base.witness_proposal(position, deadline)
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

    fn state_guide_ranks(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuideRank> {
        let mut ranks = self.base.state_guide_ranks(position);
        ranks.push(self.shadow_rank(position, position.combat.turn.turn_count));
        ranks
    }

    fn turn_generation_guide_ranks(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
    ) -> Vec<CombatStateGuideRank> {
        let mut ranks = self.base.turn_generation_guide_ranks(position);
        if self.guide == ShadowCorridorGuide::TypedFeature {
            ranks.push(
                self.shadow_rank(position, position.combat.turn.turn_count.saturating_add(1)),
            );
        }
        ranks
    }

    fn witness_proposal(
        &self,
        position: &sts_simulator::sim::combat::CombatPosition,
        deadline: Option<Instant>,
    ) -> Option<CombatPolicyWitnessProposal> {
        self.base.witness_proposal(position, deadline)
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
) -> SharedCombatActionPolicy {
    Arc::new(ExactCorridorShadowPolicy {
        base,
        rank_by_exact_hash: Arc::new(corridor.rank_by_exact_hash.clone()),
        typed_target_by_turn: Arc::new(corridor.typed_target_by_turn.clone()),
        guide,
    })
}

fn value_prototype_shadow_policy(
    base: SharedCombatActionPolicy,
    artifact: &CombatValuePrototypeArtifactV1,
) -> SharedCombatActionPolicy {
    Arc::new(ExactCorridorShadowPolicy {
        base,
        rank_by_exact_hash: Arc::new(HashMap::new()),
        typed_target_by_turn: Arc::new(artifact.targets_by_turn()),
        guide: ShadowCorridorGuide::TypedFeature,
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
    let mut typed_target_by_turn = HashMap::new();
    rank_by_exact_hash.insert(
        sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
            &position.engine,
            &position.combat,
        ),
        0,
    );
    typed_target_by_turn.insert(
        position.combat.turn.turn_count,
        (0, typed_combat_feature_components(&position)),
    );
    let mut next_turn_rank = 1i32;
    for (action_index, input) in actions.iter().enumerate() {
        if stepper.choice_for_legal_input(&position, input).is_none() {
            return Err(format!(
                "shadow corridor action {action_index} is not legal at turn {}: {input:?}",
                position.combat.turn.turn_count
            ));
        }
        let previous_turn = position.combat.turn.turn_count;
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
        if step.terminal == sts_simulator::sim::combat::CombatTerminal::Unresolved
            && position.combat.turn.turn_count != previous_turn
        {
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
    Ok(ExactTurnCorridor {
        rank_by_exact_hash,
        typed_target_by_turn,
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
        Command::CombatCase {
            case,
            max_nodes,
            wall_ms,
            max_engine_steps_per_transition,
            anchor_only,
            watch_state_hash,
            prefix_actions,
            readable,
            replay_only,
            export_prefix_case,
            shadow_corridor_actions,
            shadow_corridor_case,
            shadow_corridor_guide,
            shadow_value_prototype,
            export_witness_actions,
            export_augmented_value_prototype,
            one_turn_loss_evidence_limit,
            one_turn_viability_evidence_limit,
        } => {
            let case = load_combat_case(&case)?;
            let stepper = EngineCombatStepper;
            let initial_position = case.position.clone();
            let mut position = initial_position.clone();
            let prefix = prefix_actions
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
                    "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                    "status": format!("{:?}", report.status),
                    "elapsed_ms": started.elapsed().as_millis(),
                    "budget": {
                        "max_nodes": max_nodes,
                        "wall_ms": wall_ms,
                    },
                    "shadow_corridor": shadow_corridor_report,
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
                        "final_hp": witness.final_position.combat.entities.player.current_hp,
                        "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                        "trace": witness_trace,
                    })),
                }));
            }
            print_json(&serde_json::json!({
                "schema_name": "OracleCombatCaseProbeV1",
                "schema_version": 1,
                "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                "status": format!("{:?}", report.status),
                "elapsed_ms": started.elapsed().as_millis(),
                "budget": {
                    "max_nodes": max_nodes,
                    "wall_ms": wall_ms,
                    "max_engine_steps_per_transition": max_engine_steps_per_transition,
                },
                "shadow_corridor": shadow_corridor_report,
                "one_turn_viability_evidence": one_turn_viability_evidence,
                "one_turn_loss_evidence": one_turn_loss_evidence,
                "exported_augmented_value_prototype": export_augmented_value_prototype,
                "exported_witness_actions": witness
                    .is_some()
                    .then_some(export_witness_actions.as_ref())
                    .flatten(),
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
            anchor_only,
        } => {
            let case = load_combat_case(&case)?;
            let target: Vec<ClientInput> = serde_json::from_slice(
                &std::fs::read(&actions).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("invalid target action list: {error}"))?;
            let (target_policy_trace, target_successor_exact_state_hash, target_prefix_positions) =
                target_atomic_policy_trace(
                    &case.position,
                    &target,
                    max_engine_steps_per_transition,
                )?;
            let root = CombatDecisionRoot::new(case.position)
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
        Command::Live { endpoint, command } => run_live_command(&endpoint, command),
    }
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
    Ok(())
}

fn run_live_command(endpoint: &std::path::Path, command: LiveCommand) -> Result<(), String> {
    match command {
        LiveCommand::Status { node } => {
            let result = live_call(endpoint, OracleAnalysisServiceCommandV1::Status { node })?;
            print_json(&compact_live_node(&result))
        }
        LiveCommand::Advance {
            max_quanta,
            quantum_nodes,
            quantum_ms,
            wall_ms,
        } => {
            let before = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Status { node: None },
            )?;
            let result = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Advance {
                    max_quanta,
                    quantum_nodes,
                    quantum_ms,
                    wall_ms: Some(wall_ms),
                },
            )?;
            print_json(&compact_live_advance(&before, &result))
        }
        LiveCommand::Choose { owner_rank, node } => {
            let node = resolve_live_node(endpoint, node)?;
            let result = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Choose { node, owner_rank },
            )?;
            print_json(&compact_live_node(&result))
        }
        LiveCommand::Accept => {
            let result = live_call(endpoint, OracleAnalysisServiceCommandV1::AcceptCombat)?;
            print_json(&compact_live_node(&result))
        }
        LiveCommand::Restart => {
            let result = live_call(endpoint, OracleAnalysisServiceCommandV1::RestartCombat)?;
            print_json(&compact_live_node(&result))
        }
        LiveCommand::Timeline { node, tail } => {
            let node = resolve_live_node(endpoint, node)?;
            print_json(&live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Timeline { node, tail },
            )?)
        }
        LiveCommand::ExportCase { path, node } => {
            let node = resolve_live_node(endpoint, node)?;
            print_json(&live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::ExportCombatCase { node, path },
            )?)
        }
        LiveCommand::Combat {
            node,
            max_engine_steps_per_transition,
        } => print_json(&live_combat_diagnostic(
            endpoint,
            node,
            max_engine_steps_per_transition,
        )?),
        LiveCommand::Save => {
            print_json(&live_call(endpoint, OracleAnalysisServiceCommandV1::Save)?)
        }
        LiveCommand::Shutdown => print_json(&live_call(
            endpoint,
            OracleAnalysisServiceCommandV1::Shutdown,
        )?),
    }
}

fn compact_live_node(node: &Value) -> Value {
    json!({
        "node": node.get("node_id"),
        "parent": node.get("canonical_parent_node_id"),
        "act": node.get("act"),
        "floor": node.get("floor"),
        "hp": node.get("current_hp"),
        "max_hp": node.get("max_hp"),
        "gold": node.get("gold"),
        "boundary": node.get("boundary"),
        "choices": node.get("choices"),
        "children": node.get("children"),
        "encounter": compact_encounter(node.get("encounter")),
        "combat": compact_combat_progress(node.get("combat")),
    })
}

fn compact_live_advance(before: &Value, result: &Value) -> Value {
    let report = result.get("report");
    let before_combat = before.get("combat");
    let after_combat = report.and_then(|report| report.get("combat"));
    json!({
        "status": report.and_then(|report| report.get("status")),
        "elapsed_ms": report.and_then(|report| report.get("elapsed_ms")),
        "quanta": report.and_then(|report| report.get("quanta_served")),
        "work_delta": {
            "generation_work": value_u64(after_combat, "generation_work").saturating_sub(value_u64(before_combat, "generation_work")),
            "exact_states": value_u64(after_combat, "exact_states").saturating_sub(value_u64(before_combat, "exact_states")),
            "completed_turn_options": value_u64(after_combat, "completed_turn_options").saturating_sub(value_u64(before_combat, "completed_turn_options")),
        },
        "combat": compact_combat_progress(after_combat),
        "node": result.get("node"),
    })
}

fn value_u64(value: Option<&Value>, field: &str) -> u64 {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn compact_encounter(encounter: Option<&Value>) -> Value {
    let Some(encounter) = encounter.filter(|value| !value.is_null()) else {
        return Value::Null;
    };
    json!({
        "turn": encounter.get("turn"),
        "phase": encounter.get("phase"),
        "energy": encounter.get("energy"),
        "player_block": encounter.get("player_block"),
        "hand": encounter.get("hand").and_then(Value::as_array).map(|cards| cards.iter().map(card_value_label).collect::<Vec<_>>()),
        "draw": encounter.get("draw_pile_count"),
        "discard": encounter.get("discard_pile_count"),
        "exhaust": encounter.get("exhaust_pile_count"),
        "monsters": encounter.get("monsters"),
    })
}

fn compact_combat_progress(combat: Option<&Value>) -> Value {
    let Some(combat) = combat.filter(|value| !value.is_null()) else {
        return Value::Null;
    };
    json!({
        "generation_work": combat.get("generation_work"),
        "exact_states": combat.get("exact_states"),
        "completed_turn_options": combat.get("completed_turn_options"),
        "max_player_turn": combat.get("max_player_turn"),
        "deepest_progress": combat.get("deepest_progress_state"),
        "deepest_survival": combat.get("deepest_survival_state"),
        "incumbent_final_hp": combat.get("incumbent_final_hp"),
        "incumbent_hp_loss": combat.get("incumbent_hp_loss"),
        "incumbent_actions": combat.get("incumbent_action_count"),
        "last_status": combat.get("last_status"),
        "quantum_count": combat.get("quantum_count"),
        "remaining_nodes": combat.get("remaining_nodes"),
        "remaining_wall_ms": combat.get("remaining_wall_ms"),
        "resume_kind": combat.get("resume_kind"),
        "restart_count": combat.get("restart_count"),
    })
}

fn live_call(
    endpoint: &std::path::Path,
    command: OracleAnalysisServiceCommandV1,
) -> Result<Value, String> {
    let request = serde_json::to_string(&command)
        .map_err(|error| format!("failed to encode typed oracle command: {error}"))?;
    let response = call_oracle_analysis_tcp_v1(endpoint, &request)?;
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| format!("oracle service returned event '{}'", response.event)));
    }
    response.result.ok_or_else(|| {
        format!(
            "oracle service event '{}' returned no result",
            response.event
        )
    })
}

fn resolve_live_node(endpoint: &std::path::Path, node: Option<usize>) -> Result<usize, String> {
    if let Some(node) = node {
        return Ok(node);
    }
    live_call(
        endpoint,
        OracleAnalysisServiceCommandV1::Status { node: None },
    )?
    .get("node_id")
    .and_then(Value::as_u64)
    .and_then(|node| usize::try_from(node).ok())
    .ok_or_else(|| "oracle status did not contain a valid current node_id".to_string())
}

fn live_combat_diagnostic(
    endpoint: &std::path::Path,
    node: Option<usize>,
    max_engine_steps_per_transition: usize,
) -> Result<Value, String> {
    let node = resolve_live_node(endpoint, node)?;
    let view = live_call(
        endpoint,
        OracleAnalysisServiceCommandV1::View { node: Some(node) },
    )?;
    if view.get("encounter").is_none_or(Value::is_null) {
        return Err(format!(
            "oracle node {node} is not at an active combat boundary"
        ));
    }

    let temporary_case = std::env::temp_dir().join(format!(
        "oracle-lab-live-combat-{}-{node}.json",
        std::process::id()
    ));
    live_call(
        endpoint,
        OracleAnalysisServiceCommandV1::ExportCombatCase {
            node,
            path: temporary_case.clone(),
        },
    )?;
    let case_result = load_combat_case(&temporary_case);
    let _ = std::fs::remove_file(&temporary_case);
    let case = case_result?;

    let progress_actions = combat_action_path(&view, "deepest_progress_actions")?;
    let survival_actions = combat_action_path(&view, "deepest_survival_actions")?;
    let search = compact_combat_progress(view.get("combat"));
    let deepest_progress_trace = replay_combat_path(
        case.position.clone(),
        &progress_actions,
        max_engine_steps_per_transition,
    )?;
    let deepest_survival_trace = if survival_actions == progress_actions {
        json!({"same_as": "deepest_progress_trace"})
    } else {
        replay_combat_path(
            case.position.clone(),
            &survival_actions,
            max_engine_steps_per_transition,
        )?
    };

    Ok(json!({
        "schema_name": "OracleLiveCombatDiagnosticV1",
        "schema_version": 1,
        "node": {
            "node_id": node,
            "act": view.get("act"),
            "floor": view.get("floor"),
            "hp": view.get("current_hp"),
            "max_hp": view.get("max_hp"),
            "boundary": view.get("boundary"),
            "state_fingerprint": view.get("state_fingerprint"),
        },
        "run": {
            "deck": case.position.combat.meta.master_deck_snapshot.iter().map(card_label).collect::<Vec<_>>(),
            "relics": case.position.combat.entities.player.relics.iter().map(|relic| format!("{:?}", relic.id)).collect::<Vec<_>>(),
            "potions": case.position.combat.entities.potions.iter().map(|potion| potion.as_ref().map(|potion| format!("{:?}", potion.id))).collect::<Vec<_>>(),
        },
        "root": combat_position_snapshot(&case.position),
        "search": search,
        "deepest_progress_trace": deepest_progress_trace,
        "deepest_survival_trace": deepest_survival_trace,
    }))
}

fn combat_action_path(view: &Value, field: &str) -> Result<Vec<TurnOptionAction>, String> {
    let Some(actions) = view.get("combat").and_then(|combat| combat.get(field)) else {
        return Ok(Vec::new());
    };
    serde_json::from_value(actions.clone())
        .map_err(|error| format!("invalid oracle combat {field}: {error}"))
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
                "actions": turn_actions,
                "end": combat_turn_snapshot(&position),
                "terminal": format!("{terminal:?}"),
            }));
            turn_number = next_turn;
            turn_start_hp = position.combat.entities.player.current_hp;
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

fn card_value_label(card: &Value) -> String {
    let id = card
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("UnknownCard");
    let upgrades = card.get("upgrades").and_then(Value::as_u64).unwrap_or(0);
    if upgrades == 0 {
        id.to_string()
    } else {
        format!("{id}+{upgrades}")
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
