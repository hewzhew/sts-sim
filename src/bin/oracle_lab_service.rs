use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use clap::Parser;
use sts_simulator::runtime::branch::{
    load_oracle_analysis_workspace_v1, serve_oracle_analysis_tcp_v1,
};

#[derive(Debug, Parser)]
#[command(
    name = "oracle_lab_service",
    about = "Dedicated resident compute host for one oracle workspace"
)]
struct Cli {
    #[arg(long, hide = true)]
    canonical_fast_run: bool,
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long)]
    endpoint: PathBuf,
    #[arg(long, default_value = "127.0.0.1:0")]
    listen: SocketAddr,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    validate_canonical_launch(cli.canonical_fast_run)?;
    let workspace_path = cli.workspace.canonicalize().map_err(|error| {
        format!(
            "failed to resolve oracle workspace '{}': {error}",
            cli.workspace.display()
        )
    })?;
    let workspace = load_oracle_analysis_workspace_v1(&workspace_path)?;
    serve_oracle_analysis_tcp_v1(
        &workspace_path,
        workspace,
        cli.listen,
        &absolute_from_repository(&cli.endpoint),
    )?;
    Ok(())
}

fn absolute_from_repository(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository_root().join(path)
    }
}

fn validate_canonical_launch(canonical_fast_run: bool) -> Result<(), String> {
    if !canonical_fast_run {
        return Err(
            "oracle_lab_service is an internal resident host; start it with `cargo ol-live start`"
                .to_string(),
        );
    }
    const REQUIRED_PROFILE: &str = "fast-run";
    const BUILT_PROFILE: &str = env!("STS_CARGO_PROFILE");
    if BUILT_PROFILE != REQUIRED_PROFILE {
        return Err(format!(
            "oracle_lab_service was built with profile `{BUILT_PROFILE}`; expected `{REQUIRED_PROFILE}`"
        ));
    }
    let expected = repository_root()
        .join("target")
        .join(REQUIRED_PROFILE)
        .join(if cfg!(windows) {
            "oracle_lab_service.exe"
        } else {
            "oracle_lab_service"
        });
    let current = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("failed to identify resident oracle host: {error}"))?;
    let expected = expected.canonicalize().map_err(|error| {
        format!(
            "canonical resident oracle host is missing at {}: {error}",
            expected.display()
        )
    })?;
    if current != expected {
        return Err(format!(
            "resident oracle host refuses non-canonical artifact {}; expected {}",
            current.display(),
            expected.display()
        ));
    }
    Ok(())
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("STS_REPOSITORY_ROOT"))
}
