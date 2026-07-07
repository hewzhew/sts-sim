use std::path::PathBuf;
use std::process;

use clap::{Args as ClapArgs, Parser, Subcommand};
use sts_simulator::runtime::branch::{
    current_source_identity, default_branch_args, Args, BranchArtifactStore, PanelInspectConfig,
    PanelRunOptions, PanelSmokeRunner, PanelSummary, RunObjective, SourceIdentity,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::Panel(panel) => match panel.command {
            PanelCommand::Inspect(raw) => run_inspect(raw.into_inspect_args()?),
            PanelCommand::Smoke(raw) => run_smoke(raw.into_run_args()?),
            PanelCommand::Continue(raw) => run_continue(raw.into_run_args()?),
        },
    }
}

fn run_inspect(args: InspectArgs) -> Result<(), String> {
    let store = args.artifact_store();
    let summary = args.inspect_config(current_source_identity())?.summarize();
    let summary_path = store.write_panel_summary(args.summary_path.as_deref(), &summary)?;
    print_summary("inspect", &summary, &summary_path);
    Ok(())
}

fn run_smoke(args: RunArgs) -> Result<(), String> {
    let store = args.common.artifact_store();
    let summary = PanelSmokeRunner::run_slices(
        args.common.inspect_config(current_source_identity())?,
        PanelRunOptions::smoke(args.max_slices),
    )?;
    let summary_path = store.write_panel_summary(args.common.summary_path.as_deref(), &summary)?;
    print_summary("smoke", &summary, &summary_path);
    Ok(())
}

fn run_continue(args: RunArgs) -> Result<(), String> {
    let store = args.common.artifact_store();
    let summary = PanelSmokeRunner::run_slices(
        args.common.inspect_config(current_source_identity())?,
        PanelRunOptions::continue_existing(args.max_slices),
    )?;
    let summary_path = store.write_panel_summary(args.common.summary_path.as_deref(), &summary)?;
    print_summary("continue", &summary, &summary_path);
    Ok(())
}

#[derive(Parser)]
#[command(
    name = "branch_panel",
    about = "Inspect and schedule durable branch run panels"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Panel(PanelArgs),
}

#[derive(Parser)]
struct PanelArgs {
    #[command(subcommand)]
    command: PanelCommand,
}

#[derive(Subcommand)]
enum PanelCommand {
    Inspect(RawInspectArgs),
    Smoke(RawRunArgs),
    Continue(RawRunArgs),
}

#[derive(ClapArgs)]
struct RawInspectArgs {
    #[arg(long, num_args = 1.., required = true)]
    seeds: Vec<String>,
    #[arg(long)]
    capsule_root: PathBuf,
    #[arg(long)]
    summary_path: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    ascension: u8,
    #[arg(long, value_parser = RunObjective::parse, default_value = "first-victory")]
    objective: RunObjective,
    #[arg(long)]
    generations: Option<usize>,
    #[arg(long)]
    max_branches: Option<usize>,
    #[arg(long)]
    auto_ops: Option<usize>,
    #[arg(long)]
    search_nodes: Option<usize>,
    #[arg(long)]
    search_ms: Option<u64>,
    #[arg(long)]
    rescue_search_nodes: Option<usize>,
    #[arg(long)]
    rescue_search_ms: Option<u64>,
    #[arg(long)]
    boss_search_nodes: Option<usize>,
    #[arg(long)]
    boss_search_ms: Option<u64>,
    #[arg(long)]
    slice_ms: Option<u64>,
    #[arg(long)]
    wall_ms: Option<u64>,
    #[arg(long)]
    checkpoint_before_combat_portfolio: bool,
}

#[derive(ClapArgs)]
struct RawRunArgs {
    #[command(flatten)]
    common: RawInspectArgs,
    #[arg(long, default_value_t = 1)]
    max_slices: usize,
}

#[derive(Clone, Debug)]
struct InspectArgs {
    seeds: Vec<String>,
    capsule_root: PathBuf,
    summary_path: Option<PathBuf>,
    ascension: u8,
    objective: RunObjective,
    generations: usize,
    max_branches: usize,
    auto_ops: usize,
    search_nodes: usize,
    search_ms: u64,
    rescue_search_nodes: usize,
    rescue_search_ms: u64,
    boss_search_nodes: usize,
    boss_search_ms: u64,
    wall_ms: Option<u64>,
    checkpoint_before_combat_portfolio: bool,
}

struct RunArgs {
    common: InspectArgs,
    max_slices: usize,
}

impl RawInspectArgs {
    fn into_inspect_args(self) -> Result<InspectArgs, String> {
        let defaults = default_branch_args(1);
        Ok(InspectArgs {
            seeds: self.seeds,
            capsule_root: self.capsule_root,
            summary_path: self.summary_path,
            ascension: self.ascension,
            objective: self.objective,
            generations: self.generations.unwrap_or(defaults.generations),
            max_branches: self.max_branches.unwrap_or(defaults.max_branches),
            auto_ops: self.auto_ops.unwrap_or(defaults.auto_ops),
            search_nodes: self.search_nodes.unwrap_or(defaults.search_nodes),
            search_ms: self.search_ms.unwrap_or(defaults.search_ms),
            rescue_search_nodes: self
                .rescue_search_nodes
                .unwrap_or(defaults.rescue_search_nodes),
            rescue_search_ms: self.rescue_search_ms.unwrap_or(defaults.rescue_search_ms),
            boss_search_nodes: self.boss_search_nodes.unwrap_or(defaults.boss_search_nodes),
            boss_search_ms: self.boss_search_ms.unwrap_or(defaults.boss_search_ms),
            wall_ms: resolve_slice_ms(self.slice_ms, self.wall_ms)?.or(defaults.wall_ms),
            checkpoint_before_combat_portfolio: self.checkpoint_before_combat_portfolio
                || defaults.checkpoint_before_combat_portfolio,
        })
    }
}

impl RawRunArgs {
    fn into_run_args(self) -> Result<RunArgs, String> {
        if self.max_slices == 0 {
            return Err("--max-slices must be greater than zero".to_string());
        }
        Ok(RunArgs {
            common: self.common.into_inspect_args()?,
            max_slices: self.max_slices,
        })
    }
}

impl InspectArgs {
    fn artifact_store(&self) -> BranchArtifactStore {
        BranchArtifactStore::new(&self.capsule_root)
    }

    fn inspect_config(
        &self,
        source_identity: SourceIdentity,
    ) -> Result<PanelInspectConfig, String> {
        Ok(PanelInspectConfig {
            seeds: parse_seed_specs(&self.seeds)?,
            artifact_store: self.artifact_store(),
            args_template: self.args_template(),
            source_identity,
        })
    }

    fn args_template(&self) -> Args {
        let mut args = default_branch_args(0);
        args.ascension = self.ascension;
        args.objective = self.objective;
        args.generations = self.generations;
        args.max_branches = self.max_branches;
        args.auto_ops = self.auto_ops;
        args.search_nodes = self.search_nodes;
        args.search_ms = self.search_ms;
        args.rescue_search_nodes = self.rescue_search_nodes;
        args.rescue_search_ms = self.rescue_search_ms;
        args.boss_search_nodes = self.boss_search_nodes;
        args.boss_search_ms = self.boss_search_ms;
        args.wall_ms = self.wall_ms;
        args.checkpoint_before_combat_portfolio = self.checkpoint_before_combat_portfolio;
        args
    }
}

fn resolve_slice_ms(slice_ms: Option<u64>, wall_ms: Option<u64>) -> Result<Option<u64>, String> {
    match (slice_ms, wall_ms) {
        (Some(slice_ms), Some(wall_ms)) if slice_ms != wall_ms => Err(format!(
            "--slice-ms ({slice_ms}) conflicts with legacy --wall-ms ({wall_ms})"
        )),
        (Some(slice_ms), _) => Ok(Some(slice_ms)),
        (None, wall_ms) => Ok(wall_ms),
    }
}

fn parse_seed_specs(specs: &[String]) -> Result<Vec<u64>, String> {
    let mut seeds = Vec::new();
    for spec in specs {
        for part in spec
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            append_seed_part(part, &mut seeds)?;
        }
    }
    if seeds.is_empty() {
        return Err("--seeds must include at least one seed".to_string());
    }
    Ok(seeds)
}

fn append_seed_part(part: &str, seeds: &mut Vec<u64>) -> Result<(), String> {
    if let Some((start, end)) = part.split_once("..=") {
        return append_seed_range(start, end, seeds);
    }
    if let Some((start, end)) = part.split_once("..") {
        return append_seed_range(start, end, seeds);
    }
    seeds.push(parse_seed(part)?);
    Ok(())
}

fn append_seed_range(start: &str, end: &str, seeds: &mut Vec<u64>) -> Result<(), String> {
    let start = parse_seed(start.trim())?;
    let end = parse_seed(end.trim())?;
    if start > end {
        return Err(format!("seed range must be ascending: {start}..{end}"));
    }
    seeds.extend(start..=end);
    Ok(())
}

fn parse_seed(value: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("invalid seed value: {value}"))
}

fn print_summary(label: &str, summary: &PanelSummary, path: &PathBuf) {
    println!(
        "branch_panel {label} rows={} wrote {}",
        summary.total_rows,
        path.display()
    );
    for row in &summary.rows {
        println!(
            "seed={} action={:?} decision={:?} capsule={}",
            row.seed, row.scheduler_action, row.reuse_decision, row.capsule_path
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sts_simulator::runtime::branch::{RunObjective, SourceIdentity};

    use super::*;

    #[test]
    fn parses_seed_lists_and_inclusive_ranges() {
        let seeds = parse_seed_specs(&["11..13".to_string(), "20,22".to_string()]).unwrap();

        assert_eq!(seeds, vec![11, 12, 13, 20, 22]);
    }

    #[test]
    fn rejects_descending_seed_ranges() {
        let err = parse_seed_specs(&["3..1".to_string()]).unwrap_err();

        assert!(err.contains("ascending"));
    }

    #[test]
    fn inspect_args_build_seed_requests_with_runtime_contracts() {
        let args = InspectArgs {
            seeds: vec!["7..8".to_string()],
            capsule_root: PathBuf::from("target/panel"),
            summary_path: None,
            ascension: 2,
            objective: RunObjective::FirstTerminal,
            generations: 9,
            max_branches: 3,
            auto_ops: 4,
            search_nodes: 101,
            search_ms: 202,
            rescue_search_nodes: 303,
            rescue_search_ms: 404,
            boss_search_nodes: 505,
            boss_search_ms: 606,
            wall_ms: Some(707),
            checkpoint_before_combat_portfolio: true,
        };
        let source = SourceIdentity {
            git_commit: Some("abc123".to_string()),
            git_dirty: Some(false),
        };

        let requests = args.inspect_config(source.clone()).unwrap().requests();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].seed, 7);
        assert_eq!(requests[0].capsule_path, PathBuf::from("target/panel/7"));
        assert_eq!(requests[0].source_identity, source);
        assert_eq!(requests[0].contract.game.seed, 7);
        assert_eq!(requests[0].contract.game.ascension, 2);
        assert_eq!(requests[0].contract.objective, RunObjective::FirstTerminal);
        assert_eq!(requests[0].contract.branching.generations, 9);
        assert_eq!(requests[0].contract.branching.max_branches, 3);
        assert_eq!(requests[0].contract.automation.auto_ops, 4);
        assert_eq!(requests[0].contract.combat_search.primary_nodes, 101);
        assert_eq!(requests[0].contract.combat_search.primary_ms, 202);
        assert_eq!(requests[0].contract.combat_search.rescue_nodes, 303);
        assert_eq!(requests[0].contract.combat_search.rescue_ms, 404);
        assert_eq!(requests[0].contract.combat_search.boss_nodes, 505);
        assert_eq!(requests[0].contract.combat_search.boss_ms, 606);
        assert_eq!(requests[0].contract.slice.slice_ms, Some(707));
        assert!(
            requests[0]
                .contract
                .features
                .checkpoint_before_combat_portfolio
        );
    }

    #[test]
    fn parses_panel_smoke_command() {
        let cli = Cli::try_parse_from([
            "branch_panel",
            "panel",
            "smoke",
            "--seeds",
            "1..2",
            "--capsule-root",
            "target/panel",
        ])
        .unwrap();

        let Command::Panel(panel) = cli.command;
        let PanelCommand::Smoke(args) = panel.command else {
            panic!("expected panel smoke command");
        };

        assert_eq!(args.common.seeds, vec!["1..2".to_string()]);
        assert_eq!(args.common.capsule_root, PathBuf::from("target/panel"));
        assert_eq!(args.max_slices, 1);
    }

    #[test]
    fn parses_panel_continue_command() {
        let cli = Cli::try_parse_from([
            "branch_panel",
            "panel",
            "continue",
            "--seeds",
            "1",
            "--capsule-root",
            "target/panel",
        ])
        .unwrap();

        let Command::Panel(panel) = cli.command;
        let PanelCommand::Continue(args) = panel.command else {
            panic!("expected panel continue command");
        };

        assert_eq!(args.common.seeds, vec!["1".to_string()]);
        assert_eq!(args.common.capsule_root, PathBuf::from("target/panel"));
        assert_eq!(args.max_slices, 1);
    }

    #[test]
    fn parses_panel_run_max_slices() {
        let cli = Cli::try_parse_from([
            "branch_panel",
            "panel",
            "continue",
            "--seeds",
            "1",
            "--capsule-root",
            "target/panel",
            "--max-slices",
            "2",
        ])
        .unwrap();

        let Command::Panel(panel) = cli.command;
        let PanelCommand::Continue(args) = panel.command else {
            panic!("expected panel continue command");
        };

        assert_eq!(args.max_slices, 2);
    }

    #[test]
    fn parses_slice_ms_as_slice_contract() {
        let cli = Cli::try_parse_from([
            "branch_panel",
            "panel",
            "smoke",
            "--seeds",
            "1",
            "--capsule-root",
            "target/panel",
            "--slice-ms",
            "123",
        ])
        .unwrap();

        let Command::Panel(panel) = cli.command;
        let PanelCommand::Smoke(args) = panel.command else {
            panic!("expected panel smoke command");
        };

        let args = args.into_run_args().unwrap();

        assert_eq!(args.common.wall_ms, Some(123));
    }

    #[test]
    fn rejects_conflicting_slice_and_wall_ms() {
        let cli = Cli::try_parse_from([
            "branch_panel",
            "panel",
            "smoke",
            "--seeds",
            "1",
            "--capsule-root",
            "target/panel",
            "--slice-ms",
            "123",
            "--wall-ms",
            "456",
        ])
        .unwrap();

        let Command::Panel(panel) = cli.command;
        let PanelCommand::Smoke(args) = panel.command else {
            panic!("expected panel smoke command");
        };

        let err = match args.into_run_args() {
            Ok(_) => panic!("expected conflicting slice/wall options to fail"),
            Err(err) => err,
        };

        assert!(err.contains("conflicts"));
    }
}
