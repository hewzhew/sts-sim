use std::collections::VecDeque;
use std::path::PathBuf;

use sts_simulator::eval::run_control::{
    build_decision_surface, CombatSearchTraceSummary, RewardAutomationConfig,
    RunControlAutoAppliedStepV1, RunControlCommand, RunControlConfig, RunControlSession,
};
use sts_simulator::state::events::EventId;

#[path = "branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "branch_tiny/owners.rs"]
mod owners;
#[path = "branch_tiny/render.rs"]
mod render;
#[path = "branch_tiny/runner.rs"]
mod runner;
#[path = "branch_tiny/trace.rs"]
mod trace;

use owners::{ChoiceAnnotation, DecisionKey, OwnerChoice};

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
    action: RunControlCommand,
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

#[derive(Clone, Copy, Debug)]
enum Owner {
    NeowStart,
    CardReward,
    BossRelic,
    Event(EventId),
    RewardTiny,
    ShopTiny,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy)]
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
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (args, trace_path, combat_gap_case_dir) = parse_args()?;
    let mut trace = trace_path
        .as_ref()
        .map(|path| trace::TraceWriter::create(path))
        .transpose()?;
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
    let advance = runner::advance_to_owner_or_gap(&mut session, args);
    let mut frontier = VecDeque::from([Branch {
        id: 0,
        parent_id: None,
        path: Vec::new(),
        session,
        status: advance.status,
        boss_retry: advance.boss_retry,
        auto_steps: advance.auto_steps,
        combat_search: advance.combat_search,
    }]);
    let mut next_branch_id = 1usize;

    println!(
        "branch_tiny seed={} ascension={} generations={} max_branches={} mode=owner_audit render=timeline",
        args.seed, args.ascension, args.generations, args.max_branches
    );
    println!(
        "branch cap: {}; search={}nodes/{}ms; rescue={}nodes/{}ms diagnostic+tactical; boss_retry={}nodes/{}ms; '>' marks expanded choices",
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
    for generation in 0..=args.generations {
        let mut next = VecDeque::new();
        while let Some(branch) = frontier.pop_front() {
            let expandable = generation < args.generations
                && matches!(branch.status, BranchStatus::Running { .. });
            let choices = if expandable {
                branch_owner_choices(&branch)
            } else {
                Vec::new()
            };
            let expanded = choices
                .iter()
                .filter(|choice| choice.auto_expand_allowed())
                .count()
                .min(args.max_branches.saturating_sub(next.len()));
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
            for child in expand_registered_owner(
                &branch,
                args,
                choices
                    .into_iter()
                    .filter(|choice| choice.auto_expand_allowed())
                    .take(expanded),
                &mut next_branch_id,
            ) {
                next.push_back(child);
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
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
    candidates: impl IntoIterator<Item = OwnerChoice>,
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    let mut children = Vec::new();
    for choice in candidates {
        let mut session = branch.session.clone();
        let advance = match session.apply_command(choice.action.clone()) {
            Ok(_) => runner::advance_to_owner_or_gap(&mut session, args),
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
            action: choice.action,
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

fn parse_args() -> Result<(Args, Option<PathBuf>, Option<PathBuf>), String> {
    let mut args = Args {
        seed: 1,
        ascension: 0,
        generations: 2,
        max_branches: 24,
        auto_ops: 64,
        search_nodes: 50_000,
        search_ms: 500,
        rescue_search_nodes: 200_000,
        rescue_search_ms: 2_000,
        boss_search_nodes: 800_000,
        boss_search_ms: 8_000,
    };
    let mut trace_jsonl = None;
    let mut combat_gap_case_dir = None;
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!(
                "branch_tiny --seed N --generations N --max-branches N [--search-ms N] [--rescue-search-ms N] [--boss-search-ms N] [--trace-jsonl PATH] [--combat-gap-case-dir DIR]"
            );
            println!(
                "  owner-audit runner; ordinary combat fast-search escalates through diagnostic/tactical rescue-search, boss combat retries with boss-search"
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
                | "--trace-jsonl"
                | "--combat-gap-case-dir"
        ) {
            return Err(format!("unknown argument {key}"));
        }
        let value = raw
            .get(index + 1)
            .ok_or_else(|| format!("{key} requires a value"))?;
        match key {
            "--seed" => args.seed = parse(value, key)?,
            "--ascension" | "--a" => args.ascension = parse(value, key)?,
            "--generations" | "--layers" => args.generations = parse(value, key)?,
            "--max-branches" => args.max_branches = parse(value, key)?,
            "--auto-ops" => args.auto_ops = parse(value, key)?,
            "--search-nodes" => args.search_nodes = parse(value, key)?,
            "--search-ms" => args.search_ms = parse(value, key)?,
            "--rescue-search-nodes" => args.rescue_search_nodes = parse(value, key)?,
            "--rescue-search-ms" => args.rescue_search_ms = parse(value, key)?,
            "--boss-search-nodes" => args.boss_search_nodes = parse(value, key)?,
            "--boss-search-ms" => args.boss_search_ms = parse(value, key)?,
            "--trace-jsonl" => trace_jsonl = Some(PathBuf::from(value)),
            "--combat-gap-case-dir" => combat_gap_case_dir = Some(PathBuf::from(value)),
            _ => unreachable!("argument key was validated before value parsing"),
        }
        index += 2;
    }
    Ok((args, trace_jsonl, combat_gap_case_dir))
}

fn parse<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {key}: {value}"))
}
