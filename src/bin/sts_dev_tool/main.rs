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
use sts_simulator::fixtures::combat_case::{lower_case, CombatCase};
use sts_simulator::fixtures::live_capture::build_fixture_from_record_window;
use sts_simulator::fixtures::scenario::{
    initialize_fixture_state, ScenarioFixture, ScenarioKind, ScenarioOracleKind, ScenarioProvenance,
};

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
        /// Policy name: random_masked or rule_baseline_v0.
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

#[derive(Debug, Deserialize)]
struct FindingsValueExample {
    rust: String,
    java: String,
}

#[derive(Debug, Deserialize)]
struct FindingsFamily {
    category: String,
    key: String,
    count: usize,
    first_frame: u64,
    last_frame: u64,
    #[serde(default)]
    example_frames: Vec<u64>,
    #[serde(default)]
    example_snapshot_ids: Vec<String>,
    #[serde(default)]
    example_rust_java_values: Vec<FindingsValueExample>,
    #[serde(default)]
    combat_labels: Vec<String>,
    #[serde(default)]
    event_labels: Vec<String>,
    #[serde(default)]
    suggested_artifacts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FindingsReport {
    run_id: String,
    classification_label: String,
    counts: sts_simulator::cli::live_comm_admin::LiveRunCounts,
    families: Vec<FindingsFamily>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DecisionBotStrengthSummary {
    run_id: String,
    classification_label: String,
    highest_floor: usize,
    highest_act: usize,
    #[serde(default)]
    slow_search_count: usize,
    #[serde(default)]
    search_timeout_count: usize,
    #[serde(default)]
    exact_turn_disagree_count: usize,
    #[serde(default)]
    exact_turn_skip_count: usize,
    #[serde(default)]
    exact_turn_takeover_count: usize,
    #[serde(default)]
    strict_dominance_disagreement_count: usize,
    #[serde(default)]
    high_threat_disagreement_count: usize,
    #[serde(default)]
    regime_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
struct DecisionAuditLine {
    frame: Option<u64>,
    line_number: usize,
    snippet: String,
    skipped: bool,
    agrees: bool,
    screened_out_count: usize,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
    confidence: Option<String>,
    takeover: Option<bool>,
    takeover_reason: Option<String>,
    chosen_by: Option<String>,
    frontier_survival: Option<String>,
    exact_survival: Option<String>,
    alternatives: Option<usize>,
    rejection_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DecisionClusterExample {
    category: String,
    frame: Option<u64>,
    line_number: usize,
    snippet: String,
    screened_out_count: usize,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
    chosen_by: Option<String>,
    takeover_reason: Option<String>,
    frontier_survival: Option<String>,
    exact_survival: Option<String>,
    rejection_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DecisionExperimentReport {
    run_id: String,
    classification_label: String,
    parity_clean: bool,
    debug_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bot_strength: Option<DecisionBotStrengthSummary>,
    category_counts: BTreeMap<String, usize>,
    examples: Vec<DecisionClusterExample>,
}

#[derive(Debug, Serialize)]
struct ExportedDisagreementFixture {
    category: String,
    frame: u64,
    response_id: u64,
    fixture_path: String,
    snippet: String,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportedDisagreementFixtureReport {
    run_id: String,
    classification_label: String,
    debug_path: String,
    raw_path: String,
    window_lookback: usize,
    requested_categories: Vec<String>,
    exported: Vec<ExportedDisagreementFixture>,
    missing_frames: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct DecisionTrainingMoveRecord {
    input: String,
    avg_score: f32,
    visits: u32,
    projected_hp: i32,
    projected_block: i32,
    projected_unblocked: i32,
    projected_enemy_total: i32,
    immediate_incoming: i32,
    cluster_size: usize,
}

#[derive(Debug, Serialize)]
struct DecisionTrainingExample {
    fixture_name: String,
    fixture_path: String,
    disagreement_category: Option<String>,
    tags: Vec<String>,
    source: Option<String>,
    source_path: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    observed_command_text: Option<String>,
    audit_source: String,
    bot_chosen_action: String,
    exact_best_action: Option<String>,
    preferred_action: String,
    preferred_action_source: String,
    needs_exact_trigger_target: bool,
    has_strict_disagreement_target: bool,
    has_high_threat_target: bool,
    has_screening_activity_target: bool,
    screened_out_count: usize,
    frontier_self_consistent_target: bool,
    regime: Option<String>,
    frontier_class: Option<String>,
    dominance: Option<String>,
    confidence: Option<String>,
    takeover_reason: Option<String>,
    frontier_survival: Option<String>,
    exact_survival: Option<String>,
    chosen_by: Option<String>,
    legal_moves: usize,
    reduced_legal_moves: usize,
    timed_out: bool,
    top_moves: Vec<DecisionTrainingMoveRecord>,
    root_pipeline: Option<serde_json::Value>,
    decision_trace: Option<serde_json::Value>,
    exact_turn_verdict: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct DecisionTrainingSetSummary {
    fixture_count: usize,
    out_path: String,
    category_counts: BTreeMap<String, usize>,
    audit_source_counts: BTreeMap<String, usize>,
    preferred_action_source_counts: BTreeMap<String, usize>,
    regime_counts: BTreeMap<String, usize>,
    needs_exact_trigger_target_count: usize,
    high_threat_target_count: usize,
    strict_disagreement_target_count: usize,
    screening_activity_target_count: usize,
    frontier_self_consistent_target_count: usize,
}

#[derive(Debug, Serialize)]
struct ProposalTrainingExample {
    fixture_name: String,
    fixture_path: String,
    disagreement_category: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    audit_source: String,
    regime: Option<String>,
    needs_exact_trigger_target: bool,
    has_strict_disagreement_target: bool,
    has_high_threat_target: bool,
    proposal_input: String,
    proposal_class: Option<String>,
    disposition: String,
    is_frontier_choice: bool,
    is_exact_best: bool,
    veto_target: bool,
    exact_confidence: Option<String>,
    reasons: Vec<String>,
    frontier_outcome: Option<serde_json::Value>,
    exact_outcome: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ProposalTrainingSetSummary {
    proposal_count: usize,
    out_path: String,
    audit_source_counts: BTreeMap<String, usize>,
    disposition_counts: BTreeMap<String, usize>,
    proposal_class_counts: BTreeMap<String, usize>,
    reason_counts: BTreeMap<String, usize>,
    veto_target_count: usize,
    exact_best_count: usize,
    needs_exact_trigger_target_count: usize,
}

#[derive(Debug, Serialize)]
struct DecisionCorpusRunSummary {
    run_id: String,
    classification_label: String,
    exported_fixture_count: usize,
    live_shadow_record_count: usize,
    fixture_rerun_record_count: usize,
    missing_frame_count: usize,
}

#[derive(Debug, Serialize)]
struct DecisionCorpusSummary {
    run_count: usize,
    fixture_count: usize,
    categories: Vec<String>,
    out_dir: String,
    runs: Vec<DecisionCorpusRunSummary>,
    frame_summary: DecisionTrainingSetSummary,
    proposal_summary: ProposalTrainingSetSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StateCorpusRecord {
    sample_id: String,
    source_kind: String,
    source_path: String,
    fixture_name: Option<String>,
    combat_case_id: Option<String>,
    run_id: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    player_class: Option<String>,
    ascension_level: Option<u8>,
    engine_state: String,
    screen_type: Option<String>,
    regime: Option<String>,
    curriculum_buckets: Vec<String>,
    encounter_signature: Vec<String>,
    living_monsters: usize,
    legal_moves: usize,
    reduced_legal_moves: usize,
    timed_out: bool,
    needs_exact_trigger_target: bool,
    has_screening_activity_target: bool,
    screened_out_count: usize,
    decision_probe_source: String,
    decision_audit: serde_json::Value,
    combat_snapshot: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct StateCorpusSummary {
    candidate_count: usize,
    sample_count: usize,
    out_path: String,
    include_bucket_filters: Vec<String>,
    exclude_bucket_filters: Vec<String>,
    source_kind_counts: BTreeMap<String, usize>,
    decision_probe_source_counts: BTreeMap<String, usize>,
    regime_counts: BTreeMap<String, usize>,
    curriculum_bucket_counts: BTreeMap<String, usize>,
    player_class_counts: BTreeMap<String, usize>,
    screen_type_counts: BTreeMap<String, usize>,
    needs_exact_trigger_target_count: usize,
    screening_activity_target_count: usize,
    terminal_filtered_count: usize,
    duplicate_filtered_count: usize,
    bucket_filtered_count: usize,
}

#[derive(Debug, Serialize)]
struct StateCorpusSplitSummary {
    input_path: String,
    out_dir: String,
    include_bucket_filters: Vec<String>,
    exclude_bucket_filters: Vec<String>,
    preserve_trigger_negative_rows: usize,
    total_records: usize,
    kept_records: usize,
    bucket_filtered_count: usize,
    preserved_trigger_negative_count: usize,
    group_count: usize,
    split_counts: BTreeMap<String, usize>,
    split_group_counts: BTreeMap<String, usize>,
    split_trigger_label_counts: BTreeMap<String, BTreeMap<String, usize>>,
    trigger_coverage_adjustments: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct StateCorpusFilterStats {
    candidate_count: usize,
    terminal_filtered_count: usize,
    duplicate_filtered_count: usize,
    bucket_filtered_count: usize,
}

fn artifact_path_for_record(
    manifest_path: &std::path::Path,
    artifact: &Option<sts_simulator::cli::live_comm_admin::LiveArtifactRecord>,
) -> Option<PathBuf> {
    let artifact = artifact.as_ref()?;
    if !artifact.present {
        return None;
    }
    let run_dir = manifest_path.parent()?;
    Some(run_dir.join(&artifact.relative_path))
}

fn manifest_entry_for_run_or_latest(
    paths: &sts_simulator::cli::live_comm_admin::LiveLogPaths,
    run_id: Option<&str>,
    label: Option<&str>,
) -> Option<(
    PathBuf,
    sts_simulator::cli::live_comm_admin::LiveRunManifest,
)> {
    let mut entries =
        sts_simulator::cli::live_comm_admin::list_run_manifests_for_audit(paths).ok()?;
    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));
    for (manifest_path, manifest) in entries {
        if let Some(run_id) = run_id {
            if manifest.run_id != run_id {
                continue;
            }
        } else if let Some(label) = label {
            if manifest.classification_label != label {
                continue;
            }
        }
        return Some((manifest_path, manifest));
    }
    None
}

fn manifest_entries_for_corpus(
    paths: &sts_simulator::cli::live_comm_admin::LiveLogPaths,
    run_ids: &[String],
    label: Option<&str>,
    latest_runs: usize,
) -> Result<
    Vec<(
        PathBuf,
        sts_simulator::cli::live_comm_admin::LiveRunManifest,
    )>,
    String,
> {
    let mut entries = sts_simulator::cli::live_comm_admin::list_run_manifests_for_audit(paths)
        .map_err(|err| format!("failed to list run manifests: {err}"))?;
    entries.sort_by(|left, right| right.1.run_id.cmp(&left.1.run_id));
    if !run_ids.is_empty() {
        let requested = run_ids.iter().collect::<BTreeSet<_>>();
        let mut selected = entries
            .into_iter()
            .filter(|(_, manifest)| requested.contains(&manifest.run_id))
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| left.1.run_id.cmp(&right.1.run_id));
        if selected.is_empty() {
            return Err("no matching run manifests found for requested run_ids".to_string());
        }
        return Ok(selected);
    }
    let mut selected = entries
        .into_iter()
        .filter(|(_, manifest)| {
            label
                .map(|expected| manifest.classification_label == expected)
                .unwrap_or(true)
        })
        .take(latest_runs)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("no matching run manifests found for corpus build".to_string());
    }
    Ok(std::mem::take(&mut selected))
}

fn load_findings_report(path: &PathBuf) -> Result<FindingsReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read findings '{}': {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse findings '{}': {err}", path.display()))
}

fn build_findings_report_from_snapshots(
    manifest_path: &std::path::Path,
    manifest: &sts_simulator::cli::live_comm_admin::LiveRunManifest,
) -> Result<(FindingsReport, PathBuf), String> {
    let snapshots_path =
        artifact_path_for_record(manifest_path, &manifest.artifacts.failure_snapshots).ok_or_else(
            || {
                format!(
                    "run '{}' has neither findings.json nor failure_snapshots.jsonl",
                    manifest.run_id
                )
            },
        )?;
    let report_json = sts_simulator::cli::build_finding_report_json(
        &manifest.run_id,
        &manifest.classification_label,
        &manifest.counts,
        &snapshots_path,
    )?;
    let report: FindingsReport = serde_json::from_value(report_json)
        .map_err(|err| format!("failed to decode synthesized findings report: {err}"))?;
    Ok((report, snapshots_path))
}

fn load_bot_strength_summary(path: &PathBuf) -> Result<DecisionBotStrengthSummary, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read bot_strength '{}': {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse bot_strength '{}': {err}", path.display()))
}

fn audit_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(&rest[..end])
}

fn parse_bool_field(line: &str, key: &str) -> Option<bool> {
    match audit_field(line, key)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_usize_field(line: &str, key: &str) -> Option<usize> {
    audit_field(line, key)?.parse().ok()
}

fn parse_frame_marker(line: &str) -> Option<u64> {
    let rest = line.strip_prefix("[F")?;
    let digits = rest.split_once(']')?.0;
    digits.parse().ok()
}

fn parse_exact_turn_audit_line(
    line_number: usize,
    frame: Option<u64>,
    line: &str,
) -> Option<DecisionAuditLine> {
    if !line.contains("[AUDIT] exact_turn ") {
        return None;
    }
    Some(DecisionAuditLine {
        frame,
        line_number,
        snippet: line.trim().to_string(),
        skipped: parse_bool_field(line, "skipped").unwrap_or(false),
        agrees: parse_bool_field(line, "agrees").unwrap_or(false),
        screened_out_count: parse_usize_field(line, "screened_out").unwrap_or(0),
        regime: audit_field(line, "regime").map(str::to_string),
        frontier_class: audit_field(line, "frontier_class").map(str::to_string),
        dominance: audit_field(line, "dominance").map(str::to_string),
        confidence: audit_field(line, "confidence").map(str::to_string),
        takeover: parse_bool_field(line, "takeover"),
        takeover_reason: audit_field(line, "takeover_reason").map(str::to_string),
        chosen_by: audit_field(line, "chosen_by").map(str::to_string),
        frontier_survival: audit_field(line, "frontier_survival").map(str::to_string),
        exact_survival: audit_field(line, "exact_survival").map(str::to_string),
        alternatives: parse_usize_field(line, "alternatives"),
        rejection_reasons: audit_field(line, "rejection_reasons")
            .unwrap_or_default()
            .split(',')
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect(),
    })
}

fn survival_rank(label: Option<&str>) -> i32 {
    match label.unwrap_or_default() {
        "forced_loss" => 0,
        "severe_risk" => 1,
        "risky_but_playable" => 2,
        "stable" => 3,
        "safe" => 4,
        _ => -1,
    }
}

fn classify_audit_cluster(audit: &DecisionAuditLine) -> Option<&'static str> {
    if audit.skipped {
        return Some("exact_unavailable");
    }
    if audit.agrees {
        return None;
    }
    if audit.frontier_class.as_deref() == Some("end_turn") {
        return Some("idle_energy_end_turn");
    }
    let frontier_rank = survival_rank(audit.frontier_survival.as_deref());
    let exact_rank = survival_rank(audit.exact_survival.as_deref());
    let high_threat = audit
        .rejection_reasons
        .iter()
        .any(|reason| reason == "high_threat_disagreement");
    match audit.dominance.as_deref() {
        Some("strictly_better_in_window") if exact_rank > frontier_rank => {
            Some("survival_upgrade_not_taken")
        }
        Some("strictly_better_in_window") if high_threat => {
            Some("high_threat_exact_disagree_not_taken")
        }
        Some("strictly_better_in_window") => Some("strict_better_same_survival"),
        Some("strictly_worse_in_window") if high_threat => Some("high_threat_frontier_kept"),
        Some("strictly_worse_in_window") => Some("strict_worse_frontier_kept"),
        _ if high_threat => Some("high_threat_other_disagreement"),
        _ => Some("other_disagreement"),
    }
}

fn build_cluster_example(category: &str, audit: &DecisionAuditLine) -> DecisionClusterExample {
    DecisionClusterExample {
        category: category.to_string(),
        frame: audit.frame,
        line_number: audit.line_number,
        snippet: audit.snippet.clone(),
        screened_out_count: audit.screened_out_count,
        regime: audit.regime.clone(),
        frontier_class: audit.frontier_class.clone(),
        dominance: audit.dominance.clone(),
        chosen_by: audit.chosen_by.clone(),
        takeover_reason: audit.takeover_reason.clone(),
        frontier_survival: audit.frontier_survival.clone(),
        exact_survival: audit.exact_survival.clone(),
        rejection_reasons: audit.rejection_reasons.clone(),
    }
}

fn screening_cluster_category(audit: &DecisionAuditLine) -> Option<&'static str> {
    if audit.screened_out_count == 0 {
        return None;
    }
    if audit.skipped {
        Some("screening_active_exact_unavailable")
    } else {
        Some("screening_active")
    }
}

fn parse_idle_end_turn_examples(
    lines: &[String],
    last_audit: &mut Option<DecisionAuditLine>,
) -> Vec<DecisionClusterExample> {
    let mut examples = Vec::new();
    let mut current_frame = None;
    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;
        if let Some(frame) = parse_frame_marker(line) {
            current_frame = Some(frame);
        }
        if let Some(audit) = parse_exact_turn_audit_line(line_number, current_frame, line) {
            *last_audit = Some(audit);
            continue;
        }
        if !line.contains("[END DIAG] END") {
            continue;
        }
        let legal_plays = parse_usize_field(line, "legal_plays").unwrap_or(0);
        if legal_plays == 0 {
            continue;
        }
        let Some(context) = last_audit.as_ref() else {
            continue;
        };
        let mut snippet = vec![line.trim().to_string()];
        for follow in lines.iter().skip(idx + 1).take(6) {
            if !follow.contains("[END DIAG]") {
                break;
            }
            snippet.push(follow.trim().to_string());
        }
        let mut example = build_cluster_example("idle_energy_end_turn", context);
        example.line_number = line_number;
        example.snippet = snippet.join(" | ");
        if !example
            .rejection_reasons
            .iter()
            .any(|reason| reason == "end_diag_kept_end_turn")
        {
            example
                .rejection_reasons
                .push("end_diag_kept_end_turn".to_string());
        }
        examples.push(example);
    }
    examples
}

fn analyze_decision_debug(
    debug_path: &PathBuf,
    run_id: &str,
    classification_label: &str,
    parity_clean: bool,
    bot_strength: Option<DecisionBotStrengthSummary>,
) -> Result<DecisionExperimentReport, String> {
    let text = std::fs::read_to_string(debug_path)
        .map_err(|err| format!("failed to read debug '{}': {err}", debug_path.display()))?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    let mut category_counts = BTreeMap::new();
    let mut examples = Vec::new();
    let mut last_audit = None;
    let mut current_frame = None;

    for (idx, line) in lines.iter().enumerate() {
        if let Some(frame) = parse_frame_marker(line) {
            current_frame = Some(frame);
        }
        let Some(audit) = parse_exact_turn_audit_line(idx + 1, current_frame, line) else {
            continue;
        };
        if let Some(category) = classify_audit_cluster(&audit) {
            *category_counts.entry(category.to_string()).or_insert(0) += 1;
            examples.push(build_cluster_example(category, &audit));
        }
        if let Some(category) = screening_cluster_category(&audit) {
            *category_counts.entry(category.to_string()).or_insert(0) += 1;
            examples.push(build_cluster_example(category, &audit));
        }
        last_audit = Some(audit);
    }

    for example in parse_idle_end_turn_examples(&lines, &mut last_audit) {
        *category_counts.entry(example.category.clone()).or_insert(0) += 1;
        examples.push(example);
    }

    examples.sort_by(|left, right| {
        category_counts
            .get(&right.category)
            .unwrap_or(&0)
            .cmp(category_counts.get(&left.category).unwrap_or(&0))
            .then_with(|| left.line_number.cmp(&right.line_number))
    });

    Ok(DecisionExperimentReport {
        run_id: run_id.to_string(),
        classification_label: classification_label.to_string(),
        parity_clean,
        debug_path: debug_path.display().to_string(),
        bot_strength,
        category_counts,
        examples,
    })
}

fn load_raw_records_by_response_id(
    raw_path: &Path,
) -> Result<BTreeMap<i64, serde_json::Value>, String> {
    let text = std::fs::read_to_string(raw_path)
        .map_err(|err| format!("failed to read raw log '{}': {err}", raw_path.display()))?;
    let mut records = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let root: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse raw log '{}' line {}: {err}",
                raw_path.display(),
                idx + 1
            )
        })?;
        let response_id = root
            .get("protocol_meta")
            .and_then(|meta| meta.get("response_id"))
            .and_then(|value| value.as_i64())
            .ok_or_else(|| {
                format!(
                    "raw log '{}' line {} is missing protocol_meta.response_id",
                    raw_path.display(),
                    idx + 1
                )
            })?;
        records.insert(response_id, root);
    }
    Ok(records)
}

fn load_combat_shadow_records_by_frame(
    path: &Path,
) -> Result<BTreeMap<u64, serde_json::Value>, String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read combat shadow log '{}': {err}",
            path.display()
        )
    })?;
    let mut records = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let root: serde_json::Value = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse combat shadow log '{}' line {}: {err}",
                path.display(),
                idx + 1
            )
        })?;
        if root.get("kind").and_then(|value| value.as_str()) != Some("combat_shadow") {
            continue;
        }
        let Some(frame) = root.get("frame").and_then(|value| value.as_u64()) else {
            continue;
        };
        records.insert(frame, root);
    }
    Ok(records)
}

fn live_combat_shadow_path_for_manifest(
    manifest_path: &Path,
    manifest: &sts_simulator::cli::live_comm_admin::LiveRunManifest,
) -> Option<PathBuf> {
    artifact_path_for_record(manifest_path, &manifest.artifacts.combat_decision_audit)
        .or_else(|| artifact_path_for_record(manifest_path, &manifest.artifacts.sidecar_shadow))
}

fn cluster_example_json(example: &DecisionClusterExample) -> serde_json::Value {
    serde_json::json!({
        "category": example.category,
        "frame": example.frame,
        "line_number": example.line_number,
        "snippet": example.snippet,
        "screened_out_count": example.screened_out_count,
        "regime": example.regime,
        "frontier_class": example.frontier_class,
        "dominance": example.dominance,
        "chosen_by": example.chosen_by,
        "takeover_reason": example.takeover_reason,
        "frontier_survival": example.frontier_survival,
        "exact_survival": example.exact_survival,
        "rejection_reasons": example.rejection_reasons,
    })
}

fn response_id_for_frame(records: &BTreeMap<i64, serde_json::Value>, frame: u64) -> Option<i64> {
    records.iter().find_map(|(response_id, root)| {
        let state_frame = root
            .get("protocol_meta")
            .and_then(|meta| meta.get("state_frame_id"))
            .and_then(|value| value.as_u64())?;
        (state_frame == frame).then_some(*response_id)
    })
}

fn write_scenario_fixture_path(fixture: &ScenarioFixture, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create fixture directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(fixture).map_err(|err| {
        format!(
            "failed to serialize scenario fixture '{}': {err}",
            path.display()
        )
    })?;
    std::fs::write(path, text).map_err(|err| {
        format!(
            "failed to write scenario fixture '{}': {err}",
            path.display()
        )
    })
}

fn sanitize_category_for_filename(category: &str) -> String {
    category
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}

fn collect_export_examples<'a>(
    report: &'a DecisionExperimentReport,
    categories: &[String],
    limit: usize,
) -> Vec<&'a DecisionClusterExample> {
    let category_filter = if categories.is_empty() {
        None
    } else {
        Some(
            categories
                .iter()
                .map(|entry| entry.to_ascii_lowercase())
                .collect::<BTreeSet<_>>(),
        )
    };
    let mut seen_frames = BTreeSet::new();
    let mut selected = Vec::new();
    for example in &report.examples {
        let Some(frame) = example.frame else {
            continue;
        };
        if category_filter
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(&example.category.to_ascii_lowercase()))
        {
            continue;
        }
        if !seen_frames.insert(frame) {
            continue;
        }
        selected.push(example);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn export_disagreement_fixtures(
    raw_path: &Path,
    report: &DecisionExperimentReport,
    combat_shadows_by_frame: Option<&BTreeMap<u64, serde_json::Value>>,
    categories: &[String],
    limit: usize,
    window_lookback: usize,
    out_dir: &Path,
) -> Result<ExportedDisagreementFixtureReport, String> {
    let records = load_raw_records_by_response_id(raw_path)?;
    let mut exported = Vec::new();
    let mut missing_frames = Vec::new();
    for example in collect_export_examples(report, categories, limit) {
        let Some(frame) = example.frame else {
            continue;
        };
        let Some(response_id) = response_id_for_frame(&records, frame) else {
            missing_frames.push(frame);
            continue;
        };
        let start_response_id = std::cmp::max(1_i64, response_id - window_lookback as i64);
        let fixture_name = format!(
            "live_comm_disagreement_{}_f{}",
            sanitize_category_for_filename(&example.category),
            frame
        );
        let mut debug_context_summary = serde_json::json!({
            "live_cluster": cluster_example_json(example),
        });
        if let Some(shadow) = combat_shadows_by_frame
            .and_then(|entries| entries.get(&frame))
            .cloned()
        {
            debug_context_summary
                .as_object_mut()
                .expect("debug_context_summary should be an object")
                .insert("live_combat_shadow".to_string(), shadow);
        }
        let provenance = Some(ScenarioProvenance {
            source: Some("live_comm_disagreement_export".to_string()),
            source_path: Some(raw_path.display().to_string()),
            response_id_range: Some((start_response_id as u64, response_id as u64)),
            failure_frame: Some(frame),
            assertion_source_frames: vec![frame],
            assertion_source_response_ids: vec![response_id as u64],
            debug_context_summary: Some(debug_context_summary),
            notes: vec![format!(
                "exported from decision category '{}' at debug line {}",
                example.category, example.line_number
            )],
            ..ScenarioProvenance::default()
        });
        let fixture = build_fixture_from_record_window(
            &records,
            start_response_id,
            response_id,
            fixture_name.clone(),
            Vec::new(),
            vec![
                "live_comm_disagreement".to_string(),
                example.category.clone(),
                format!("run:{}", report.run_id),
            ],
            provenance,
        )?;
        let output_path = out_dir.join(format!(
            "{}_f{}_{}.fixture.json",
            report.run_id,
            frame,
            sanitize_category_for_filename(&example.category)
        ));
        write_scenario_fixture_path(&fixture, &output_path)?;
        exported.push(ExportedDisagreementFixture {
            category: example.category.clone(),
            frame,
            response_id: response_id as u64,
            fixture_path: output_path.display().to_string(),
            snippet: example.snippet.clone(),
            regime: example.regime.clone(),
            frontier_class: example.frontier_class.clone(),
            dominance: example.dominance.clone(),
        });
    }
    missing_frames.sort_unstable();
    missing_frames.dedup();
    Ok(ExportedDisagreementFixtureReport {
        run_id: report.run_id.clone(),
        classification_label: report.classification_label.clone(),
        debug_path: report.debug_path.clone(),
        raw_path: raw_path.display().to_string(),
        window_lookback,
        requested_categories: categories.to_vec(),
        exported,
        missing_frames,
    })
}

fn render_decision_experiment_report(report: &DecisionExperimentReport, limit: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "run={} classification={} parity_clean={} debug={}\n",
        report.run_id, report.classification_label, report.parity_clean, report.debug_path
    ));
    if let Some(bot_strength) = report.bot_strength.as_ref() {
        out.push_str(&format!(
            "progression: floor={} act={} exact_turn_disagree={} strict_dominance={} high_threat={} takeovers={} timeouts={}\n",
            bot_strength.highest_floor,
            bot_strength.highest_act,
            bot_strength.exact_turn_disagree_count,
            bot_strength.strict_dominance_disagreement_count,
            bot_strength.high_threat_disagreement_count,
            bot_strength.exact_turn_takeover_count,
            bot_strength.search_timeout_count
        ));
        if !bot_strength.regime_counts.is_empty() {
            let regimes = bot_strength
                .regime_counts
                .iter()
                .map(|(regime, count)| format!("{regime}={count}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("regimes: {regimes}\n"));
        }
    }
    out.push_str("categories:\n");
    let mut counts = report.category_counts.iter().collect::<Vec<_>>();
    counts.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    for (category, count) in counts {
        out.push_str(&format!("- {category}={count}\n"));
    }
    out.push_str("examples:\n");
    for example in report.examples.iter().take(limit) {
        let reasons = if example.rejection_reasons.is_empty() {
            "-".to_string()
        } else {
            example.rejection_reasons.join(",")
        };
        out.push_str(&format!(
            "- [{}] line={} regime={} frontier_class={} dominance={} chosen_by={} takeover_reason={} survival={}->{} reasons={} :: {}\n",
            example.category,
            example.line_number,
            example.regime.as_deref().unwrap_or("-"),
            example.frontier_class.as_deref().unwrap_or("-"),
            example.dominance.as_deref().unwrap_or("-"),
            example.chosen_by.as_deref().unwrap_or("-"),
            example.takeover_reason.as_deref().unwrap_or("-"),
            example.frontier_survival.as_deref().unwrap_or("-"),
            example.exact_survival.as_deref().unwrap_or("-"),
            reasons,
            example.snippet
        ));
    }
    out
}

fn render_exported_disagreement_fixture_report(
    report: &ExportedDisagreementFixtureReport,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "run={} classification={} raw={} debug={} exported={} missing_frames={}\n",
        report.run_id,
        report.classification_label,
        report.raw_path,
        report.debug_path,
        report.exported.len(),
        report.missing_frames.len()
    ));
    if !report.requested_categories.is_empty() {
        out.push_str(&format!(
            "categories={}\n",
            report.requested_categories.join(",")
        ));
    }
    out.push_str(&format!("window_lookback={}\n", report.window_lookback));
    for export in &report.exported {
        out.push_str(&format!(
            "- [{}] frame={} response_id={} regime={} frontier_class={} dominance={} fixture={}\n",
            export.category,
            export.frame,
            export.response_id,
            export.regime.as_deref().unwrap_or("-"),
            export.frontier_class.as_deref().unwrap_or("-"),
            export.dominance.as_deref().unwrap_or("-"),
            export.fixture_path
        ));
    }
    if !report.missing_frames.is_empty() {
        out.push_str(&format!(
            "missing_frames={}\n",
            report
                .missing_frames
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    out
}

fn collect_fixture_paths(
    fixtures: &[PathBuf],
    fixture_dirs: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    let mut paths = BTreeSet::new();
    for path in fixtures {
        if path.is_file() {
            paths.insert(path.clone());
        } else {
            return Err(format!("fixture path '{}' is not a file", path.display()));
        }
    }
    for dir in fixture_dirs {
        let entries = std::fs::read_dir(dir)
            .map_err(|err| format!("failed to read fixture dir '{}': {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "failed to read fixture dir entry '{}': {err}",
                    dir.display()
                )
            })?;
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".fixture.json"))
            {
                paths.insert(path);
            }
        }
    }
    if paths.is_empty() {
        return Err("no fixture paths found".to_string());
    }
    Ok(paths.into_iter().collect())
}

fn disagreement_category_from_tags(tags: &[String]) -> Option<String> {
    tags.iter()
        .find(|tag| *tag != "live_comm_disagreement" && !tag.starts_with("run:"))
        .cloned()
}

fn move_record_from_stat(
    stat: &sts_simulator::bot::combat::CombatMoveStat,
) -> DecisionTrainingMoveRecord {
    DecisionTrainingMoveRecord {
        input: format!("{:?}", stat.input),
        avg_score: stat.avg_score,
        visits: stat.visits,
        projected_hp: stat.projected_hp,
        projected_block: stat.projected_block,
        projected_unblocked: stat.projected_unblocked,
        projected_enemy_total: stat.projected_enemy_total,
        immediate_incoming: stat.immediate_incoming,
        cluster_size: stat.cluster_size,
    }
}

fn move_record_from_live_top_candidate(value: &serde_json::Value) -> DecisionTrainingMoveRecord {
    DecisionTrainingMoveRecord {
        input: json_string_field(value, "move_label").unwrap_or_default(),
        avg_score: value
            .get("avg_score")
            .and_then(|inner| inner.as_f64())
            .unwrap_or_default() as f32,
        visits: 0,
        projected_hp: 0,
        projected_block: 0,
        projected_unblocked: value
            .get("projected_unblocked")
            .and_then(|inner| inner.as_i64())
            .unwrap_or_default() as i32,
        projected_enemy_total: value
            .get("projected_enemy_total")
            .and_then(|inner| inner.as_i64())
            .unwrap_or_default() as i32,
        immediate_incoming: 0,
        cluster_size: value
            .get("cluster_size")
            .and_then(|inner| inner.as_u64())
            .unwrap_or_default() as usize,
    }
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|inner| inner.as_str())
        .map(str::to_string)
}

fn json_string_vec_field(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|inner| inner.as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

fn json_bool_field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(|inner| inner.as_bool())
}

fn build_decision_training_example_from_live_shadow(
    run_id: &str,
    raw_path: &Path,
    fixture_path: Option<&Path>,
    example: &DecisionClusterExample,
    response_id: Option<u64>,
    live_shadow: &serde_json::Value,
) -> DecisionTrainingExample {
    let audit = live_shadow
        .get("decision_audit")
        .unwrap_or(&serde_json::Value::Null);
    let exact_turn_verdict = audit.get("exact_turn_verdict").cloned();
    let decision_trace = audit.get("decision_trace").cloned();
    let root_pipeline = audit.get("root_pipeline").cloned();
    let exact_best_action = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("best_first_input"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let dominance = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("dominance"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.dominance.clone());
    let confidence = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("confidence"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let regime = audit
        .get("regime")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.regime.clone());
    let frontier_class = decision_trace
        .as_ref()
        .and_then(|value| value.get("frontier_proposal_class"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.frontier_class.clone());
    let chosen_by = decision_trace
        .as_ref()
        .and_then(|value| value.get("chosen_by"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.chosen_by.clone());
    let takeover_reason = audit
        .get("takeover_policy")
        .and_then(|value| value.get("takeover_reason"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.takeover_reason.clone());
    let frontier_survival = decision_trace
        .as_ref()
        .and_then(|value| value.get("decision_outcomes"))
        .and_then(|value| value.get("frontier"))
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.frontier_survival.clone());
    let exact_survival = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| example.exact_survival.clone());
    let screened_out_count = root_pipeline
        .as_ref()
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .unwrap_or(example.screened_out_count);
    let bot_chosen_action = live_shadow
        .get("chosen_move")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let observed_command_text = (!bot_chosen_action.is_empty()).then(|| bot_chosen_action.clone());
    let rejection_reasons = decision_trace
        .as_ref()
        .map(|value| json_string_vec_field(value, "rejection_reasons"))
        .filter(|reasons| !reasons.is_empty())
        .unwrap_or_else(|| example.rejection_reasons.clone());
    let disagreement_category = Some(example.category.clone());
    let has_strict_disagreement_target = matches!(
        dominance.as_deref(),
        Some("strictly_better_in_window" | "strictly_worse_in_window")
    );
    let has_high_threat_target = disagreement_category
        .as_deref()
        .map(|category| category.starts_with("high_threat_"))
        .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "high_threat_disagreement");
    let has_screening_activity_target = screened_out_count > 0;
    let needs_exact_trigger_target = has_high_threat_target
        || has_strict_disagreement_target
        || matches!(regime.as_deref(), Some("fragile" | "crisis"));
    let (preferred_action, preferred_action_source) =
        if matches!(dominance.as_deref(), Some("strictly_better_in_window"))
            && !matches!(confidence.as_deref(), Some("unavailable"))
        {
            (
                exact_best_action
                    .clone()
                    .unwrap_or_else(|| bot_chosen_action.clone()),
                "exact_turn_strict_better".to_string(),
            )
        } else if let Some(observed) = observed_command_text.clone() {
            (observed, "observed_command".to_string())
        } else {
            (bot_chosen_action.clone(), "frontier_self".to_string())
        };
    let frontier_self_consistent_target = matches!(dominance.as_deref(), Some("incomparable"))
        || exact_best_action
            .as_deref()
            .map(|action| action == bot_chosen_action)
            .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "frontier_agrees");

    DecisionTrainingExample {
        fixture_name: format!(
            "{}_f{}_{}",
            run_id,
            example.frame.unwrap_or_default(),
            sanitize_category_for_filename(&example.category)
        ),
        fixture_path: fixture_path
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        disagreement_category,
        tags: vec![
            "live_comm_disagreement".to_string(),
            example.category.clone(),
            format!("run:{run_id}"),
        ],
        source: Some("live_combat_shadow".to_string()),
        source_path: Some(raw_path.display().to_string()),
        response_id,
        frame_id: example.frame,
        observed_command_text,
        audit_source: "live_combat_shadow".to_string(),
        bot_chosen_action,
        exact_best_action,
        preferred_action,
        preferred_action_source,
        needs_exact_trigger_target,
        has_strict_disagreement_target,
        has_high_threat_target,
        has_screening_activity_target,
        screened_out_count,
        frontier_self_consistent_target,
        regime,
        frontier_class,
        dominance,
        confidence,
        takeover_reason,
        frontier_survival,
        exact_survival,
        chosen_by,
        legal_moves: live_shadow
            .get("legal_moves")
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as usize,
        reduced_legal_moves: live_shadow
            .get("reduced_legal_moves")
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as usize,
        timed_out: audit
            .get("exact_turn_shadow")
            .and_then(|value| json_bool_field(value, "timed_out"))
            .unwrap_or(false),
        top_moves: live_shadow
            .get("top_candidates")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .map(move_record_from_live_top_candidate)
            .collect(),
        root_pipeline,
        decision_trace,
        exact_turn_verdict,
    }
}

fn build_decision_training_example(
    fixture_path: &Path,
    fixture: &ScenarioFixture,
    depth: u32,
) -> Result<DecisionTrainingExample, String> {
    let initial = initialize_fixture_state(fixture);
    let diagnostics = diagnose_root_search_with_depth_and_runtime(
        &initial.engine_state,
        &initial.combat,
        depth,
        0,
        SearchRuntimeBudget::default(),
    );
    let live_shadow = fixture
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.debug_context_summary.as_ref())
        .and_then(|value| value.get("live_combat_shadow"))
        .filter(|value| !value.is_null());
    let live_cluster = fixture
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.debug_context_summary.as_ref())
        .and_then(|value| value.get("live_cluster"));
    let audit = live_shadow
        .and_then(|value| value.get("decision_audit"))
        .unwrap_or(&diagnostics.decision_audit);
    let exact_turn_verdict = audit.get("exact_turn_verdict").cloned();
    let decision_trace = audit.get("decision_trace").cloned();
    let root_pipeline = audit.get("root_pipeline").cloned();
    let exact_best_action = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("best_first_input"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let dominance = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("dominance"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let confidence = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("confidence"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let regime = audit
        .get("regime")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| json_string_field(live_cluster.unwrap_or(&serde_json::Value::Null), "regime"));
    let frontier_class = decision_trace
        .as_ref()
        .and_then(|value| value.get("frontier_proposal_class"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "frontier_class",
            )
        });
    let chosen_by = decision_trace
        .as_ref()
        .and_then(|value| value.get("chosen_by"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "chosen_by",
            )
        });
    let takeover_reason = audit
        .get("takeover_policy")
        .and_then(|value| value.get("takeover_reason"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "takeover_reason",
            )
        });
    let frontier_survival = decision_trace
        .as_ref()
        .and_then(|value| value.get("decision_outcomes"))
        .and_then(|value| value.get("frontier"))
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "frontier_survival",
            )
        });
    let exact_survival = exact_turn_verdict
        .as_ref()
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_string_field(
                live_cluster.unwrap_or(&serde_json::Value::Null),
                "exact_survival",
            )
        });
    let screened_out_count = root_pipeline
        .as_ref()
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .or_else(|| {
            live_cluster
                .and_then(|value| value.get("screened_out_count"))
                .and_then(|value| value.as_u64())
                .map(|value| value as usize)
        })
        .unwrap_or(0);
    let observed_command_text = fixture.steps.first().map(|step| step.command.clone());
    let bot_chosen_action = live_shadow
        .and_then(|value| value.get("chosen_move"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{:?}", diagnostics.chosen_move));
    let rejection_reasons = decision_trace
        .as_ref()
        .map(|value| json_string_vec_field(value, "rejection_reasons"))
        .filter(|reasons| !reasons.is_empty())
        .or_else(|| live_cluster.map(|value| json_string_vec_field(value, "rejection_reasons")))
        .unwrap_or_default();
    let disagreement_category = disagreement_category_from_tags(&fixture.tags);
    let has_strict_disagreement_target = matches!(
        dominance.as_deref(),
        Some("strictly_better_in_window" | "strictly_worse_in_window")
    );
    let has_high_threat_target = disagreement_category
        .as_deref()
        .map(|category| category.starts_with("high_threat_"))
        .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "high_threat_disagreement");
    let has_screening_activity_target = screened_out_count > 0;
    let needs_exact_trigger_target = has_high_threat_target
        || has_strict_disagreement_target
        || matches!(regime.as_deref(), Some("fragile" | "crisis"));
    let (preferred_action, preferred_action_source) =
        if matches!(dominance.as_deref(), Some("strictly_better_in_window"))
            && !matches!(confidence.as_deref(), Some("unavailable"))
        {
            (
                exact_best_action
                    .clone()
                    .unwrap_or_else(|| bot_chosen_action.clone()),
                "exact_turn_strict_better".to_string(),
            )
        } else if let Some(observed) = observed_command_text.clone() {
            (observed, "observed_command".to_string())
        } else {
            (bot_chosen_action.clone(), "frontier_self".to_string())
        };
    let frontier_self_consistent_target = matches!(dominance.as_deref(), Some("incomparable"))
        || exact_best_action
            .as_deref()
            .map(|action| action == bot_chosen_action)
            .unwrap_or(false)
        || rejection_reasons
            .iter()
            .any(|reason| reason == "frontier_agrees");

    Ok(DecisionTrainingExample {
        fixture_name: fixture.name.clone(),
        fixture_path: fixture_path.display().to_string(),
        disagreement_category,
        tags: fixture.tags.clone(),
        source: fixture
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.source.clone()),
        source_path: fixture
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.source_path.clone()),
        response_id: initial.response_id,
        frame_id: initial.frame_id,
        observed_command_text,
        audit_source: if live_shadow.is_some() {
            "live_combat_shadow".to_string()
        } else if live_cluster.is_some() {
            "fixture_live_cluster".to_string()
        } else {
            "fixture_rerun".to_string()
        },
        bot_chosen_action,
        exact_best_action,
        preferred_action,
        preferred_action_source,
        needs_exact_trigger_target,
        has_strict_disagreement_target,
        has_high_threat_target,
        has_screening_activity_target,
        screened_out_count,
        frontier_self_consistent_target,
        regime,
        frontier_class,
        dominance,
        confidence,
        takeover_reason,
        frontier_survival,
        exact_survival,
        chosen_by,
        legal_moves: live_shadow
            .and_then(|value| value.get("legal_moves"))
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(diagnostics.legal_moves),
        reduced_legal_moves: live_shadow
            .and_then(|value| value.get("reduced_legal_moves"))
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
            .unwrap_or(diagnostics.reduced_legal_moves),
        timed_out: diagnostics.timed_out,
        top_moves: diagnostics
            .top_moves
            .iter()
            .map(move_record_from_stat)
            .collect(),
        root_pipeline,
        decision_trace,
        exact_turn_verdict,
    })
}

fn build_decision_training_set(
    fixture_paths: &[PathBuf],
    depth: u32,
) -> Result<Vec<DecisionTrainingExample>, String> {
    fixture_paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path)
                .map_err(|err| format!("failed to read fixture '{}': {err}", path.display()))?;
            let fixture: ScenarioFixture = serde_json::from_str(&text)
                .map_err(|err| format!("failed to parse fixture '{}': {err}", path.display()))?;
            build_decision_training_example(path, &fixture, depth)
        })
        .collect()
}

fn summarize_decision_training_set(
    records: &[DecisionTrainingExample],
    out: &Path,
) -> DecisionTrainingSetSummary {
    let mut category_counts = BTreeMap::new();
    let mut audit_source_counts = BTreeMap::new();
    let mut preferred_action_source_counts = BTreeMap::new();
    let mut regime_counts = BTreeMap::new();
    let mut needs_exact_trigger_target_count = 0usize;
    let mut high_threat_target_count = 0usize;
    let mut strict_disagreement_target_count = 0usize;
    let mut screening_activity_target_count = 0usize;
    let mut frontier_self_consistent_target_count = 0usize;
    for record in records {
        if let Some(category) = record.disagreement_category.as_ref() {
            *category_counts.entry(category.clone()).or_insert(0) += 1;
        }
        *audit_source_counts
            .entry(record.audit_source.clone())
            .or_insert(0) += 1;
        *preferred_action_source_counts
            .entry(record.preferred_action_source.clone())
            .or_insert(0) += 1;
        if let Some(regime) = record.regime.as_ref() {
            *regime_counts.entry(regime.clone()).or_insert(0) += 1;
        }
        needs_exact_trigger_target_count += usize::from(record.needs_exact_trigger_target);
        high_threat_target_count += usize::from(record.has_high_threat_target);
        strict_disagreement_target_count += usize::from(record.has_strict_disagreement_target);
        screening_activity_target_count += usize::from(record.has_screening_activity_target);
        frontier_self_consistent_target_count +=
            usize::from(record.frontier_self_consistent_target);
    }
    DecisionTrainingSetSummary {
        fixture_count: records.len(),
        out_path: out.display().to_string(),
        category_counts,
        audit_source_counts,
        preferred_action_source_counts,
        regime_counts,
        needs_exact_trigger_target_count,
        high_threat_target_count,
        strict_disagreement_target_count,
        screening_activity_target_count,
        frontier_self_consistent_target_count,
    }
}

fn build_proposal_training_set(
    records: &[DecisionTrainingExample],
) -> Vec<ProposalTrainingExample> {
    let mut proposals = Vec::new();
    for record in records {
        let Some(decision_trace) = record.decision_trace.as_ref() else {
            continue;
        };
        let mut seen_screened_out = BTreeSet::new();
        if let Some(why_not_others) = decision_trace
            .get("why_not_others")
            .and_then(|value| value.as_array())
        {
            for proposal in why_not_others {
                let proposal_input = json_string_field(proposal, "input").unwrap_or_default();
                let proposal_class = json_string_field(proposal, "proposal_class");
                let disposition = json_string_field(proposal, "disposition")
                    .unwrap_or_else(|| "considered".to_string());
                let exact_confidence = json_string_field(proposal, "exact_confidence");
                let reasons = json_string_vec_field(proposal, "reasons");
                proposals.push(ProposalTrainingExample {
                    fixture_name: record.fixture_name.clone(),
                    fixture_path: record.fixture_path.clone(),
                    disagreement_category: record.disagreement_category.clone(),
                    response_id: record.response_id,
                    frame_id: record.frame_id,
                    audit_source: record.audit_source.clone(),
                    regime: record.regime.clone(),
                    needs_exact_trigger_target: record.needs_exact_trigger_target,
                    has_strict_disagreement_target: record.has_strict_disagreement_target,
                    has_high_threat_target: record.has_high_threat_target,
                    proposal_input: proposal_input.clone(),
                    proposal_class,
                    disposition: disposition.clone(),
                    is_frontier_choice: disposition == "frontier_chosen"
                        || proposal_input == record.bot_chosen_action,
                    is_exact_best: record
                        .exact_best_action
                        .as_deref()
                        .map(|action| action == proposal_input)
                        .unwrap_or(false),
                    veto_target: disposition == "screened_out",
                    exact_confidence,
                    reasons,
                    frontier_outcome: proposal.get("frontier_outcome").cloned(),
                    exact_outcome: proposal.get("exact_outcome").cloned(),
                });
            }
        }
        if let Some(screened_out) = decision_trace
            .get("screened_out")
            .and_then(|value| value.as_array())
        {
            for proposal in screened_out {
                let proposal_input = json_string_field(proposal, "input").unwrap_or_default();
                if !seen_screened_out.insert(proposal_input.clone()) {
                    continue;
                }
                let reason = json_string_field(proposal, "reason")
                    .unwrap_or_else(|| "screened_out".to_string());
                proposals.push(ProposalTrainingExample {
                    fixture_name: record.fixture_name.clone(),
                    fixture_path: record.fixture_path.clone(),
                    disagreement_category: record.disagreement_category.clone(),
                    response_id: record.response_id,
                    frame_id: record.frame_id,
                    audit_source: record.audit_source.clone(),
                    regime: record.regime.clone(),
                    needs_exact_trigger_target: record.needs_exact_trigger_target,
                    has_strict_disagreement_target: record.has_strict_disagreement_target,
                    has_high_threat_target: record.has_high_threat_target,
                    proposal_input,
                    proposal_class: json_string_field(proposal, "proposal_class"),
                    disposition: "screened_out".to_string(),
                    is_frontier_choice: false,
                    is_exact_best: false,
                    veto_target: true,
                    exact_confidence: Some("unavailable".to_string()),
                    reasons: vec![reason],
                    frontier_outcome: proposal.get("frontier_outcome").cloned(),
                    exact_outcome: None,
                });
            }
        }
    }
    proposals
}

fn summarize_proposal_training_set(
    records: &[ProposalTrainingExample],
    out: &Path,
) -> ProposalTrainingSetSummary {
    let mut audit_source_counts = BTreeMap::new();
    let mut disposition_counts = BTreeMap::new();
    let mut proposal_class_counts = BTreeMap::new();
    let mut reason_counts = BTreeMap::new();
    let mut veto_target_count = 0usize;
    let mut exact_best_count = 0usize;
    let mut needs_exact_trigger_target_count = 0usize;
    for record in records {
        *audit_source_counts
            .entry(record.audit_source.clone())
            .or_insert(0) += 1;
        *disposition_counts
            .entry(record.disposition.clone())
            .or_insert(0) += 1;
        if let Some(proposal_class) = record.proposal_class.as_ref() {
            *proposal_class_counts
                .entry(proposal_class.clone())
                .or_insert(0) += 1;
        }
        for reason in &record.reasons {
            *reason_counts.entry(reason.clone()).or_insert(0) += 1;
        }
        veto_target_count += usize::from(record.veto_target);
        exact_best_count += usize::from(record.is_exact_best);
        needs_exact_trigger_target_count += usize::from(record.needs_exact_trigger_target);
    }
    ProposalTrainingSetSummary {
        proposal_count: records.len(),
        out_path: out.display().to_string(),
        audit_source_counts,
        disposition_counts,
        proposal_class_counts,
        reason_counts,
        veto_target_count,
        exact_best_count,
        needs_exact_trigger_target_count,
    }
}

fn collect_json_paths(explicit: &[PathBuf], dirs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut paths = explicit.iter().cloned().collect::<BTreeSet<_>>();
    for dir in dirs {
        let entries = std::fs::read_dir(dir)
            .map_err(|err| format!("failed to read directory '{}': {err}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|err| {
                format!(
                    "failed to read directory entry in '{}': {err}",
                    dir.display()
                )
            })?;
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                paths.insert(path);
            }
        }
    }
    Ok(paths.into_iter().collect())
}

fn engine_state_label(engine_state: &sts_simulator::state::core::EngineState) -> String {
    format!("{engine_state:?}")
}

fn living_monster_count(combat: &sts_simulator::runtime::combat::CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            monster.current_hp > 0 && !monster.is_dying && !monster.half_dead && !monster.is_escaped
        })
        .count()
}

fn encounter_signature(combat: &sts_simulator::runtime::combat::CombatState) -> Vec<String> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp > 0 && !monster.is_dying && !monster.half_dead)
        .map(|monster| {
            sts_simulator::content::monsters::EnemyId::from_id(monster.monster_type)
                .map(|enemy_id| format!("{enemy_id:?}"))
                .unwrap_or_else(|| format!("monster_type_{}", monster.monster_type))
        })
        .collect()
}

fn screen_type_from_game_state(game_state: &Value) -> Option<String> {
    game_state
        .get("screen_type")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn player_class_from_game_state(game_state: &Value) -> Option<String> {
    game_state
        .get("class")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn ascension_from_game_state(game_state: &Value) -> Option<u8> {
    game_state
        .get("ascension_level")
        .and_then(|value| value.as_u64().or_else(|| value.as_i64().map(|v| v as u64)))
        .map(|value| value as u8)
}

fn compact_power_snapshot(
    combat: &sts_simulator::runtime::combat::CombatState,
    entity_id: sts_simulator::EntityId,
) -> Vec<serde_json::Value> {
    combat
        .entities
        .power_db
        .get(&entity_id)
        .into_iter()
        .flat_map(|powers| powers.iter())
        .map(|power| {
            serde_json::json!({
                "id": format!("{:?}", power.power_type),
                "amount": power.amount,
                "extra_data": power.extra_data,
            })
        })
        .collect()
}

fn compact_card_snapshot(card: &sts_simulator::runtime::combat::CombatCard) -> serde_json::Value {
    serde_json::json!({
        "id": format!("{:?}", card.id),
        "uuid": card.uuid,
        "upgrades": card.upgrades,
        "cost": card.get_cost(),
        "cost_for_turn": card.cost_for_turn,
        "free_to_play_once": card.free_to_play_once,
    })
}

fn compact_combat_snapshot(
    combat: &sts_simulator::runtime::combat::CombatState,
) -> serde_json::Value {
    serde_json::json!({
        "player": {
            "current_hp": combat.entities.player.current_hp,
            "max_hp": combat.entities.player.max_hp,
            "block": combat.entities.player.block,
            "energy_master": combat.entities.player.energy_master,
            "gold": combat.entities.player.gold,
            "stance": format!("{:?}", combat.entities.player.stance),
            "relics": combat.entities.player.relics.iter().map(|relic| format!("{:?}", relic.id)).collect::<Vec<_>>(),
            "powers": compact_power_snapshot(combat, combat.entities.player.id),
            "potions": combat.entities.potions.iter().map(|potion| {
                potion.as_ref().map(|p| format!("{:?}", p.id))
            }).collect::<Vec<_>>(),
        },
        "monsters": combat.entities.monsters.iter().map(|monster| {
            serde_json::json!({
                "id": sts_simulator::content::monsters::EnemyId::from_id(monster.monster_type)
                    .map(|enemy_id| format!("{enemy_id:?}"))
                    .unwrap_or_else(|| format!("monster_type_{}", monster.monster_type)),
                "entity_id": monster.id,
                "slot": monster.slot,
                "current_hp": monster.current_hp,
                "max_hp": monster.max_hp,
                "block": monster.block,
                "is_dying": monster.is_dying,
                "is_escaped": monster.is_escaped,
                "half_dead": monster.half_dead,
                "planned_move_id": monster.planned_move_id(),
                "powers": compact_power_snapshot(combat, monster.id),
            })
        }).collect::<Vec<_>>(),
        "zones": {
            "hand": combat.zones.hand.iter().map(compact_card_snapshot).collect::<Vec<_>>(),
            "draw_count": combat.zones.draw_pile.len(),
            "discard_count": combat.zones.discard_pile.len(),
            "exhaust_count": combat.zones.exhaust_pile.len(),
            "limbo_count": combat.zones.limbo.len(),
            "queued_count": combat.zones.queued_cards.len(),
        },
        "turn": {
            "turn_count": combat.turn.turn_count,
            "phase": format!("{:?}", combat.turn.current_phase),
            "energy": combat.turn.energy,
            "cards_played_this_turn": combat.turn.counters.cards_played_this_turn,
            "attacks_played_this_turn": combat.turn.counters.attacks_played_this_turn,
        },
        "runtime": {
            "action_queue_len": combat.action_queue_len(),
            "combat_smoked": combat.runtime.combat_smoked,
            "combat_mugged": combat.runtime.combat_mugged,
        }
    })
}

fn count_status_like_cards(cards: &[sts_simulator::runtime::combat::CombatCard]) -> usize {
    cards
        .iter()
        .filter(|card| {
            matches!(
                sts_simulator::content::cards::get_card_definition(card.id).card_type,
                sts_simulator::content::cards::CardType::Status
                    | sts_simulator::content::cards::CardType::Curse
            )
        })
        .count()
}

fn curriculum_buckets_for_state(
    combat: &sts_simulator::runtime::combat::CombatState,
    regime: Option<&str>,
    audit: &serde_json::Value,
) -> Vec<String> {
    let mut buckets = Vec::new();

    if combat.meta.is_elite_fight {
        buckets.push("elite".to_string());
    }
    if combat.meta.is_boss_fight {
        buckets.push("boss".to_string());
    }
    match regime {
        Some("crisis") => buckets.push("regime_crisis".to_string()),
        Some("fragile") => buckets.push("regime_fragile".to_string()),
        _ => {}
    }

    let hand_status = count_status_like_cards(&combat.zones.hand);
    let total_status = hand_status
        + count_status_like_cards(&combat.zones.draw_pile)
        + count_status_like_cards(&combat.zones.discard_pile)
        + count_status_like_cards(&combat.zones.exhaust_pile);
    if hand_status >= 2 || total_status >= 5 {
        buckets.push("status_heavy".to_string());
    }

    let proposal_class_counts = audit
        .get("root_pipeline")
        .and_then(|value| value.get("proposal_class_counts"))
        .and_then(|value| value.as_object());
    let attack_count = proposal_class_counts
        .and_then(|counts| counts.get("attack"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let setup_count = proposal_class_counts
        .map(|counts| {
            ["power", "skill_utility"]
                .iter()
                .map(|key| {
                    counts
                        .get(*key)
                        .and_then(|value| value.as_u64())
                        .unwrap_or(0)
                })
                .sum::<u64>()
        })
        .unwrap_or(0);
    if combat.turn.energy > 0 && attack_count > 0 && setup_count > 0 {
        buckets.push("setup_window".to_string());
    }

    buckets.sort();
    buckets.dedup();
    buckets
}

fn build_state_record(
    sample_id: String,
    source_kind: &str,
    source_path: &Path,
    fixture_name: Option<String>,
    combat_case_id: Option<String>,
    run_id: Option<String>,
    response_id: Option<u64>,
    frame_id: Option<u64>,
    player_class: Option<String>,
    ascension_level: Option<u8>,
    screen_type: Option<String>,
    engine_state: &sts_simulator::state::core::EngineState,
    combat: &sts_simulator::runtime::combat::CombatState,
    depth: u32,
) -> Result<StateCorpusRecord, String> {
    let diagnostics = diagnose_root_search_with_depth_and_runtime(
        engine_state,
        combat,
        depth,
        0,
        SearchRuntimeBudget::default(),
    );
    let audit = diagnostics.decision_audit.clone();
    let regime = audit
        .get("regime")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let screened_out_count = audit
        .get("root_pipeline")
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .unwrap_or(0);
    let has_screening_activity_target = screened_out_count > 0;
    let exact_verdict = audit.get("exact_turn_verdict");
    let dominance = exact_verdict
        .and_then(|value| value.get("dominance"))
        .and_then(|value| value.as_str());
    let needs_exact_trigger_target = has_screening_activity_target
        || matches!(regime.as_deref(), Some("fragile" | "crisis"))
        || matches!(
            dominance,
            Some("strictly_better_in_window" | "strictly_worse_in_window")
        );
    let curriculum_buckets = curriculum_buckets_for_state(combat, regime.as_deref(), &audit);

    Ok(StateCorpusRecord {
        sample_id,
        source_kind: source_kind.to_string(),
        source_path: source_path.display().to_string(),
        fixture_name,
        combat_case_id,
        run_id,
        response_id,
        frame_id,
        player_class,
        ascension_level,
        engine_state: engine_state_label(engine_state),
        screen_type,
        regime,
        curriculum_buckets,
        encounter_signature: encounter_signature(combat),
        living_monsters: living_monster_count(combat),
        legal_moves: diagnostics.legal_moves,
        reduced_legal_moves: diagnostics.reduced_legal_moves,
        timed_out: diagnostics.timed_out,
        needs_exact_trigger_target,
        has_screening_activity_target,
        screened_out_count,
        decision_probe_source: "root_search_runtime".to_string(),
        decision_audit: audit,
        combat_snapshot: compact_combat_snapshot(combat),
    })
}

fn build_state_record_from_fixture(path: &Path, depth: u32) -> Result<StateCorpusRecord, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read fixture '{}': {err}", path.display()))?;
    let fixture: ScenarioFixture = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse fixture '{}': {err}", path.display()))?;
    let initial = initialize_fixture_state(&fixture);
    build_state_record(
        format!(
            "fixture:{}:{}",
            fixture.name,
            initial.frame_id.unwrap_or_default()
        ),
        "scenario_fixture",
        path,
        Some(fixture.name.clone()),
        None,
        fixture
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("run:").map(str::to_string)),
        initial.response_id,
        initial.frame_id,
        player_class_from_game_state(&fixture.initial_game_state),
        ascension_from_game_state(&fixture.initial_game_state),
        screen_type_from_game_state(&fixture.initial_game_state),
        &initial.engine_state,
        &initial.combat,
        depth,
    )
}

fn build_state_record_from_combat_case(
    path: &Path,
    depth: u32,
) -> Result<StateCorpusRecord, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read combat case '{}': {err}", path.display()))?;
    let case: CombatCase = serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse combat case '{}': {err}", path.display()))?;
    let lowered = lower_case(&case)?;
    let screen_type = match &case.basis {
        sts_simulator::fixtures::combat_case::CombatCaseBasis::ProtocolSnapshot(protocol) => {
            protocol.root_meta.screen_type.clone()
        }
        sts_simulator::fixtures::combat_case::CombatCaseBasis::EncounterTemplate(_) => None,
        sts_simulator::fixtures::combat_case::CombatCaseBasis::LiveWindow(_) => None,
    };
    build_state_record(
        format!(
            "combat_case:{}:{}",
            case.id,
            lowered.frame_id.unwrap_or_default()
        ),
        "combat_case",
        path,
        None,
        Some(case.id.clone()),
        case.tags
            .iter()
            .find_map(|tag| tag.strip_prefix("run:").map(str::to_string)),
        lowered.response_id,
        lowered.frame_id,
        lowered.player_class,
        lowered.ascension_level,
        screen_type,
        &lowered.engine_state,
        &lowered.combat,
        depth,
    )
}

fn raw_record_is_combat_decision_point(root: &serde_json::Value) -> bool {
    let game_state = root.get("game_state");
    game_state
        .and_then(|value| value.get("combat_truth"))
        .is_some()
        && game_state
            .and_then(|value| value.get("combat_observation"))
            .is_some()
}

fn raw_fixture_from_record(
    root: &serde_json::Value,
    run_id: Option<&str>,
    raw_path: &Path,
) -> Option<ScenarioFixture> {
    let game_state = root.get("game_state")?.clone();
    let protocol_meta = root.get("protocol_meta")?.clone();
    let response_id = protocol_meta
        .get("response_id")
        .and_then(|value| value.as_i64())?;
    let frame_id = protocol_meta
        .get("state_frame_id")
        .and_then(|value| value.as_i64())
        .and_then(|value| u64::try_from(value).ok());
    let mut tags = Vec::new();
    if let Some(run_id) = run_id {
        tags.push(format!("run:{run_id}"));
    }
    Some(ScenarioFixture {
        name: format!(
            "raw_state_{}_{}",
            run_id.unwrap_or("adhoc"),
            frame_id.unwrap_or_default()
        ),
        kind: ScenarioKind::Combat,
        oracle_kind: ScenarioOracleKind::Live,
        initial_game_state: game_state,
        initial_protocol_meta: Some(protocol_meta),
        steps: Vec::new(),
        assertions: Vec::new(),
        provenance: Some(ScenarioProvenance {
            source: Some("raw_state_corpus".to_string()),
            source_path: Some(raw_path.display().to_string()),
            response_id_range: Some((response_id as u64, response_id as u64)),
            failure_frame: frame_id,
            ..ScenarioProvenance::default()
        }),
        tags,
    })
}

fn build_state_records_from_raw(
    raw_path: &Path,
    run_id: Option<&str>,
    limit_per_raw: usize,
    depth: u32,
) -> Result<Vec<StateCorpusRecord>, String> {
    let records = load_raw_records_by_response_id(raw_path)?;
    let mut selected = records
        .iter()
        .rev()
        .filter(|(_, root)| raw_record_is_combat_decision_point(root))
        .filter_map(|(response_id, root)| {
            let fixture = raw_fixture_from_record(root, run_id, raw_path)?;
            let initial = initialize_fixture_state(&fixture);
            if !matches!(
                initial.engine_state,
                sts_simulator::state::core::EngineState::CombatPlayerTurn
                    | sts_simulator::state::core::EngineState::PendingChoice(_)
            ) {
                return None;
            }
            Some((response_id, fixture, initial))
        })
        .take(if limit_per_raw == 0 {
            usize::MAX
        } else {
            limit_per_raw
        })
        .collect::<Vec<_>>();
    selected.sort_by_key(|(response_id, _, _)| *response_id);

    let mut out = Vec::new();
    for (response_id, fixture, initial) in selected {
        out.push(build_state_record(
            format!(
                "raw:{}:{}",
                run_id.unwrap_or("adhoc"),
                initial.frame_id.unwrap_or_default()
            ),
            "live_snapshot",
            raw_path,
            Some(fixture.name.clone()),
            None,
            run_id.map(str::to_string),
            Some(*response_id as u64),
            initial.frame_id,
            player_class_from_game_state(&fixture.initial_game_state),
            ascension_from_game_state(&fixture.initial_game_state),
            screen_type_from_game_state(&fixture.initial_game_state),
            &initial.engine_state,
            &initial.combat,
            depth,
        )?);
    }
    Ok(out)
}

fn state_corpus_source_priority(source_kind: &str) -> u8 {
    match source_kind {
        "combat_case" => 3,
        "scenario_fixture" => 2,
        "live_snapshot" => 1,
        _ => 0,
    }
}

fn state_corpus_terminal_like(record: &StateCorpusRecord) -> bool {
    if matches!(record.screen_type.as_deref(), Some("GAME_OVER")) {
        return true;
    }
    if record.living_monsters == 0 {
        return true;
    }
    record
        .combat_snapshot
        .get("player")
        .and_then(|player| player.get("current_hp"))
        .and_then(|value| value.as_i64())
        .is_some_and(|hp| hp <= 0)
}

fn state_corpus_dedup_key(record: &StateCorpusRecord) -> Option<String> {
    if let (Some(run_id), Some(response_id), Some(frame_id)) =
        (&record.run_id, record.response_id, record.frame_id)
    {
        return Some(format!(
            "run:{run_id}:response:{response_id}:frame:{frame_id}"
        ));
    }
    if let Some(combat_case_id) = &record.combat_case_id {
        return Some(format!("combat_case:{combat_case_id}"));
    }
    if let Some(fixture_name) = &record.fixture_name {
        if let Some(run_id) = &record.run_id {
            if let Some(frame_id) = record.frame_id {
                return Some(format!(
                    "fixture_run:{run_id}:frame:{frame_id}:{fixture_name}"
                ));
            }
        }
        return Some(format!("fixture:{fixture_name}"));
    }
    None
}

fn clean_state_corpus_records(
    records: Vec<StateCorpusRecord>,
) -> (Vec<StateCorpusRecord>, StateCorpusFilterStats) {
    let mut stats = StateCorpusFilterStats {
        candidate_count: records.len(),
        ..StateCorpusFilterStats::default()
    };
    let mut kept: Vec<StateCorpusRecord> = Vec::new();
    let mut seen = BTreeMap::<String, usize>::new();

    for record in records {
        if state_corpus_terminal_like(&record) {
            stats.terminal_filtered_count += 1;
            continue;
        }

        if let Some(key) = state_corpus_dedup_key(&record) {
            if let Some(existing_idx) = seen.get(&key).copied() {
                stats.duplicate_filtered_count += 1;
                if state_corpus_source_priority(&record.source_kind)
                    > state_corpus_source_priority(&kept[existing_idx].source_kind)
                {
                    kept[existing_idx] = record;
                }
                continue;
            }
            seen.insert(key, kept.len());
        }

        kept.push(record);
    }

    (kept, stats)
}

fn filter_state_corpus_by_buckets(
    records: Vec<StateCorpusRecord>,
    include_buckets: &[String],
    exclude_buckets: &[String],
    preserve_trigger_negative_rows: usize,
    stats: &mut StateCorpusFilterStats,
) -> (Vec<StateCorpusRecord>, usize) {
    let include = include_buckets
        .iter()
        .map(|value| value.as_str())
        .collect::<BTreeSet<_>>();
    let exclude = exclude_buckets
        .iter()
        .map(|value| value.as_str())
        .collect::<BTreeSet<_>>();

    let mut kept = Vec::new();
    let mut preserved_candidates = Vec::new();

    for record in records {
        let record_buckets = record
            .curriculum_buckets
            .iter()
            .map(|value| value.as_str())
            .collect::<BTreeSet<_>>();
        let include_ok = include.is_empty() || !record_buckets.is_disjoint(&include);
        let exclude_hit = !exclude.is_empty() && !record_buckets.is_disjoint(&exclude);
        let keep = include_ok && !exclude_hit;
        if keep {
            kept.push(record);
            continue;
        }
        stats.bucket_filtered_count += 1;
        if preserve_trigger_negative_rows > 0
            && !record.needs_exact_trigger_target
            && !exclude_hit
            && !include_ok
        {
            preserved_candidates.push(record);
        }
    }

    preserved_candidates.sort_by(|left, right| {
        state_corpus_split_group_key(left)
            .cmp(&state_corpus_split_group_key(right))
            .then_with(|| left.frame_id.cmp(&right.frame_id))
            .then_with(|| left.response_id.cmp(&right.response_id))
            .then_with(|| left.sample_id.cmp(&right.sample_id))
    });

    let mut preserved_count = 0usize;
    for record in preserved_candidates
        .into_iter()
        .take(preserve_trigger_negative_rows)
    {
        kept.push(record);
        preserved_count += 1;
    }

    (kept, preserved_count)
}

fn summarize_state_corpus(
    records: &[StateCorpusRecord],
    out: &Path,
    include_bucket_filters: &[String],
    exclude_bucket_filters: &[String],
    stats: StateCorpusFilterStats,
) -> StateCorpusSummary {
    let mut source_kind_counts = BTreeMap::new();
    let mut decision_probe_source_counts = BTreeMap::new();
    let mut regime_counts = BTreeMap::new();
    let mut curriculum_bucket_counts = BTreeMap::new();
    let mut player_class_counts = BTreeMap::new();
    let mut screen_type_counts = BTreeMap::new();
    let mut needs_exact_trigger_target_count = 0usize;
    let mut screening_activity_target_count = 0usize;

    for record in records {
        *source_kind_counts
            .entry(record.source_kind.clone())
            .or_insert(0) += 1;
        *decision_probe_source_counts
            .entry(record.decision_probe_source.clone())
            .or_insert(0) += 1;
        if let Some(regime) = record.regime.as_ref() {
            *regime_counts.entry(regime.clone()).or_insert(0) += 1;
        }
        for bucket in &record.curriculum_buckets {
            *curriculum_bucket_counts.entry(bucket.clone()).or_insert(0) += 1;
        }
        if let Some(player_class) = record.player_class.as_ref() {
            *player_class_counts.entry(player_class.clone()).or_insert(0) += 1;
        }
        if let Some(screen_type) = record.screen_type.as_ref() {
            *screen_type_counts.entry(screen_type.clone()).or_insert(0) += 1;
        }
        needs_exact_trigger_target_count += usize::from(record.needs_exact_trigger_target);
        screening_activity_target_count += usize::from(record.has_screening_activity_target);
    }

    StateCorpusSummary {
        candidate_count: stats.candidate_count,
        sample_count: records.len(),
        out_path: out.display().to_string(),
        include_bucket_filters: include_bucket_filters.to_vec(),
        exclude_bucket_filters: exclude_bucket_filters.to_vec(),
        source_kind_counts,
        decision_probe_source_counts,
        regime_counts,
        curriculum_bucket_counts,
        player_class_counts,
        screen_type_counts,
        needs_exact_trigger_target_count,
        screening_activity_target_count,
        terminal_filtered_count: stats.terminal_filtered_count,
        duplicate_filtered_count: stats.duplicate_filtered_count,
        bucket_filtered_count: stats.bucket_filtered_count,
    }
}

fn load_state_corpus_records(path: &Path) -> Result<Vec<StateCorpusRecord>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read state corpus '{}': {err}", path.display()))?;
    let mut records = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record: StateCorpusRecord = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse state corpus '{}' line {}: {err}",
                path.display(),
                line_idx + 1
            )
        })?;
        records.push(record);
    }
    Ok(records)
}

fn stable_fnv1a_64(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn state_corpus_split_group_key(record: &StateCorpusRecord) -> String {
    if let Some(combat_case_id) = &record.combat_case_id {
        return format!("combat_case:{combat_case_id}");
    }
    if let Some(run_id) = &record.run_id {
        if !record.encounter_signature.is_empty() {
            return format!(
                "run:{run_id}:encounter:{}",
                record.encounter_signature.join("+")
            );
        }
        if let Some(frame_id) = record.frame_id {
            return format!("run:{run_id}:frame:{frame_id}");
        }
    }
    if let Some(fixture_name) = &record.fixture_name {
        return format!("fixture:{fixture_name}");
    }
    record.source_path.clone()
}

fn split_name_for_group_key(group_key: &str, train_pct: u8, val_pct: u8) -> &'static str {
    let bucket = (stable_fnv1a_64(group_key) % 100) as u8;
    if bucket < train_pct {
        "train"
    } else if bucket < train_pct.saturating_add(val_pct) {
        "val"
    } else {
        "test"
    }
}

fn split_enabled(split: &str, train_pct: u8, val_pct: u8) -> bool {
    match split {
        "train" => train_pct > 0,
        "val" => val_pct > 0,
        "test" => train_pct as u16 + (val_pct as u16) < 100,
        _ => false,
    }
}

fn group_has_trigger_positive(records: &[StateCorpusRecord]) -> bool {
    records
        .iter()
        .any(|record| record.needs_exact_trigger_target)
}

fn group_has_trigger_negative(records: &[StateCorpusRecord]) -> bool {
    records
        .iter()
        .any(|record| !record.needs_exact_trigger_target)
}

fn count_split_trigger_labels(
    split_groups: &BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
) -> BTreeMap<String, BTreeMap<String, usize>> {
    let mut counts = BTreeMap::new();
    for (split, groups) in split_groups {
        let mut positives = 0usize;
        let mut negatives = 0usize;
        for (_, records) in groups {
            for record in records {
                if record.needs_exact_trigger_target {
                    positives += 1;
                } else {
                    negatives += 1;
                }
            }
        }
        counts.insert(
            split.clone(),
            BTreeMap::from([
                ("positive".to_string(), positives),
                ("negative".to_string(), negatives),
            ]),
        );
    }
    counts
}

fn split_groups_with_label(
    split_groups: &BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
    split: &str,
    want_positive: bool,
) -> usize {
    let Some(groups) = split_groups.get(split) else {
        return 0;
    };
    groups
        .iter()
        .filter(|(_, records)| {
            if want_positive {
                group_has_trigger_positive(records)
            } else {
                group_has_trigger_negative(records)
            }
        })
        .count()
}

fn move_group_between_splits(
    split_groups: &mut BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
    from_split: &str,
    to_split: &str,
    group_key: &str,
) -> bool {
    let Some(source_groups) = split_groups.get_mut(from_split) else {
        return false;
    };
    let Some(index) = source_groups
        .iter()
        .position(|(existing_key, _)| existing_key == group_key)
    else {
        return false;
    };
    let group = source_groups.remove(index);
    split_groups
        .entry(to_split.to_string())
        .or_default()
        .push(group);
    true
}

fn enforce_trigger_label_coverage(
    split_groups: &mut BTreeMap<String, Vec<(String, Vec<StateCorpusRecord>)>>,
    train_pct: u8,
    val_pct: u8,
) -> Vec<String> {
    let mut adjustments = Vec::new();
    let trigger_positive_available = split_groups
        .values()
        .flat_map(|groups| groups.iter())
        .any(|(_, records)| group_has_trigger_positive(records));
    let trigger_negative_available = split_groups
        .values()
        .flat_map(|groups| groups.iter())
        .any(|(_, records)| group_has_trigger_negative(records));

    if !trigger_positive_available && !trigger_negative_available {
        return adjustments;
    }

    for split in ["train", "val", "test"] {
        if !split_enabled(split, train_pct, val_pct) {
            continue;
        }

        let counts = count_split_trigger_labels(split_groups);
        let split_counts = counts.get(split).cloned().unwrap_or_default();
        let split_positive = *split_counts.get("positive").unwrap_or(&0);
        if trigger_positive_available && split_positive == 0 {
            let mut candidate: Option<(String, String, usize)> = None;
            for donor in ["train", "val", "test"] {
                if donor == split {
                    continue;
                }
                let donor_label_groups = split_groups_with_label(split_groups, donor, true);
                let donor_can_spare = if split == "train" {
                    donor_label_groups >= 1
                } else {
                    donor_label_groups > 1
                };
                if !donor_can_spare {
                    continue;
                }
                if let Some(groups) = split_groups.get(donor) {
                    for (group_key, records) in groups {
                        if !group_has_trigger_positive(records) {
                            continue;
                        }
                        let group_len = records.len();
                        let choice = (donor.to_string(), group_key.clone(), group_len);
                        match &candidate {
                            Some((_, _, best_len)) if *best_len <= group_len => {}
                            _ => candidate = Some(choice),
                        }
                    }
                }
            }
            if let Some((from_split, group_key, _)) = candidate {
                if move_group_between_splits(split_groups, &from_split, split, &group_key) {
                    adjustments.push(format!(
                        "moved trigger-positive group '{group_key}' from {from_split} to {split}"
                    ));
                }
            }
        }

        let counts = count_split_trigger_labels(split_groups);
        let split_counts = counts.get(split).cloned().unwrap_or_default();
        let split_negative = *split_counts.get("negative").unwrap_or(&0);
        if trigger_negative_available && split_negative == 0 {
            let mut candidate: Option<(String, String, usize)> = None;
            for donor in ["train", "val", "test"] {
                if donor == split {
                    continue;
                }
                let donor_label_groups = split_groups_with_label(split_groups, donor, false);
                let donor_can_spare = if split == "train" {
                    donor_label_groups >= 1
                } else {
                    donor_label_groups > 1
                };
                if !donor_can_spare {
                    continue;
                }
                if let Some(groups) = split_groups.get(donor) {
                    for (group_key, records) in groups {
                        if !group_has_trigger_negative(records) {
                            continue;
                        }
                        let group_len = records.len();
                        let choice = (donor.to_string(), group_key.clone(), group_len);
                        match &candidate {
                            Some((_, _, best_len)) if *best_len <= group_len => {}
                            _ => candidate = Some(choice),
                        }
                    }
                }
            }
            if let Some((from_split, group_key, _)) = candidate {
                if move_group_between_splits(split_groups, &from_split, split, &group_key) {
                    adjustments.push(format!(
                        "moved trigger-negative group '{group_key}' from {from_split} to {split}"
                    ));
                }
            }
        }
    }

    adjustments
}

fn split_state_corpus_records(
    records: Vec<StateCorpusRecord>,
    include_buckets: &[String],
    exclude_buckets: &[String],
    train_pct: u8,
    val_pct: u8,
    preserve_trigger_negative_rows: usize,
) -> Result<
    (
        BTreeMap<String, Vec<StateCorpusRecord>>,
        StateCorpusSplitSummary,
    ),
    String,
> {
    if train_pct as u16 + val_pct as u16 > 100 {
        return Err(format!(
            "invalid split ratios: train_pct({train_pct}) + val_pct({val_pct}) must be <= 100"
        ));
    }
    let input_count = records.len();
    let mut filter_stats = StateCorpusFilterStats::default();
    let (filtered, preserved_trigger_negative_count) = filter_state_corpus_by_buckets(
        records,
        include_buckets,
        exclude_buckets,
        preserve_trigger_negative_rows,
        &mut filter_stats,
    );
    let mut grouped = BTreeMap::<String, Vec<StateCorpusRecord>>::new();
    for record in filtered {
        grouped
            .entry(state_corpus_split_group_key(&record))
            .or_default()
            .push(record);
    }

    let mut split_groups = BTreeMap::<String, Vec<(String, Vec<StateCorpusRecord>)>>::new();

    for (group_key, group_records) in grouped {
        let split = split_name_for_group_key(&group_key, train_pct, val_pct).to_string();
        split_groups
            .entry(split)
            .or_default()
            .push((group_key, group_records));
    }

    let trigger_coverage_adjustments =
        enforce_trigger_label_coverage(&mut split_groups, train_pct, val_pct);

    let mut split_records = BTreeMap::<String, Vec<StateCorpusRecord>>::new();
    let mut split_counts = BTreeMap::<String, usize>::new();
    let mut split_group_counts = BTreeMap::<String, usize>::new();
    for (split, groups) in &split_groups {
        *split_group_counts.entry(split.clone()).or_insert(0) += groups.len();
        let split_rows = split_records.entry(split.clone()).or_default();
        for (_, group_records) in groups {
            *split_counts.entry(split.clone()).or_insert(0) += group_records.len();
            split_rows.extend(group_records.iter().cloned());
        }
    }
    let split_trigger_label_counts = count_split_trigger_labels(&split_groups);

    let summary = StateCorpusSplitSummary {
        input_path: String::new(),
        out_dir: String::new(),
        include_bucket_filters: include_buckets.to_vec(),
        exclude_bucket_filters: exclude_buckets.to_vec(),
        preserve_trigger_negative_rows,
        total_records: input_count,
        kept_records: split_counts.values().copied().sum(),
        bucket_filtered_count: filter_stats.bucket_filtered_count,
        preserved_trigger_negative_count,
        group_count: split_group_counts.values().copied().sum(),
        split_counts,
        split_group_counts,
        split_trigger_label_counts,
        trigger_coverage_adjustments,
    };
    Ok((split_records, summary))
}

fn build_decision_corpus(
    run_entries: &[(
        PathBuf,
        sts_simulator::cli::live_comm_admin::LiveRunManifest,
    )],
    categories: &[String],
    limit_per_run: usize,
    window_lookback: usize,
    depth: u32,
    out_dir: &Path,
) -> Result<DecisionCorpusSummary, String> {
    let fixtures_root = out_dir.join("fixtures");
    let frame_out = out_dir.join("decision_training.jsonl");
    let frame_summary_out = out_dir.join("decision_training_summary.json");
    let proposal_out = out_dir.join("proposal_training.jsonl");
    let proposal_summary_out = out_dir.join("proposal_training_summary.json");
    let corpus_summary_out = out_dir.join("corpus_summary.json");
    std::fs::create_dir_all(&fixtures_root).map_err(|err| {
        format!(
            "failed to create fixtures root '{}': {err}",
            fixtures_root.display()
        )
    })?;

    let mut all_fixture_paths = Vec::new();
    let mut frame_records = Vec::new();
    let mut run_summaries = Vec::new();

    for (manifest_path, manifest) in run_entries {
        let debug_path = match artifact_path_for_record(manifest_path, &manifest.artifacts.debug) {
            Some(path) => path,
            None => continue,
        };
        let raw_path = match artifact_path_for_record(manifest_path, &manifest.artifacts.raw) {
            Some(path) => path,
            None => continue,
        };
        let bot_strength =
            artifact_path_for_record(manifest_path, &manifest.artifacts.bot_strength)
                .map(|path| load_bot_strength_summary(&path))
                .transpose()?;
        let report = analyze_decision_debug(
            &debug_path,
            &manifest.run_id,
            &manifest.classification_label,
            manifest.counts.engine_bugs == 0
                && manifest.counts.content_gaps == 0
                && manifest.counts.timing_diffs == 0
                && manifest.counts.replay_failures == 0,
            bot_strength,
        )?;
        let selected_examples = collect_export_examples(&report, categories, limit_per_run);
        let combat_shadows_by_frame = live_combat_shadow_path_for_manifest(manifest_path, manifest)
            .map(|path| load_combat_shadow_records_by_frame(&path))
            .transpose()?;
        let run_fixture_dir = fixtures_root.join(&manifest.run_id);
        let export_report = export_disagreement_fixtures(
            &raw_path,
            &report,
            combat_shadows_by_frame.as_ref(),
            categories,
            limit_per_run,
            window_lookback,
            &run_fixture_dir,
        )?;
        let exported_by_frame = export_report
            .exported
            .iter()
            .map(|exported| (exported.frame, PathBuf::from(&exported.fixture_path)))
            .collect::<BTreeMap<_, _>>();
        let raw_records = load_raw_records_by_response_id(&raw_path)?;
        let mut live_shadow_record_count = 0usize;
        let mut fixture_rerun_record_count = 0usize;
        for example in selected_examples {
            let Some(frame) = example.frame else {
                continue;
            };
            let fixture_path = exported_by_frame.get(&frame);
            if let Some(shadow) = combat_shadows_by_frame
                .as_ref()
                .and_then(|entries| entries.get(&frame))
            {
                frame_records.push(build_decision_training_example_from_live_shadow(
                    &manifest.run_id,
                    &raw_path,
                    fixture_path.map(PathBuf::as_path),
                    example,
                    response_id_for_frame(&raw_records, frame).map(|id| id as u64),
                    shadow,
                ));
                live_shadow_record_count += 1;
            } else if let Some(path) = fixture_path {
                all_fixture_paths.push(path.clone());
                fixture_rerun_record_count += 1;
            }
        }
        run_summaries.push(DecisionCorpusRunSummary {
            run_id: manifest.run_id.clone(),
            classification_label: manifest.classification_label.clone(),
            exported_fixture_count: export_report.exported.len(),
            live_shadow_record_count,
            fixture_rerun_record_count,
            missing_frame_count: export_report.missing_frames.len(),
        });
    }

    frame_records.extend(build_decision_training_set(&all_fixture_paths, depth)?);
    let proposal_records = build_proposal_training_set(&frame_records);
    let frame_summary = summarize_decision_training_set(&frame_records, &frame_out);
    let proposal_summary = summarize_proposal_training_set(&proposal_records, &proposal_out);

    if let Some(parent) = frame_out.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create corpus output directory '{}': {err}",
                parent.display()
            )
        })?;
    }

    let mut frame_lines = String::new();
    for record in &frame_records {
        frame_lines.push_str(
            &serde_json::to_string(record)
                .map_err(|err| format!("failed to serialize frame training record: {err}"))?,
        );
        frame_lines.push('\n');
    }
    std::fs::write(&frame_out, frame_lines).map_err(|err| {
        format!(
            "failed to write frame training set '{}': {err}",
            frame_out.display()
        )
    })?;

    let mut proposal_lines = String::new();
    for record in &proposal_records {
        proposal_lines.push_str(
            &serde_json::to_string(record)
                .map_err(|err| format!("failed to serialize proposal training record: {err}"))?,
        );
        proposal_lines.push('\n');
    }
    std::fs::write(&proposal_out, proposal_lines).map_err(|err| {
        format!(
            "failed to write proposal training set '{}': {err}",
            proposal_out.display()
        )
    })?;

    std::fs::write(
        &frame_summary_out,
        serde_json::to_string_pretty(&frame_summary)
            .map_err(|err| format!("failed to serialize frame summary: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write frame summary '{}': {err}",
            frame_summary_out.display()
        )
    })?;
    std::fs::write(
        &proposal_summary_out,
        serde_json::to_string_pretty(&proposal_summary)
            .map_err(|err| format!("failed to serialize proposal summary: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write proposal summary '{}': {err}",
            proposal_summary_out.display()
        )
    })?;

    let corpus_summary = DecisionCorpusSummary {
        run_count: run_summaries.len(),
        fixture_count: all_fixture_paths.len(),
        categories: categories.to_vec(),
        out_dir: out_dir.display().to_string(),
        runs: run_summaries,
        frame_summary,
        proposal_summary,
    };
    std::fs::write(
        &corpus_summary_out,
        serde_json::to_string_pretty(&corpus_summary)
            .map_err(|err| format!("failed to serialize corpus summary: {err}"))?,
    )
    .map_err(|err| {
        format!(
            "failed to write corpus summary '{}': {err}",
            corpus_summary_out.display()
        )
    })?;

    Ok(corpus_summary)
}

fn recommended_source_files(family: &FindingsFamily) -> Vec<&'static str> {
    let key_lower = family.key.to_ascii_lowercase();
    let combat_labels = family
        .combat_labels
        .iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let event_labels = family
        .event_labels
        .iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let has_combat_label = |needle: &str| combat_labels.iter().any(|label| label.contains(needle));
    let has_event_label = |needle: &str| event_labels.iter().any(|label| label.contains(needle));

    let mut files = Vec::new();
    let mut push = |path: &'static str| {
        if !files.contains(&path) {
            files.push(path);
        }
    };

    match family.category.as_str() {
        "engine_bug" | "content_gap" | "timing" => {
            push("src/engine/action_handlers/damage.rs");
            push("src/content/powers/mod.rs");
            push("../cardcrawl/powers/");
        }
        "validation_failure" => {
            push("src/cli/live_comm_noncombat.rs");
            push("src/cli/live_comm/combat.rs");
            push("../CommunicationMod/src/main/java/communicationmod/GameStateConverter.java");
        }
        _ => {}
    }

    if key_lower.contains("power[strength]") || key_lower.contains("strength") {
        push("src/engine/action_handlers/damage.rs");
        push("src/content/powers/ironclad/rupture.rs");
        push("src/content/powers/core/lose_strength.rs");
        push("src/content/cards/ironclad/flex.rs");
        push("../cardcrawl/powers/RupturePower.java");
        push("../cardcrawl/powers/LoseStrengthPower.java");
        push("../cardcrawl/cards/red/Flex.java");
    }

    if key_lower.contains("modeshift")
        || key_lower.contains("guardianthreshold")
        || has_combat_label("guardian")
    {
        push("src/content/monsters/exordium/the_guardian.rs");
        push("src/content/powers/core/mode_shift.rs");
        push("../cardcrawl/monsters/exordium/TheGuardian.java");
    }

    if key_lower.contains("stasis")
        || has_combat_label("bronze orb")
        || has_combat_label("bronze automaton")
    {
        push("src/content/monsters/city/bronze_orb.rs");
        push("src/engine/action_handlers/cards.rs");
        push("../cardcrawl/actions/unique/ApplyStasisAction.java");
        push("../cardcrawl/powers/StasisPower.java");
    }

    if key_lower.contains("potion")
        || key_lower.contains("elixir")
        || key_lower.contains("blocked_reason")
        || has_event_label("shop")
    {
        push("src/cli/live_comm_noncombat.rs");
        push("src/bot/card_knowledge.rs");
        push("src/bot/noncombat_families/shop.rs");
        push("../CommunicationMod/src/main/java/communicationmod/GameStateConverter.java");
    }

    files
}

fn render_findings_family(family: &FindingsFamily) -> String {
    let labels = if !family.combat_labels.is_empty() {
        family.combat_labels.join(", ")
    } else if !family.event_labels.is_empty() {
        family.event_labels.join(", ")
    } else {
        "n/a".to_string()
    };
    let examples = if family.example_rust_java_values.is_empty() {
        "n/a".to_string()
    } else {
        family
            .example_rust_java_values
            .iter()
            .take(2)
            .map(|value| format!("Rust={} Java={}", value.rust, value.java))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let frames = if family.example_frames.is_empty() {
        "n/a".to_string()
    } else {
        family
            .example_frames
            .iter()
            .map(|frame| frame.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let snapshots = if family.example_snapshot_ids.is_empty() {
        "n/a".to_string()
    } else {
        family.example_snapshot_ids.join(", ")
    };
    let artifacts = if family.suggested_artifacts.is_empty() {
        "n/a".to_string()
    } else {
        family.suggested_artifacts.join(", ")
    };
    let source_files = {
        let files = recommended_source_files(family);
        if files.is_empty() {
            "n/a".to_string()
        } else {
            files.join(", ")
        }
    };

    format!(
        "- [{category}] {key}\n  count={count} frames={first}-{last} labels={labels}\n  example_frames={frames}\n  example_snapshots={snapshots}\n  suggested_artifacts={artifacts}\n  suggested_source_files={source_files}\n  example_values={examples}",
        category = family.category,
        key = family.key,
        count = family.count,
        first = family.first_frame,
        last = family.last_frame,
    )
}

#[derive(Debug, Serialize)]
struct LearningBaselineManifest {
    version: u32,
    generated_at: String,
    source: &'static str,
    selected_runs: Vec<LearningBaselineRun>,
    accepted_run_ids: Vec<String>,
    combat_lab_fixtures: Vec<String>,
    reward_case_run_ids: Vec<String>,
    event_case_run_ids: Vec<String>,
    combat_case_run_ids: Vec<String>,
    failure_snapshot_case_run_ids: Vec<String>,
    shadow_case_run_ids: Vec<String>,
    known_noise: Vec<LearningBaselineNoise>,
}

#[derive(Debug, Serialize)]
struct LearningBaselineRun {
    run_id: String,
    classification_label: String,
    validation_status: Option<String>,
    selection_score: i32,
    engine_bugs: usize,
    content_gaps: usize,
    replay_failures: usize,
    manifest_path: String,
    raw_path: Option<String>,
    reward_audit_path: Option<String>,
    event_audit_path: Option<String>,
    combat_suspects_path: Option<String>,
    failure_snapshots_path: Option<String>,
    validation_path: Option<String>,
    sidecar_shadow_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct LearningBaselineNoise {
    run_id: String,
    classification_label: String,
    engine_bugs: usize,
    content_gaps: usize,
    replay_failures: usize,
}

fn python_wrapper(script_rel_path: &str, args: &[&str]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script_path = PathBuf::from(manifest_dir).join(script_rel_path);

    let status = Command::new("python")
        .arg(&script_path)
        .args(args)
        .status()
        .expect("Failed to execute python script. Make sure python is in PATH.");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

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
        } => {
            let policy_kind = match policy.to_ascii_lowercase().as_str() {
                "random_masked" => sts_simulator::cli::full_run_smoke::RunPolicyKind::RandomMasked,
                "rule_baseline_v0" => {
                    sts_simulator::cli::full_run_smoke::RunPolicyKind::RuleBaselineV0
                }
                other => {
                    eprintln!(
                        "unsupported policy '{other}'; expected random_masked or rule_baseline_v0"
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
