use std::collections::VecDeque;

use sts_simulator::eval::run_control::{
    build_decision_surface, RewardAutomationConfig, RunControlAutoStepOptions, RunControlCommand,
    RunControlConfig, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession,
};
use sts_simulator::state::core::{EngineState, RunResult};

#[derive(Clone)]
struct Branch {
    id: String,
    path: Vec<String>,
    session: RunControlSession,
    status: BranchStatus,
}

#[derive(Clone)]
enum BranchStatus {
    Running { boundary: String, owner: Owner },
    Terminal(&'static str),
    AutomationGap { boundary: String, owner_key: String },
    CombatGap { boundary: String, reason: String },
    BudgetGap { boundary: String, reason: String },
    ApplyFailed(String),
    AdvanceFailed(String),
}

#[derive(Clone, Copy, Debug)]
enum Owner {
    NeowStart,
    CardReward,
    EventSsssserpent,
    ShopTiny,
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
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
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
    let status = advance_to_owner_or_gap(&mut session, args);
    let mut frontier = VecDeque::from([Branch {
        id: "root".to_string(),
        path: Vec::new(),
        session,
        status,
    }]);

    println!(
        "branch_tiny seed={} ascension={} generations={} max_branches={} mode=owner_audit",
        args.seed, args.ascension, args.generations, args.max_branches
    );
    for generation in 0..=args.generations {
        println!("generation {generation} branches={}", frontier.len());
        let mut next = VecDeque::new();
        let mut truncated = false;
        while let Some(branch) = frontier.pop_front() {
            print_branch(&branch);
            if generation == args.generations
                || !matches!(branch.status, BranchStatus::Running { .. })
            {
                continue;
            }
            for child in expand_registered_owner(&branch, args) {
                if next.len() >= args.max_branches {
                    truncated = true;
                    break;
                }
                next.push_back(child);
            }
        }
        if truncated {
            println!("  generation_truncated cap={}", args.max_branches);
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    Ok(())
}

fn expand_registered_owner(branch: &Branch, args: Args) -> Vec<Branch> {
    let owner = match &branch.status {
        BranchStatus::Running { owner, .. } => *owner,
        _ => return Vec::new(),
    };
    let surface = build_decision_surface(&branch.session);
    let candidates = owner_candidates(owner, &surface);
    let mut children = Vec::new();
    for (index, (candidate_id, label)) in candidates.into_iter().enumerate() {
        let mut session = branch.session.clone();
        let status = match session.apply_command(RunControlCommand::Candidate(candidate_id.clone()))
        {
            Ok(_) => advance_to_owner_or_gap(&mut session, args),
            Err(err) => BranchStatus::ApplyFailed(err),
        };
        let mut path = branch.path.clone();
        path.push(format!("{candidate_id}:{label}"));
        children.push(Branch {
            id: format!("{}.{}", branch.id, index),
            path,
            session,
            status,
        });
    }
    children
}

fn owner_candidates(
    owner: Owner,
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<(String, String)> {
    match owner {
        Owner::NeowStart => executable_candidates(surface),
        Owner::CardReward => executable_candidates(surface)
            .into_iter()
            .filter(|(id, _)| is_card_reward_candidate_id(id))
            .collect(),
        Owner::EventSsssserpent | Owner::ShopTiny => Vec::new(),
    }
}

fn advance_to_owner_or_gap(session: &mut RunControlSession, args: Args) -> BranchStatus {
    let mut policy_steps = 0usize;
    loop {
        let options = RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                max_nodes: Some(args.search_nodes),
                wall_ms: Some(args.search_ms),
                max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
                ..Default::default()
            },
            max_operations: Some(args.auto_ops),
            route: RunControlRouteAutomationMode::Planner,
        };
        match session.apply_command(RunControlCommand::AutoRun(options)) {
            Ok(_) if terminal_label(session).is_some() => {
                return BranchStatus::Terminal(terminal_label(session).unwrap());
            }
            Ok(_) => {
                return BranchStatus::AdvanceFailed(
                    "auto_run returned non-terminal success".to_string(),
                );
            }
            Err(err) if err.starts_with("auto_run_incomplete:") => {
                let status = classify_boundary(session, &err);
                let owner = match &status {
                    BranchStatus::Running { owner, .. } => *owner,
                    _ => return status,
                };
                if owner_is_branching(owner) {
                    return status;
                }
                policy_steps += 1;
                if policy_steps > 16 {
                    return BranchStatus::BudgetGap {
                        boundary: build_decision_surface(session).view.header.title.clone(),
                        reason: "owner policy step budget exhausted".to_string(),
                    };
                }
                if let Err(err) = apply_policy_owner(session, owner) {
                    return BranchStatus::AdvanceFailed(format!(
                        "owner policy {owner:?} failed: {err}"
                    ));
                }
            }
            Err(err) => return BranchStatus::AdvanceFailed(err),
        }
    }
}

fn owner_is_branching(owner: Owner) -> bool {
    matches!(owner, Owner::NeowStart | Owner::CardReward)
}

fn apply_policy_owner(session: &mut RunControlSession, owner: Owner) -> Result<(), String> {
    let candidate_id = match owner {
        Owner::ShopTiny => find_policy_candidate(session, &["leave"], &["leave shop", "leave"])?,
        Owner::EventSsssserpent => {
            find_policy_candidate(session, &[], &["disagree", "leave", "decline", "refuse"])?
        }
        Owner::NeowStart | Owner::CardReward => {
            return Err("branching owner cannot be consumed as policy".to_string());
        }
    };
    session
        .apply_command(RunControlCommand::Candidate(candidate_id))
        .map(|_| ())
}

fn find_policy_candidate(
    session: &RunControlSession,
    preferred_ids: &[&str],
    label_terms: &[&str],
) -> Result<String, String> {
    let surface = build_decision_surface(session);
    let candidates = executable_candidates(&surface);
    for preferred_id in preferred_ids {
        if candidates
            .iter()
            .any(|(candidate_id, _)| candidate_id == preferred_id)
        {
            return Ok((*preferred_id).to_string());
        }
    }
    for (candidate_id, label) in &candidates {
        let label = label.to_ascii_lowercase();
        if label_terms.iter().any(|term| label.contains(term)) {
            return Ok(candidate_id.clone());
        }
    }
    Err(format!(
        "no safe policy candidate at {} among [{}]",
        surface.view.header.title,
        candidates
            .iter()
            .map(|(id, label)| format!("{id}:{label}"))
            .collect::<Vec<_>>()
            .join(" | ")
    ))
}

fn classify_boundary(session: &RunControlSession, message: &str) -> BranchStatus {
    if let Some(result) = terminal_label(session) {
        return BranchStatus::Terminal(result);
    }
    let surface = build_decision_surface(session);
    let boundary = surface.view.header.title.clone();
    let reason = first_reason(message).unwrap_or_else(|| "auto_run_incomplete".to_string());
    if reason.starts_with("operation budget exhausted") {
        return BranchStatus::BudgetGap { boundary, reason };
    }
    if boundary == "Combat" || reason.starts_with("combat search did not find") {
        return BranchStatus::CombatGap { boundary, reason };
    }
    if let Some(owner) = owner_for_current_boundary(session) {
        return BranchStatus::Running { boundary, owner };
    }
    BranchStatus::AutomationGap {
        boundary,
        owner_key: owner_key_for_current_boundary(session),
    }
}

fn owner_for_current_boundary(session: &RunControlSession) -> Option<Owner> {
    match &session.engine_state {
        EngineState::EventRoom
            if session
                .run_state
                .event_state
                .as_ref()
                .is_some_and(|event| event.id == sts_simulator::state::events::EventId::Neow) =>
        {
            Some(Owner::NeowStart)
        }
        EngineState::EventRoom
            if session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == sts_simulator::state::events::EventId::Ssssserpent
            }) =>
        {
            Some(Owner::EventSsssserpent)
        }
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            Some(Owner::CardReward)
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            Some(Owner::CardReward)
        }
        EngineState::Shop(_) => Some(Owner::ShopTiny),
        _ => None,
    }
}

fn owner_key_for_current_boundary(session: &RunControlSession) -> String {
    match &session.engine_state {
        EngineState::EventRoom => session
            .run_state
            .event_state
            .as_ref()
            .map(|event| format!("event:{:?}", event.id))
            .unwrap_or_else(|| "event:unknown".to_string()),
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => "route".to_string(),
        EngineState::Shop(_) => "shop".to_string(),
        EngineState::Campfire => "campfire".to_string(),
        EngineState::BossRelicSelect(_) => "boss_relic".to_string(),
        EngineState::RunPendingChoice(_) => "run_choice".to_string(),
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => "reward".to_string(),
        EngineState::CombatStart(_)
        | EngineState::CombatProcessing
        | EngineState::CombatPlayerTurn => "combat".to_string(),
        EngineState::PendingChoice(_) => "combat_pending_choice".to_string(),
        EngineState::TreasureRoom(_) => "treasure".to_string(),
        EngineState::GameOver(_) => "terminal".to_string(),
    }
}

fn print_branch(branch: &Branch) {
    println!(
        "  {} A{}F{} hp={}/{} deck={} status={} boundary=\"{}\" owner=\"{}\" path=\"{}\"",
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        branch.session.run_state.current_hp,
        branch.session.run_state.max_hp,
        branch.session.run_state.master_deck.len(),
        status_label(&branch.status),
        status_boundary(&branch.status),
        status_owner(&branch.status),
        if branch.path.is_empty() {
            "-".to_string()
        } else {
            branch.path.join(" -> ")
        }
    );
    if let BranchStatus::Running { owner, .. } = &branch.status {
        let surface = build_decision_surface(&branch.session);
        let candidates = owner_candidates(*owner, &surface)
            .into_iter()
            .map(|(id, label)| format!("{id}:{label}"))
            .collect::<Vec<_>>();
        if !candidates.is_empty() {
            println!("    owner_candidates: {}", candidates.join(" | "));
        }
    }
}

fn status_label(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { .. } => "running".to_string(),
        BranchStatus::Terminal(result) => format!("terminal:{result}"),
        BranchStatus::AutomationGap { .. } => "automation_gap".to_string(),
        BranchStatus::CombatGap { reason, .. } => format!("combat_gap:{}", one_line(reason)),
        BranchStatus::BudgetGap { reason, .. } => format!("budget_gap:{}", one_line(reason)),
        BranchStatus::ApplyFailed(err) => format!("apply_failed:{}", one_line(err)),
        BranchStatus::AdvanceFailed(err) => format!("advance_failed:{}", one_line(err)),
    }
}

fn status_boundary(status: &BranchStatus) -> &str {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary,
        BranchStatus::Terminal(_)
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => "-",
    }
}

fn status_owner(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { owner, .. } => format!("{owner:?}"),
        BranchStatus::AutomationGap { owner_key, .. } => owner_key.clone(),
        BranchStatus::CombatGap { .. } => "combat_search".to_string(),
        BranchStatus::BudgetGap { .. } => "automation_budget".to_string(),
        BranchStatus::Terminal(_) => "terminal".to_string(),
        BranchStatus::ApplyFailed(_) => "candidate_apply".to_string(),
        BranchStatus::AdvanceFailed(_) => "automation".to_string(),
    }
}

fn executable_candidates(
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<(String, String)> {
    surface
        .view
        .candidates
        .iter()
        .filter(|candidate| !is_navigation_only_candidate(&candidate.id))
        .filter(|candidate| candidate.action.executable_input().is_some())
        .map(|candidate| (candidate.id.clone(), candidate.label.clone()))
        .collect()
}

fn terminal_label(session: &RunControlSession) -> Option<&'static str> {
    match &session.engine_state {
        EngineState::GameOver(RunResult::Victory) => Some("victory"),
        EngineState::GameOver(RunResult::Defeat) => Some("defeat"),
        _ => None,
    }
}

fn first_reason(message: &str) -> Option<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("Reason: ").map(str::to_string))
}

fn is_navigation_only_candidate(id: &str) -> bool {
    matches!(id, "back" | "cancel")
}

fn is_card_reward_candidate_id(id: &str) -> bool {
    id.parse::<usize>().is_ok() || matches!(id, "skip" | "bowl") || id.starts_with("card-")
}

fn one_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .take(160)
        .collect()
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        seed: 1,
        ascension: 0,
        generations: 2,
        max_branches: 24,
        auto_ops: 64,
        search_nodes: 20_000,
        search_ms: 300,
    };
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!("branch_tiny --seed N --generations N --max-branches N");
            println!(
                "  owner-audit runner; branching owners: neow_start, card_reward; policy owners: shop, event:Ssssserpent"
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
            _ => unreachable!("argument key was validated before value parsing"),
        }
        index += 2;
    }
    Ok(args)
}

fn parse<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {key}: {value}"))
}
