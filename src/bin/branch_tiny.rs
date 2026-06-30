use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sts_simulator::eval::run_control::{
    build_decision_surface, CombatSearchTraceSummary, RewardAutomationConfig,
    RunControlAutoAppliedStepV1, RunControlConfig, RunControlSession,
};
use sts_simulator::state::core::EngineState;
use sts_simulator::state::events::{EventId, EventState};

#[path = "branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "branch_tiny/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "branch_tiny/owners.rs"]
mod owners;
#[path = "branch_tiny/render.rs"]
mod render;
#[path = "branch_tiny/run_capsule.rs"]
mod run_capsule;
#[path = "branch_tiny/runner.rs"]
mod runner;
#[path = "branch_tiny/trace.rs"]
mod trace;

use owners::{ChoiceAnnotation, DecisionKey, OwnerChoice, OwnerDecision, OwnerRoutine};
use run_capsule::{RunCapsule, RunCapsuleSave};

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
    #[serde(skip)]
    wall_capped_search_budget: bool,
    #[serde(skip)]
    wall_capped_boss_budget: bool,
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

#[derive(Clone, Copy)]
struct EventOwnerProbeArgs {
    event_id: EventId,
    screen: usize,
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
        mut combat_gap_case_dir,
        frontier_checkpoint_path,
        resume_frontier,
        run_capsule_path,
        event_owner_probe,
    ) = parse_args()?;
    if let Some(probe) = event_owner_probe {
        return run_event_owner_probe(args, probe);
    }
    let run_capsule = run_capsule_path.map(RunCapsule::new);
    if combat_gap_case_dir.is_none() {
        combat_gap_case_dir = run_capsule
            .as_ref()
            .map(RunCapsule::combat_cases_dir)
            .or_else(|| {
                default_combat_gap_case_dir(
                    trace_path.as_ref(),
                    frontier_checkpoint_path.as_ref(),
                    resume_frontier.as_ref(),
                )
            });
    }
    if let Some(capsule) = run_capsule.as_ref() {
        capsule.write_running_manifest(args)?;
    }
    let mut capsule_frontier_saved = false;
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
    let mut recent_expanded_keys = Vec::new();

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
    let mut last_generation = generation_start;
    for generation in generation_start..=args.generations {
        last_generation = generation;
        let mut generation_result = None;
        if deadline.should_soft_stop(args) {
            capsule_frontier_saved |= save_wall_stop(
                frontier_checkpoint_output_path(
                    &frontier_checkpoint_path,
                    &resume_frontier,
                    run_capsule.as_ref(),
                ),
                run_capsule.as_ref(),
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
            let (branch, expandable, choices) =
                prepare_branch_work(branch, args, generation, deadline);
            work.push((branch, expandable, choices));
        }
        let expanded_masks = expansion_masks(&work, args.max_branches, &mut recent_expanded_keys);
        let total_expanded = expanded_masks
            .iter()
            .flatten()
            .filter(|expanded| **expanded)
            .count();
        if total_expanded > 0 && deadline.would_cap_core_search(args, total_expanded) {
            frontier = work
                .into_iter()
                .map(|(branch, _, _)| branch)
                .collect::<VecDeque<_>>();
            capsule_frontier_saved |= save_wall_stop(
                frontier_checkpoint_output_path(
                    &frontier_checkpoint_path,
                    &resume_frontier,
                    run_capsule.as_ref(),
                ),
                run_capsule.as_ref(),
                args,
                generation,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
            break;
        }
        let child_args = deadline.cap_args(args, total_expanded.max(1));
        for ((branch, expandable, choices), expanded_mask) in work.into_iter().zip(expanded_masks) {
            render::print_branch_timeline(generation, &branch, &choices, &expanded_mask);
            if let Some(trace) = trace.as_mut() {
                trace.record_node(generation, &branch, &choices, &expanded_mask)?;
            }
            if let Some(dir) = combat_gap_case_dir.as_ref() {
                if matches!(
                    branch.status,
                    BranchStatus::CombatGap { .. } | BranchStatus::BudgetGap { .. }
                ) {
                    match combat_gap_case::save_combat_gap_case(dir, args, generation, &branch) {
                        Ok(Some(path)) => println!("  combat_gap_case: {}", path.display()),
                        Ok(None) => {}
                        Err(err) => println!("  combat_gap_case_error: {}", render::one_line(&err)),
                    }
                }
            }
            if !expandable {
                if let Some(trace) = trace.as_mut() {
                    trace.record_branch_snapshot(generation, "stopped", &branch)?;
                }
                if matches!(branch.status, BranchStatus::Running { .. }) {
                    deferred.push_back(branch);
                    continue;
                }
                generation_result = Some((generation, branch.clone()));
                continue;
            }
            if !expanded_mask.iter().any(|expanded| *expanded) {
                deferred.push_back(branch);
                continue;
            }
            for child in expand_registered_owner(
                &branch,
                child_args,
                deadline,
                choices
                    .into_iter()
                    .enumerate()
                    .filter(|(index, _)| expanded_mask[*index])
                    .map(|(_, choice)| choice),
                &mut next_branch_id,
            ) {
                next.push_back(child);
            }
        }
        next.append(&mut deferred);
        retain_frontier(&mut next, args.max_branches);
        if next.is_empty() {
            if let (Some(capsule), Some((result_generation, branch))) =
                (run_capsule.as_ref(), generation_result.as_ref())
            {
                capsule.save_result(args, *result_generation, branch)?;
                println!("run_capsule_result: {}", capsule.result_path().display());
            }
            break;
        }
        frontier = next;
        if deadline.should_soft_stop(args) {
            capsule_frontier_saved |= save_wall_stop(
                frontier_checkpoint_output_path(
                    &frontier_checkpoint_path,
                    &resume_frontier,
                    run_capsule.as_ref(),
                ),
                run_capsule.as_ref(),
                args,
                generation + 1,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
            break;
        }
    }
    if let Some(trace) = trace.as_mut() {
        trace.record_frontier_snapshot(last_generation, &frontier)?;
    }
    if let Some(capsule) = run_capsule.as_ref().filter(|_| !capsule_frontier_saved) {
        print_capsule_save(
            capsule.save_recovery(args, last_generation, next_branch_id, &frontier)?,
            capsule,
        );
    }
    Ok(())
}

fn prepare_branch_work(
    mut branch: Branch,
    args: Args,
    generation: usize,
    deadline: RunDeadline,
) -> (Branch, bool, Vec<OwnerChoice>) {
    let mut expandable =
        generation < args.generations && matches!(branch.status, BranchStatus::Running { .. });
    let mut choices = if expandable {
        branch_owner_choices(&branch)
    } else {
        Vec::new()
    };
    if expandable && choices.is_empty() {
        let advance = runner::advance_to_owner_or_gap(
            &mut branch.session,
            deadline.cap_args(args, 1),
            deadline,
        );
        branch.status = advance.status;
        branch.boss_retry = advance.boss_retry;
        branch.auto_steps = advance.auto_steps;
        branch.combat_search = advance.combat_search;
        expandable =
            generation < args.generations && matches!(branch.status, BranchStatus::Running { .. });
        choices = if expandable {
            branch_owner_choices(&branch)
        } else {
            Vec::new()
        };
    }
    (branch, expandable, choices)
}

fn expansion_masks(
    work: &[(Branch, bool, Vec<OwnerChoice>)],
    max_branches: usize,
    recent_expanded_keys: &mut Vec<DecisionKey>,
) -> Vec<Vec<bool>> {
    let mut expanded = work
        .iter()
        .map(|(_, _, choices)| vec![false; choices.len()])
        .collect::<Vec<_>>();
    let mut remaining = max_branches;
    let mut prefer_unused_keys = false;
    while remaining > 0 {
        let mut progressed = false;
        for (branch_index, (_, expandable, choices)) in work.iter().enumerate() {
            if !*expandable {
                continue;
            }
            let Some(choice_index) = next_expansion_choice(
                choices,
                &expanded[branch_index],
                recent_expanded_keys,
                prefer_unused_keys,
            ) else {
                continue;
            };
            expanded[branch_index][choice_index] = true;
            if let Some(key) = choices[choice_index].key.clone() {
                recent_expanded_keys.push(key);
            }
            remaining -= 1;
            progressed = true;
            if remaining == 0 {
                break;
            }
        }
        if !progressed {
            break;
        }
        prefer_unused_keys = true;
    }
    trim_recent_expanded_keys(recent_expanded_keys);
    expanded
}

fn trim_recent_expanded_keys(keys: &mut Vec<DecisionKey>) {
    const RECENT_KEY_LIMIT: usize = 64;
    if keys.len() > RECENT_KEY_LIMIT {
        keys.drain(0..keys.len() - RECENT_KEY_LIMIT);
    }
}

fn next_expansion_choice(
    choices: &[OwnerChoice],
    expanded: &[bool],
    used_keys: &[DecisionKey],
    prefer_unused_keys: bool,
) -> Option<usize> {
    let candidates = choices
        .iter()
        .enumerate()
        .filter(|(index, choice)| choice.auto_expand_allowed() && !expanded[*index]);
    if prefer_unused_keys {
        if let Some((index, _)) = candidates.clone().find(|(_, choice)| {
            choice
                .key
                .as_ref()
                .is_some_and(|key| !used_keys.contains(key))
        }) {
            return Some(index);
        }
    }
    candidates.map(|(index, _)| index).next()
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

    fn should_soft_stop(self, args: Args) -> bool {
        self.remaining_ms()
            .is_some_and(|remaining| remaining <= self.soft_stop_guard_ms(args))
    }

    fn would_cap_core_search(self, args: Args, child_count: usize) -> bool {
        self.cap_args(args, child_count).wall_capped_search_budget
    }

    fn soft_stop_guard_ms(self, args: Args) -> u64 {
        WALL_STOP_GUARD_MS + args.search_ms.max(args.rescue_search_ms)
    }

    fn cap_args(self, mut args: Args, child_count: usize) -> Args {
        let Some(remaining) = self.remaining_ms() else {
            return args;
        };
        let per_child =
            (remaining.saturating_sub(WALL_STOP_GUARD_MS) / child_count.max(1) as u64).max(1);
        let search_ms = args.search_ms.min(per_child);
        let rescue_search_ms = args.rescue_search_ms.min(per_child);
        let boss_search_ms = args.boss_search_ms.min(per_child);
        if search_ms != args.search_ms || rescue_search_ms != args.rescue_search_ms {
            args.wall_capped_search_budget = true;
        }
        if boss_search_ms != args.boss_search_ms {
            args.wall_capped_boss_budget = true;
        }
        args.search_ms = search_ms;
        args.rescue_search_ms = rescue_search_ms;
        args.boss_search_ms = boss_search_ms;
        args
    }

    fn remaining_ms(self) -> Option<u64> {
        self.0
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .map(|remaining| remaining.as_millis().min(u128::from(u64::MAX)) as u64)
    }
}

fn frontier_checkpoint_output_path<'a>(
    frontier_checkpoint_path: &'a Option<PathBuf>,
    resume_frontier: &'a Option<PathBuf>,
    capsule: Option<&RunCapsule>,
) -> Option<&'a PathBuf> {
    if frontier_checkpoint_path.is_some() {
        return frontier_checkpoint_path.as_ref();
    }
    if capsule.is_some() {
        return None;
    }
    resume_frontier.as_ref()
}

fn save_wall_stop(
    path: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
    deadline: &RunDeadline,
) -> Result<bool, String> {
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
            return Ok(false);
        }
        frontier_checkpoint::save(path, args, generation, next_branch_id, frontier)?;
        println!(
            "frontier_checkpoint: {} running={}",
            path.display(),
            running
        );
    } else if capsule.is_none() {
        println!("wall_soft_stop reached without --frontier-checkpoint");
    }
    if let Some(capsule) = capsule {
        return Ok(print_capsule_save(
            capsule.save_recovery(args, generation, next_branch_id, frontier)?,
            capsule,
        ));
    }
    Ok(false)
}

fn print_capsule_save(save: RunCapsuleSave, capsule: &RunCapsule) -> bool {
    match save {
        RunCapsuleSave::None => false,
        RunCapsuleSave::Frontier { running } => {
            println!("run_capsule_frontier: running={running}");
            true
        }
        RunCapsuleSave::Result => {
            println!("run_capsule_result: {}", capsule.result_path().display());
            true
        }
    }
}

fn branch_owner_choices(branch: &Branch) -> Vec<OwnerChoice> {
    let owner = match &branch.status {
        BranchStatus::Running { owner, .. } => *owner,
        _ => return Vec::new(),
    };
    let surface = build_decision_surface(&branch.session);
    match owners::owner_decision(&branch.session, owner, &surface) {
        OwnerDecision::Candidates(choices) => choices,
        OwnerDecision::Routine(_) | OwnerDecision::Gap(_) => Vec::new(),
    }
}

fn run_event_owner_probe(args: Args, probe: EventOwnerProbeArgs) -> Result<(), String> {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: args.seed,
        ascension_level: args.ascension,
        ..Default::default()
    });
    let mut event_state = EventState::new(probe.event_id);
    event_state.current_screen = probe.screen;
    session.run_state.event_state = Some(event_state);
    session.engine_state = EngineState::EventRoom;

    let surface = build_decision_surface(&session);
    println!(
        "event_owner_probe event={:?} screen={} candidates={}",
        probe.event_id,
        probe.screen,
        surface.view.candidates.len()
    );
    for candidate in &surface.view.candidates {
        println!(
            "  candidate id={} key={:?} label={} command={:?}",
            candidate.id,
            candidate.key,
            candidate.label,
            candidate.action.executable_command()
        );
    }

    match owners::owner_decision(&session, Owner::Event(probe.event_id), &surface) {
        OwnerDecision::Routine(OwnerRoutine::Command(command)) => {
            println!("  owner_decision=command {command:?}");
        }
        OwnerDecision::Routine(OwnerRoutine::RewardTinyAutomation) => {
            println!("  owner_decision=unexpected_reward_tiny_automation");
        }
        OwnerDecision::Routine(OwnerRoutine::AdvanceEmptyCampfire) => {
            println!("  owner_decision=unexpected_advance_empty_campfire");
        }
        OwnerDecision::Candidates(choices) => {
            println!("  owner_decision=candidates count={}", choices.len());
            for choice in choices {
                println!(
                    "    choice key={:?} label={} command={:?}",
                    choice.key, choice.label, choice.action
                );
            }
        }
        OwnerDecision::Gap(reason) => {
            println!("  owner_decision=gap {reason}");
        }
    }
    Ok(())
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
        Option<PathBuf>,
        Option<EventOwnerProbeArgs>,
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
        wall_capped_search_budget: false,
        wall_capped_boss_budget: false,
    };
    let mut overrides = ArgsOverrides::default();
    let mut trace_jsonl = None;
    let mut combat_gap_case_dir = None;
    let mut frontier_checkpoint = None;
    let mut resume_frontier = None;
    let mut run_capsule = None;
    let mut probe_event_owner = None;
    let mut probe_event_screen = 0usize;
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!(
                "branch_tiny --seed N --generations N --max-branches N [--wall-ms N] [--trace-jsonl PATH] [--frontier-checkpoint PATH] [--resume-frontier PATH]"
            );
            println!("  optional: --run-capsule DIR writes manifest/frontier/result/path JSON");
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
                | "--probe-event-owner"
                | "--probe-event-screen"
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
            "--run-capsule" => run_capsule = Some(PathBuf::from(value)),
            "--probe-event-owner" => probe_event_owner = Some(parse_event_id(value)?),
            "--probe-event-screen" => probe_event_screen = parse(value, key)?,
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
        run_capsule,
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

fn default_combat_gap_case_dir(
    trace_path: Option<&PathBuf>,
    frontier_checkpoint_path: Option<&PathBuf>,
    resume_frontier: Option<&PathBuf>,
) -> Option<PathBuf> {
    trace_path
        .or(frontier_checkpoint_path)
        .or(resume_frontier)
        .and_then(|path| path.parent().map(|parent| parent.join("combat_gap_cases")))
}
