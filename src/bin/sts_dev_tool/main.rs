use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use sts_simulator::fixtures::author_spec::CombatAuthorSpec;
use sts_simulator::fixtures::combat_case::{compile_combat_author_case, lower_case};
use sts_simulator::fixtures::live_capture::build_fixture_from_record_window;
use sts_simulator::fixtures::scenario::{ScenarioFixture, ScenarioProvenance};
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

    /// Batch-run full offline episodes with an explicit action selector for simulator smoke checks.
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
        /// Action selector name. Only random_masked is supported in full-run smoke.
        #[arg(long, default_value = "random_masked")]
        action_selector: String,
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
        /// Output directory for generated disagreement fixtures.
        #[arg(long)]
        out_dir: PathBuf,
        /// Optional JSON output path for the structured export report.
        #[arg(long)]
        json_out: Option<PathBuf>,
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
}

include!("log_analysis_impl.rs");

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
            python_wrapper(
                "tools/schema_builder/java_crawler.py",
                &[j_src.as_ref(), p_j.as_ref()],
            );

            println!(">> 2. Running rust_crawler...");
            python_wrapper(
                "tools/schema_builder/rust_crawler.py",
                &[r_src.as_ref(), p_r.as_ref()],
            );

            println!(">> 3. Running heuristic_matcher...");
            python_wrapper(
                "tools/schema_builder/heuristic_matcher.py",
                &[
                    p_j.as_ref(),
                    p_r.as_ref(),
                    p_skeleton.as_str(),
                    p_overrides.as_str(),
                    p_o.as_ref(),
                    p_m.as_ref(),
                ],
            );

            println!(">> 4. Running schema_compiler...");
            python_wrapper(
                "tools/schema_builder/schema_compiler.py",
                &[
                    p_m.as_ref(),
                    p_b.as_ref(),
                    p_o.as_ref(),
                    p_c.as_ref(),
                    p_a.as_ref(),
                ],
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
            action_selector,
            trace_dir,
            summary_out,
            determinism_check,
        } => {
            let action_selector_kind = match action_selector.to_ascii_lowercase().as_str() {
                "random_masked" => {
                    sts_simulator::cli::full_run_smoke::RunActionSelectorKind::RandomMasked
                }
                other => {
                    eprintln!("unsupported action selector '{other}'; expected random_masked");
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
                action_selector: action_selector_kind,
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
        },
    }
}
