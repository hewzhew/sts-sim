use clap::{Parser, Subcommand};
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
            let live_comm = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("live_comm_raw.jsonl");

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
            let live_comm_raw = manifest_dir.join("live_comm_raw.jsonl");
            let live_comm_sidecar = manifest_dir.join("live_comm_signatures.jsonl");

            let replay_inputs =
                sts_simulator::interaction_coverage::default_replay_inputs(&manifest_dir);
            let mut generated_from: Vec<String> = replay_inputs
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            let mut notes = Vec::new();
            let mut records = Vec::new();

            for replay in &replay_inputs {
                records
                    .extend(sts_simulator::interaction_coverage::replay_records_from_path(replay));
            }

            if live_comm_sidecar.exists() {
                generated_from.push(live_comm_sidecar.to_string_lossy().into_owned());
                records.extend(sts_simulator::interaction_coverage::load_live_comm_records(
                    &live_comm_sidecar,
                ));
            } else if live_comm_raw.exists() {
                notes.push(format!(
                    "{} present but omitted from strict signature extraction because it lacks command context",
                    live_comm_raw.display()
                ));
            }

            if let Err(err) = sts_simulator::interaction_coverage::write_coverage_outputs(
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
    }
}
