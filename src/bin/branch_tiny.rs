use std::collections::VecDeque;

use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission, render_reward_admission_compact, skip_reward_admission,
    RewardAdmission, RewardAdmissionClass,
};
use sts_simulator::eval::run_control::DecisionCandidateKey;
use sts_simulator::eval::run_control::{
    build_decision_surface, RewardAutomationConfig, RunControlAutoStepOptions,
    RunControlAutoStopKind, RunControlAutoStopV1, RunControlCommand, RunControlConfig,
    RunControlHpLossLimit, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
    RunControlSession,
};
use sts_simulator::state::core::{ClientInput, EngineState, RunResult};
use sts_simulator::state::events::EventId;

#[derive(Clone)]
struct Branch {
    id: String,
    path: Vec<BranchPathStep>,
    session: RunControlSession,
    status: BranchStatus,
}

#[derive(Clone)]
struct OwnerChoice {
    key: Option<DecisionCandidateKey>,
    action: RunControlCommand,
    label: String,
    admission: Option<RewardAdmission>,
}

#[derive(Clone)]
struct BranchPathStep {
    key: Option<DecisionCandidateKey>,
    action: RunControlCommand,
    label: String,
    admission: Option<RewardAdmission>,
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
    let candidates = owner_choices(&branch.session, owner, &surface);
    let mut children = Vec::new();
    for (index, choice) in candidates.into_iter().enumerate() {
        let mut session = branch.session.clone();
        let status = match session.apply_command(choice.action.clone()) {
            Ok(_) => advance_to_owner_or_gap(&mut session, args),
            Err(err) => BranchStatus::ApplyFailed(err),
        };
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            key: choice.key,
            action: choice.action,
            label: choice.label,
            admission: choice.admission,
        });
        children.push(Branch {
            id: format!("{}.{}", branch.id, index),
            path,
            session,
            status,
        });
    }
    children
}

fn owner_choices(
    session: &RunControlSession,
    owner: Owner,
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<OwnerChoice> {
    match owner {
        Owner::NeowStart => executable_choices(surface),
        Owner::CardReward => card_reward_owner_choices(session, surface),
        Owner::Event(_) | Owner::RewardTiny | Owner::ShopTiny => Vec::new(),
    }
}

fn card_reward_owner_choices(
    session: &RunControlSession,
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<OwnerChoice> {
    let deck = session
        .run_state
        .master_deck
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(|choice| is_card_reward_choice(choice))
        .map(|mut choice| {
            choice.admission = reward_admission_for_choice(&deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (card_reward_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn reward_admission_for_choice(
    deck: &[sts_simulator::content::cards::CardId],
    choice: &OwnerChoice,
) -> Option<RewardAdmission> {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, .. }) => {
            Some(assess_reward_admission(deck, card))
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => Some(skip_reward_admission()),
        Some(DecisionCandidateKey::CardRewardOpen { .. })
        | Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | None => None,
        _ => None,
    }
}

fn card_reward_choice_rank(choice: &OwnerChoice) -> (u8, u8) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => (0, 0),
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .admission
                .as_ref()
                .map(|admission| admission.class.rank())
                .unwrap_or(RewardAdmissionClass::EmptyOrDeferred.rank()),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (1, 6),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (1, 7),
        _ => (2, 0),
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
            Ok(outcome) => {
                let Some(stop) = outcome.auto_stop.as_ref() else {
                    return BranchStatus::AdvanceFailed(
                        "auto_run returned non-terminal success without auto_stop".to_string(),
                    );
                };
                let status = classify_boundary(session, stop);
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
    let input = match owner {
        Owner::ShopTiny => require_visible_input(session, ClientInput::Proceed)?,
        Owner::RewardTiny => reward_tiny_policy_input(session)?,
        Owner::Event(_) => require_visible_input(
            session,
            sts_simulator::content::events::owner_policy::conservative_owner_policy_input(
                &session.run_state,
            )
            .map_err(|err| format!("{err:?}"))?,
        )?,
        Owner::NeowStart | Owner::CardReward => {
            return Err("branching owner cannot be consumed as policy".to_string());
        }
    };
    session
        .apply_command(RunControlCommand::Input(input))
        .map(|_| ())
}

fn reward_tiny_policy_input(session: &RunControlSession) -> Result<ClientInput, String> {
    let (reward, exit) = match &session.engine_state {
        EngineState::RewardScreen(reward) => (reward, ClientInput::Proceed),
        EngineState::RewardOverlay { reward_state, .. } => (reward_state, ClientInput::Cancel),
        _ => return Err("RewardTiny owner requires reward surface".to_string()),
    };
    if reward.pending_card_choice.is_some() || reward.has_card_reward_item() {
        return Err("RewardTiny owner received card reward surface".to_string());
    }
    let only_unclaimable_potions = !reward.items.is_empty()
        && reward.items.iter().all(|item| {
            matches!(
                item,
                sts_simulator::state::rewards::RewardItem::Potion { .. }
            )
        })
        && session.run_state.find_empty_potion_slot().is_none();
    if reward.items.is_empty() || only_unclaimable_potions {
        return require_visible_input(session, exit);
    }
    Err(format!(
        "RewardTiny owner has strategic residual reward items: {:?}",
        reward.items
    ))
}

fn require_visible_input(
    session: &RunControlSession,
    input: ClientInput,
) -> Result<ClientInput, String> {
    let surface = build_decision_surface(session);
    let visible_inputs = executable_inputs(&surface);
    if visible_inputs
        .iter()
        .any(|visible_input| visible_input == &input)
    {
        return Ok(input);
    }
    Err(format!(
        "input {:?} is not visible at {} among [{}]",
        input,
        surface.view.header.title,
        executable_choices(&surface)
            .iter()
            .map(render_choice)
            .collect::<Vec<_>>()
            .join(" | ")
    ))
}

fn classify_boundary(session: &RunControlSession, stop: &RunControlAutoStopV1) -> BranchStatus {
    if let Some(result) = terminal_label(session) {
        return BranchStatus::Terminal(result);
    }
    let surface = build_decision_surface(session);
    let boundary = surface.view.header.title.clone();
    if stop.kind == RunControlAutoStopKind::OperationBudgetExhausted {
        return BranchStatus::BudgetGap {
            boundary,
            reason: stop.reason.clone(),
        };
    }
    if is_combat_gap(session, stop.kind) {
        return BranchStatus::CombatGap {
            boundary,
            reason: stop.reason.clone(),
        };
    }
    if let Some(owner) = owner_for_current_boundary(session) {
        return BranchStatus::Running { boundary, owner };
    }
    BranchStatus::AutomationGap {
        boundary,
        site: boundary_site(session),
    }
}

fn is_combat_gap(session: &RunControlSession, stop_kind: RunControlAutoStopKind) -> bool {
    matches!(
        stop_kind,
        RunControlAutoStopKind::CombatSearchNoCompleteWin
            | RunControlAutoStopKind::HpLossGateRequired
    ) || matches!(
        session.engine_state,
        EngineState::CombatStart(_)
            | EngineState::CombatProcessing
            | EngineState::CombatPlayerTurn
            | EngineState::PendingChoice(_)
    )
}

fn owner_for_current_boundary(session: &RunControlSession) -> Option<Owner> {
    match &session.engine_state {
        EngineState::EventRoom => {
            let event = session.run_state.event_state.as_ref()?;
            if event.id == EventId::Neow {
                Some(Owner::NeowStart)
            } else {
                Some(Owner::Event(event.id))
            }
        }
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            Some(Owner::CardReward)
        }
        EngineState::RewardScreen(reward) if reward.has_card_reward_item() => {
            Some(Owner::CardReward)
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            Some(Owner::CardReward)
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_state.has_card_reward_item() => {
            Some(Owner::CardReward)
        }
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => Some(Owner::RewardTiny),
        EngineState::Shop(_) => Some(Owner::ShopTiny),
        _ => None,
    }
}

fn boundary_site(session: &RunControlSession) -> BoundarySite {
    match &session.engine_state {
        EngineState::EventRoom => session
            .run_state
            .event_state
            .as_ref()
            .map(|event| BoundarySite::Event(event.id))
            .unwrap_or(BoundarySite::Unknown),
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => BoundarySite::Reward,
        EngineState::Shop(_) => BoundarySite::Shop,
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => BoundarySite::Route,
        EngineState::Campfire => BoundarySite::Campfire,
        EngineState::BossRelicSelect(_) => BoundarySite::BossRelic,
        EngineState::RunPendingChoice(_) => BoundarySite::RunChoice,
        EngineState::TreasureRoom(_) => BoundarySite::Treasure,
        EngineState::GameOver(_) => BoundarySite::Terminal,
        _ => BoundarySite::Unknown,
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
            render_path(&branch.path)
        }
    );
    print_reward_gap_detail(&branch.session, &branch.status);
    if let BranchStatus::Running { owner, .. } = &branch.status {
        let surface = build_decision_surface(&branch.session);
        let candidates = owner_choices(&branch.session, *owner, &surface)
            .into_iter()
            .map(|choice| render_choice(&choice))
            .collect::<Vec<_>>();
        if !candidates.is_empty() {
            println!("    owner_candidates: {}", candidates.join(" | "));
        }
    }
}

fn print_reward_gap_detail(session: &RunControlSession, status: &BranchStatus) {
    if !matches!(
        status,
        BranchStatus::AutomationGap {
            site: BoundarySite::Reward,
            ..
        }
    ) {
        return;
    }
    let surface = build_decision_surface(session);
    let candidates = executable_choices(&surface)
        .into_iter()
        .map(|choice| render_choice(&choice))
        .collect::<Vec<_>>();
    if !candidates.is_empty() {
        println!("    reward_gap_candidates: {}", candidates.join(" | "));
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
        BranchStatus::Running { owner, .. } => owner_label(*owner),
        BranchStatus::AutomationGap { site, .. } => site_label(*site),
        BranchStatus::CombatGap { .. } => "combat_search".to_string(),
        BranchStatus::BudgetGap { .. } => "automation_budget".to_string(),
        BranchStatus::Terminal(_) => "terminal".to_string(),
        BranchStatus::ApplyFailed(_) => "candidate_apply".to_string(),
        BranchStatus::AdvanceFailed(_) => "automation".to_string(),
    }
}

fn owner_label(owner: Owner) -> String {
    match owner {
        Owner::NeowStart => "NeowStart".to_string(),
        Owner::CardReward => "CardReward".to_string(),
        Owner::Event(event_id) => format!("Event({event_id:?})"),
        Owner::RewardTiny => "RewardTiny".to_string(),
        Owner::ShopTiny => "ShopTiny".to_string(),
    }
}

fn site_label(site: BoundarySite) -> String {
    match site {
        BoundarySite::Event(event_id) => format!("Event({event_id:?})"),
        BoundarySite::Reward => "Reward".to_string(),
        BoundarySite::Shop => "Shop".to_string(),
        BoundarySite::Route => "Route".to_string(),
        BoundarySite::Campfire => "Campfire".to_string(),
        BoundarySite::BossRelic => "BossRelic".to_string(),
        BoundarySite::RunChoice => "RunChoice".to_string(),
        BoundarySite::Treasure => "Treasure".to_string(),
        BoundarySite::Terminal => "Terminal".to_string(),
        BoundarySite::Unknown => "Unknown".to_string(),
    }
}

fn render_path(path: &[BranchPathStep]) -> String {
    path.iter()
        .map(render_path_step)
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn render_path_step(step: &BranchPathStep) -> String {
    let base = match &step.key {
        Some(key) => render_candidate_key(key),
        None => format!("{}:{}", command_hint(&step.action), step.label),
    };
    append_admission(base, step.admission.as_ref())
}

fn render_choice(choice: &OwnerChoice) -> String {
    let base = match &choice.key {
        Some(key) => render_candidate_key(key),
        None => format!("{}:{}", command_hint(&choice.action), choice.label),
    };
    append_admission(base, choice.admission.as_ref())
}

fn append_admission(base: String, admission: Option<&RewardAdmission>) -> String {
    match admission {
        Some(admission) => format!("{base} [{}]", render_reward_admission_compact(admission)),
        None => base,
    }
}

fn render_candidate_key(key: &DecisionCandidateKey) -> String {
    match key {
        DecisionCandidateKey::EventOption {
            event_id,
            screen,
            option_index,
            action,
        } => format!("event:{event_id:?}/screen:{screen}/option:{option_index}/{action:?}"),
        DecisionCandidateKey::CardRewardPick {
            reward_item_index,
            option_index,
            card,
            upgrades,
        } => format!("reward:{reward_item_index}:pick:{option_index}:{card:?}+{upgrades}"),
        DecisionCandidateKey::CardRewardOpen { reward_item_index } => {
            format!("reward:{reward_item_index}:open")
        }
        DecisionCandidateKey::CardRewardSingingBowl {
            reward_item_index,
            option_index,
        } => {
            format!("reward:{reward_item_index}:bowl:{option_index}")
        }
        DecisionCandidateKey::CardRewardSkip { reward_item_index } => {
            format!("reward:{reward_item_index}:skip")
        }
        DecisionCandidateKey::ShopPurgeCard {
            deck_index,
            card,
            upgrades,
        } => format!("shop:purge:{deck_index}:{card:?}+{upgrades}"),
        DecisionCandidateKey::SelectionSubmit {
            scope,
            reason,
            min_choices,
            max_choices,
            item_count,
        } => format!("select:{scope:?}:{reason:?}:{min_choices}-{max_choices}/{item_count}"),
        DecisionCandidateKey::ShopLeave => "shop:leave".to_string(),
    }
}

fn command_hint(command: &RunControlCommand) -> String {
    match command {
        RunControlCommand::Input(input) => format!("{input:?}"),
        RunControlCommand::BranchSkipCardReward(index) => {
            format!("BranchSkipCardReward({index})")
        }
        RunControlCommand::SingingBowlVisibleCardReward(index) => {
            format!("SingingBowlVisibleCardReward({index})")
        }
        _ => format!("{command:?}"),
    }
}

fn executable_choices(
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<OwnerChoice> {
    surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let action = candidate.action.executable_command()?;
            if is_navigation_only_command(&action) {
                return None;
            }
            Some(OwnerChoice {
                key: candidate.key.clone(),
                action,
                label: candidate.label.clone(),
                admission: None,
            })
        })
        .collect()
}

fn executable_inputs(
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<ClientInput> {
    executable_choices(surface)
        .into_iter()
        .filter_map(|choice| match choice.action {
            RunControlCommand::Input(input) => Some(input),
            _ => None,
        })
        .collect()
}

fn terminal_label(session: &RunControlSession) -> Option<&'static str> {
    match &session.engine_state {
        EngineState::GameOver(RunResult::Victory) => Some("victory"),
        EngineState::GameOver(RunResult::Defeat) => Some("defeat"),
        _ => None,
    }
}

fn is_navigation_only_command(command: &RunControlCommand) -> bool {
    matches!(command, RunControlCommand::Input(ClientInput::Cancel))
}

fn is_card_reward_choice(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(
            DecisionCandidateKey::CardRewardOpen { .. }
                | DecisionCandidateKey::CardRewardPick { .. }
                | DecisionCandidateKey::CardRewardSingingBowl { .. }
                | DecisionCandidateKey::CardRewardSkip { .. }
        )
    )
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
                "  owner-audit runner; branching owners: NeowStart, CardReward; policy owners: ShopTiny, marked Event options"
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
