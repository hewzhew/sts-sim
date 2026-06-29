use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sts_simulator::eval::run_control::{
    build_decision_surface, CombatSearchTraceSummary, RewardAutomationConfig,
    RunControlAutoAppliedStepV1, RunControlConfig, RunControlSession,
};
use sts_simulator::state::events::EventId;

#[path = "branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "branch_tiny/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "branch_tiny/owners.rs"]
mod owners;
#[path = "branch_tiny/render.rs"]
mod render;
#[path = "branch_tiny/runner.rs"]
mod runner;
#[path = "branch_tiny/trace.rs"]
mod trace;

use owners::{ChoiceAnnotation, DecisionKey, OwnerChoice};

const WALL_STOP_GUARD_MS: u64 = 1_500;

#[derive(Clone)]
struct Branch {
    id: usize,
    parent_id: Option<usize>,
    path: Vec<BranchPathStep>,
    session: RunControlSession,
    status: BranchStatus,
    boss_retry: Option<BossRetryReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
}

#[derive(Clone)]
struct BossRetryReport {
    status: BossRetryStatus,
    max_nodes: usize,
    wall_ms: u64,
    action_keys: Vec<String>,
    attempts: Vec<BossRetryAttemptReport>,
}

#[derive(Clone)]
struct BossRetryAttemptReport {
    label: &'static str,
    status: BossRetryStatus,
    max_nodes: usize,
    wall_ms: u64,
    potion_policy: &'static str,
    max_potions_used: Option<u32>,
    action_keys: Vec<String>,
}

#[derive(Clone)]
enum BossRetryStatus {
    Failed(String),
    Advanced(String),
    Terminal(&'static str),
}

#[derive(Clone)]
struct BranchPathStep {
    key: Option<DecisionKey>,
    action_debug: String,
    label: String,
    annotation: ChoiceAnnotation,
}

#[derive(Clone)]
enum BranchStatus {
    Running {
        boundary: String,
        owner: Owner,
    },
    Terminal(&'static str),
    AutomationGap {
        boundary: String,
        site: BoundarySite,
    },
    CombatGap {
        boundary: String,
        reason: String,
    },
    BudgetGap {
        boundary: String,
        reason: String,
    },
    ApplyFailed(String),
    AdvanceFailed(String),
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
enum Owner {
    NeowStart,
    CardReward,
    BossRelic,
    Event(EventId),
    RewardTiny,
    ShopTiny,
    Campfire,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
enum BoundarySite {
    Event(EventId),
    Reward,
    Shop,
    Route,
    Campfire,
    BossRelic,
    RunChoice,
    Treasure,
    Terminal,
    Unknown,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
struct Args {
    seed: u64,
    ascension: u8,
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
}

#[derive(Default)]
struct ArgsOverrides {
    generations: Option<usize>,
    max_branches: Option<usize>,
    auto_ops: Option<usize>,
    search_nodes: Option<usize>,
    search_ms: Option<u64>,
    rescue_search_nodes: Option<usize>,
    rescue_search_ms: Option<u64>,
    boss_search_nodes: Option<usize>,
    boss_search_ms: Option<u64>,
    wall_ms: Option<u64>,
}

impl ArgsOverrides {
    fn apply_to(self, args: &mut Args) {
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
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (
        mut args,
        overrides,
        trace_path,
        combat_gap_case_dir,
        frontier_checkpoint_path,
        resume_frontier,
    ) = parse_args()?;
    let started = Instant::now();
    let mut trace = trace_path
        .as_ref()
        .map(|path| trace::TraceWriter::create(path))
        .transpose()?;
    let mut generation_start = 0usize;
    let (mut frontier, mut next_branch_id) = if let Some(path) = resume_frontier.as_ref() {
        let checkpoint = frontier_checkpoint::load(path)?;
        args = checkpoint.args;
        overrides.apply_to(&mut args);
        generation_start = checkpoint.generation;
        checkpoint.into_frontier()?
    } else {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: args.seed,
            ascension_level: args.ascension,
            reward_automation: RewardAutomationConfig {
                claim_gold: true,
                claim_potion_with_empty_slot: true,
                claim_safe_relic_without_sapphire_key: true,
            },
            ..Default::default()
        });
        let deadline = RunDeadline::new(started, args.wall_ms);
        let advance =
            runner::advance_to_owner_or_gap(&mut session, deadline.cap_args(args, 1), deadline);
        (
            VecDeque::from([Branch {
                id: 0,
                parent_id: None,
                path: Vec::new(),
                session,
                status: advance.status,
                boss_retry: advance.boss_retry,
                auto_steps: advance.auto_steps,
                combat_search: advance.combat_search,
            }]),
            1usize,
        )
    };
    let deadline = RunDeadline::new(started, args.wall_ms);

    println!(
        "branch_tiny seed={} ascension={} generations={} max_branches={} mode=owner_audit render=timeline{}",
        args.seed,
        args.ascension,
        args.generations,
        args.max_branches,
        if resume_frontier.is_some() { " resume=frontier" } else { "" }
    );
    println!(
        "branch cap: {}; search={}nodes/{}ms; rescue={}nodes/{}ms diagnostic; boss_retry={}nodes/{}ms; '>' marks expanded choices",
        args.max_branches,
        args.search_nodes,
        args.search_ms,
        args.rescue_search_nodes,
        args.rescue_search_ms,
        args.boss_search_nodes,
        args.boss_search_ms
    );
    if let Some(trace) = trace.as_mut() {
        trace.record_run(args)?;
    }
    for generation in generation_start..=args.generations {
        if deadline.should_stop() {
            save_wall_stop(
                checkpoint_path(&frontier_checkpoint_path, &resume_frontier),
                args,
                generation,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
            break;
        }
        let mut next = VecDeque::new();
        let mut deferred = VecDeque::new();
        let mut work = Vec::new();
        while let Some(branch) = frontier.pop_front() {
            let expandable = generation < args.generations
                && matches!(branch.status, BranchStatus::Running { .. });
            let choices = if expandable {
                branch_owner_choices(&branch)
            } else {
                Vec::new()
            };
            work.push((branch, expandable, choices));
        }
        let expanded_counts = expansion_counts(&work, args.max_branches);
        let total_expanded = expanded_counts.iter().sum::<usize>();
        let child_args = deadline.cap_args(args, total_expanded.max(1));
        for ((branch, expandable, choices), expanded) in work.into_iter().zip(expanded_counts) {
            render::print_branch_timeline(generation, &branch, &choices, expanded);
            if let Some(trace) = trace.as_mut() {
                trace.record_node(generation, &branch, &choices, expanded)?;
            }
            if let Some(dir) = combat_gap_case_dir.as_ref() {
                if matches!(branch.status, BranchStatus::CombatGap { .. }) {
                    match combat_gap_case::save_combat_gap_case(dir, args, generation, &branch) {
                        Ok(Some(path)) => println!("  combat_gap_case: {}", path.display()),
                        Ok(None) => {}
                        Err(err) => println!("  combat_gap_case_error: {}", render::one_line(&err)),
                    }
                }
            }
            if !expandable {
                continue;
            }
            if expanded == 0 {
                deferred.push_back(branch);
                continue;
            }
            for child in expand_registered_owner(
                &branch,
                child_args,
                deadline,
                choices
                    .into_iter()
                    .filter(|choice| choice.auto_expand_allowed())
                    .take(expanded),
                &mut next_branch_id,
            ) {
                next.push_back(child);
            }
        }
        next.append(&mut deferred);
        retain_frontier(&mut next, args.max_branches);
        if next.is_empty() {
            break;
        }
        frontier = next;
        if deadline.should_stop() {
            save_wall_stop(
                checkpoint_path(&frontier_checkpoint_path, &resume_frontier),
                args,
                generation + 1,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
            break;
        }
    }
    Ok(())
}

fn expansion_counts(work: &[(Branch, bool, Vec<OwnerChoice>)], max_branches: usize) -> Vec<usize> {
    let auto_counts = work
        .iter()
        .map(|(_, expandable, choices)| {
            if *expandable {
                choices
                    .iter()
                    .filter(|choice| choice.auto_expand_allowed())
                    .count()
            } else {
                0
            }
        })
        .collect::<Vec<_>>();
    let mut expanded = vec![0usize; work.len()];
    let mut remaining = max_branches;
    while remaining > 0 {
        let mut progressed = false;
        for (index, count) in auto_counts.iter().enumerate() {
            if expanded[index] < *count {
                expanded[index] += 1;
                remaining -= 1;
                progressed = true;
                if remaining == 0 {
                    break;
                }
            }
        }
        if !progressed {
            break;
        }
    }
    expanded
}

fn retain_frontier(frontier: &mut VecDeque<Branch>, limit: usize) {
    if frontier.len() <= limit {
        return;
    }
    let mut branches = frontier.drain(..).collect::<Vec<_>>();
    branches.sort_by(|a, b| {
        frontier_retention_key(b)
            .cmp(&frontier_retention_key(a))
            .then_with(|| a.id.cmp(&b.id))
    });
    branches.truncate(limit);
    *frontier = branches.into();
}

fn frontier_retention_key(branch: &Branch) -> (u8, u8, i32, u32, i32) {
    let status = match branch.status {
        BranchStatus::Running { .. } => 3,
        BranchStatus::Terminal("win") => 2,
        BranchStatus::CombatGap { .. } | BranchStatus::BudgetGap { .. } => 1,
        BranchStatus::Terminal(_)
        | BranchStatus::AutomationGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => 0,
    };
    let hp = branch.session.run_state.current_hp;
    let max_hp = branch.session.run_state.max_hp.max(1);
    let hp_ratio = (hp.max(0) as u32).saturating_mul(1000) / max_hp as u32;
    (
        status,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        hp_ratio,
        hp,
    )
}

#[derive(Clone, Copy)]
struct RunDeadline(Option<Instant>);

impl RunDeadline {
    fn new(started: Instant, wall_ms: Option<u64>) -> Self {
        Self(wall_ms.map(|ms| started + Duration::from_millis(ms)))
    }

    fn should_stop(self) -> bool {
        self.remaining_ms()
            .is_some_and(|remaining| remaining <= WALL_STOP_GUARD_MS)
    }

    fn cap_args(self, mut args: Args, child_count: usize) -> Args {
        let Some(remaining) = self.remaining_ms() else {
            return args;
        };
        let per_child =
            (remaining.saturating_sub(WALL_STOP_GUARD_MS) / child_count.max(1) as u64).max(1);
        args.search_ms = args.search_ms.min(per_child);
        args.rescue_search_ms = args.rescue_search_ms.min(per_child);
        args.boss_search_ms = args.boss_search_ms.min(per_child);
        args
    }

    fn remaining_ms(self) -> Option<u64> {
        self.0
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .map(|remaining| remaining.as_millis().min(u128::from(u64::MAX)) as u64)
    }
}

fn checkpoint_path<'a>(
    frontier_checkpoint_path: &'a Option<PathBuf>,
    resume_frontier: &'a Option<PathBuf>,
) -> Option<&'a PathBuf> {
    frontier_checkpoint_path
        .as_ref()
        .or(resume_frontier.as_ref())
}

fn save_wall_stop(
    path: Option<&PathBuf>,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
    deadline: &RunDeadline,
) -> Result<(), String> {
    println!(
        "wall_soft_stop: generation={} remaining_ms={}",
        generation,
        deadline.remaining_ms().unwrap_or(0)
    );
    if let Some(path) = path {
        let running = frontier
            .iter()
            .filter(|branch| matches!(branch.status, BranchStatus::Running { .. }))
            .count();
        if running == 0 {
            println!("frontier_checkpoint skipped: no running branches");
            return Ok(());
        }
        frontier_checkpoint::save(path, args, generation, next_branch_id, frontier)?;
        println!(
            "frontier_checkpoint: {} running={}",
            path.display(),
            running
        );
    } else {
        println!("wall_soft_stop reached without --frontier-checkpoint");
    }
    Ok(())
}

fn branch_owner_choices(branch: &Branch) -> Vec<OwnerChoice> {
    let owner = match &branch.status {
        BranchStatus::Running { owner, .. } => *owner,
        _ => return Vec::new(),
    };
    let surface = build_decision_surface(&branch.session);
    owners::owner_choices(&branch.session, owner, &surface)
}

fn expand_registered_owner(
    branch: &Branch,
    args: Args,
    deadline: RunDeadline,
    candidates: impl IntoIterator<Item = OwnerChoice>,
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    let mut children = Vec::new();
    for choice in candidates {
        let mut session = branch.session.clone();
        let advance = match session.apply_command(choice.action.clone()) {
            Ok(_) => runner::advance_to_owner_or_gap(&mut session, args, deadline),
            Err(err) => runner::AdvanceResult {
                status: BranchStatus::ApplyFailed(err),
                boss_retry: None,
                auto_steps: Vec::new(),
                combat_search: Vec::new(),
            },
        };
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            key: choice.key,
            action_debug: format!("{:?}", choice.action),
            label: choice.label,
            annotation: choice.annotation,
        });
        let id = *next_branch_id;
        *next_branch_id += 1;
        children.push(Branch {
            id,
            parent_id: Some(branch.id),
            path,
            session,
            status: advance.status,
            boss_retry: advance.boss_retry,
            auto_steps: advance.auto_steps,
            combat_search: advance.combat_search,
        });
    }
    children
}

fn parse_args() -> Result<
    (
        Args,
        ArgsOverrides,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
        Option<PathBuf>,
    ),
    String,
> {
    let mut args = Args {
        seed: 1,
        ascension: 0,
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
    };
    let mut overrides = ArgsOverrides::default();
    let mut trace_jsonl = None;
    let mut combat_gap_case_dir = None;
    let mut frontier_checkpoint = None;
    let mut resume_frontier = None;
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!(
                "branch_tiny --seed N --generations N --max-branches N [--wall-ms N] [--frontier-checkpoint PATH] [--resume-frontier PATH]"
            );
            println!(
                "  owner-audit runner; ordinary combat uses diagnostic rescue-search, boss combat retries with boss-search"
            );
            std::process::exit(0);
        }
        if !matches!(
            key,
            "--seed"
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
        ) {
            return Err(format!("unknown argument {key}"));
        }
        let value = raw
            .get(index + 1)
            .ok_or_else(|| format!("{key} requires a value"))?;
        match key {
            "--seed" => args.seed = parse(value, key)?,
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
            _ => unreachable!("argument key was validated before value parsing"),
        }
        index += 2;
    }
    Ok((
        args,
        overrides,
        trace_jsonl,
        combat_gap_case_dir,
        frontier_checkpoint,
        resume_frontier,
    ))
}

fn parse<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {key}: {value}"))
}
