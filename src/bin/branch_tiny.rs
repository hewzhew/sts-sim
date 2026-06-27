use std::collections::VecDeque;

use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, boss_relic_admission_order_rank,
    render_boss_relic_admission_compact, skip_boss_relic_admission, BossRelicAdmission,
};
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission, render_reward_admission_compact, reward_admission_order_key_v1,
    skip_reward_admission, RewardAdmission, RewardAdmissionOrderKeyV1,
};
use sts_simulator::eval::run_control::DecisionCandidateKey;
use sts_simulator::eval::run_control::{
    build_decision_surface, render_auto_applied_step_compact_v1, RewardAutomationConfig,
    RunControlAutoAppliedKindV1, RunControlAutoAppliedStepV1, RunControlAutoStepOptions,
    RunControlAutoStopKind, RunControlAutoStopV1, RunControlCommand, RunControlConfig,
    RunControlHpLossLimit, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
    RunControlSession,
};
use sts_simulator::state::core::{ClientInput, EngineState, RunResult};
use sts_simulator::state::events::EventId;

#[derive(Clone)]
struct Branch {
    id: usize,
    path: Vec<BranchPathStep>,
    session: RunControlSession,
    status: BranchStatus,
    boss_retry: Option<BossRetryReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
}

#[derive(Clone)]
struct OwnerChoice {
    key: Option<DecisionCandidateKey>,
    action: RunControlCommand,
    label: String,
    annotation: ChoiceAnnotation,
}

#[derive(Clone)]
enum ChoiceAnnotation {
    None,
    Reward(RewardAdmission),
    BossRelic(BossRelicAdmission),
}

impl ChoiceAnnotation {
    fn reward(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Reward(admission) => Some(admission),
            _ => None,
        }
    }

    fn boss_relic(&self) -> Option<&BossRelicAdmission> {
        match self {
            ChoiceAnnotation::BossRelic(admission) => Some(admission),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct BossRetryReport {
    status: BossRetryStatus,
    max_nodes: usize,
    wall_ms: u64,
    action_keys: Vec<String>,
}

#[derive(Clone)]
enum BossRetryStatus {
    Failed(String),
    Won,
    Advanced(String),
}

#[derive(Clone)]
struct BranchPathStep {
    key: Option<DecisionCandidateKey>,
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
    let (status, boss_retry, auto_steps) = advance_to_owner_or_gap(&mut session, args);
    let mut frontier = VecDeque::from([Branch {
        id: 0,
        path: Vec::new(),
        session,
        status,
        boss_retry,
        auto_steps,
    }]);
    let mut next_branch_id = 1usize;

    println!(
        "branch_tiny seed={} ascension={} generations={} max_branches={} mode=owner_audit render=timeline",
        args.seed, args.ascension, args.generations, args.max_branches
    );
    println!(
        "branch cap: {}; search={}nodes/{}ms; boss_retry={}nodes/{}ms; '>' marks expanded choices",
        args.max_branches,
        args.search_nodes,
        args.search_ms,
        args.boss_search_nodes,
        args.boss_search_ms
    );
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
                .len()
                .min(args.max_branches.saturating_sub(next.len()));
            print_branch_timeline(generation, &branch, &choices, expanded);
            if !expandable {
                continue;
            }
            for child in expand_registered_owner(
                &branch,
                args,
                choices.into_iter().take(expanded),
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
    owner_choices(&branch.session, owner, &surface)
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
        let (status, boss_retry, auto_steps) = match session.apply_command(choice.action.clone()) {
            Ok(_) => advance_to_owner_or_gap(&mut session, args),
            Err(err) => (BranchStatus::ApplyFailed(err), None, Vec::new()),
        };
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            key: choice.key,
            action: choice.action,
            label: choice.label,
            annotation: choice.annotation,
        });
        children.push(Branch {
            id: {
                let id = *next_branch_id;
                *next_branch_id += 1;
                id
            },
            path,
            session,
            status,
            boss_retry,
            auto_steps,
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
        Owner::BossRelic => boss_relic_owner_choices(session, surface),
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
            choice.annotation = reward_annotation_for_choice(&deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (card_reward_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn boss_relic_owner_choices(
    session: &RunControlSession,
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<OwnerChoice> {
    let EngineState::BossRelicSelect(_) = &session.engine_state else {
        return Vec::new();
    };
    let mut choices = executable_choices_including_cancel(surface)
        .into_iter()
        .filter(is_boss_relic_choice)
        .map(|mut choice| {
            choice.annotation = boss_relic_annotation_for_choice(session, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (boss_relic_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn boss_relic_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => {
            ChoiceAnnotation::BossRelic(assess_boss_relic_admission(&session.run_state, relic))
        }
        Some(DecisionCandidateKey::BossRelicSkip) => {
            ChoiceAnnotation::BossRelic(skip_boss_relic_admission())
        }
        _ => ChoiceAnnotation::None,
    }
}

fn reward_annotation_for_choice(
    deck: &[sts_simulator::content::cards::CardId],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, .. }) => {
            ChoiceAnnotation::Reward(assess_reward_admission(deck, card))
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => {
            ChoiceAnnotation::Reward(skip_reward_admission())
        }
        Some(DecisionCandidateKey::CardRewardOpen { .. })
        | Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | None => ChoiceAnnotation::None,
        _ => ChoiceAnnotation::None,
    }
}

fn card_reward_choice_rank(choice: &OwnerChoice) -> (u8, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => {
            (0, RewardAdmissionOrderKeyV1::empty_or_deferred())
        }
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => {
            (1, RewardAdmissionOrderKeyV1::unscored_optional_reward())
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (2, RewardAdmissionOrderKeyV1::empty_or_deferred()),
    }
}

fn boss_relic_choice_rank(choice: &OwnerChoice) -> (u8, u8) {
    let skip_order = boss_relic_admission_order_rank(&skip_boss_relic_admission());
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { .. }) => (
            0,
            choice
                .annotation
                .boss_relic()
                .map(boss_relic_admission_order_rank)
                .unwrap_or(skip_order),
        ),
        Some(DecisionCandidateKey::BossRelicSkip) => (1, skip_order),
        _ => (2, skip_order),
    }
}

fn advance_to_owner_or_gap(
    session: &mut RunControlSession,
    args: Args,
) -> (
    BranchStatus,
    Option<BossRetryReport>,
    Vec<RunControlAutoAppliedStepV1>,
) {
    let mut policy_steps = 0usize;
    let mut auto_steps = Vec::new();
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
                return (
                    BranchStatus::Terminal(terminal_label(session).unwrap()),
                    None,
                    auto_steps,
                );
            }
            Ok(outcome) => {
                auto_steps.extend(outcome.auto_applied_steps.clone());
                let Some(stop) = outcome.auto_stop.as_ref() else {
                    return (
                        BranchStatus::AdvanceFailed(
                            "auto_run returned non-terminal success without auto_stop".to_string(),
                        ),
                        None,
                        auto_steps,
                    );
                };
                let status = classify_boundary(session, stop);
                if matches!(status, BranchStatus::CombatGap { .. }) && is_boss_combat(session) {
                    if let Some(result) = try_boss_retry(session, args) {
                        return (result.0, result.1, auto_steps);
                    }
                }
                let owner = match &status {
                    BranchStatus::Running { owner, .. } => *owner,
                    _ => return (status, None, auto_steps),
                };
                if owner_is_branching(owner) {
                    return (status, None, auto_steps);
                }
                policy_steps += 1;
                if policy_steps > 16 {
                    return (
                        BranchStatus::BudgetGap {
                            boundary: build_decision_surface(session).view.header.title.clone(),
                            reason: "owner policy step budget exhausted".to_string(),
                        },
                        None,
                        auto_steps,
                    );
                }
                match apply_policy_owner(session, owner) {
                    Ok(outcome) => {
                        auto_steps.push(RunControlAutoAppliedStepV1 {
                            kind: RunControlAutoAppliedKindV1::OwnerPolicy,
                            label: format!("owner policy {owner:?}"),
                            action_result: outcome.action_result,
                        });
                    }
                    Err(err) => {
                        return (
                            BranchStatus::AdvanceFailed(format!(
                                "owner policy {owner:?} failed: {err}"
                            )),
                            None,
                            auto_steps,
                        );
                    }
                }
            }
            Err(err) => return (BranchStatus::AdvanceFailed(err), None, auto_steps),
        }
    }
}

fn owner_is_branching(owner: Owner) -> bool {
    matches!(
        owner,
        Owner::NeowStart | Owner::CardReward | Owner::BossRelic
    )
}

fn is_boss_combat(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|combat| combat.combat_state.meta.is_boss_fight)
}

fn try_boss_retry(
    session: &mut RunControlSession,
    args: Args,
) -> Option<(BranchStatus, Option<BossRetryReport>)> {
    let options = RunControlAutoStepOptions {
        search: RunControlSearchCombatOptions {
            max_nodes: Some(args.boss_search_nodes),
            wall_ms: Some(args.boss_search_ms),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            ..Default::default()
        },
        max_operations: Some(args.auto_ops),
        route: RunControlRouteAutomationMode::Planner,
    };
    let outcome = match session.apply_command(RunControlCommand::AutoRun(options)) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Some((
                BranchStatus::AdvanceFailed(err.clone()),
                Some(BossRetryReport {
                    status: BossRetryStatus::Failed(err),
                    max_nodes: args.boss_search_nodes,
                    wall_ms: args.boss_search_ms,
                    action_keys: Vec::new(),
                }),
            ));
        }
    };
    let action_keys = session
        .last_completed_combat_automation_trajectory()
        .map(|record| {
            record
                .actions
                .iter()
                .map(|action| action.action_key.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let report_status = if !action_keys.is_empty() {
        BossRetryStatus::Won
    } else {
        BossRetryStatus::Failed(one_line(&outcome.message))
    };
    let report = BossRetryReport {
        status: report_status,
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        action_keys,
    };
    if terminal_label(session).is_some() {
        return Some((
            BranchStatus::Terminal(terminal_label(session).unwrap()),
            Some(report),
        ));
    }
    let stop = match outcome.auto_stop.as_ref() {
        Some(stop) => stop,
        None => {
            return Some((
                BranchStatus::AdvanceFailed(
                    "boss retry returned non-terminal success without auto_stop".to_string(),
                ),
                Some(report),
            ));
        }
    };
    let status = classify_boundary(session, stop);
    let report = if matches!(report.status, BossRetryStatus::Won)
        && !matches!(status, BranchStatus::CombatGap { .. })
    {
        BossRetryReport {
            status: BossRetryStatus::Advanced(status_boundary(&status).to_string()),
            ..report
        }
    } else {
        report
    };
    Some((status, Some(report)))
}

fn apply_policy_owner(
    session: &mut RunControlSession,
    owner: Owner,
) -> Result<sts_simulator::eval::run_control::RunControlCommandOutcome, String> {
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
        Owner::NeowStart | Owner::CardReward | Owner::BossRelic => {
            return Err("branching owner cannot be consumed as policy".to_string());
        }
    };
    session.apply_command(RunControlCommand::Input(input))
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
    if surface
        .visible_executable_inputs
        .iter()
        .any(|visible_input| visible_input == &input)
    {
        return Ok(input);
    }
    Err(format!(
        "input {:?} is not visible at {} among [{}]",
        input,
        surface.view.header.title,
        executable_choices_including_cancel(&surface)
            .iter()
            .map(render_timeline_choice)
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
        EngineState::BossRelicSelect(_) => Some(Owner::BossRelic),
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

fn print_branch_timeline(
    generation: usize,
    branch: &Branch,
    choices: &[OwnerChoice],
    expanded: usize,
) {
    println!(
        "\n[{generation:02}] b{:04} A{}F{} {} owner={} hp={}/{} deck={} status={}",
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        status_boundary(&branch.status),
        status_owner(&branch.status),
        branch.session.run_state.current_hp,
        branch.session.run_state.max_hp,
        branch.session.run_state.master_deck.len(),
        status_label(&branch.status),
    );
    if let Some(previous) = branch.path.last() {
        println!("  arrived: {}", render_timeline_step(previous));
    }
    print_auto_steps(&branch.auto_steps);
    if let Some(retry) = branch.boss_retry.as_ref() {
        print_boss_retry(retry);
    }
    print_reward_gap_detail(&branch.session, &branch.status);
    if choices.is_empty() {
        return;
    }
    println!("  choices:");
    for (rank, choice) in choices.iter().enumerate() {
        let marker = if rank < expanded { ">" } else { " " };
        println!(
            "  {marker} {:>2}. {}",
            rank + 1,
            render_timeline_choice(choice)
        );
    }
    if expanded < choices.len() {
        println!(
            "  expansion: expanded {} hidden {}",
            expanded,
            choices.len() - expanded
        );
    }
}

fn print_auto_steps(steps: &[RunControlAutoAppliedStepV1]) {
    if steps.is_empty() {
        return;
    }
    let shown = steps.iter().take(12).collect::<Vec<_>>();
    println!("  auto:");
    for step in shown {
        println!("    - {}", render_auto_applied_step_compact_v1(step));
    }
    if steps.len() > 12 {
        println!("    ... {} more auto steps", steps.len() - 12);
    }
}

fn print_boss_retry(retry: &BossRetryReport) {
    println!(
        "  boss_retry: {} budget={}nodes/{}ms",
        boss_retry_status_label(&retry.status),
        retry.max_nodes,
        retry.wall_ms
    );
    if retry.action_keys.is_empty() {
        return;
    }
    let shown = retry
        .action_keys
        .iter()
        .take(12)
        .cloned()
        .collect::<Vec<_>>();
    println!("    win_path: {}", shown.join(" -> "));
    if retry.action_keys.len() > shown.len() {
        println!(
            "    ... {} more actions",
            retry.action_keys.len() - shown.len()
        );
    }
}

fn boss_retry_status_label(status: &BossRetryStatus) -> String {
    match status {
        BossRetryStatus::Failed(reason) => format!("failed ({})", one_line(reason)),
        BossRetryStatus::Won => "combat-win".to_string(),
        BossRetryStatus::Advanced(boundary) => format!("combat-win -> {boundary}"),
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
        .map(|choice| render_timeline_choice(&choice))
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
        Owner::BossRelic => "BossRelic".to_string(),
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

fn render_timeline_step(step: &BranchPathStep) -> String {
    let base = match &step.key {
        Some(key) => render_choice_key_timeline(key),
        None => format!("{}:{}", command_hint(&step.action), step.label),
    };
    match &step.annotation {
        ChoiceAnnotation::Reward(admission) => {
            format!("{base}  {}", render_admission_timeline(admission))
        }
        ChoiceAnnotation::BossRelic(admission) => {
            format!("{base}  {}", render_boss_relic_timeline(admission))
        }
        ChoiceAnnotation::None => base,
    }
}

fn render_timeline_choice(choice: &OwnerChoice) -> String {
    let base = match &choice.key {
        Some(key) => render_choice_key_timeline(key),
        None => format!("{}:{}", command_hint(&choice.action), choice.label),
    };
    match &choice.annotation {
        ChoiceAnnotation::Reward(admission) => {
            format!("{:<34} {}", base, render_admission_timeline(admission))
        }
        ChoiceAnnotation::BossRelic(admission) => {
            format!("{:<34} {}", base, render_boss_relic_timeline(admission))
        }
        ChoiceAnnotation::None => base,
    }
}

fn render_choice_key_timeline(key: &DecisionCandidateKey) -> String {
    match key {
        DecisionCandidateKey::EventOption {
            option_index,
            action,
            ..
        } => format!("option {option_index} {action:?}"),
        DecisionCandidateKey::CardRewardPick {
            option_index,
            card,
            upgrades,
            ..
        } => format!("slot {option_index} {card:?}+{upgrades}"),
        DecisionCandidateKey::CardRewardOpen { reward_item_index } => {
            format!("open reward {reward_item_index}")
        }
        DecisionCandidateKey::CardRewardSingingBowl { option_index, .. } => {
            format!("bowl slot {option_index}")
        }
        DecisionCandidateKey::CardRewardSkip { .. } => "skip".to_string(),
        DecisionCandidateKey::BossRelicPick {
            option_index,
            relic,
        } => format!("boss relic {option_index} {relic:?}"),
        DecisionCandidateKey::BossRelicSkip => "skip boss relic".to_string(),
        DecisionCandidateKey::ShopPurgeCard {
            deck_index,
            card,
            upgrades,
        } => format!("purge {deck_index} {card:?}+{upgrades}"),
        DecisionCandidateKey::SelectionSubmit { reason, .. } => format!("select {reason:?}"),
        DecisionCandidateKey::ShopLeave => "leave shop".to_string(),
    }
}

fn render_admission_timeline(admission: &RewardAdmission) -> String {
    render_reward_admission_compact(admission)
}

fn render_boss_relic_timeline(admission: &BossRelicAdmission) -> String {
    render_boss_relic_admission_compact(admission)
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
    executable_choices_with_cancel(surface, false)
}

fn executable_choices_including_cancel(
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, true)
}

fn executable_choices_with_cancel(
    surface: &sts_simulator::eval::run_control::DecisionSurface,
    include_cancel: bool,
) -> Vec<OwnerChoice> {
    surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let action = candidate.action.executable_command()?;
            if !include_cancel && is_navigation_only_command(&action) {
                return None;
            }
            Some(OwnerChoice {
                key: candidate.key.clone(),
                action,
                label: candidate.label.clone(),
                annotation: ChoiceAnnotation::None,
            })
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

fn is_boss_relic_choice(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::BossRelicPick { .. } | DecisionCandidateKey::BossRelicSkip)
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
        boss_search_nodes: 2_000_000,
        boss_search_ms: 15_000,
    };
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!("branch_tiny --seed N --generations N --max-branches N");
            println!(
                "  owner-audit runner; boss combat gaps retry once with --boss-search-nodes/--boss-search-ms"
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
                | "--boss-search-nodes"
                | "--boss-search-ms"
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
            "--boss-search-nodes" => args.boss_search_nodes = parse(value, key)?,
            "--boss-search-ms" => args.boss_search_ms = parse(value, key)?,
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
