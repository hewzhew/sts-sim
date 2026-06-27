use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, build_decision_surface, RunControlAutoAppliedKindV1,
    RunControlAutoAppliedStepV1, RunControlAutoStepOptions, RunControlAutoStopKind,
    RunControlAutoStopV1, RunControlCommand, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession,
};
use sts_simulator::state::core::{ClientInput, EngineState, RunResult};

use super::render;
use super::{Args, BossRetryReport, BossRetryStatus, BoundarySite, BranchStatus, Owner};

pub(super) fn advance_to_owner_or_gap(
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
        match apply_owner_audit_auto_run(session, options) {
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
        Owner::NeowStart | Owner::CardReward | Owner::BossRelic | Owner::ShopTiny
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
    let outcome = match apply_owner_audit_auto_run(session, options) {
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
        BossRetryStatus::Failed(render::one_line(&outcome.message))
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
            status: BossRetryStatus::Advanced(render::status_boundary(&status).to_string()),
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
        Owner::ShopTiny => return Err("ShopTiny has no automatic policy".to_string()),
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

fn terminal_label(session: &RunControlSession) -> Option<&'static str> {
    match &session.engine_state {
        EngineState::GameOver(RunResult::Victory) => Some("victory"),
        EngineState::GameOver(RunResult::Defeat) => Some("defeat"),
        _ => None,
    }
}
