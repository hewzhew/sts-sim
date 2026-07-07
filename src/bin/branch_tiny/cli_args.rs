use std::path::PathBuf;

use sts_simulator::state::events::EventId;

use super::run_contract::RunObjective;
pub(super) use sts_simulator::runtime::branch::Args;

#[derive(Clone, Copy, Default)]
pub(super) struct ArgsOverrides {
    pub(super) objective: Option<RunObjective>,
    pub(super) generations: Option<usize>,
    pub(super) max_branches: Option<usize>,
    pub(super) auto_ops: Option<usize>,
    pub(super) search_nodes: Option<usize>,
    pub(super) search_ms: Option<u64>,
    pub(super) rescue_search_nodes: Option<usize>,
    pub(super) rescue_search_ms: Option<u64>,
    pub(super) boss_search_nodes: Option<usize>,
    pub(super) boss_search_ms: Option<u64>,
    pub(super) wall_ms: Option<u64>,
    pub(super) checkpoint_before_combat_portfolio: bool,
}

#[derive(Clone, Copy)]
pub(super) struct EventOwnerProbeArgs {
    pub(super) event_id: EventId,
    pub(super) screen: usize,
}

pub(super) struct ContinueCapsuleArgs {
    pub(super) capsule: PathBuf,
    pub(super) max_slices: usize,
}

impl ArgsOverrides {
    pub(super) fn apply_to(self, args: &mut Args) {
        if let Some(value) = self.objective {
            args.objective = value;
        }
        if let Some(value) = self.generations {
            args.generations = value;
        }
        if let Some(value) = self.max_branches {
            args.max_branches = value;
        }
        if let Some(value) = self.auto_ops {
            args.auto_ops = value;
        }
        if let Some(value) = self.search_nodes {
            args.search_nodes = value;
        }
        if let Some(value) = self.search_ms {
            args.search_ms = value;
        }
        if let Some(value) = self.rescue_search_nodes {
            args.rescue_search_nodes = value;
        }
        if let Some(value) = self.rescue_search_ms {
            args.rescue_search_ms = value;
        }
        if let Some(value) = self.boss_search_nodes {
            args.boss_search_nodes = value;
        }
        if let Some(value) = self.boss_search_ms {
            args.boss_search_ms = value;
        }
        if let Some(value) = self.wall_ms {
            args.wall_ms = Some(value);
        }
        if self.checkpoint_before_combat_portfolio {
            args.checkpoint_before_combat_portfolio = true;
        }
    }
}

pub(super) fn parse_args() -> Result<
    (
        Args,
        ArgsOverrides,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<ContinueCapsuleArgs>,
        Option<EventOwnerProbeArgs>,
    ),
    String,
> {
    let mut args = Args {
        seed: 1,
        ascension: 0,
        objective: RunObjective::FirstVictory,
        generations: 2,
        max_branches: 24,
        auto_ops: 64,
        search_nodes: 50_000,
        search_ms: 500,
        rescue_search_nodes: 200_000,
        rescue_search_ms: 3_000,
        boss_search_nodes: 800_000,
        boss_search_ms: 8_000,
        wall_ms: None,
        checkpoint_before_combat_portfolio: false,
        wall_capped_search_budget: false,
        wall_capped_boss_budget: false,
    };
    let mut overrides = ArgsOverrides::default();
    let mut trace_jsonl = None;
    let mut combat_gap_case_dir = None;
    let mut frontier_checkpoint = None;
    let mut resume_frontier = None;
    let mut run_capsule = None;
    let mut resume_capsule = None;
    let mut continue_capsule = None;
    let mut continue_slices = None;
    let mut probe_event_owner = None;
    let mut probe_event_screen = 0usize;
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!(
                "branch_tiny --seed N --generations N --max-branches N [--objective first-victory|first-terminal|exhaust-frontier] [--wall-ms N] [--trace-jsonl PATH] [--frontier-checkpoint PATH] [--resume-frontier PATH]"
            );
            println!("  optional: --run-capsule DIR writes manifest/frontier/result/path JSON");
            println!(
                "  optional: --resume-capsule DIR resumes DIR/frontier.json into the same capsule"
            );
            println!(
                "  optional: --continue-capsule DIR --continue-slices N runs repeated wall slices"
            );
            println!(
                "  optional: --checkpoint-before-combat-portfolio saves a resumable combat portfolio checkpoint"
            );
            println!("branch_tiny --probe-event-owner EVENT [--probe-event-screen N]");
            println!(
                "  owner-audit runner; ordinary combat uses diagnostic rescue-search, boss combat retries with boss-search"
            );
            println!(
                "  combat-gap cases default to a sibling combat_gap_cases directory when an artifact path is supplied; override with --combat-gap-case-dir PATH"
            );
            std::process::exit(0);
        }
        if !matches!(
            key,
            "--seed"
                | "--objective"
                | "--ascension"
                | "--a"
                | "--generations"
                | "--layers"
                | "--max-branches"
                | "--auto-ops"
                | "--search-nodes"
                | "--search-ms"
                | "--rescue-search-nodes"
                | "--rescue-search-ms"
                | "--boss-search-nodes"
                | "--boss-search-ms"
                | "--wall-ms"
                | "--trace-jsonl"
                | "--combat-gap-case-dir"
                | "--frontier-checkpoint"
                | "--resume-frontier"
                | "--run-capsule"
                | "--resume-capsule"
                | "--continue-capsule"
                | "--continue-slices"
                | "--checkpoint-before-combat-portfolio"
                | "--probe-event-owner"
                | "--probe-event-screen"
        ) {
            return Err(format!("unknown argument {key}"));
        }
        if key == "--checkpoint-before-combat-portfolio" {
            args.checkpoint_before_combat_portfolio = true;
            overrides.checkpoint_before_combat_portfolio = true;
            index += 1;
            continue;
        }
        let value = raw
            .get(index + 1)
            .ok_or_else(|| format!("{key} requires a value"))?;
        match key {
            "--seed" => args.seed = parse(value, key)?,
            "--objective" => {
                args.objective = RunObjective::parse(value)?;
                overrides.objective = Some(args.objective);
            }
            "--ascension" | "--a" => args.ascension = parse(value, key)?,
            "--generations" | "--layers" => {
                args.generations = parse(value, key)?;
                overrides.generations = Some(args.generations);
            }
            "--max-branches" => {
                args.max_branches = parse(value, key)?;
                overrides.max_branches = Some(args.max_branches);
            }
            "--auto-ops" => {
                args.auto_ops = parse(value, key)?;
                overrides.auto_ops = Some(args.auto_ops);
            }
            "--search-nodes" => {
                args.search_nodes = parse(value, key)?;
                overrides.search_nodes = Some(args.search_nodes);
            }
            "--search-ms" => {
                args.search_ms = parse(value, key)?;
                overrides.search_ms = Some(args.search_ms);
            }
            "--rescue-search-nodes" => {
                args.rescue_search_nodes = parse(value, key)?;
                overrides.rescue_search_nodes = Some(args.rescue_search_nodes);
            }
            "--rescue-search-ms" => {
                args.rescue_search_ms = parse(value, key)?;
                overrides.rescue_search_ms = Some(args.rescue_search_ms);
            }
            "--boss-search-nodes" => {
                args.boss_search_nodes = parse(value, key)?;
                overrides.boss_search_nodes = Some(args.boss_search_nodes);
            }
            "--boss-search-ms" => {
                args.boss_search_ms = parse(value, key)?;
                overrides.boss_search_ms = Some(args.boss_search_ms);
            }
            "--wall-ms" => {
                args.wall_ms = Some(parse(value, key)?);
                overrides.wall_ms = args.wall_ms;
            }
            "--trace-jsonl" => trace_jsonl = Some(PathBuf::from(value)),
            "--combat-gap-case-dir" => combat_gap_case_dir = Some(PathBuf::from(value)),
            "--frontier-checkpoint" => frontier_checkpoint = Some(PathBuf::from(value)),
            "--resume-frontier" => resume_frontier = Some(PathBuf::from(value)),
            "--run-capsule" => run_capsule = Some(PathBuf::from(value)),
            "--resume-capsule" => resume_capsule = Some(PathBuf::from(value)),
            "--continue-capsule" => continue_capsule = Some(PathBuf::from(value)),
            "--continue-slices" => continue_slices = Some(parse(value, key)?),
            "--probe-event-owner" => probe_event_owner = Some(parse_event_id(value)?),
            "--probe-event-screen" => probe_event_screen = parse(value, key)?,
            _ => unreachable!("argument key was validated before value parsing"),
        }
        index += 2;
    }
    if continue_slices.is_some() && continue_capsule.is_none() {
        return Err("--continue-slices requires --continue-capsule".to_string());
    }
    if continue_capsule.is_some()
        && (resume_capsule.is_some()
            || resume_frontier.is_some()
            || run_capsule.is_some()
            || frontier_checkpoint.is_some())
    {
        return Err(
            "--continue-capsule cannot be combined with resume/run checkpoint flags".to_string(),
        );
    }
    let continue_capsule = continue_capsule
        .map(|capsule| {
            let max_slices = continue_slices.unwrap_or(1);
            if max_slices == 0 {
                return Err("--continue-slices must be greater than zero".to_string());
            }
            Ok(ContinueCapsuleArgs {
                capsule,
                max_slices,
            })
        })
        .transpose()?;
    Ok((
        args,
        overrides,
        trace_jsonl,
        combat_gap_case_dir,
        frontier_checkpoint,
        resume_frontier,
        run_capsule,
        resume_capsule,
        continue_capsule,
        probe_event_owner.map(|event_id| EventOwnerProbeArgs {
            event_id,
            screen: probe_event_screen,
        }),
    ))
}

fn parse_event_id(value: &str) -> Result<EventId, String> {
    sts_simulator::engine::event_handler::event_id_from_name(value)
        .or_else(|| {
            sts_simulator::engine::event_handler::event_id_from_name(&value.replace('_', " "))
        })
        .ok_or_else(|| format!("unknown event for --probe-event-owner: {value}"))
}

fn parse<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {key}: {value}"))
}

pub(super) fn default_combat_gap_case_dir(
    trace_path: Option<&PathBuf>,
    frontier_checkpoint_path: Option<&PathBuf>,
    resume_frontier: Option<&PathBuf>,
) -> Option<PathBuf> {
    trace_path
        .or(frontier_checkpoint_path)
        .or(resume_frontier)
        .and_then(|path| path.parent().map(|parent| parent.join("combat_gap_cases")))
}
