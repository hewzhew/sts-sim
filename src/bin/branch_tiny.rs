use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::ai::strategy::decision_pipeline::candidate_lane_label;
use sts_simulator::eval::run_control::{
    build_decision_surface, CombatSearchTraceSummary, RewardAutomationConfig,
    RunControlAutoAppliedStepV1, RunControlConfig, RunControlSession,
};
use sts_simulator::state::core::EngineState;
use sts_simulator::state::events::{EventId, EventState};

#[path = "branch_tiny/boundary_router.rs"]
mod boundary_router;
#[path = "branch_tiny/branch_scheduler.rs"]
mod branch_scheduler;
#[path = "branch_tiny/candidate_ir_adapter.rs"]
mod candidate_ir_adapter;
#[path = "branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "branch_tiny/combat_rescue.rs"]
mod combat_rescue;
#[path = "branch_tiny/decision_delta.rs"]
mod decision_delta;
#[path = "branch_tiny/expansion_policy.rs"]
mod expansion_policy;
#[path = "branch_tiny/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "branch_tiny/neow_owner.rs"]
mod neow_owner;
#[path = "branch_tiny/owner_model.rs"]
mod owner_model;
#[path = "branch_tiny/owner_orchestrator.rs"]
mod owner_orchestrator;
#[path = "branch_tiny/owners.rs"]
mod owners;
#[path = "branch_tiny/render.rs"]
mod render;
#[path = "branch_tiny/reward_shop_boss_owner.rs"]
mod reward_shop_boss_owner;
#[path = "branch_tiny/run_capsule.rs"]
mod run_capsule;
#[path = "branch_tiny/run_chain.rs"]
mod run_chain;
#[path = "branch_tiny/run_choice_owner.rs"]
mod run_choice_owner;
#[path = "branch_tiny/run_contract.rs"]
mod run_contract;
#[path = "branch_tiny/run_persistence.rs"]
mod run_persistence;
#[path = "branch_tiny/runner.rs"]
mod runner;
#[path = "branch_tiny/trace.rs"]
mod trace;

use owner_model::{ChoiceAnnotation, DecisionKey, OwnerDecision, OwnerRoutine};
use run_capsule::RunCapsule;
use run_contract::{default_run_objective, RunObjective};

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
    Terminal(TerminalOutcome),
}

#[derive(Clone)]
struct BranchPathStep {
    key: Option<DecisionKey>,
    action_debug: String,
    label: String,
    annotation: ChoiceAnnotationSnapshot,
    state_before: Option<BranchPathState>,
    decision_delta: Option<decision_delta::DecisionDeltaSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BranchPathState {
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    gold: i32,
    deck_size: usize,
    boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ScoreComponentSnapshot {
    by: String,
    value: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ChoiceAnnotationSnapshot {
    None,
    Candidate {
        lane: String,
        score: i32,
        #[serde(default)]
        scores: Vec<ScoreComponentSnapshot>,
        candidate: Value,
        admission: Option<Value>,
        detail: String,
    },
    BossRelic {
        relic: Value,
        lane: String,
        class: String,
        detail: String,
    },
}

impl ChoiceAnnotationSnapshot {
    fn none() -> Self {
        Self::None
    }

    fn from_annotation(annotation: &ChoiceAnnotation) -> Self {
        match annotation {
            ChoiceAnnotation::None => Self::None,
            ChoiceAnnotation::Candidate(decision) => Self::Candidate {
                lane: candidate_lane_label(decision.evaluation.lane).to_string(),
                score: decision.evaluation.total_score(),
                scores: decision
                    .evaluation
                    .scores
                    .iter()
                    .map(|score| ScoreComponentSnapshot {
                        by: score.by.to_string(),
                        value: score.value,
                    })
                    .collect(),
                candidate: trace::candidate_kind_value(decision.evaluation.candidate.kind),
                admission: decision.admission.as_ref().map(|admission| {
                    json!({
                        "card": admission.card,
                        "class": format!("{:?}", admission.class),
                    })
                }),
                detail: render::render_candidate_decision_compact(decision),
            },
            ChoiceAnnotation::BossRelic(admission) => Self::BossRelic {
                relic: json!(admission.relic),
                lane: format!("{:?}", admission.lane),
                class: format!("{:?}", admission.class),
                detail: sts_simulator::ai::strategy::boss_relic_admission::render_boss_relic_admission_compact(admission),
            },
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Candidate { detail, .. } | Self::BossRelic { detail, .. } => Some(detail),
        }
    }
}

impl BranchPathState {
    fn from_branch(branch: &Branch) -> Self {
        let run = &branch.session.run_state;
        Self {
            act: run.act_num,
            floor: run.floor_num,
            hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            deck_size: run.master_deck.len(),
            boundary: branch_status_boundary_label(&branch.status),
        }
    }
}

#[derive(Clone)]
enum BranchStatus {
    Running {
        boundary: String,
        owner: Owner,
    },
    AwaitingAuto {
        boundary: String,
        reason: String,
    },
    Terminal(TerminalOutcome),
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

impl BranchStatus {
    fn is_resumable(&self) -> bool {
        matches!(
            self,
            BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. }
        )
    }

    fn is_expandable_now(&self) -> bool {
        matches!(self, BranchStatus::Running { .. })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum TerminalOutcome {
    Victory,
    Defeat,
}

impl TerminalOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
        }
    }
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
    RunChoice,
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
    #[serde(default = "default_run_objective")]
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
    #[serde(skip)]
    checkpoint_before_boss_retry: bool,
    #[serde(skip)]
    wall_capped_search_budget: bool,
    #[serde(skip)]
    wall_capped_boss_budget: bool,
}

#[derive(Clone, Copy, Default)]
struct ArgsOverrides {
    objective: Option<RunObjective>,
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
    checkpoint_before_boss_retry: bool,
}

#[derive(Clone, Copy)]
struct EventOwnerProbeArgs {
    event_id: EventId,
    screen: usize,
}

struct ContinueCapsuleArgs {
    capsule: PathBuf,
    max_slices: usize,
}

impl ArgsOverrides {
    fn apply_to(self, args: &mut Args) {
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
        if self.checkpoint_before_boss_retry {
            args.checkpoint_before_boss_retry = true;
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
        mut resume_frontier,
        mut run_capsule_path,
        resume_capsule_path,
        continue_capsule,
        event_owner_probe,
    ) = parse_args()?;
    if let Some(continue_capsule) = continue_capsule {
        return run_chain::run(args, overrides, continue_capsule);
    }
    if let Some(path) = resume_capsule_path {
        if resume_frontier.is_some() || run_capsule_path.is_some() {
            return Err(
                "--resume-capsule cannot be combined with --resume-frontier or --run-capsule"
                    .to_string(),
            );
        }
        resume_frontier = Some(path.join("frontier.json"));
        run_capsule_path = Some(path);
    }
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
        "branch_tiny seed={} ascension={} objective={:?} generations={} max_branches={} mode=owner_audit render=timeline{}",
        args.seed,
        args.ascension,
        args.objective,
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
            capsule_frontier_saved |= run_persistence::save_wall_stop(
                run_persistence::frontier_checkpoint_output_path(
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
                branch_scheduler::prepare_branch_work(branch, args, generation, deadline);
            work.push((branch, expandable, choices));
        }
        let expanded_masks =
            branch_scheduler::expansion_masks(&work, args.max_branches, &mut recent_expanded_keys);
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
            capsule_frontier_saved |= run_persistence::save_wall_stop(
                run_persistence::frontier_checkpoint_output_path(
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
                if let Some(capsule) = run_capsule.as_ref() {
                    capsule.save_terminal_result(args, generation, &branch)?;
                }
                if let Some(reason) = run_contract::satisfied(args.objective, &branch.status) {
                    run_persistence::finalize_objective_result(
                        run_capsule.as_ref(),
                        args,
                        generation,
                        &branch,
                        reason.as_str(),
                    )?;
                    return Ok(());
                }
                if branch.status.is_resumable() {
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
            for child in branch_scheduler::expand_registered_owner(
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
                if let Some(capsule) = run_capsule.as_ref() {
                    capsule.save_terminal_result(args, generation + 1, &child)?;
                }
                if let Some(reason) = run_contract::satisfied(args.objective, &child.status) {
                    run_persistence::finalize_objective_result(
                        run_capsule.as_ref(),
                        args,
                        generation + 1,
                        &child,
                        reason.as_str(),
                    )?;
                    return Ok(());
                }
                next.push_back(child);
            }
        }
        next.append(&mut deferred);
        branch_scheduler::retain_frontier(&mut next, args.max_branches);
        if next.is_empty() {
            if let (Some(capsule), Some((result_generation, branch))) =
                (run_capsule.as_ref(), generation_result.as_ref())
            {
                capsule.save_result(args, *result_generation, branch)?;
                println!("run_capsule_result: {}", capsule.result_path().display());
            }
            break;
        }
        if next
            .iter()
            .any(|branch| matches!(branch.status, BranchStatus::AwaitingAuto { .. }))
        {
            frontier = next;
            capsule_frontier_saved |= run_persistence::save_wall_stop(
                run_persistence::frontier_checkpoint_output_path(
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
        frontier = next;
        if deadline.should_soft_stop(args) {
            capsule_frontier_saved |= run_persistence::save_wall_stop(
                run_persistence::frontier_checkpoint_output_path(
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
        run_persistence::print_capsule_save(
            capsule.save_recovery(args, last_generation, next_branch_id, &frontier)?,
            capsule,
        );
    }
    Ok(())
}

fn branch_status_boundary_label(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary.clone(),
        BranchStatus::Terminal(_) => "Terminal".to_string(),
        BranchStatus::ApplyFailed(_) => "ApplyFailed".to_string(),
        BranchStatus::AdvanceFailed(_) => "AdvanceFailed".to_string(),
    }
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

fn parse_args() -> Result<
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
        checkpoint_before_boss_retry: false,
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
                "  optional: --checkpoint-before-boss-retry saves a resumable boss retry checkpoint"
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
                | "--checkpoint-before-boss-retry"
                | "--probe-event-owner"
                | "--probe-event-screen"
        ) {
            return Err(format!("unknown argument {key}"));
        }
        if key == "--checkpoint-before-boss-retry" {
            args.checkpoint_before_boss_retry = true;
            overrides.checkpoint_before_boss_retry = true;
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
