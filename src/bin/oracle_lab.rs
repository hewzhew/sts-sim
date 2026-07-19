use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use sts_simulator::eval::run_control::OracleAnalysisAdvanceRequestV1;
use sts_simulator::runtime::branch::{
    call_oracle_analysis_tcp_v1, load_oracle_analysis_workspace_v1,
    load_oracle_run_continuation_v1, save_oracle_analysis_workspace_v1,
    serve_oracle_analysis_jsonl_v1, serve_oracle_analysis_tcp_v1, OracleAnalysisWorkspaceV1,
    OracleRunBudget, OracleRunConfig,
};

#[derive(Debug, Parser)]
#[command(
    name = "oracle_lab",
    about = "Inspect and steer exact oracle-run variations without editing checkpoints"
)]
struct Cli {
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
    /// Import the exact selected state from an oracle_run continuation.
    Import {
        #[arg(long)]
        continuation: PathBuf,
        #[arg(long)]
        workspace: PathBuf,
        #[command(flatten)]
        budget: BudgetArgs,
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

fn main() -> Result<(), String> {
    match Cli::parse().command {
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
            budget,
        } => {
            let continuation = load_oracle_run_continuation_v1(&continuation)?;
            let config = OracleRunConfig {
                seed: continuation.seed,
                ascension: continuation.ascension,
                budget: budget.into_budget(),
            };
            let analysis = OracleAnalysisWorkspaceV1::from_continuation(config, continuation)?;
            let view = analysis.view()?;
            save_oracle_analysis_workspace_v1(&workspace, &analysis)?;
            print_json(&view)
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
