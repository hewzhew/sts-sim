use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use sts_simulator::bot::combat::{
    diagnose_root_search_with_depth_and_runtime, SearchRuntimeBudget,
};
use sts_simulator::fixtures::author_spec::CombatAuthorSpec;
use sts_simulator::fixtures::combat_case::{compile_combat_author_case, lower_case, CombatCase};
use sts_simulator::fixtures::live_capture::build_fixture_from_record_window;
use sts_simulator::fixtures::scenario::{
    initialize_fixture_state, ScenarioFixture, ScenarioKind, ScenarioOracleKind, ScenarioProvenance,
};
use sts_simulator::protocol::java::card_id_from_java;

/// Slay the Spire Simulator Developer Tool
/// Unifies schema queries, AST parsing, and log diffing into a single interface.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query schema entries by name.
    Query {
        /// Substring to match (case-insensitive).
        search: String,

        /// Type of entity to search
        #[arg(short, long)]
        entity_type: Option<String>,
    },

    /// Batch-check a range of entities (e.g. relics) for `.rs` implementation, Java hooks, and scattered logic.
    Audit {
        /// Prefix to match Rust enum name (e.g., 'T' or 'Snecko')
        prefix: String,

        /// Type of entity to audit
        #[arg(short, long, default_value = "relic")]
        entity_type: String,
    },

    /// Auto-detect addToBot/addToTop mismatches between Java and Rust.
    CheckInsertion {
        /// Prefix to match Rust enum name
        prefix: String,
    },

    /// Extract Java class AST into structured markdown for parity review.
    ParseAst {
        /// Path to the Java file to parse
        file: PathBuf,
    },

    /// Diff two Slay the Spire combat JSON logs sequentially
    Diff {
        /// Path to the Java (gold standard) log
        java_log: PathBuf,
        /// Path to the Rust simulation log
        rust_log: PathBuf,
    },

    /// Rebuild the compiled protocol schema from raw facts, baseline, and heuristic matching.
    SyncSchema,

    /// Generate interaction-signature coverage artifacts from replay logs and live_comm sidecar.
    InteractionCoverage,

    /// Batch-run full offline episodes with a masked random policy for RL-readiness smoke checks.
    RunBatch {
        /// Number of episodes to run.
        #[arg(long, default_value_t = 100)]
        episodes: usize,
        /// Base seed; episode N uses base_seed + N.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Ascension level.
        #[arg(long, default_value_t = 0)]
        ascension: u8,
        /// Player class: ironclad, silent, defect, watcher.
        #[arg(long, default_value = "ironclad")]
        class: String,
        /// Enable Act 4 key logic.
        #[arg(long, default_value_t = false)]
        final_act: bool,
        /// Maximum decision steps per episode before step-cap termination.
        #[arg(long, default_value_t = 2000)]
        max_steps: usize,
        /// Policy name: random_masked, rule_baseline_v0, rule_baseline_v0_control, rule_baseline_v1_candidate, or plan_query_v0.
        #[arg(long, default_value = "random_masked")]
        policy: String,
        /// Optional output directory for per-episode action traces.
        #[arg(long)]
        trace_dir: Option<PathBuf>,
        /// Optional JSON summary output path.
        #[arg(long)]
        summary_out: Option<PathBuf>,
        /// Re-run each episode from its recorded actions and compare terminal summary.
        #[arg(long, default_value_t = true)]
        determinism_check: bool,
        /// Immediate reward shaping profile for learning smoke: baseline or plan_deficit_v0.
        #[arg(long, default_value = "baseline")]
        reward_shaping_profile: String,
    },

    /// Manage run-first live_comm logs.
    Logs {
        #[command(subcommand)]
        command: LogCommands,
    },
    /// Build combat-state corpora from fixtures, combat cases, and raw logs.
    Combat {
        #[command(subcommand)]
        command: CombatCommands,
    },
}

#[derive(Subcommand, Debug)]
enum LogCommands {
    Status,
    Gc,
    FreezeBaseline {
        #[arg(long, default_value = "tools/artifacts/learning_baseline.json")]
        out: PathBuf,
        #[arg(long, default_value_t = 3)]
        latest_runs: usize,
        #[arg(long, value_delimiter = ',')]
        fixture_specs: Vec<PathBuf>,
    },
    Pin {
        run_id: String,
    },
    Unpin {
        run_id: String,
    },
    Replay {
        run_id: String,
    },
    Latest {
        #[arg(long)]
        label: Option<String>,
        #[arg(long, default_value = "raw")]
        artifact: String,
    },
    InspectFindings {
        /// Explicit run id to inspect. If omitted, uses the latest matching run.
        run_id: Option<String>,
        /// Restrict to runs carrying this classification label when run_id is omitted.
        #[arg(long)]
        label: Option<String>,
        /// Case-insensitive substring filter on finding family key.
        #[arg(long)]
        family: Option<String>,
        /// Restrict to a single category such as engine_bug, content_gap, validation_failure, timing.
        #[arg(long)]
        category: Option<String>,
        /// Maximum families to print.
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    AnalyzeDecisions {
        /// Explicit run id to inspect. If omitted, uses the latest matching run.
        run_id: Option<String>,
        /// Restrict to runs carrying this classification label when run_id is omitted.
        #[arg(long)]
        label: Option<String>,
        /// Maximum example lines to print.
        #[arg(long, default_value_t = 12)]
        limit: usize,
        /// Optional JSON output path for the structured report.
        #[arg(long)]
        json_out: Option<PathBuf>,
    },
    ExportDisagreementFixtures {
        /// Explicit run id to inspect. If omitted, uses the latest matching run.
        run_id: Option<String>,
        /// Restrict to runs carrying this classification label when run_id is omitted.
        #[arg(long)]
        label: Option<String>,
        /// Restrict export to these disagreement categories.
        #[arg(long, value_delimiter = ',')]
        categories: Vec<String>,
        /// Maximum unique frames to export.
        #[arg(long, default_value_t = 8)]
        limit: usize,
        /// Include N previous responses in the generated ScenarioFixture.
        #[arg(long, default_value_t = 0)]
        window_lookback: usize,
        /// Output directory for generated combat_lab fixtures.
        #[arg(long)]
        out_dir: PathBuf,
        /// Optional JSON output path for the structured export report.
        #[arg(long)]
        json_out: Option<PathBuf>,
    },
    ExportDecisionTrainingSet {
        /// Individual fixture paths to include.
        #[arg(long, value_delimiter = ',')]
        fixtures: Vec<PathBuf>,
        /// Directories containing exported disagreement fixtures.
        #[arg(long, value_delimiter = ',')]
        fixture_dirs: Vec<PathBuf>,
        /// Output JSONL path.
        #[arg(long)]
        out: PathBuf,
        /// Optional proposal-level JSONL output path.
        #[arg(long)]
        proposal_out: Option<PathBuf>,
        /// Optional JSON summary path.
        #[arg(long)]
        summary_out: Option<PathBuf>,
        /// Optional proposal-level JSON summary path.
        #[arg(long)]
        proposal_summary_out: Option<PathBuf>,
        /// Search depth used to regenerate decision traces.
        #[arg(long, default_value_t = 6)]
        depth: u32,
    },
    BuildDecisionCorpus {
        /// Explicit run ids to include. When omitted, uses latest matching runs.
        #[arg(long, value_delimiter = ',')]
        run_ids: Vec<String>,
        /// Restrict to runs carrying this classification label when run_ids are omitted.
        #[arg(long)]
        label: Option<String>,
        /// Number of latest matching runs to scan when run_ids are omitted.
        #[arg(long, default_value_t = 5)]
        latest_runs: usize,
        /// Restrict exported disagreement frames to these categories.
        #[arg(long, value_delimiter = ',')]
        categories: Vec<String>,
        /// Maximum unique frames to export per run.
        #[arg(long, default_value_t = 8)]
        limit_per_run: usize,
        /// Include N previous responses in the generated ScenarioFixture.
        #[arg(long, default_value_t = 0)]
        window_lookback: usize,
        /// Search depth used to regenerate decision traces.
        #[arg(long, default_value_t = 6)]
        depth: u32,
        /// Output directory for the corpus bundle.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum CombatCommands {
    PlanProbe {
        /// Full-run trace JSON to replay into a combat decision point.
        #[arg(long)]
        trace_file: PathBuf,
        /// Step index from the trace to probe before its chosen action is applied.
        #[arg(long)]
        step_index: usize,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
        /// Current-turn probe max action depth.
        #[arg(long, default_value_t = 6)]
        max_depth: usize,
        /// Current-turn probe node budget.
        #[arg(long, default_value_t = 2000)]
        max_nodes: usize,
        /// Current-turn probe branch width.
        #[arg(long, default_value_t = 32)]
        beam_width: usize,
        /// Engine ticks allowed after each candidate action.
        #[arg(long, default_value_t = 200)]
        max_engine_steps_per_action: usize,
    },
    PlanProbeAuthorSpec {
        /// Synthetic combat author spec JSON to lower directly into a combat state.
        #[arg(long)]
        author_spec: PathBuf,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Current-turn probe max action depth.
        #[arg(long, default_value_t = 6)]
        max_depth: usize,
        /// Current-turn probe node budget.
        #[arg(long, default_value_t = 2000)]
        max_nodes: usize,
        /// Current-turn probe branch width.
        #[arg(long, default_value_t = 32)]
        beam_width: usize,
        /// Engine ticks allowed after each candidate action.
        #[arg(long, default_value_t = 200)]
        max_engine_steps_per_action: usize,
    },
    DrawMarginalProbeAuthorSpec {
        /// Synthetic combat author spec JSON to lower directly into a combat state.
        #[arg(long)]
        author_spec: PathBuf,
        /// Target draw/search/resource card to force or forbid, e.g. BattleTrance or "Battle Trance".
        #[arg(long)]
        action_card: String,
        /// Optional target hand index. When set, only that card instance is forced/forbidden.
        #[arg(long)]
        hand_index: Option<usize>,
        /// Optional exact root action key. When set, forced branch uses that exact first action.
        #[arg(long)]
        target_action_key: Option<String>,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Current-turn probe max action depth.
        #[arg(long, default_value_t = 6)]
        max_depth: usize,
        /// Current-turn probe node budget.
        #[arg(long, default_value_t = 2000)]
        max_nodes: usize,
        /// Current-turn probe branch width.
        #[arg(long, default_value_t = 32)]
        beam_width: usize,
        /// Engine ticks allowed after each candidate action.
        #[arg(long, default_value_t = 200)]
        max_engine_steps_per_action: usize,
    },
    DrawMarginalProbe {
        /// Full-run trace JSON to replay.
        #[arg(long)]
        trace_file: PathBuf,
        /// Step index from the trace to probe before its chosen action is applied.
        #[arg(long)]
        step_index: usize,
        /// Target draw/search/resource card to force or forbid, e.g. BattleTrance or "Battle Trance".
        #[arg(long)]
        action_card: String,
        /// Optional target hand index. When set, only that card instance is forced/forbidden.
        #[arg(long)]
        hand_index: Option<usize>,
        /// Optional exact root action key. When set, forced branch uses that exact first action.
        #[arg(long)]
        target_action_key: Option<String>,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
        /// Current-turn probe max action depth.
        #[arg(long, default_value_t = 6)]
        max_depth: usize,
        /// Current-turn probe node budget.
        #[arg(long, default_value_t = 2000)]
        max_nodes: usize,
        /// Current-turn probe branch width.
        #[arg(long, default_value_t = 32)]
        beam_width: usize,
        /// Engine ticks allowed after each candidate action.
        #[arg(long, default_value_t = 200)]
        max_engine_steps_per_action: usize,
    },
    CandidateOutcomePack {
        /// Full-run trace JSON to replay into a combat decision point.
        #[arg(long)]
        trace_file: PathBuf,
        /// Step index from the trace to evaluate before its chosen action is applied.
        #[arg(long)]
        step_index: usize,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
        /// Exact-turn node budget per forced root candidate.
        #[arg(long, default_value_t = 10_000)]
        max_exact_nodes_per_candidate: usize,
        /// Engine ticks allowed after each candidate action.
        #[arg(long, default_value_t = 200)]
        max_engine_steps_per_action: usize,
        /// Optional cap for smoke runs. Omit to evaluate all legal candidates.
        #[arg(long)]
        max_candidates: Option<usize>,
        /// Restrict root candidates to the controlled V0 surface: play_card and end_turn.
        #[arg(long)]
        controlled_v0: bool,
    },
    CandidateOutcomePackBatch {
        /// Trace JSON files or directories containing trace JSON files.
        #[arg(long, value_delimiter = ',', required = true)]
        trace_input: Vec<PathBuf>,
        /// Output directory for per-budget packs and suite summary.
        #[arg(long)]
        out_dir: PathBuf,
        /// First trace step index to consider, inclusive.
        #[arg(long, default_value_t = 0)]
        step_start: usize,
        /// Last trace step index to consider, exclusive.
        #[arg(long)]
        step_end: Option<usize>,
        /// Optional cap on selected controlled combat states.
        #[arg(long)]
        step_limit: Option<usize>,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
        /// Comma-separated exact-turn node budgets to sweep.
        #[arg(long, default_value = "1000,5000,20000,50000")]
        budgets: String,
        /// Engine ticks allowed after each candidate action.
        #[arg(long, default_value_t = 200)]
        max_engine_steps_per_action: usize,
        /// Minimum non-truncated candidates required for trainable manifest eligibility.
        #[arg(long, default_value_t = 4)]
        min_eligible_candidates: usize,
        /// Minimum bounded pairwise labels required before a batch can be called oracle-ready.
        #[arg(long, default_value_t = 100)]
        min_trainable_pairs: usize,
        /// Median per-candidate exact-turn runtime gate for automatic budget selection.
        #[arg(long, default_value_t = 500)]
        median_runtime_ms_limit: u128,
    },
    RecursiveRolloutValidation {
        /// Trace JSON files or directories containing trace JSON files.
        #[arg(long, value_delimiter = ',', required = true)]
        trace_input: Vec<PathBuf>,
        /// Output directory for recursive rollout validation packs.
        #[arg(long)]
        out_dir: PathBuf,
        /// First trace step index to consider, inclusive.
        #[arg(long, default_value_t = 0)]
        step_start: usize,
        /// Last trace step index to consider, exclusive.
        #[arg(long)]
        step_end: Option<usize>,
        /// Optional cap on selected controlled combat states.
        #[arg(long)]
        step_limit: Option<usize>,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
        /// Decisions to roll forward after forcing the root candidate.
        #[arg(long, default_value_t = 8)]
        horizon_decisions: usize,
        /// Continuation policy after the forced root action: rule_baseline_v0, rule_baseline_v0_control, rule_baseline_v1_candidate, or plan_query_v0.
        #[arg(long, default_value = "rule_baseline_v0")]
        continuation_policy: String,
        /// Optional cap for smoke runs. Omit to evaluate all controlled legal candidates.
        #[arg(long)]
        max_candidates: Option<usize>,
        /// Restrict root candidates to play_card and end_turn.
        #[arg(long)]
        controlled_v0: bool,
    },
    RunBranchFromTrace {
        /// Full-run trace JSON to replay to a decision point.
        #[arg(long)]
        trace_file: PathBuf,
        /// Step index from the trace to branch before its chosen action is applied.
        #[arg(long)]
        step_index: usize,
        /// Exact legal action key to force at the branch point.
        #[arg(long)]
        target_action_key: Option<String>,
        /// Legal action index to force at the branch point.
        #[arg(long)]
        target_action_index: Option<usize>,
        /// Continuation policy after the forced action.
        #[arg(long, default_value = "rule_baseline_v1_candidate")]
        continuation_policy: String,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Include compact branch action trace in the output.
        #[arg(long)]
        include_trace: bool,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
    },
    PlanSearchFromTrace {
        /// Full-run trace JSON to replay to a combat decision point.
        #[arg(long)]
        trace_file: PathBuf,
        /// Step index from the trace to use as the combat search root.
        #[arg(long)]
        step_index: usize,
        /// Output JSON report path.
        #[arg(long)]
        out: PathBuf,
        /// Optional ascension override for old traces without embedded config.
        #[arg(long)]
        ascension: Option<u8>,
        /// Optional player class override for old traces without embedded config.
        #[arg(long)]
        class: Option<String>,
        /// Optional final-act override for old traces without embedded config.
        #[arg(long)]
        final_act: Option<bool>,
        /// Optional replay max step cap.
        #[arg(long)]
        max_steps: Option<usize>,
        /// Maximum generated search nodes.
        #[arg(long, default_value_t = 25_000)]
        max_nodes: usize,
        /// Beam width retained after each decision depth.
        #[arg(long, default_value_t = 128)]
        beam_width: usize,
        /// Maximum combat decisions to search from the root.
        #[arg(long, default_value_t = 80)]
        max_depth_decisions: usize,
        /// Beam retained while enumerating one current-turn action sequence before the next turn boundary.
        #[arg(long, default_value_t = 256)]
        turn_sequence_beam_width: usize,
        /// Maximum actions inside a single current-turn sequence.
        #[arg(long, default_value_t = 24)]
        max_turn_sequence_actions: usize,
        /// Optional cap on child branches retained per expanded node after scheduler ranking.
        #[arg(long)]
        max_branching: Option<usize>,
        /// Include top censored frontier nodes in the output.
        #[arg(long)]
        include_frontier: bool,
    },
    BuildStateCorpus {
        /// Individual ScenarioFixture paths to include.
        #[arg(long, value_delimiter = ',')]
        fixtures: Vec<PathBuf>,
        /// Directories containing ScenarioFixture json files.
        #[arg(long, value_delimiter = ',')]
        fixture_dirs: Vec<PathBuf>,
        /// Individual CombatCase paths to include.
        #[arg(long, value_delimiter = ',')]
        combat_cases: Vec<PathBuf>,
        /// Directories containing CombatCase json files.
        #[arg(long, value_delimiter = ',')]
        combat_case_dirs: Vec<PathBuf>,
        /// Raw live_comm logs to sample directly.
        #[arg(long, value_delimiter = ',')]
        raw: Vec<PathBuf>,
        /// Explicit run ids whose raw logs should be included.
        #[arg(long, value_delimiter = ',')]
        run_ids: Vec<String>,
        /// Restrict raw run selection to this classification label when run_ids are omitted.
        #[arg(long)]
        label: Option<String>,
        /// Number of latest matching runs to scan for raw logs when run_ids are omitted.
        #[arg(long, default_value_t = 5)]
        latest_runs: usize,
        /// Maximum decision-point states to keep from each raw log. Use 0 for all.
        #[arg(long, default_value_t = 64)]
        limit_per_raw: usize,
        /// Search depth used for the decision probe attached to each state.
        #[arg(long, default_value_t = 4)]
        depth: u32,
        /// Keep only states that match at least one of these curriculum buckets.
        #[arg(long, value_delimiter = ',')]
        include_buckets: Vec<String>,
        /// Drop any state that matches one of these curriculum buckets.
        #[arg(long, value_delimiter = ',')]
        exclude_buckets: Vec<String>,
        /// Output JSONL path.
        #[arg(long)]
        out: PathBuf,
        /// Optional summary JSON path.
        #[arg(long)]
        summary_out: Option<PathBuf>,
    },
    SplitStateCorpus {
        /// Existing state_corpus JSONL path.
        #[arg(long)]
        input: PathBuf,
        /// Output directory for train/val/test JSONL files and split summary.
        #[arg(long)]
        out_dir: PathBuf,
        /// Keep only states that match at least one of these curriculum buckets.
        #[arg(long, value_delimiter = ',')]
        include_buckets: Vec<String>,
        /// Drop any state that matches one of these curriculum buckets.
        #[arg(long, value_delimiter = ',')]
        exclude_buckets: Vec<String>,
        /// Percentage of groups assigned to train.
        #[arg(long, default_value_t = 80)]
        train_pct: u8,
        /// Percentage of groups assigned to val. Test gets the remainder.
        #[arg(long, default_value_t = 10)]
        val_pct: u8,
        /// Preserve up to this many trigger-negative rows that miss include buckets
        /// but do not hit exclude buckets.
        #[arg(long, default_value_t = 0)]
        preserve_trigger_negative_rows: usize,
    },
}

include!("learning_corpus_impl.rs");

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Query {
            search,
            entity_type,
        } => {
            println!(">> Delegating to query_relics.py query...");
            let mut args = vec!["query", search.as_str()];
            if let Some(t) = entity_type {
                args.push("--type");
                args.push(t.as_str());
            }
            python_wrapper("tools/query_relics.py", &args);
        }
        Commands::Audit {
            prefix,
            entity_type,
        } => {
            println!(">> Delegating to query_relics.py audit...");
            python_wrapper(
                "tools/query_relics.py",
                &["audit", prefix, "--type", entity_type],
            );
        }
        Commands::CheckInsertion { prefix } => {
            println!(">> Delegating to query_relics.py check-insertion...");
            python_wrapper("tools/query_relics.py", &["check-insertion", prefix]);
        }
        Commands::ParseAst { file } => {
            println!(">> Delegating to monster_ast.py parse...");
            // As per python wrapper strategy for AST
            if let Some(file_str) = file.to_str() {
                python_wrapper("tools/source_extractor/monster_ast.py", &[file_str]);
            }
        }
        Commands::Diff { java_log, rust_log } => {
            println!(">> Diffing logs...");
            println!("Java: {}", java_log.display());
            println!("Rust: {}", rust_log.display());

            let java_content = match std::fs::read_to_string(java_log) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to load Java log: {}", e);
                    return;
                }
            };
            let rust_content = match std::fs::read_to_string(rust_log) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to load Rust log: {}", e);
                    return;
                }
            };

            let java_states: Vec<serde_json::Value> = java_content
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            let rust_states: Vec<serde_json::Value> = rust_content
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();

            if java_states.is_empty() || rust_states.is_empty() {
                eprintln!("One or both logs contain no valid JSON lines.");
                return;
            }

            println!(
                "Loaded {} java steps, {} rust steps.",
                java_states.len(),
                rust_states.len()
            );
            // TODO: Deep semantic diff mapping using generated schema and sts_simulator::diff::delta
            println!("(Semantic Diff algorithm available via delta.rs...)");
        }
        Commands::SyncSchema => {
            println!(">> Initiating ASA Engine Schema Synchronization...");
            let tmp_dir = std::env::temp_dir();
            let java_json = tmp_dir.join("extracted_java_nodes.json");
            let rust_json = tmp_dir.join("extracted_rust_nodes.json");
            let observed_json = tmp_dir.join("observed_ids.json");
            let matched_json = tmp_dir.join("matched_schema.json");

            // Hardcoded paths relative to the cargo root
            let java_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cardcrawl");
            let rust_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
            let baseline = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tools/protocol_schema_baseline.json");
            let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tools/compiled_protocol_schema.json");
            let audit =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/schema_audit_report.json");
            let run_logs_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/runs");
            let live_comm =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("logs/current/live_comm_raw.jsonl");

            let p_j = java_json.to_string_lossy();
            let p_r = rust_json.to_string_lossy();
            let p_o = observed_json.to_string_lossy();
            let j_src = java_src.to_string_lossy();
            let r_src = rust_src.to_string_lossy();

            let config_dir =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/schema_builder/config");
            let p_skeleton = config_dir
                .join("schema_skeleton.json")
                .to_string_lossy()
                .into_owned();
            let p_overrides = config_dir
                .join("override_rules.json")
                .to_string_lossy()
                .into_owned();

            let p_b = baseline.to_string_lossy();
            let p_m = matched_json.to_string_lossy();
            let p_c = compiled.to_string_lossy();
            let p_a = audit.to_string_lossy();

            println!(">> 0. Extracting observed IDs...");
            let mut observed_args_owned = vec![p_o.to_string()];
            if run_logs_dir.exists() {
                let mut replay_files: Vec<_> = std::fs::read_dir(&run_logs_dir)
                    .expect("read_dir logs/runs should succeed")
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .map(|path| path.join("raw.jsonl"))
                    .filter(|path| path.exists())
                    .collect();
                replay_files.sort();
                let replay_args: Vec<String> = replay_files
                    .into_iter()
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect();
                observed_args_owned.extend(replay_args);
            }
            if live_comm.exists() {
                observed_args_owned.push(live_comm.to_string_lossy().into_owned());
            }
            let observed_args: Vec<&str> = observed_args_owned.iter().map(String::as_str).collect();
            python_wrapper(
                "tools/schema_builder/observed_id_extractor.py",
                &observed_args,
            );

            println!(">> 1. Running java_crawler...");
            python_wrapper("tools/schema_builder/java_crawler.py", &[&j_src, &p_j]);

            println!(">> 2. Running rust_crawler...");
            python_wrapper("tools/schema_builder/rust_crawler.py", &[&r_src, &p_r]);

            println!(">> 3. Running heuristic_matcher...");
            python_wrapper(
                "tools/schema_builder/heuristic_matcher.py",
                &[&p_j, &p_r, &p_skeleton, &p_overrides, &p_o, &p_m],
            );

            println!(">> 4. Running schema_compiler...");
            python_wrapper(
                "tools/schema_builder/schema_compiler.py",
                &[&p_m, &p_b, &p_o, &p_c, &p_a],
            );

            println!("========================================");
            println!("Sync Complete! Compiled schema updated at {:?}", compiled);
            println!("Audit report written to {:?}", audit);
            println!("Run `cargo build` to propagate the compiled schema bindings.");
        }
        Commands::InteractionCoverage => {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let coverage_path = manifest_dir.join("tools/artifacts/interaction_coverage.json");
            let report_path = manifest_dir.join("tools/artifacts/interaction_coverage_report.json");
            let live_comm_raw = manifest_dir.join("logs/current/live_comm_raw.jsonl");
            let live_comm_sidecar = manifest_dir.join("logs/current/live_comm_signatures.jsonl");

            let mut generated_from: Vec<String> = Vec::new();
            let mut notes = Vec::new();
            let mut records = Vec::new();

            if live_comm_sidecar.exists() {
                generated_from.push(live_comm_sidecar.to_string_lossy().into_owned());
                match sts_simulator::cli::coverage_tools::load_live_comm_records(&live_comm_sidecar)
                {
                    Ok(live_records) => records.extend(live_records),
                    Err(err) => {
                        eprintln!("Failed to load live_comm signature records: {}", err);
                        std::process::exit(1);
                    }
                }
            } else if live_comm_raw.exists() {
                notes.push(format!(
                    "{} present but omitted from strict signature extraction because it lacks command context",
                    live_comm_raw.display()
                ));
            } else {
                notes.push(
                    "interaction coverage now only ingests live_comm_signatures.jsonl inputs"
                        .to_string(),
                );
            }

            if let Err(err) = sts_simulator::cli::coverage_tools::write_coverage_outputs(
                &records,
                generated_from,
                &coverage_path,
                &report_path,
                notes,
            ) {
                eprintln!("Failed to write interaction coverage artifacts: {}", err);
                std::process::exit(1);
            }

            println!(
                "Interaction coverage written to {:?} and {:?}",
                coverage_path, report_path
            );
            println!("Records: {}", records.len());
        }
        Commands::RunBatch {
            episodes,
            seed,
            ascension,
            class,
            final_act,
            max_steps,
            policy,
            trace_dir,
            summary_out,
            determinism_check,
            reward_shaping_profile,
        } => {
            let policy_kind = match policy.to_ascii_lowercase().as_str() {
                "random_masked" => sts_simulator::cli::full_run_smoke::RunPolicyKind::RandomMasked,
                "rule_baseline_v0" => {
                    sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0
                }
                "rule_baseline_v0_control" | "v0_control" => {
                    sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0Control
                }
                "rule_baseline_v1_candidate" | "v1_candidate" => {
                    sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV1Candidate
                }
                "plan_query_v0" => sts_simulator::cli::full_run_smoke::RunPolicyKind::PlanQueryV0,
                other => {
                    eprintln!(
                        "unsupported policy '{other}'; expected random_masked, rule_baseline_v0, rule_baseline_v0_control, rule_baseline_v1_candidate, or plan_query_v0"
                    );
                    std::process::exit(2);
                }
            };
            let player_class = match class.to_ascii_lowercase().as_str() {
                "ironclad" | "red" => "Ironclad",
                "silent" | "green" => "Silent",
                "defect" | "blue" => "Defect",
                "watcher" | "purple" => "Watcher",
                other => {
                    eprintln!(
                        "unsupported class '{other}'; expected ironclad, silent, defect, or watcher"
                    );
                    std::process::exit(2);
                }
            };
            let config = sts_simulator::cli::full_run_smoke::RunBatchConfig {
                episodes: *episodes,
                base_seed: *seed,
                ascension: *ascension,
                final_act: *final_act,
                player_class,
                max_steps: *max_steps,
                policy: policy_kind,
                trace_dir: trace_dir.clone(),
                determinism_check: *determinism_check,
                reward_shaping_profile:
                    match sts_simulator::cli::full_run_smoke::RewardShapingProfile::parse(
                        reward_shaping_profile,
                    ) {
                        Ok(profile) => profile,
                        Err(err) => {
                            eprintln!("{err}");
                            std::process::exit(2);
                        }
                    },
            };
            let summary =
                sts_simulator::cli::full_run_smoke::run_batch(&config).unwrap_or_else(|err| {
                    eprintln!("run-batch failed: {err}");
                    std::process::exit(1);
                });
            if let Some(summary_out) = summary_out {
                if let Some(parent) = summary_out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("run-batch summary parent should be creatable");
                }
                std::fs::write(
                    summary_out,
                    serde_json::to_string_pretty(&summary)
                        .expect("run-batch summary should serialize"),
                )
                .expect("run-batch summary should write");
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&summary)
                    .expect("run-batch summary should serialize for stdout")
            );
        }
        Commands::Logs { command } => {
            let paths = sts_simulator::cli::live_comm_admin::LiveLogPaths::default_paths();
            match command {
                LogCommands::Status => {
                    let status = sts_simulator::cli::live_comm_admin::logs_status(&paths)
                        .expect("logs status should load");
                    println!("runs={}", status.total_runs);
                    println!("clean_runs={}", status.clean_runs);
                    println!("tainted_runs={}", status.tainted_runs);
                    println!("pinned_runs={}", status.pinned_runs);
                    println!(
                        "latest_run={}",
                        status.latest_run_id.unwrap_or_else(|| "<none>".to_string())
                    );
                    for (label, count) in status.labels {
                        println!("label[{label}]={count}");
                    }
                }
                LogCommands::Gc => {
                    let summary = sts_simulator::cli::live_comm_admin::gc_runs(&paths)
                        .expect("gc should run");
                    println!(
                        "gc pruned_runs={} pruned_debug={} pruned_replay={} pruned_watch={}",
                        summary.pruned_run_artifacts,
                        summary.pruned_debug,
                        summary.pruned_replay,
                        summary.pruned_watch
                    );
                }
                LogCommands::FreezeBaseline {
                    out,
                    latest_runs,
                    fixture_specs,
                } => {
                    let entries =
                        sts_simulator::cli::live_comm_admin::list_run_manifests_for_audit(&paths)
                            .expect("manifest listing should succeed");
                    let mut entries = entries;
                    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));

                    let run_selection_score =
                        |manifest: &sts_simulator::cli::live_comm_admin::LiveRunManifest| {
                            let mut score = 0i32;
                            let clean = manifest.counts.engine_bugs == 0
                                && manifest.counts.replay_failures == 0
                                && !manifest.classification_label.contains("tainted");
                            if clean {
                                score += 100;
                            }
                            if manifest.counts.content_gaps == 0 {
                                score += 20;
                            }
                            if manifest
                                .artifacts
                                .reward_audit
                                .as_ref()
                                .is_some_and(|artifact| artifact.present)
                            {
                                score += 40;
                            }
                            if manifest
                                .artifacts
                                .event_audit
                                .as_ref()
                                .is_some_and(|artifact| artifact.present)
                            {
                                score += 20;
                            }
                            if manifest
                                .artifacts
                                .combat_suspects
                                .as_ref()
                                .is_some_and(|artifact| artifact.present)
                            {
                                score += 20;
                            }
                            if manifest
                                .artifacts
                                .validation
                                .as_ref()
                                .is_some_and(|artifact| artifact.present)
                            {
                                score += 10;
                            }
                            if manifest
                                .artifacts
                                .sidecar_shadow
                                .as_ref()
                                .is_some_and(|artifact| artifact.present)
                            {
                                score += 10;
                            }
                            score
                        };

                    let mut selected_entries = entries
                        .iter()
                        .filter(|(_, manifest)| {
                            manifest
                                .validation
                                .as_ref()
                                .is_some_and(|validation| validation.status.starts_with("ok"))
                        })
                        .collect::<Vec<_>>();
                    selected_entries.sort_by(|left, right| {
                        run_selection_score(&right.1)
                            .cmp(&run_selection_score(&left.1))
                            .then_with(|| right.1.run_id.cmp(&left.1.run_id))
                    });

                    let selected = selected_entries
                        .into_iter()
                        .take(*latest_runs)
                        .map(|(manifest_path, manifest)| {
                            let run_dir = manifest_path.parent().expect("run dir");
                            let artifact_path = |record: &Option<
                                sts_simulator::cli::live_comm_admin::LiveArtifactRecord,
                            >| {
                                record.as_ref().filter(|artifact| artifact.present).map(
                                    |artifact| {
                                        run_dir.join(&artifact.relative_path).display().to_string()
                                    },
                                )
                            };
                            LearningBaselineRun {
                                run_id: manifest.run_id.clone(),
                                classification_label: manifest.classification_label.clone(),
                                validation_status: manifest
                                    .validation
                                    .as_ref()
                                    .map(|validation| validation.status.clone()),
                                selection_score: run_selection_score(manifest),
                                engine_bugs: manifest.counts.engine_bugs,
                                content_gaps: manifest.counts.content_gaps,
                                replay_failures: manifest.counts.replay_failures,
                                manifest_path: manifest_path.display().to_string(),
                                raw_path: artifact_path(&manifest.artifacts.raw),
                                reward_audit_path: artifact_path(&manifest.artifacts.reward_audit),
                                event_audit_path: artifact_path(&manifest.artifacts.event_audit),
                                combat_suspects_path: artifact_path(
                                    &manifest.artifacts.combat_suspects,
                                ),
                                failure_snapshots_path: artifact_path(
                                    &manifest.artifacts.failure_snapshots,
                                ),
                                validation_path: artifact_path(&manifest.artifacts.validation),
                                sidecar_shadow_path: artifact_path(
                                    &manifest.artifacts.sidecar_shadow,
                                ),
                            }
                        })
                        .collect::<Vec<_>>();

                    let reward_case_run_ids = selected
                        .iter()
                        .filter(|run| run.reward_audit_path.is_some())
                        .map(|run| run.run_id.clone())
                        .collect::<Vec<_>>();
                    let event_case_run_ids = selected
                        .iter()
                        .filter(|run| run.event_audit_path.is_some())
                        .map(|run| run.run_id.clone())
                        .collect::<Vec<_>>();
                    let combat_case_run_ids = selected
                        .iter()
                        .filter(|run| run.combat_suspects_path.is_some())
                        .map(|run| run.run_id.clone())
                        .collect::<Vec<_>>();
                    let failure_snapshot_case_run_ids = selected
                        .iter()
                        .filter(|run| run.failure_snapshots_path.is_some())
                        .map(|run| run.run_id.clone())
                        .collect::<Vec<_>>();
                    let shadow_case_run_ids = selected
                        .iter()
                        .filter(|run| run.sidecar_shadow_path.is_some())
                        .map(|run| run.run_id.clone())
                        .collect::<Vec<_>>();
                    let accepted_run_ids = selected
                        .iter()
                        .map(|run| run.run_id.clone())
                        .collect::<Vec<_>>();
                    let known_noise = entries
                        .iter()
                        .filter(|(_, manifest)| {
                            manifest.counts.engine_bugs > 0
                                || manifest.counts.content_gaps > 0
                                || manifest.counts.replay_failures > 0
                        })
                        .take(*latest_runs)
                        .map(|(_, manifest)| LearningBaselineNoise {
                            run_id: manifest.run_id.clone(),
                            classification_label: manifest.classification_label.clone(),
                            engine_bugs: manifest.counts.engine_bugs,
                            content_gaps: manifest.counts.content_gaps,
                            replay_failures: manifest.counts.replay_failures,
                        })
                        .collect::<Vec<_>>();

                    let default_fixtures = [
                        "data/combat_lab/specs/jaw_worm_opening.json",
                        "data/combat_lab/specs/survival_override_guardrail.json",
                        "data/combat_lab/specs/second_wind_uses_status_fuel_under_pressure.json",
                    ];
                    let combat_lab_fixtures = if fixture_specs.is_empty() {
                        default_fixtures.iter().map(|s| s.to_string()).collect()
                    } else {
                        fixture_specs
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect()
                    };

                    let baseline = LearningBaselineManifest {
                        version: 1,
                        generated_at: sts_simulator::cli::live_comm_admin::timestamp_string(),
                        source: "sts_dev_tool logs freeze-baseline",
                        selected_runs: selected,
                        accepted_run_ids,
                        combat_lab_fixtures,
                        reward_case_run_ids,
                        event_case_run_ids,
                        combat_case_run_ids,
                        failure_snapshot_case_run_ids,
                        shadow_case_run_ids,
                        known_noise,
                    };

                    if let Some(parent) = out.parent() {
                        std::fs::create_dir_all(parent)
                            .expect("baseline output parent should be creatable");
                    }
                    std::fs::write(
                        out,
                        serde_json::to_string_pretty(&baseline)
                            .expect("baseline manifest should serialize"),
                    )
                    .expect("baseline manifest should write");
                    println!("{}", out.display());
                }
                LogCommands::Pin { run_id } => {
                    let path =
                        sts_simulator::cli::live_comm_admin::set_run_pin(&paths, run_id, true)
                            .expect("pin should succeed");
                    println!("pinned {} -> {}", run_id, path.display());
                }
                LogCommands::Unpin { run_id } => {
                    let path =
                        sts_simulator::cli::live_comm_admin::set_run_pin(&paths, run_id, false)
                            .expect("unpin should succeed");
                    println!("unpinned {} -> {}", run_id, path.display());
                }
                LogCommands::Replay { run_id } => {
                    let path =
                        sts_simulator::cli::live_comm_admin::regenerate_run_replay(&paths, run_id)
                            .expect("replay regeneration should succeed");
                    println!("replay regenerated: {}", path.display());
                }
                LogCommands::Latest { label, artifact } => {
                    let path = sts_simulator::cli::live_comm_admin::latest_run_artifact_path(
                        &paths,
                        label.as_deref(),
                        artifact,
                    )
                    .or_else(|| {
                        if artifact == "raw" {
                            sts_simulator::cli::live_comm_admin::latest_raw_path(&paths)
                        } else if artifact == "combat_suspects" {
                            sts_simulator::cli::live_comm_admin::latest_combat_suspect_path(&paths)
                        } else if artifact == "findings" {
                            sts_simulator::cli::live_comm_admin::latest_run_artifact_path(
                                &paths,
                                label.as_deref(),
                                "findings",
                            )
                        } else {
                            None
                        }
                    })
                    .expect("no matching artifact found");
                    println!("{}", path.display());
                }
                LogCommands::InspectFindings {
                    run_id,
                    label,
                    family,
                    category,
                    limit,
                } => {
                    let (manifest_path, manifest) = manifest_entry_for_run_or_latest(
                        &paths,
                        run_id.as_deref(),
                        label.as_deref(),
                    )
                    .expect("no matching run manifest found");

                    let (mut report, findings_path, synthesized) = match artifact_path_for_record(
                        &manifest_path,
                        &manifest.artifacts.findings,
                    ) {
                        Some(findings_path) => (
                            load_findings_report(&findings_path)
                                .expect("findings report should load"),
                            findings_path,
                            false,
                        ),
                        None => {
                            let (report, snapshots_path) =
                                build_findings_report_from_snapshots(&manifest_path, &manifest)
                                    .expect("failure snapshots fallback should build findings");
                            (report, snapshots_path, true)
                        }
                    };

                    if let Some(category_filter) = category.as_ref() {
                        report
                            .families
                            .retain(|entry| entry.category == *category_filter);
                    }
                    if let Some(family_filter) = family.as_ref() {
                        let needle = family_filter.to_ascii_lowercase();
                        report.families.retain(|entry| {
                            entry.key.to_ascii_lowercase().contains(&needle)
                                || entry
                                    .combat_labels
                                    .iter()
                                    .any(|label| label.to_ascii_lowercase().contains(&needle))
                                || entry
                                    .event_labels
                                    .iter()
                                    .any(|label| label.to_ascii_lowercase().contains(&needle))
                        });
                    }

                    report.families.sort_by(|left, right| {
                        right
                            .count
                            .cmp(&left.count)
                            .then_with(|| left.first_frame.cmp(&right.first_frame))
                            .then_with(|| left.key.cmp(&right.key))
                    });

                    println!(
                        "run={} classification={} findings_source={}{}",
                        report.run_id,
                        report.classification_label,
                        findings_path.display(),
                        if synthesized {
                            " (synthesized from failure_snapshots.jsonl)"
                        } else {
                            ""
                        }
                    );
                    println!(
                        "counts: engine_bugs={} content_gaps={} timing_diffs={} replay_failures={}",
                        report.counts.engine_bugs,
                        report.counts.content_gaps,
                        report.counts.timing_diffs,
                        report.counts.replay_failures
                    );
                    println!("matching_families={}", report.families.len());

                    for finding in report.families.iter().take(*limit) {
                        println!();
                        println!("{}", render_findings_family(finding));
                    }
                }
                LogCommands::AnalyzeDecisions {
                    run_id,
                    label,
                    limit,
                    json_out,
                } => {
                    let (manifest_path, manifest) = manifest_entry_for_run_or_latest(
                        &paths,
                        run_id.as_deref(),
                        label.as_deref(),
                    )
                    .expect("no matching run manifest found");

                    let debug_path =
                        artifact_path_for_record(&manifest_path, &manifest.artifacts.debug)
                            .expect("matching run is missing debug.txt");
                    let bot_strength =
                        artifact_path_for_record(&manifest_path, &manifest.artifacts.bot_strength)
                            .map(|path| load_bot_strength_summary(&path))
                            .transpose()
                            .expect("bot_strength.json should load when present");

                    let report = analyze_decision_debug(
                        &debug_path,
                        &manifest.run_id,
                        &manifest.classification_label,
                        manifest.counts.engine_bugs == 0
                            && manifest.counts.content_gaps == 0
                            && manifest.counts.timing_diffs == 0
                            && manifest.counts.replay_failures == 0,
                        bot_strength,
                    )
                    .expect("decision debug analysis should succeed");

                    if let Some(json_out) = json_out {
                        if let Some(parent) = json_out.parent() {
                            std::fs::create_dir_all(parent)
                                .expect("decision report output parent should be creatable");
                        }
                        std::fs::write(
                            &json_out,
                            serde_json::to_string_pretty(&report)
                                .expect("decision report should serialize"),
                        )
                        .expect("decision report should write");
                    }

                    println!("{}", render_decision_experiment_report(&report, *limit));
                }
                LogCommands::ExportDisagreementFixtures {
                    run_id,
                    label,
                    categories,
                    limit,
                    window_lookback,
                    out_dir,
                    json_out,
                } => {
                    let (manifest_path, manifest) = manifest_entry_for_run_or_latest(
                        &paths,
                        run_id.as_deref(),
                        label.as_deref(),
                    )
                    .expect("no matching run manifest found");

                    let debug_path =
                        artifact_path_for_record(&manifest_path, &manifest.artifacts.debug)
                            .expect("matching run is missing debug.txt");
                    let raw_path =
                        artifact_path_for_record(&manifest_path, &manifest.artifacts.raw)
                            .expect("matching run is missing raw.jsonl");
                    let bot_strength =
                        artifact_path_for_record(&manifest_path, &manifest.artifacts.bot_strength)
                            .map(|path| load_bot_strength_summary(&path))
                            .transpose()
                            .expect("bot_strength.json should load when present");

                    let report = analyze_decision_debug(
                        &debug_path,
                        &manifest.run_id,
                        &manifest.classification_label,
                        manifest.counts.engine_bugs == 0
                            && manifest.counts.content_gaps == 0
                            && manifest.counts.timing_diffs == 0
                            && manifest.counts.replay_failures == 0,
                        bot_strength,
                    )
                    .expect("decision debug analysis should succeed");
                    let combat_shadows_by_frame =
                        live_combat_shadow_path_for_manifest(&manifest_path, &manifest)
                            .map(|path| load_combat_shadow_records_by_frame(&path))
                            .transpose()
                            .expect("combat shadow should load when present");

                    let export_report = export_disagreement_fixtures(
                        &raw_path,
                        &report,
                        combat_shadows_by_frame.as_ref(),
                        categories,
                        *limit,
                        *window_lookback,
                        out_dir,
                    )
                    .expect("fixture export should succeed");

                    if let Some(json_out) = json_out {
                        if let Some(parent) = json_out.parent() {
                            std::fs::create_dir_all(parent)
                                .expect("fixture export report output parent should be creatable");
                        }
                        std::fs::write(
                            json_out,
                            serde_json::to_string_pretty(&export_report)
                                .expect("fixture export report should serialize"),
                        )
                        .expect("fixture export report should write");
                    }

                    println!(
                        "{}",
                        render_exported_disagreement_fixture_report(&export_report)
                    );
                }
                LogCommands::ExportDecisionTrainingSet {
                    fixtures,
                    fixture_dirs,
                    out,
                    proposal_out,
                    summary_out,
                    proposal_summary_out,
                    depth,
                } => {
                    let fixture_paths = collect_fixture_paths(fixtures, fixture_dirs)
                        .expect("fixture collection should succeed");
                    let records = build_decision_training_set(&fixture_paths, *depth)
                        .expect("decision training set build should succeed");

                    if let Some(parent) = out.parent() {
                        std::fs::create_dir_all(parent)
                            .expect("decision training set output parent should be creatable");
                    }
                    let mut lines = String::new();
                    for record in &records {
                        lines.push_str(
                            &serde_json::to_string(record)
                                .expect("decision training record should serialize"),
                        );
                        lines.push('\n');
                    }
                    std::fs::write(out, lines).expect("decision training set should write");

                    let summary = summarize_decision_training_set(&records, out);
                    let proposal_records = build_proposal_training_set(&records);
                    if let Some(summary_out) = summary_out {
                        if let Some(parent) = summary_out.parent() {
                            std::fs::create_dir_all(parent)
                                .expect("decision training summary parent should be creatable");
                        }
                        std::fs::write(
                            summary_out,
                            serde_json::to_string_pretty(&summary)
                                .expect("decision training summary should serialize"),
                        )
                        .expect("decision training summary should write");
                    }
                    if let Some(proposal_out) = proposal_out {
                        if let Some(parent) = proposal_out.parent() {
                            std::fs::create_dir_all(parent)
                                .expect("proposal training set output parent should be creatable");
                        }
                        let mut lines = String::new();
                        for record in &proposal_records {
                            lines.push_str(
                                &serde_json::to_string(record)
                                    .expect("proposal training record should serialize"),
                            );
                            lines.push('\n');
                        }
                        std::fs::write(proposal_out, lines)
                            .expect("proposal training set should write");
                        if let Some(summary_out) = proposal_summary_out {
                            let proposal_summary =
                                summarize_proposal_training_set(&proposal_records, proposal_out);
                            if let Some(parent) = summary_out.parent() {
                                std::fs::create_dir_all(parent).expect(
                                    "proposal training summary output parent should be creatable",
                                );
                            }
                            std::fs::write(
                                summary_out,
                                serde_json::to_string_pretty(&proposal_summary)
                                    .expect("proposal training summary should serialize"),
                            )
                            .expect("proposal training summary should write");
                        }
                    }

                    println!(
                        "decision training set wrote {} record(s) to {}",
                        records.len(),
                        out.display()
                    );
                    println!(
                        "proposal training set staged {} record(s)",
                        proposal_records.len()
                    );
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&summary)
                            .expect("decision training summary should serialize for stdout")
                    );
                }
                LogCommands::BuildDecisionCorpus {
                    run_ids,
                    label,
                    latest_runs,
                    categories,
                    limit_per_run,
                    window_lookback,
                    depth,
                    out_dir,
                } => {
                    let run_entries = manifest_entries_for_corpus(
                        &paths,
                        run_ids,
                        label.as_deref(),
                        *latest_runs,
                    )
                    .expect("corpus run selection should succeed");
                    let corpus_summary = build_decision_corpus(
                        &run_entries,
                        categories,
                        *limit_per_run,
                        *window_lookback,
                        *depth,
                        out_dir,
                    )
                    .expect("decision corpus build should succeed");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&corpus_summary)
                            .expect("decision corpus summary should serialize for stdout")
                    );
                }
            }
        }
        Commands::Combat { command } => match command {
            CombatCommands::PlanProbe {
                trace_file,
                step_index,
                out,
                ascension,
                class,
                final_act,
                max_steps,
                max_depth,
                max_nodes,
                beam_width,
                max_engine_steps_per_action,
            } => {
                let report = sts_simulator::cli::full_run_smoke::probe_combat_plan_from_trace(
                    &sts_simulator::cli::full_run_smoke::FullRunTracePlanProbeConfig {
                        trace_file: trace_file.clone(),
                        step_index: *step_index,
                        ascension: *ascension,
                        final_act: *final_act,
                        player_class: class.clone(),
                        max_steps: *max_steps,
                        probe_config: sts_simulator::bot::combat::CombatTurnPlanProbeConfig {
                            max_depth: *max_depth,
                            max_nodes: *max_nodes,
                            beam_width: *beam_width,
                            max_engine_steps_per_action: *max_engine_steps_per_action,
                        },
                    },
                )
                .expect("combat plan-probe should succeed");

                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("combat plan-probe output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("combat plan-probe report should serialize"),
                )
                .expect("combat plan-probe report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .expect("combat plan-probe report should serialize for stdout")
                );
            }
            CombatCommands::PlanProbeAuthorSpec {
                author_spec,
                out,
                max_depth,
                max_nodes,
                beam_width,
                max_engine_steps_per_action,
            } => {
                let payload = std::fs::read_to_string(author_spec)
                    .expect("combat plan-probe author spec should be readable");
                let spec: CombatAuthorSpec =
                    serde_json::from_str(&payload).expect("combat author spec should parse");
                let case = compile_combat_author_case(&spec)
                    .expect("combat author spec should compile to combat case");
                let seed = lower_case(&case).expect("combat author spec case should lower");
                let mut report = sts_simulator::bot::combat::probe_turn_plans(
                    &seed.engine_state,
                    &seed.combat,
                    sts_simulator::bot::combat::CombatTurnPlanProbeConfig {
                        max_depth: *max_depth,
                        max_nodes: *max_nodes,
                        beam_width: *beam_width,
                        max_engine_steps_per_action: *max_engine_steps_per_action,
                    },
                );
                report.source_trace = serde_json::json!({
                    "source": "author_spec",
                    "author_spec": author_spec,
                    "case_id": case.id,
                    "tags": case.tags,
                });

                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("combat plan-probe author-spec output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("combat plan-probe author-spec report should serialize"),
                )
                .expect("combat plan-probe author-spec report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .expect("combat plan-probe author-spec report should serialize for stdout")
                );
            }
            CombatCommands::DrawMarginalProbeAuthorSpec {
                author_spec,
                action_card,
                hand_index,
                target_action_key,
                out,
                max_depth,
                max_nodes,
                beam_width,
                max_engine_steps_per_action,
            } => {
                let payload = std::fs::read_to_string(author_spec)
                    .expect("combat draw-marginal author spec should be readable");
                let spec: CombatAuthorSpec =
                    serde_json::from_str(&payload).expect("combat author spec should parse");
                let case = compile_combat_author_case(&spec)
                    .expect("combat author spec should compile to combat case");
                let seed = lower_case(&case).expect("combat author spec case should lower");
                let target_card = card_id_from_java(action_card)
                    .unwrap_or_else(|| panic!("unknown Java card id or alias '{}'", action_card));
                let mut target = if let Some(hand_index) = hand_index {
                    let card = seed.combat.zones.hand.get(*hand_index).unwrap_or_else(|| {
                        panic!(
                            "target hand index {} out of range for hand size {}",
                            hand_index,
                            seed.combat.zones.hand.len()
                        )
                    });
                    if card.id != target_card {
                        panic!(
                            "target hand index {} has {:?}, expected {:?}",
                            hand_index, card.id, target_card
                        );
                    }
                    sts_simulator::bot::combat::CombatDrawMarginalTarget::hand_instance(
                        target_card,
                        *hand_index,
                        card.uuid,
                    )
                } else {
                    sts_simulator::bot::combat::CombatDrawMarginalTarget::card(target_card)
                };
                if let Some(target_action_key) = target_action_key {
                    target = target.with_root_action_key(target_action_key.clone());
                }
                let mut report = sts_simulator::bot::combat::probe_draw_marginal_value_for_target(
                    &seed.engine_state,
                    &seed.combat,
                    target,
                    sts_simulator::bot::combat::CombatTurnPlanProbeConfig {
                        max_depth: *max_depth,
                        max_nodes: *max_nodes,
                        beam_width: *beam_width,
                        max_engine_steps_per_action: *max_engine_steps_per_action,
                    },
                );
                report.source_trace = serde_json::json!({
                    "source": "author_spec",
                    "author_spec": author_spec,
                    "case_id": case.id,
                    "tags": case.tags,
                });

                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("combat draw-marginal output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("combat draw-marginal report should serialize"),
                )
                .expect("combat draw-marginal report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .expect("combat draw-marginal report should serialize for stdout")
                );
            }
            CombatCommands::DrawMarginalProbe {
                trace_file,
                step_index,
                action_card,
                hand_index,
                target_action_key,
                out,
                ascension,
                class,
                final_act,
                max_steps,
                max_depth,
                max_nodes,
                beam_width,
                max_engine_steps_per_action,
            } => {
                let target_card = card_id_from_java(action_card)
                    .unwrap_or_else(|| panic!("unknown Java card id or alias '{}'", action_card));
                let report =
                    sts_simulator::cli::full_run_smoke::probe_combat_draw_marginal_from_trace(
                        &sts_simulator::cli::full_run_smoke::FullRunTraceDrawMarginalProbeConfig {
                            trace_file: trace_file.clone(),
                            step_index: *step_index,
                            target_card,
                            target_hand_index: *hand_index,
                            target_action_key: target_action_key.clone(),
                            ascension: *ascension,
                            final_act: *final_act,
                            player_class: class.clone(),
                            max_steps: *max_steps,
                            probe_config: sts_simulator::bot::combat::CombatTurnPlanProbeConfig {
                                max_depth: *max_depth,
                                max_nodes: *max_nodes,
                                beam_width: *beam_width,
                                max_engine_steps_per_action: *max_engine_steps_per_action,
                            },
                        },
                    )
                    .expect("combat draw-marginal trace probe should succeed");

                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("combat draw-marginal trace output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("combat draw-marginal trace report should serialize"),
                )
                .expect("combat draw-marginal trace report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .expect("combat draw-marginal trace report should serialize for stdout")
                );
            }
            CombatCommands::CandidateOutcomePack {
                trace_file,
                step_index,
                out,
                ascension,
                class,
                final_act,
                max_steps,
                max_exact_nodes_per_candidate,
                max_engine_steps_per_action,
                max_candidates,
                controlled_v0,
            } => {
                let report =
                    sts_simulator::cli::full_run_smoke::build_candidate_outcome_pack_from_trace(
                        &sts_simulator::cli::full_run_smoke::FullRunTraceCandidateOutcomePackConfig {
                            trace_file: trace_file.clone(),
                            step_index: *step_index,
                            ascension: *ascension,
                            final_act: *final_act,
                            player_class: class.clone(),
                            max_steps: *max_steps,
                            max_exact_nodes_per_candidate: *max_exact_nodes_per_candidate,
                            max_engine_steps_per_action: *max_engine_steps_per_action,
                            max_candidates: *max_candidates,
                            controlled_v0: *controlled_v0,
                            min_eligible_candidates: 4,
                        },
                    )
                    .expect("combat candidate-outcome pack should build");

                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("combat candidate-outcome output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("combat candidate-outcome report should serialize"),
                )
                .expect("combat candidate-outcome report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": &report.schema_version,
                        "out": out.display().to_string(),
                        "split_group_key": &report.split_group_key,
                        "candidate_count": report.candidate_count,
                        "pack_oracle_quality": &report.pack_oracle_quality,
                        "truth_warnings": &report.truth_warnings,
                    }))
                    .expect("combat candidate-outcome summary should serialize for stdout")
                );
            }
            CombatCommands::CandidateOutcomePackBatch {
                trace_input,
                out_dir,
                step_start,
                step_end,
                step_limit,
                ascension,
                class,
                final_act,
                max_steps,
                budgets,
                max_engine_steps_per_action,
                min_eligible_candidates,
                min_trainable_pairs,
                median_runtime_ms_limit,
            } => {
                let parsed_budgets = budgets
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        value.parse::<usize>().unwrap_or_else(|_| {
                            panic!("invalid candidate-outcome budget '{value}'")
                        })
                    })
                    .collect::<Vec<_>>();
                if parsed_budgets.is_empty() {
                    panic!("candidate-outcome-pack-batch requires at least one budget");
                }
                let report =
                    sts_simulator::cli::full_run_smoke::build_candidate_outcome_pack_batch_from_traces(
                        &sts_simulator::cli::full_run_smoke::FullRunTraceCandidateOutcomePackBatchConfig {
                            trace_inputs: trace_input.clone(),
                            out_dir: out_dir.clone(),
                            step_start: *step_start,
                            step_end: *step_end,
                            step_limit: *step_limit,
                            ascension: *ascension,
                            final_act: *final_act,
                            player_class: class.clone(),
                            max_steps: *max_steps,
                            budgets: parsed_budgets,
                            max_engine_steps_per_action: *max_engine_steps_per_action,
                            min_eligible_candidates: *min_eligible_candidates,
                            min_trainable_pairs: *min_trainable_pairs,
                            median_runtime_ms_limit: *median_runtime_ms_limit,
                        },
                    )
                    .expect("combat candidate-outcome batch should build");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": &report.schema_version,
                        "out_dir": &report.out_dir,
                        "selected_budget": report.selected_budget,
                        "oracle_ready": report.oracle_ready,
                        "oracle_ready_reason": &report.oracle_ready_reason,
                        "trainable_manifest_count": report.trainable_manifest.len(),
                        "diagnostic_manifest_count": report.diagnostic_manifest.len(),
                        "error_count": report.errors.len(),
                    }))
                    .expect("combat candidate-outcome batch summary should serialize for stdout")
                );
            }
            CombatCommands::RecursiveRolloutValidation {
                trace_input,
                out_dir,
                step_start,
                step_end,
                step_limit,
                ascension,
                class,
                final_act,
                max_steps,
                horizon_decisions,
                continuation_policy,
                max_candidates,
                controlled_v0,
            } => {
                let policy_kind = match continuation_policy.to_ascii_lowercase().as_str() {
                    "rule_baseline_v0" | "rule" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0
                    }
                    "rule_baseline_v0_control" | "v0_control" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0Control
                    }
                    "rule_baseline_v1_candidate" | "v1_candidate" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV1Candidate
                    }
                    "plan_query_v0" | "plan" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::PlanQueryV0
                    }
                    other => {
                        eprintln!(
                            "unsupported continuation policy '{other}'; expected rule_baseline_v0, rule_baseline_v0_control, rule_baseline_v1_candidate, or plan_query_v0"
                        );
                        std::process::exit(2);
                    }
                };
                let report =
                    sts_simulator::cli::full_run_smoke::run_recursive_rollout_validation_from_traces(
                        &sts_simulator::cli::full_run_smoke::FullRunTraceRecursiveRolloutValidationConfig {
                            trace_inputs: trace_input.clone(),
                            out_dir: out_dir.clone(),
                            step_start: *step_start,
                            step_end: *step_end,
                            step_limit: *step_limit,
                            ascension: *ascension,
                            final_act: *final_act,
                            player_class: class.clone(),
                            max_steps: *max_steps,
                            horizon_decisions: *horizon_decisions,
                            continuation_policy: policy_kind,
                            max_candidates: *max_candidates,
                            controlled_v0: *controlled_v0,
                        },
                    )
                    .expect("combat recursive rollout validation should run");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": report.get("schema_version").cloned().unwrap_or(serde_json::Value::Null),
                        "out_dir": report.get("out_dir").cloned().unwrap_or(serde_json::Value::Null),
                        "pack_count": report.get("pack_count").cloned().unwrap_or(serde_json::Value::Null),
                        "trainable_pack_count": report.get("trainable_pack_count").cloned().unwrap_or(serde_json::Value::Null),
                        "candidate_count": report.get("candidate_count").cloned().unwrap_or(serde_json::Value::Null),
                        "pairwise_label_count": report.get("pairwise_label_count").cloned().unwrap_or(serde_json::Value::Null),
                        "elapsed_ms": report.get("elapsed_ms").cloned().unwrap_or(serde_json::Value::Null),
                        "median_rollout_elapsed_ms": report.get("median_rollout_elapsed_ms").cloned().unwrap_or(serde_json::Value::Null),
                        "rollouts_per_second": report.get("rollouts_per_second").cloned().unwrap_or(serde_json::Value::Null),
                        "terminal_counts": report.get("terminal_counts").cloned().unwrap_or(serde_json::Value::Null),
                        "error_count": report.get("errors").and_then(serde_json::Value::as_array).map(|values| values.len()).unwrap_or(0),
                    }))
                    .expect("combat recursive rollout summary should serialize for stdout")
                );
            }
            CombatCommands::RunBranchFromTrace {
                trace_file,
                step_index,
                target_action_key,
                target_action_index,
                continuation_policy,
                out,
                include_trace,
                ascension,
                class,
                final_act,
                max_steps,
            } => {
                let policy_kind = match continuation_policy.to_ascii_lowercase().as_str() {
                    "rule_baseline_v0" | "rule" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0
                    }
                    "rule_baseline_v0_control" | "v0_control" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0Control
                    }
                    "rule_baseline_v1_candidate" | "v1_candidate" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV1Candidate
                    }
                    "plan_query_v0" | "plan" => {
                        sts_simulator::cli::full_run_smoke::RunPolicyKind::PlanQueryV0
                    }
                    other => {
                        eprintln!(
                            "unsupported continuation policy '{other}'; expected rule_baseline_v0, rule_baseline_v0_control, rule_baseline_v1_candidate, or plan_query_v0"
                        );
                        std::process::exit(2);
                    }
                };
                let report = sts_simulator::cli::full_run_smoke::run_branch_from_trace(
                    &sts_simulator::cli::full_run_smoke::FullRunTraceBranchRunConfig {
                        trace_file: trace_file.clone(),
                        step_index: *step_index,
                        target_action_key: target_action_key.clone(),
                        target_action_index: *target_action_index,
                        ascension: *ascension,
                        final_act: *final_act,
                        player_class: class.clone(),
                        max_steps: *max_steps,
                        continuation_policy: policy_kind,
                        include_trace: *include_trace,
                    },
                )
                .expect("combat run-branch-from-trace should run");
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("run-branch-from-trace output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("run-branch-from-trace report should serialize"),
                )
                .expect("run-branch-from-trace report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": report.get("schema_version").cloned().unwrap_or(serde_json::Value::Null),
                        "out": out.display().to_string(),
                        "forced_action_key": report.get("forced_action_key").cloned().unwrap_or(serde_json::Value::Null),
                        "source_chosen_action_key": report.get("source_chosen_action_key").cloned().unwrap_or(serde_json::Value::Null),
                        "result": report.get("result").cloned().unwrap_or(serde_json::Value::Null),
                        "floor": report.get("floor").cloned().unwrap_or(serde_json::Value::Null),
                        "act": report.get("act").cloned().unwrap_or(serde_json::Value::Null),
                        "hp": report.get("hp").cloned().unwrap_or(serde_json::Value::Null),
                        "combat_win_count": report.get("combat_win_count").cloned().unwrap_or(serde_json::Value::Null),
                        "branch_decisions": report.get("branch_decisions").cloned().unwrap_or(serde_json::Value::Null),
                    }))
                    .expect("run-branch-from-trace stdout summary should serialize")
                );
            }
            CombatCommands::PlanSearchFromTrace {
                trace_file,
                step_index,
                out,
                ascension,
                class,
                final_act,
                max_steps,
                max_nodes,
                beam_width,
                max_depth_decisions,
                turn_sequence_beam_width,
                max_turn_sequence_actions,
                max_branching,
                include_frontier,
            } => {
                let report =
                    sts_simulator::cli::full_run_smoke::search_combat_plan_from_trace(
                        &sts_simulator::cli::full_run_smoke::FullRunTraceCombatPlanSearchConfig {
                            trace_file: trace_file.clone(),
                            step_index: *step_index,
                            ascension: *ascension,
                            final_act: *final_act,
                            player_class: class.clone(),
                            max_steps: *max_steps,
                            max_nodes: *max_nodes,
                            beam_width: *beam_width,
                            max_depth_decisions: *max_depth_decisions,
                            turn_sequence_beam_width: *turn_sequence_beam_width,
                            max_turn_sequence_actions: *max_turn_sequence_actions,
                            max_branching: *max_branching,
                            include_frontier: *include_frontier,
                        },
                    )
                    .expect("combat plan-search-from-trace should run");
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("combat plan-search output parent should be creatable");
                }
                std::fs::write(
                    out,
                    serde_json::to_string_pretty(&report)
                        .expect("combat plan-search report should serialize"),
                )
                .expect("combat plan-search report should write");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "schema_version": report.get("schema_version").cloned().unwrap_or(serde_json::Value::Null),
                        "out": out.display().to_string(),
                        "start": report.get("start").cloned().unwrap_or(serde_json::Value::Null),
                        "search_summary": report.get("search_summary").cloned().unwrap_or(serde_json::Value::Null),
                        "has_complete_clear": report.get("best_complete_clear").is_some_and(|value| !value.is_null()),
                        "best_complete_clear_first_action": report
                            .get("best_complete_clear")
                            .and_then(|value| value.get("first_action_key"))
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "best_alive_censored_first_action": report
                            .get("best_alive_censored")
                            .and_then(|value| value.get("first_action_key"))
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    }))
                    .expect("combat plan-search stdout summary should serialize")
                );
            }
            CombatCommands::BuildStateCorpus {
                fixtures,
                fixture_dirs,
                combat_cases,
                combat_case_dirs,
                raw,
                run_ids,
                label,
                latest_runs,
                limit_per_raw,
                depth,
                include_buckets,
                exclude_buckets,
                out,
                summary_out,
            } => {
                let fixture_paths = collect_json_paths(fixtures, fixture_dirs)
                    .expect("fixture collection should succeed");
                let combat_case_paths = collect_json_paths(combat_cases, combat_case_dirs)
                    .expect("combat case collection should succeed");

                let paths = sts_simulator::cli::live_comm_admin::LiveLogPaths::default_paths();
                let run_entries = if !run_ids.is_empty() || label.is_some() {
                    manifest_entries_for_corpus(&paths, run_ids, label.as_deref(), *latest_runs)
                        .expect("state corpus run selection should succeed")
                } else {
                    Vec::new()
                };

                let mut raw_paths = raw.clone();
                let run_lookup = run_entries
                    .iter()
                    .filter_map(|(manifest_path, manifest)| {
                        artifact_path_for_record(manifest_path, &manifest.artifacts.raw)
                            .map(|path| (path, manifest.run_id.clone()))
                    })
                    .collect::<BTreeMap<_, _>>();
                raw_paths.extend(run_lookup.keys().cloned());
                raw_paths.sort();
                raw_paths.dedup();

                let mut records = Vec::new();
                for path in &fixture_paths {
                    records.push(
                        build_state_record_from_fixture(path, *depth)
                            .expect("fixture state extraction should succeed"),
                    );
                }
                for path in &combat_case_paths {
                    records.push(
                        build_state_record_from_combat_case(path, *depth)
                            .expect("combat case state extraction should succeed"),
                    );
                }
                for raw_path in &raw_paths {
                    let run_id = run_lookup.get(raw_path).map(String::as_str);
                    records.extend(
                        build_state_records_from_raw(raw_path, run_id, *limit_per_raw, *depth)
                            .expect("raw state extraction should succeed"),
                    );
                }

                let (records, filter_stats) = clean_state_corpus_records(records);
                let mut filter_stats = filter_stats;
                let (records, _) = filter_state_corpus_by_buckets(
                    records,
                    include_buckets,
                    exclude_buckets,
                    0,
                    &mut filter_stats,
                );

                if records.is_empty() {
                    panic!(
                        "build-state-corpus found no usable fixture, combat case, or raw states"
                    );
                }

                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("state corpus output parent should be creatable");
                }
                let mut lines = String::new();
                for record in &records {
                    lines.push_str(
                        &serde_json::to_string(record)
                            .expect("state corpus record should serialize"),
                    );
                    lines.push('\n');
                }
                std::fs::write(out, lines).expect("state corpus should write");

                let summary_path = summary_out
                    .clone()
                    .unwrap_or_else(|| out.with_extension("summary.json"));
                let summary = summarize_state_corpus(
                    &records,
                    out,
                    include_buckets,
                    exclude_buckets,
                    filter_stats,
                );
                if let Some(parent) = summary_path.parent() {
                    std::fs::create_dir_all(parent)
                        .expect("state corpus summary parent should be creatable");
                }
                std::fs::write(
                    &summary_path,
                    serde_json::to_string_pretty(&summary)
                        .expect("state corpus summary should serialize"),
                )
                .expect("state corpus summary should write");

                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary)
                        .expect("state corpus summary should serialize for stdout")
                );
            }
            CombatCommands::SplitStateCorpus {
                input,
                out_dir,
                include_buckets,
                exclude_buckets,
                train_pct,
                val_pct,
                preserve_trigger_negative_rows,
            } => {
                let records =
                    load_state_corpus_records(input).expect("state corpus input should load");
                let (split_records, mut summary) = split_state_corpus_records(
                    records,
                    include_buckets,
                    exclude_buckets,
                    *train_pct,
                    *val_pct,
                    *preserve_trigger_negative_rows,
                )
                .expect("state corpus split should succeed");

                std::fs::create_dir_all(out_dir)
                    .expect("state corpus split output dir should be creatable");
                for split_name in ["train", "val", "test"] {
                    let path = out_dir.join(format!("{split_name}.jsonl"));
                    let mut lines = String::new();
                    if let Some(records) = split_records.get(split_name) {
                        for record in records {
                            lines.push_str(
                                &serde_json::to_string(record)
                                    .expect("split state corpus record should serialize"),
                            );
                            lines.push('\n');
                        }
                    }
                    std::fs::write(&path, lines)
                        .expect("split state corpus partition should write");
                }

                summary.input_path = input.display().to_string();
                summary.out_dir = out_dir.display().to_string();
                let summary_path = out_dir.join("split_summary.json");
                std::fs::write(
                    &summary_path,
                    serde_json::to_string_pretty(&summary)
                        .expect("state corpus split summary should serialize"),
                )
                .expect("state corpus split summary should write");

                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary)
                        .expect("state corpus split summary should serialize for stdout")
                );
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_decision_training_example_from_live_shadow, build_proposal_training_set,
        classify_audit_cluster, clean_state_corpus_records, collect_export_examples,
        disagreement_category_from_tags, filter_state_corpus_by_buckets,
        parse_exact_turn_audit_line, parse_frame_marker, parse_idle_end_turn_examples,
        split_name_for_group_key, split_state_corpus_records, state_corpus_split_group_key,
        summarize_decision_training_set, summarize_proposal_training_set, summarize_state_corpus,
        DecisionClusterExample, DecisionExperimentReport, DecisionTrainingExample,
        ProposalTrainingExample, StateCorpusFilterStats, StateCorpusRecord,
    };
    use std::collections::BTreeMap;
    use std::path::Path;

    #[test]
    fn parse_exact_turn_audit_line_extracts_new_decision_fields() {
        let line = "[AUDIT] exact_turn best=PlayCard { card_index: 1, target: Some(2) } line_len=3 ends=18 nodes=14 prunes=0 cycles=0 truncated=false agrees=false regime=contested frontier_class=attack screened_out=0 alternatives=5 dominance=strictly_better_in_window confidence=exact takeover=false takeover_reason=regime_not_takeover chosen_by=frontier frontier_survival=safe exact_survival=safe rejection_reasons=regime_not_takeover,high_threat_disagreement resources=hp80/blk0/pots0/lost0/exh0";
        let audit =
            parse_exact_turn_audit_line(463, Some(218), line).expect("audit line should parse");
        assert_eq!(audit.frame, Some(218));
        assert!(!audit.agrees);
        assert_eq!(audit.screened_out_count, 0);
        assert_eq!(audit.regime.as_deref(), Some("contested"));
        assert_eq!(audit.frontier_class.as_deref(), Some("attack"));
        assert_eq!(
            audit.dominance.as_deref(),
            Some("strictly_better_in_window")
        );
        assert_eq!(audit.confidence.as_deref(), Some("exact"));
        assert_eq!(audit.alternatives, Some(5));
        assert!(audit
            .rejection_reasons
            .iter()
            .any(|reason| reason == "high_threat_disagreement"));
    }

    #[test]
    fn classify_audit_cluster_prioritizes_high_threat_strict_better_lines() {
        let line = "[AUDIT] exact_turn best=PlayCard { card_index: 3, target: None } line_len=3 ends=123 nodes=64 prunes=0 cycles=0 truncated=false agrees=false regime=fragile frontier_class=skill_utility screened_out=0 alternatives=7 dominance=strictly_better_in_window confidence=exact takeover=false takeover_reason=fragile_not_better_survival chosen_by=frontier frontier_survival=severe_risk exact_survival=severe_risk rejection_reasons=fragile_without_survival_upgrade,high_threat_disagreement resources=hp17/blk0/pots0/lost0/exh1";
        let audit =
            parse_exact_turn_audit_line(5442, Some(307), line).expect("audit line should parse");
        assert_eq!(
            classify_audit_cluster(&audit),
            Some("high_threat_exact_disagree_not_taken")
        );
    }

    #[test]
    fn parse_frame_marker_extracts_current_frame() {
        assert_eq!(parse_frame_marker("[F218] COMBAT  HP=14/74"), Some(218));
        assert_eq!(parse_frame_marker("[AUDIT] exact_turn ..."), None);
    }

    #[test]
    fn parse_idle_end_turn_examples_uses_end_diag_with_legal_plays() {
        let lines = vec![
            "[F307] COMBAT  HP=29/74".to_string(),
            "[AUDIT] exact_turn best=EndTurn line_len=1 ends=2 nodes=2 prunes=0 cycles=0 truncated=false agrees=true regime=contested frontier_class=end_turn screened_out=0 alternatives=2 dominance=incomparable confidence=exact takeover=false takeover_reason=frontier_agrees chosen_by=frontier frontier_survival=risky_but_playable exact_survival=risky_but_playable rejection_reasons=frontier_agrees resources=hp43/blk0/pots0/lost0/exh0".to_string(),
            "[END DIAG] END score=24.9 legal_plays=1".to_string(),
            "[END DIAG] play move=PlayCard { card_index: 6, target: None } score=-22.7 visits=2".to_string(),
            "[END DIAG] search_kept_end_turn chosen=EndTurn".to_string(),
        ];
        let mut last_audit = None;
        let examples = parse_idle_end_turn_examples(&lines, &mut last_audit);
        assert_eq!(examples.len(), 1);
        let example = &examples[0];
        assert_eq!(example.category, "idle_energy_end_turn");
        assert_eq!(example.frame, Some(307));
        assert_eq!(example.screened_out_count, 0);
        assert_eq!(example.regime.as_deref(), Some("contested"));
        assert!(example
            .rejection_reasons
            .iter()
            .any(|reason| reason == "end_diag_kept_end_turn"));
    }

    #[test]
    fn collect_export_examples_deduplicates_frames_and_filters_categories() {
        let report = DecisionExperimentReport {
            run_id: "run".to_string(),
            classification_label: "loss_clean".to_string(),
            parity_clean: true,
            debug_path: "debug.txt".to_string(),
            bot_strength: None,
            category_counts: BTreeMap::new(),
            examples: vec![
                super::DecisionClusterExample {
                    category: "high_threat_exact_disagree_not_taken".to_string(),
                    frame: Some(307),
                    line_number: 10,
                    snippet: "a".to_string(),
                    screened_out_count: 0,
                    regime: None,
                    frontier_class: None,
                    dominance: None,
                    chosen_by: None,
                    takeover_reason: None,
                    frontier_survival: None,
                    exact_survival: None,
                    rejection_reasons: Vec::new(),
                },
                super::DecisionClusterExample {
                    category: "strict_better_same_survival".to_string(),
                    frame: Some(307),
                    line_number: 11,
                    snippet: "b".to_string(),
                    screened_out_count: 0,
                    regime: None,
                    frontier_class: None,
                    dominance: None,
                    chosen_by: None,
                    takeover_reason: None,
                    frontier_survival: None,
                    exact_survival: None,
                    rejection_reasons: Vec::new(),
                },
                super::DecisionClusterExample {
                    category: "idle_energy_end_turn".to_string(),
                    frame: Some(401),
                    line_number: 12,
                    snippet: "c".to_string(),
                    screened_out_count: 0,
                    regime: None,
                    frontier_class: None,
                    dominance: None,
                    chosen_by: None,
                    takeover_reason: None,
                    frontier_survival: None,
                    exact_survival: None,
                    rejection_reasons: Vec::new(),
                },
            ],
        };

        let selected = collect_export_examples(
            &report,
            &[
                "high_threat_exact_disagree_not_taken".to_string(),
                "idle_energy_end_turn".to_string(),
            ],
            8,
        );
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].frame, Some(307));
        assert_eq!(selected[1].frame, Some(401));
    }

    #[test]
    fn disagreement_category_from_tags_skips_run_tag() {
        let tags = vec![
            "live_comm_disagreement".to_string(),
            "idle_energy_end_turn".to_string(),
            "run:20260421_145156".to_string(),
        ];
        assert_eq!(
            disagreement_category_from_tags(&tags).as_deref(),
            Some("idle_energy_end_turn")
        );
    }

    #[test]
    fn summarize_decision_training_set_counts_categories_and_sources() {
        let records = vec![
            DecisionTrainingExample {
                fixture_name: "a".to_string(),
                fixture_path: "a.fixture.json".to_string(),
                disagreement_category: Some("idle_energy_end_turn".to_string()),
                tags: Vec::new(),
                source: None,
                source_path: None,
                response_id: None,
                frame_id: None,
                observed_command_text: None,
                audit_source: "fixture_rerun".to_string(),
                bot_chosen_action: "EndTurn".to_string(),
                exact_best_action: None,
                preferred_action: "EndTurn".to_string(),
                preferred_action_source: "frontier_self".to_string(),
                needs_exact_trigger_target: false,
                has_strict_disagreement_target: false,
                has_high_threat_target: false,
                has_screening_activity_target: false,
                screened_out_count: 0,
                frontier_self_consistent_target: true,
                regime: Some("contested".to_string()),
                frontier_class: None,
                dominance: None,
                confidence: None,
                takeover_reason: None,
                frontier_survival: None,
                exact_survival: None,
                chosen_by: None,
                legal_moves: 1,
                reduced_legal_moves: 1,
                timed_out: false,
                top_moves: Vec::new(),
                root_pipeline: None,
                decision_trace: None,
                exact_turn_verdict: None,
            },
            DecisionTrainingExample {
                fixture_name: "b".to_string(),
                fixture_path: "b.fixture.json".to_string(),
                disagreement_category: Some("high_threat_exact_disagree_not_taken".to_string()),
                tags: Vec::new(),
                source: None,
                source_path: None,
                response_id: None,
                frame_id: None,
                observed_command_text: None,
                audit_source: "fixture_rerun".to_string(),
                bot_chosen_action: "Play".to_string(),
                exact_best_action: Some("PlayBetter".to_string()),
                preferred_action: "PlayBetter".to_string(),
                preferred_action_source: "exact_turn_strict_better".to_string(),
                needs_exact_trigger_target: true,
                has_strict_disagreement_target: true,
                has_high_threat_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                frontier_self_consistent_target: false,
                regime: Some("fragile".to_string()),
                frontier_class: None,
                dominance: None,
                confidence: None,
                takeover_reason: None,
                frontier_survival: None,
                exact_survival: None,
                chosen_by: None,
                legal_moves: 2,
                reduced_legal_moves: 2,
                timed_out: false,
                top_moves: Vec::new(),
                root_pipeline: None,
                decision_trace: None,
                exact_turn_verdict: None,
            },
        ];
        let summary = summarize_decision_training_set(&records, Path::new("tmp/out.jsonl"));
        assert_eq!(summary.fixture_count, 2);
        assert_eq!(
            summary.category_counts.get("idle_energy_end_turn"),
            Some(&1usize)
        );
        assert_eq!(
            summary
                .preferred_action_source_counts
                .get("exact_turn_strict_better"),
            Some(&1usize)
        );
        assert_eq!(summary.regime_counts.get("fragile"), Some(&1usize));
        assert_eq!(summary.needs_exact_trigger_target_count, 1);
        assert_eq!(summary.high_threat_target_count, 1);
        assert_eq!(summary.strict_disagreement_target_count, 1);
        assert_eq!(summary.screening_activity_target_count, 0);
        assert_eq!(summary.frontier_self_consistent_target_count, 1);
    }

    #[test]
    fn build_proposal_training_set_emits_frontier_and_screened_rows() {
        let record = DecisionTrainingExample {
            fixture_name: "fixture".to_string(),
            fixture_path: "fixture.fixture.json".to_string(),
            disagreement_category: Some("high_threat_exact_disagree_not_taken".to_string()),
            tags: Vec::new(),
            source: None,
            source_path: None,
            response_id: Some(12),
            frame_id: Some(34),
            observed_command_text: None,
            audit_source: "fixture_rerun".to_string(),
            bot_chosen_action: "PlayCard { card_index: 0, target: Some(1) }".to_string(),
            exact_best_action: Some("UsePotion { potion_index: 0, target: None }".to_string()),
            preferred_action: "UsePotion { potion_index: 0, target: None }".to_string(),
            preferred_action_source: "exact_turn_strict_better".to_string(),
            needs_exact_trigger_target: true,
            has_strict_disagreement_target: true,
            has_high_threat_target: true,
            has_screening_activity_target: false,
            screened_out_count: 0,
            frontier_self_consistent_target: false,
            regime: Some("fragile".to_string()),
            frontier_class: Some("attack".to_string()),
            dominance: Some("strictly_better_in_window".to_string()),
            confidence: Some("exact".to_string()),
            takeover_reason: Some("fragile_not_better_survival".to_string()),
            frontier_survival: Some("safe".to_string()),
            exact_survival: Some("safe".to_string()),
            chosen_by: Some("frontier".to_string()),
            legal_moves: 4,
            reduced_legal_moves: 3,
            timed_out: false,
            top_moves: Vec::new(),
            root_pipeline: None,
            decision_trace: Some(serde_json::json!({
                "why_not_others": [
                    {
                        "input": "PlayCard { card_index: 0, target: Some(1) }",
                        "proposal_class": "attack",
                        "disposition": "frontier_chosen",
                        "exact_confidence": "exact",
                        "reasons": [],
                        "frontier_outcome": {"survival": "safe"},
                        "exact_outcome": {"survival": "safe"}
                    },
                    {
                        "input": "UsePotion { potion_index: 0, target: None }",
                        "proposal_class": "potion",
                        "disposition": "considered",
                        "exact_confidence": "exact",
                        "reasons": ["ranked_below_frontier_after_deeper_search"],
                        "frontier_outcome": {"survival": "safe"},
                        "exact_outcome": {"survival": "safe"}
                    }
                ],
                "screened_out": [
                    {
                        "input": "EndTurn",
                        "proposal_class": "end_turn",
                        "reason": "end_turn_worse_than_playable_alternative",
                        "frontier_outcome": {"survival": "risky_but_playable"}
                    }
                ]
            })),
            exact_turn_verdict: None,
        };

        let rows = build_proposal_training_set(&[record]);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().any(|row| row.is_frontier_choice));
        assert!(rows.iter().any(|row| row.is_exact_best));
        assert!(rows.iter().any(|row| row.veto_target));
        assert!(rows.iter().any(|row| {
            row.disposition == "screened_out"
                && row
                    .reasons
                    .iter()
                    .any(|reason| reason == "end_turn_worse_than_playable_alternative")
        }));
    }

    #[test]
    fn summarize_proposal_training_set_counts_dispositions_and_reasons() {
        let rows = vec![
            ProposalTrainingExample {
                fixture_name: "a".to_string(),
                fixture_path: "a.fixture.json".to_string(),
                disagreement_category: Some("idle_energy_end_turn".to_string()),
                response_id: Some(1),
                frame_id: Some(10),
                audit_source: "live_combat_shadow".to_string(),
                regime: Some("contested".to_string()),
                needs_exact_trigger_target: true,
                has_strict_disagreement_target: false,
                has_high_threat_target: false,
                proposal_input: "EndTurn".to_string(),
                proposal_class: Some("end_turn".to_string()),
                disposition: "screened_out".to_string(),
                is_frontier_choice: false,
                is_exact_best: false,
                veto_target: true,
                exact_confidence: Some("unavailable".to_string()),
                reasons: vec!["end_turn_worse_than_playable_alternative".to_string()],
                frontier_outcome: None,
                exact_outcome: None,
            },
            ProposalTrainingExample {
                fixture_name: "b".to_string(),
                fixture_path: "b.fixture.json".to_string(),
                disagreement_category: Some("high_threat_exact_disagree_not_taken".to_string()),
                response_id: Some(2),
                frame_id: Some(20),
                audit_source: "fixture_rerun".to_string(),
                regime: Some("fragile".to_string()),
                needs_exact_trigger_target: true,
                has_strict_disagreement_target: true,
                has_high_threat_target: true,
                proposal_input: "UsePotion".to_string(),
                proposal_class: Some("potion".to_string()),
                disposition: "frontier_chosen".to_string(),
                is_frontier_choice: true,
                is_exact_best: true,
                veto_target: false,
                exact_confidence: Some("exact".to_string()),
                reasons: Vec::new(),
                frontier_outcome: None,
                exact_outcome: None,
            },
        ];
        let summary = summarize_proposal_training_set(&rows, Path::new("tmp/proposals.jsonl"));
        assert_eq!(summary.proposal_count, 2);
        assert_eq!(
            summary.disposition_counts.get("screened_out"),
            Some(&1usize)
        );
        assert_eq!(summary.proposal_class_counts.get("end_turn"), Some(&1usize));
        assert_eq!(
            summary
                .reason_counts
                .get("end_turn_worse_than_playable_alternative"),
            Some(&1usize)
        );
        assert_eq!(summary.veto_target_count, 1);
        assert_eq!(summary.exact_best_count, 1);
        assert_eq!(summary.needs_exact_trigger_target_count, 2);
        assert_eq!(
            summary.audit_source_counts.get("live_combat_shadow"),
            Some(&1usize)
        );
        assert_eq!(
            summary.audit_source_counts.get("fixture_rerun"),
            Some(&1usize)
        );
    }

    #[test]
    fn live_shadow_examples_preserve_screened_out_veto_signal() {
        let example = DecisionClusterExample {
            category: "screening_active".to_string(),
            frame: Some(42),
            line_number: 10,
            snippet: "snippet".to_string(),
            screened_out_count: 1,
            regime: Some("fragile".to_string()),
            frontier_class: Some("attack".to_string()),
            dominance: Some("strictly_better_in_window".to_string()),
            chosen_by: Some("frontier".to_string()),
            takeover_reason: Some("regime_not_takeover".to_string()),
            frontier_survival: Some("severe_risk".to_string()),
            exact_survival: Some("stable".to_string()),
            rejection_reasons: vec!["high_threat_disagreement".to_string()],
        };
        let live_shadow = serde_json::json!({
            "chosen_move": "EndTurn",
            "legal_moves": 4,
            "reduced_legal_moves": 2,
            "top_candidates": [
                {
                    "move_label": "PlayCard { card_index: 0, target: None }",
                    "avg_score": 3.0,
                    "projected_unblocked": 0,
                    "projected_enemy_total": 8,
                    "cluster_size": 1
                }
            ],
            "decision_audit": {
                "regime": "fragile",
                "root_pipeline": {
                    "screened_out": [
                        {
                            "input": "EndTurn",
                            "proposal_class": "end_turn",
                            "reason": "end_turn_worse_than_playable_alternative",
                            "frontier_outcome": { "survival": "severe_risk" }
                        }
                    ]
                },
                "decision_trace": {
                    "frontier_proposal_class": "attack",
                    "chosen_by": "frontier",
                    "rejection_reasons": ["high_threat_disagreement"],
                    "screened_out": [
                        {
                            "input": "EndTurn",
                            "proposal_class": "end_turn",
                            "reason": "end_turn_worse_than_playable_alternative",
                            "frontier_outcome": { "survival": "severe_risk" }
                        }
                    ],
                    "why_not_others": []
                },
                "exact_turn_verdict": {
                    "best_first_input": "PlayCard { card_index: 0, target: None }",
                    "dominance": "strictly_better_in_window",
                    "confidence": "exact",
                    "survival": "stable"
                },
                "exact_turn_shadow": {
                    "timed_out": false
                }
            }
        });

        let record = build_decision_training_example_from_live_shadow(
            "run",
            Path::new("logs/raw.jsonl"),
            None,
            &example,
            Some(7),
            &live_shadow,
        );
        assert_eq!(record.audit_source, "live_combat_shadow");
        assert_eq!(record.screened_out_count, 1);
        let proposal_rows = build_proposal_training_set(&[record]);
        assert_eq!(proposal_rows.len(), 1);
        assert!(proposal_rows[0].veto_target);
        assert_eq!(proposal_rows[0].audit_source, "live_combat_shadow");
        assert_eq!(
            proposal_rows[0].reasons,
            vec!["end_turn_worse_than_playable_alternative".to_string()]
        );
    }

    #[test]
    fn summarize_state_corpus_counts_sources_and_regimes() {
        let records = vec![
            StateCorpusRecord {
                sample_id: "fixture:a".to_string(),
                source_kind: "scenario_fixture".to_string(),
                source_path: "a.json".to_string(),
                fixture_name: Some("a".to_string()),
                combat_case_id: None,
                run_id: None,
                response_id: None,
                frame_id: Some(10),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("fragile".to_string()),
                curriculum_buckets: vec![
                    "elite".to_string(),
                    "regime_fragile".to_string(),
                    "setup_window".to_string(),
                ],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 4,
                reduced_legal_moves: 3,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 42}}),
            },
            StateCorpusRecord {
                sample_id: "raw:b".to_string(),
                source_kind: "live_snapshot".to_string(),
                source_path: "b.jsonl".to_string(),
                fixture_name: Some("b".to_string()),
                combat_case_id: None,
                run_id: Some("20260421_213431".to_string()),
                response_id: Some(20),
                frame_id: Some(30),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "PendingChoice(DiscoverySelect)".to_string(),
                screen_type: Some("HAND_SELECT".to_string()),
                regime: Some("crisis".to_string()),
                curriculum_buckets: vec![
                    "boss".to_string(),
                    "regime_crisis".to_string(),
                    "status_heavy".to_string(),
                ],
                encounter_signature: vec!["BookOfStabbing".to_string()],
                living_monsters: 1,
                legal_moves: 2,
                reduced_legal_moves: 2,
                timed_out: true,
                needs_exact_trigger_target: true,
                has_screening_activity_target: true,
                screened_out_count: 1,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 7}}),
            },
        ];

        let summary = summarize_state_corpus(
            &records,
            Path::new("tmp/state_corpus.jsonl"),
            &Vec::new(),
            &Vec::new(),
            StateCorpusFilterStats {
                candidate_count: 2,
                ..StateCorpusFilterStats::default()
            },
        );
        assert_eq!(summary.candidate_count, 2);
        assert_eq!(summary.sample_count, 2);
        assert_eq!(
            summary.source_kind_counts.get("scenario_fixture"),
            Some(&1usize)
        );
        assert_eq!(
            summary.source_kind_counts.get("live_snapshot"),
            Some(&1usize)
        );
        assert_eq!(summary.regime_counts.get("fragile"), Some(&1usize));
        assert_eq!(summary.regime_counts.get("crisis"), Some(&1usize));
        assert_eq!(summary.curriculum_bucket_counts.get("elite"), Some(&1usize));
        assert_eq!(
            summary.curriculum_bucket_counts.get("status_heavy"),
            Some(&1usize)
        );
        assert_eq!(
            summary.curriculum_bucket_counts.get("setup_window"),
            Some(&1usize)
        );
        assert_eq!(summary.player_class_counts.get("IRONCLAD"), Some(&2usize));
        assert_eq!(summary.screen_type_counts.get("NONE"), Some(&1usize));
        assert_eq!(summary.needs_exact_trigger_target_count, 2);
        assert_eq!(summary.screening_activity_target_count, 1);
        assert!(summary.include_bucket_filters.is_empty());
        assert!(summary.exclude_bucket_filters.is_empty());
        assert_eq!(summary.bucket_filtered_count, 0);
    }

    #[test]
    fn clean_state_corpus_filters_terminal_and_dedups_live_snapshot_under_fixture() {
        let records = vec![
            StateCorpusRecord {
                sample_id: "raw".to_string(),
                source_kind: "live_snapshot".to_string(),
                source_path: "raw.jsonl".to_string(),
                fixture_name: Some("raw_state".to_string()),
                combat_case_id: None,
                run_id: Some("run1".to_string()),
                response_id: Some(10),
                frame_id: Some(20),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("contested".to_string()),
                curriculum_buckets: vec![],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 3,
                reduced_legal_moves: 2,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 30}}),
            },
            StateCorpusRecord {
                sample_id: "fixture".to_string(),
                source_kind: "scenario_fixture".to_string(),
                source_path: "fixture.json".to_string(),
                fixture_name: Some("live_comm_disagreement".to_string()),
                combat_case_id: None,
                run_id: Some("run1".to_string()),
                response_id: Some(10),
                frame_id: Some(20),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("fragile".to_string()),
                curriculum_buckets: vec!["regime_fragile".to_string()],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 4,
                reduced_legal_moves: 3,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 30}}),
            },
            StateCorpusRecord {
                sample_id: "terminal".to_string(),
                source_kind: "live_snapshot".to_string(),
                source_path: "raw.jsonl".to_string(),
                fixture_name: Some("raw_state_terminal".to_string()),
                combat_case_id: None,
                run_id: Some("run1".to_string()),
                response_id: Some(11),
                frame_id: Some(21),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("GAME_OVER".to_string()),
                regime: Some("crisis".to_string()),
                curriculum_buckets: vec!["regime_crisis".to_string()],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 1,
                reduced_legal_moves: 1,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 0}}),
            },
        ];

        let (cleaned, stats) = clean_state_corpus_records(records);
        assert_eq!(stats.candidate_count, 3);
        assert_eq!(stats.duplicate_filtered_count, 1);
        assert_eq!(stats.terminal_filtered_count, 1);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0].source_kind, "scenario_fixture");
        assert_eq!(cleaned[0].regime.as_deref(), Some("fragile"));
    }

    #[test]
    fn filter_state_corpus_by_buckets_applies_include_any_and_exclude_any() {
        let records = vec![
            StateCorpusRecord {
                sample_id: "elite_setup".to_string(),
                source_kind: "scenario_fixture".to_string(),
                source_path: "a.json".to_string(),
                fixture_name: Some("a".to_string()),
                combat_case_id: None,
                run_id: None,
                response_id: None,
                frame_id: Some(1),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("fragile".to_string()),
                curriculum_buckets: vec!["elite".to_string(), "setup_window".to_string()],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 3,
                reduced_legal_moves: 2,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 30}}),
            },
            StateCorpusRecord {
                sample_id: "boss_crisis".to_string(),
                source_kind: "live_snapshot".to_string(),
                source_path: "b.json".to_string(),
                fixture_name: Some("b".to_string()),
                combat_case_id: None,
                run_id: None,
                response_id: None,
                frame_id: Some(2),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("crisis".to_string()),
                curriculum_buckets: vec!["boss".to_string(), "regime_crisis".to_string()],
                encounter_signature: vec!["Hexaghost".to_string()],
                living_monsters: 1,
                legal_moves: 2,
                reduced_legal_moves: 1,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 9}}),
            },
        ];

        let mut stats = StateCorpusFilterStats::default();
        let (filtered, preserved_count) = filter_state_corpus_by_buckets(
            records,
            &vec!["elite".to_string(), "setup_window".to_string()],
            &vec!["regime_crisis".to_string()],
            0,
            &mut stats,
        );
        assert_eq!(preserved_count, 0);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].sample_id, "elite_setup");
        assert_eq!(stats.bucket_filtered_count, 1);
    }

    #[test]
    fn filter_state_corpus_by_buckets_preserves_trigger_negative_rows_outside_include() {
        let records = vec![
            StateCorpusRecord {
                sample_id: "fragile_positive".to_string(),
                source_kind: "scenario_fixture".to_string(),
                source_path: "a.json".to_string(),
                fixture_name: Some("a".to_string()),
                combat_case_id: None,
                run_id: None,
                response_id: None,
                frame_id: Some(1),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("fragile".to_string()),
                curriculum_buckets: vec!["regime_fragile".to_string()],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 3,
                reduced_legal_moves: 2,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 20}}),
            },
            StateCorpusRecord {
                sample_id: "contested_negative".to_string(),
                source_kind: "scenario_fixture".to_string(),
                source_path: "b.json".to_string(),
                fixture_name: Some("b".to_string()),
                combat_case_id: None,
                run_id: None,
                response_id: None,
                frame_id: Some(2),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("contested".to_string()),
                curriculum_buckets: vec!["elite".to_string()],
                encounter_signature: vec!["Lagavulin".to_string()],
                living_monsters: 1,
                legal_moves: 3,
                reduced_legal_moves: 2,
                timed_out: false,
                needs_exact_trigger_target: false,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 45}}),
            },
            StateCorpusRecord {
                sample_id: "crisis_negative_excluded".to_string(),
                source_kind: "scenario_fixture".to_string(),
                source_path: "c.json".to_string(),
                fixture_name: Some("c".to_string()),
                combat_case_id: None,
                run_id: None,
                response_id: None,
                frame_id: Some(3),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("crisis".to_string()),
                curriculum_buckets: vec!["regime_crisis".to_string()],
                encounter_signature: vec!["Hexaghost".to_string()],
                living_monsters: 1,
                legal_moves: 2,
                reduced_legal_moves: 1,
                timed_out: false,
                needs_exact_trigger_target: false,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 4}}),
            },
        ];

        let mut stats = StateCorpusFilterStats::default();
        let (filtered, preserved_count) = filter_state_corpus_by_buckets(
            records,
            &vec!["regime_fragile".to_string()],
            &vec!["regime_crisis".to_string()],
            1,
            &mut stats,
        );

        assert_eq!(preserved_count, 1);
        let kept_ids = filtered
            .iter()
            .map(|record| record.sample_id.as_str())
            .collect::<Vec<_>>();
        assert!(kept_ids.contains(&"fragile_positive"));
        assert!(kept_ids.contains(&"contested_negative"));
        assert!(!kept_ids.contains(&"crisis_negative_excluded"));
    }

    #[test]
    fn split_state_corpus_records_keeps_groups_together() {
        let make_record =
            |sample_id: &str, frame_id: u64, run_id: &str, encounter: &str| StateCorpusRecord {
                sample_id: sample_id.to_string(),
                source_kind: "live_snapshot".to_string(),
                source_path: "raw.jsonl".to_string(),
                fixture_name: None,
                combat_case_id: None,
                run_id: Some(run_id.to_string()),
                response_id: Some(frame_id),
                frame_id: Some(frame_id),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("contested".to_string()),
                curriculum_buckets: vec!["elite".to_string()],
                encounter_signature: vec![encounter.to_string()],
                living_monsters: 1,
                legal_moves: 3,
                reduced_legal_moves: 2,
                timed_out: false,
                needs_exact_trigger_target: true,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 30}}),
            };

        let records = vec![
            make_record("a1", 10, "run1", "Lagavulin"),
            make_record("a2", 11, "run1", "Lagavulin"),
            make_record("b1", 20, "run2", "BookOfStabbing"),
        ];
        let group_a = state_corpus_split_group_key(&records[0]);
        let group_b = state_corpus_split_group_key(&records[2]);
        assert_eq!(group_a, state_corpus_split_group_key(&records[1]));
        assert_ne!(group_a, group_b);

        let (splits, summary) =
            split_state_corpus_records(records, &[], &[], 80, 10, 0).expect("split should succeed");
        assert_eq!(summary.total_records, 3);
        assert_eq!(summary.kept_records, 3);
        assert_eq!(summary.group_count, 2);

        let mut seen_group_a = 0usize;
        for rows in splits.values() {
            let has_a1 = rows.iter().any(|row| row.sample_id == "a1");
            let has_a2 = rows.iter().any(|row| row.sample_id == "a2");
            assert_eq!(has_a1, has_a2);
            if has_a1 {
                seen_group_a += 1;
            }
        }
        assert_eq!(seen_group_a, 1);
    }

    #[test]
    fn split_state_corpus_records_backfills_trigger_negative_into_train() {
        let make_record =
            |sample_id: &str,
             frame_id: u64,
             run_id: &str,
             encounter: &str,
             needs_exact_trigger_target: bool| StateCorpusRecord {
                sample_id: sample_id.to_string(),
                source_kind: "live_snapshot".to_string(),
                source_path: "raw.jsonl".to_string(),
                fixture_name: None,
                combat_case_id: None,
                run_id: Some(run_id.to_string()),
                response_id: Some(frame_id),
                frame_id: Some(frame_id),
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                engine_state: "CombatPlayerTurn".to_string(),
                screen_type: Some("NONE".to_string()),
                regime: Some("contested".to_string()),
                curriculum_buckets: vec!["elite".to_string()],
                encounter_signature: vec![encounter.to_string()],
                living_monsters: 1,
                legal_moves: 3,
                reduced_legal_moves: 2,
                timed_out: false,
                needs_exact_trigger_target,
                has_screening_activity_target: false,
                screened_out_count: 0,
                decision_probe_source: "root_search_runtime".to_string(),
                decision_audit: serde_json::json!({}),
                combat_snapshot: serde_json::json!({"player": {"current_hp": 30}}),
            };

        let mut negative_run = None;
        for idx in 0..512u64 {
            let run_id = format!("neg_run_{idx}");
            let probe = make_record("neg_probe", idx, &run_id, "Lagavulin", false);
            let group_key = state_corpus_split_group_key(&probe);
            if split_name_for_group_key(&group_key, 80, 10) != "train" {
                negative_run = Some(run_id);
                break;
            }
        }
        let negative_run = negative_run.expect("should find a non-train hash bucket");

        let mut positive_train_run = None;
        for idx in 0..512u64 {
            let run_id = format!("pos_run_{idx}");
            let probe = make_record("pos_probe", idx, &run_id, "Cultist", true);
            let group_key = state_corpus_split_group_key(&probe);
            if split_name_for_group_key(&group_key, 80, 10) == "train" {
                positive_train_run = Some(run_id);
                break;
            }
        }
        let positive_train_run = positive_train_run.expect("should find a train hash bucket");

        let records = vec![
            make_record("pos_a1", 10, &positive_train_run, "Cultist", true),
            make_record("pos_a2", 11, &positive_train_run, "Cultist", true),
            make_record("pos_b1", 20, "train_like_b", "JawWorm", true),
            make_record("neg_1", 30, &negative_run, "Lagavulin", false),
        ];

        let (_, summary) =
            split_state_corpus_records(records, &[], &[], 80, 10, 0).expect("split should succeed");
        let train_counts = summary
            .split_trigger_label_counts
            .get("train")
            .expect("train trigger counts should exist");
        assert!(
            *train_counts.get("positive").unwrap_or(&0) >= 1,
            "train should contain trigger-positive rows"
        );
        assert!(
            *train_counts.get("negative").unwrap_or(&0) >= 1,
            "train should contain trigger-negative rows after coverage repair"
        );
        assert!(
            summary
                .trigger_coverage_adjustments
                .iter()
                .any(|entry| entry.contains("trigger-negative")),
            "summary should record the coverage repair"
        );
    }
}
