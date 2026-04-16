use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::Command;

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

    /// Manage run-first live_comm logs.
    Logs {
        #[command(subcommand)]
        command: LogCommands,
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

fn findings_path_for_run(
    paths: &sts_simulator::cli::live_comm_admin::LiveLogPaths,
    run_id: &str,
) -> Option<PathBuf> {
    let entries =
        sts_simulator::cli::live_comm_admin::list_run_manifests_for_audit(paths).ok()?;
    for (manifest_path, manifest) in entries {
        if manifest.run_id != run_id {
            continue;
        }
        let run_dir = manifest_path.parent()?;
        let artifact = manifest.artifacts.findings.as_ref()?;
        if !artifact.present {
            return None;
        }
        return Some(run_dir.join(&artifact.relative_path));
    }
    None
}

fn load_findings_report(path: &PathBuf) -> Result<FindingsReport, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read findings '{}': {err}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|err| format!("failed to parse findings '{}': {err}", path.display()))
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

    format!(
        "- [{category}] {key}\n  count={count} frames={first}-{last} labels={labels}\n  example_frames={frames}\n  example_snapshots={snapshots}\n  suggested_artifacts={artifacts}\n  example_values={examples}",
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
            let replays_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/replays");
            let replay_short =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/replay_short.jsonl");
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
            if replay_short.exists() {
                observed_args_owned.push(replay_short.to_string_lossy().into_owned());
            }
            if replays_dir.exists() {
                let mut replay_files: Vec<_> = std::fs::read_dir(&replays_dir)
                    .expect("read_dir tools/replays should succeed")
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
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

            let replay_inputs =
                sts_simulator::cli::coverage_tools::default_replay_inputs(&manifest_dir);
            let mut generated_from: Vec<String> = replay_inputs
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            let mut notes = Vec::new();
            let mut records = Vec::new();

            for replay in &replay_inputs {
                records.extend(sts_simulator::cli::coverage_tools::replay_records_from_path(
                    replay,
                ));
            }

            if live_comm_sidecar.exists() {
                generated_from.push(live_comm_sidecar.to_string_lossy().into_owned());
                match sts_simulator::cli::coverage_tools::load_live_comm_records(
                    &live_comm_sidecar,
                ) {
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
                    let summary =
                        sts_simulator::cli::live_comm_admin::gc_runs(&paths)
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
                            sts_simulator::cli::live_comm_admin::latest_combat_suspect_path(
                                &paths,
                            )
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
                    let findings_path = if let Some(run_id) = run_id {
                        findings_path_for_run(&paths, run_id)
                    } else {
                        sts_simulator::cli::live_comm_admin::latest_run_artifact_path(
                            &paths,
                            label.as_deref(),
                            "findings",
                        )
                    }
                    .expect("no matching findings artifact found");

                    let mut report =
                        load_findings_report(&findings_path).expect("findings report should load");

                    if let Some(category_filter) = category.as_ref() {
                        report.families.retain(|entry| entry.category == *category_filter);
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
                        "run={} classification={} findings_path={}",
                        report.run_id,
                        report.classification_label,
                        findings_path.display()
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
            }
        }
    }
}
