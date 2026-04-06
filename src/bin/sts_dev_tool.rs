use clap::{Parser, Subcommand};
use std::process::Command;
use std::path::PathBuf;
use std::env;

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

    /// Fully rebuild the protocol_schema.json from the ground up using dual extraction & heuristic matching.
    SyncSchema,
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
        Commands::Query { search, entity_type } => {
            println!(">> Delegating to query_relics.py query...");
            let mut args = vec!["query", search.as_str()];
            if let Some(t) = entity_type {
                args.push("--type");
                args.push(t.as_str());
            }
            python_wrapper("tools/query_relics.py", &args);
        }
        Commands::Audit { prefix, entity_type } => {
            println!(">> Delegating to query_relics.py audit...");
            python_wrapper("tools/query_relics.py", &["audit", prefix, "--type", entity_type]);
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
                Err(e) => { eprintln!("Failed to load Java log: {}", e); return; }
            };
            let rust_content = match std::fs::read_to_string(rust_log) {
                Ok(c) => c,
                Err(e) => { eprintln!("Failed to load Rust log: {}", e); return; }
            };
            
            let java_states: Vec<serde_json::Value> = java_content.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
            let rust_states: Vec<serde_json::Value> = rust_content.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
            
            if java_states.is_empty() || rust_states.is_empty() {
                eprintln!("One or both logs contain no valid JSON lines.");
                return;
            }
            
            println!("Loaded {} java steps, {} rust steps.", java_states.len(), rust_states.len());
            // TODO: Deep semantic diff mapping using generated schema and sts_simulator::diff::delta
            println!("(Semantic Diff algorithm available via delta.rs...)");
        }
        Commands::SyncSchema => {
            println!(">> Initiating ASA Engine Schema Synchronization...");
            let tmp_dir = std::env::temp_dir();
            let java_json = tmp_dir.join("extracted_java_nodes.json");
            let rust_json = tmp_dir.join("extracted_rust_nodes.json");
            let matched_json = tmp_dir.join("matched_schema.json");
            
            // Hardcoded paths relative to the cargo root
            let java_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cardcrawl");
            let rust_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
            let baseline = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/protocol_schema.json");
            
            let p_j = java_json.to_string_lossy();
            let p_r = rust_json.to_string_lossy();
            let j_src = java_src.to_string_lossy();
            let r_src = rust_src.to_string_lossy();
            
            let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/schema_builder/config");
            let p_skeleton = config_dir.join("schema_skeleton.json").to_string_lossy().into_owned();
            let p_overrides = config_dir.join("override_rules.json").to_string_lossy().into_owned();
            
            let p_b = baseline.to_string_lossy();
            let p_m = matched_json.to_string_lossy();

            println!(">> 1. Running java_crawler...");
            python_wrapper("tools/schema_builder/java_crawler.py", &[&j_src, &p_j]);

            println!(">> 2. Running rust_crawler...");
            python_wrapper("tools/schema_builder/rust_crawler.py", &[&r_src, &p_r]);

            println!(">> 3. Running heuristic_matcher (Pure Gen Mode)...");
            python_wrapper("tools/schema_builder/heuristic_matcher.py", &[&p_j, &p_r, &p_skeleton, &p_overrides, &p_m]);

            println!(">> 4. Running schema_compiler...");
            python_wrapper("tools/schema_builder/schema_compiler.py", &[&p_m, &p_b]);

            println!("========================================");
            println!("Sync Complete! Schema effectively updated at {:?}", baseline);
            println!("(Any unmapped rust anomalies will be printed above. Run 'cargo build' to propagate bindings)");
        }
    }
}
