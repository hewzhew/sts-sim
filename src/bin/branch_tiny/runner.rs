use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::ai::strategy::campfire_upgrade_quality::{
    rank_campfire_upgrades, should_rest_before_smith, CampfireUpgradeTier,
};
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardId, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, build_decision_surface, CombatAutomationTrajectorySource,
    CombatSearchTraceSummary, DecisionCandidateKey, RunControlAutoAppliedKindV1,
    RunControlAutoAppliedStepV1, RunControlAutoStepOptions, RunControlAutoStopKind,
    RunControlAutoStopV1, RunControlCommand, RunControlCommandOutcome, RunControlHpLossLimit,
    RunControlRouteAutomationMode, RunControlSearchCombatOptions, RunControlSession,
    RunControlTraceAnnotationV1,
};
use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState, RunResult};
use sts_simulator::state::selection::DomainEventSource;

use super::render;
use super::{
    Args, BossRetryAttemptReport, BossRetryReport, BossRetryStatus, BoundarySite, BranchStatus,
    Owner, RunDeadline,
};

pub(super) struct AdvanceResult {
    pub(super) status: BranchStatus,
    pub(super) boss_retry: Option<BossRetryReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}

pub(super) fn advance_to_owner_or_gap(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
) -> AdvanceResult {
    let mut policy_steps = 0usize;
    let mut auto_ops_used = 0usize;
    let mut auto_steps = Vec::new();
    let mut combat_search = Vec::new();
    loop {
        let run_args = deadline.cap_args(args, 1);
        match apply_owner_audit_auto_run(session, primary_auto_step_options(run_args)) {
            Ok(outcome) => {
                let stop_kind = outcome.auto_stop.as_ref().map(|stop| stop.kind);
                auto_ops_used = auto_ops_used.saturating_add(
                    outcome
                        .auto_stop
                        .as_ref()
                        .map(|stop| stop.applied_operations)
                        .unwrap_or(0),
                );
                combat_search.extend(combat_search_summaries(&outcome));
                auto_steps.extend(outcome.auto_applied_steps.clone());
                let mut status = classify_auto_outcome(session, &outcome);
                if stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
                    && auto_ops_used < args.auto_ops
                    && !deadline.should_stop()
                {
                    continue;
                }
                if matches!(status, BranchStatus::CombatGap { .. }) && is_boss_combat(session) {
                    if let Some(result) = try_boss_retry(session, deadline.cap_args(args, 1)) {
                        combat_search.extend(result.2);
                        return advance_result(result.0, Some(result.1), auto_steps, combat_search);
                    }
                }
                if matches!(status, BranchStatus::CombatGap { .. }) && !is_boss_combat(session) {
                    match apply_owner_audit_auto_run(
                        session,
                        diagnostic_rescue_auto_step_options(args),
                    ) {
                        Ok(rescue) => {
                            combat_search.extend(combat_search_summaries(&rescue));
                            auto_steps.extend(rescue.auto_applied_steps.clone());
                            status = classify_auto_outcome(session, &rescue);
                        }
                        Err(err) => {
                            return advance_result(
                                BranchStatus::AdvanceFailed(format!(
                                    "diagnostic combat rescue failed: {err}"
                                )),
                                None,
                                auto_steps,
                                combat_search,
                            );
                        }
                    }
                }
                if let BranchStatus::Terminal(result) = status {
                    return advance_result(
                        BranchStatus::Terminal(result),
                        None,
                        auto_steps,
                        combat_search,
                    );
                }
                let owner = match &status {
                    BranchStatus::Running { owner, .. } => *owner,
                    _ => return advance_result(status, None, auto_steps, combat_search),
                };
                if owner_is_branching(owner) {
                    return advance_result(status, None, auto_steps, combat_search);
                }
                policy_steps += 1;
                if policy_steps > 16 {
                    return advance_result(
                        BranchStatus::BudgetGap {
                            boundary: build_decision_surface(session).view.header.title.clone(),
                            reason: "owner policy step budget exhausted".to_string(),
                        },
                        None,
                        auto_steps,
                        combat_search,
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
                        return advance_result(
                            BranchStatus::AdvanceFailed(format!(
                                "owner policy {owner:?} failed: {err}"
                            )),
                            None,
                            auto_steps,
                            combat_search,
                        );
                    }
                }
            }
            Err(err) => {
                return advance_result(
                    BranchStatus::AdvanceFailed(err),
                    None,
                    auto_steps,
                    combat_search,
                )
            }
        }
    }
}

fn classify_auto_outcome(
    session: &RunControlSession,
    outcome: &RunControlCommandOutcome,
) -> BranchStatus {
    if let Some(result) = terminal_label(session) {
        return BranchStatus::Terminal(result);
    }
    outcome
        .auto_stop
        .as_ref()
        .map(|stop| classify_boundary(session, stop))
        .unwrap_or_else(|| {
            BranchStatus::AdvanceFailed(
                "auto_run returned non-terminal success without auto_stop".to_string(),
            )
        })
}

fn advance_result(
    status: BranchStatus,
    boss_retry: Option<BossRetryReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
) -> AdvanceResult {
    AdvanceResult {
        status,
        boss_retry,
        auto_steps,
        combat_search,
    }
}

fn combat_search_summaries(outcome: &RunControlCommandOutcome) -> Vec<CombatSearchTraceSummary> {
    sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
        .collect()
}

fn owner_is_branching(owner: Owner) -> bool {
    matches!(
        owner,
        Owner::NeowStart | Owner::CardReward | Owner::BossRelic | Owner::ShopTiny
    )
}

fn is_boss_combat(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|combat| combat.combat_state.meta.is_boss_fight)
}

fn primary_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    auto_step_options(
        args.search_nodes,
        args.search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
    )
}

fn diagnostic_rescue_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    auto_step_options(
        args.rescue_search_nodes,
        args.rescue_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
    )
}

fn auto_step_options(
    max_nodes: usize,
    wall_ms: u64,
    auto_ops: usize,
    wall_limited: bool,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
) -> RunControlAutoStepOptions {
    RunControlAutoStepOptions {
        search: RunControlSearchCombatOptions {
            max_nodes: Some(max_nodes),
            wall_ms: Some(wall_ms),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            turn_plan_policy: Some(turn_plan_policy),
            ..Default::default()
        },
        max_operations: Some(auto_run_chunk_ops(auto_ops, wall_limited)),
        route: RunControlRouteAutomationMode::Planner,
    }
}

fn auto_run_chunk_ops(auto_ops: usize, wall_limited: bool) -> usize {
    if wall_limited {
        1
    } else {
        auto_ops
    }
}

fn try_boss_retry(
    session: &mut RunControlSession,
    args: Args,
) -> Option<(BranchStatus, BossRetryReport, Vec<CombatSearchTraceSummary>)> {
    let mut all_search = Vec::new();
    let mut attempts = Vec::new();
    let no_potion = boss_retry_options(args, CombatSearchV2PotionPolicy::Never, Some(0));
    let (status, attempt, search) = run_boss_retry_attempt(session, args, "no_potion", no_potion);
    all_search.extend(search);
    attempts.push(attempt);
    if !matches!(status, BranchStatus::CombatGap { .. }) {
        let report = boss_retry_report(args, status.clone(), attempts);
        return Some((status, report, all_search));
    }

    let max_potions = session
        .active_combat
        .as_ref()
        .and_then(|active| {
            sts_simulator::ai::combat_search_v2::high_stakes_semantic_potion_budget(
                &active.combat_state,
            )
        })
        .unwrap_or(1);
    let rescue = boss_retry_options(args, CombatSearchV2PotionPolicy::All, Some(max_potions));
    let (status, attempt, search) = run_boss_retry_attempt(session, args, "potion_rescue", rescue);
    all_search.extend(search);
    attempts.push(attempt);
    let report = boss_retry_report(args, status.clone(), attempts);
    Some((status, report, all_search))
}

fn boss_retry_options(
    args: Args,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
) -> RunControlAutoStepOptions {
    let mut options = auto_step_options(
        args.boss_search_nodes,
        args.boss_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
    );
    options.search.potion_policy = Some(potion_policy);
    options.search.max_potions_used = max_potions_used;
    options
}

fn run_boss_retry_attempt(
    session: &mut RunControlSession,
    args: Args,
    label: &'static str,
    options: RunControlAutoStepOptions,
) -> (
    BranchStatus,
    BossRetryAttemptReport,
    Vec<CombatSearchTraceSummary>,
) {
    let potion_policy = options
        .search
        .potion_policy
        .unwrap_or(CombatSearchV2PotionPolicy::Never);
    let max_potions_used = options.search.max_potions_used;
    let outcome = match apply_owner_audit_auto_run(session, options) {
        Ok(outcome) => outcome,
        Err(err) => {
            let status = BranchStatus::AdvanceFailed(err);
            let attempt = boss_retry_attempt_report(
                args,
                label,
                potion_policy,
                max_potions_used,
                &status,
                Vec::new(),
            );
            return (status, attempt, Vec::new());
        }
    };
    let combat_search = combat_search_summaries(&outcome);
    let status = if terminal_label(session).is_some() {
        BranchStatus::Terminal(terminal_label(session).unwrap())
    } else if let Some(stop) = outcome.auto_stop.as_ref() {
        classify_boundary(session, stop)
    } else {
        BranchStatus::AdvanceFailed(
            "boss retry returned non-terminal success without auto_stop".to_string(),
        )
    };
    let action_keys = retry_complete_search_action_keys(&outcome);
    let attempt = boss_retry_attempt_report(
        args,
        label,
        potion_policy,
        max_potions_used,
        &status,
        action_keys,
    );
    (status, attempt, combat_search)
}

fn boss_retry_report(
    args: Args,
    status: BranchStatus,
    attempts: Vec<BossRetryAttemptReport>,
) -> BossRetryReport {
    let action_keys = attempts
        .last()
        .map(|attempt| attempt.action_keys.clone())
        .unwrap_or_default();
    let status = boss_retry_status(&status);
    BossRetryReport {
        status,
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        action_keys,
        attempts,
    }
}

fn boss_retry_attempt_report(
    args: Args,
    label: &'static str,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    status: &BranchStatus,
    action_keys: Vec<String>,
) -> BossRetryAttemptReport {
    BossRetryAttemptReport {
        label,
        status: boss_retry_status(status),
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        potion_policy: potion_policy_label(potion_policy),
        max_potions_used,
        action_keys,
    }
}

fn boss_retry_status(status: &BranchStatus) -> BossRetryStatus {
    match status {
        BranchStatus::CombatGap { reason, .. } => BossRetryStatus::Failed(reason.clone()),
        BranchStatus::ApplyFailed(err)
        | BranchStatus::AdvanceFailed(err)
        | BranchStatus::BudgetGap { reason: err, .. } => BossRetryStatus::Failed(err.clone()),
        BranchStatus::Terminal("defeat") => {
            BossRetryStatus::Failed("retry ended in defeat".to_string())
        }
        BranchStatus::Terminal(result) => BossRetryStatus::Terminal(result),
        _ => BossRetryStatus::Advanced(render::status_boundary(status).to_string()),
    }
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
}

fn retry_complete_search_action_keys(outcome: &RunControlCommandOutcome) -> Vec<String> {
    outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source, actions, ..
            } if *source == CombatAutomationTrajectorySource::SearchCombat => Some(
                actions
                    .iter()
                    .map(|action| action.action_key.clone())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn apply_policy_owner(
    session: &mut RunControlSession,
    owner: Owner,
) -> Result<sts_simulator::eval::run_control::RunControlCommandOutcome, String> {
    let input = match owner {
        Owner::ShopTiny => return Err("ShopTiny has no automatic policy".to_string()),
        Owner::RewardTiny => return apply_reward_tiny_policy(session),
        Owner::Campfire => return apply_campfire_owner_policy(session),
        Owner::Event(_) => require_visible_input(
            session,
            sts_simulator::content::events::owner_policy::event_owner_policy_input(
                &session.engine_state,
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

fn apply_campfire_owner_policy(
    session: &mut RunControlSession,
) -> Result<sts_simulator::eval::run_control::RunControlCommandOutcome, String> {
    if sts_simulator::engine::campfire_handler::get_available_options(&session.run_state).is_empty()
    {
        sts_simulator::engine::run_loop::tick_run_active_with_observer(
            &mut session.engine_state,
            &mut session.run_state,
            &mut session.active_combat,
            None,
        );
        return session.apply_command(RunControlCommand::Noop);
    }
    let input = require_visible_input(
        session,
        ClientInput::CampfireOption(choose_campfire_owner_action(session)?),
    )?;
    session.apply_command(RunControlCommand::Input(input))
}

fn choose_campfire_owner_action(session: &RunControlSession) -> Result<CampfireChoice, String> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return Err("Campfire owner requires Campfire engine state".to_string());
    }
    let options =
        sts_simulator::engine::campfire_handler::get_available_options(&session.run_state);
    let has_rest = options.contains(&CampfireChoice::Rest);
    let has_smith = options
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Smith(_)));

    if has_rest
        && (!has_smith
            || should_rest_before_smith(session.run_state.current_hp, session.run_state.max_hp))
    {
        return Ok(CampfireChoice::Rest);
    }
    if let Some(choice) = best_campfire_toke(session, &options) {
        return Ok(choice);
    }
    if has_smith {
        let ranked = rank_campfire_upgrades(&session.run_state.master_deck);
        if let Some(best) = ranked
            .iter()
            .find(|target| target.tier >= CampfireUpgradeTier::Low)
            .or_else(|| ranked.first())
        {
            return Ok(CampfireChoice::Smith(best.deck_index));
        }
    }
    for fallback in [
        CampfireChoice::Dig,
        CampfireChoice::Lift,
        CampfireChoice::Recall,
        CampfireChoice::Rest,
    ] {
        if options.contains(&fallback) {
            return Ok(fallback);
        }
    }
    Err("Campfire owner found no policy action".to_string())
}

fn best_campfire_toke(
    session: &RunControlSession,
    options: &[CampfireChoice],
) -> Option<CampfireChoice> {
    if !options
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Toke(_)))
    {
        return None;
    }
    let surface = build_decision_surface(session);
    surface
        .visible_executable_inputs
        .iter()
        .filter_map(|input| {
            let ClientInput::CampfireOption(CampfireChoice::Toke(index)) = input else {
                return None;
            };
            session
                .run_state
                .master_deck
                .get(*index)
                .map(|card| (*index, card.id))
        })
        .min_by_key(|(_, card)| campfire_toke_rank(*card))
        .map(|(index, _)| CampfireChoice::Toke(index))
}

fn campfire_toke_rank(card: CardId) -> u8 {
    let definition = get_card_definition(card);
    match definition.card_type {
        CardType::Curse => 0,
        CardType::Status => 1,
        _ if is_starter_basic(card) => 2,
        _ => 9,
    }
}

fn apply_reward_tiny_policy(
    session: &mut RunControlSession,
) -> Result<sts_simulator::eval::run_control::RunControlCommandOutcome, String> {
    if let Some(outcome) = sts_simulator::eval::run_control::apply_reward_tiny_automation(session)?
    {
        return Ok(outcome);
    }
    if let Some(outcome) = open_visible_card_reward(session)? {
        return Ok(outcome);
    }
    session.apply_command(RunControlCommand::Input(reward_tiny_exit_input(session)?))
}

fn open_visible_card_reward(
    session: &mut RunControlSession,
) -> Result<Option<sts_simulator::eval::run_control::RunControlCommandOutcome>, String> {
    let command = build_decision_surface(session)
        .view
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::CardRewardOpen { .. })
            )
        })
        .and_then(|candidate| candidate.action.executable_command());
    command
        .map(|command| session.apply_command(command))
        .transpose()
}

fn reward_tiny_exit_input(session: &RunControlSession) -> Result<ClientInput, String> {
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
        super::owners::executable_choices_including_cancel(&surface)
            .iter()
            .map(render::render_timeline_choice)
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
        if let Some(owner) = owner_for_current_boundary(session) {
            return BranchStatus::Running { boundary, owner };
        }
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
            if event.id == sts_simulator::state::events::EventId::Neow {
                Some(Owner::NeowStart)
            } else {
                Some(Owner::Event(event.id))
            }
        }
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            Some(Owner::CardReward)
        }
        EngineState::RewardScreen(reward) if reward.has_card_reward_item() => {
            Some(Owner::RewardTiny)
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            Some(Owner::CardReward)
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_state.has_card_reward_item() => {
            Some(Owner::RewardTiny)
        }
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => Some(Owner::RewardTiny),
        EngineState::BossRelicSelect(_) => Some(Owner::BossRelic),
        EngineState::Shop(_) => Some(Owner::ShopTiny),
        EngineState::Campfire => Some(Owner::Campfire),
        EngineState::RunPendingChoice(choice) => match choice.source {
            DomainEventSource::Event(sts_simulator::state::events::EventId::LivingWall) => Some(
                Owner::Event(sts_simulator::state::events::EventId::LivingWall),
            ),
            _ => None,
        },
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

fn terminal_label(session: &RunControlSession) -> Option<&'static str> {
    match &session.engine_state {
        EngineState::GameOver(RunResult::Victory) => Some("victory"),
        EngineState::GameOver(RunResult::Defeat) => Some("defeat"),
        _ => None,
    }
}
