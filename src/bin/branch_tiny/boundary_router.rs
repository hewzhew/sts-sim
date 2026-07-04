use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlAutoStopKind, RunControlAutoStopV1, RunControlSession,
};
use sts_simulator::state::core::{EngineState, RunResult};
use sts_simulator::state::selection::DomainEventSource;

use super::{BoundarySite, BranchStatus, Owner, TerminalOutcome};

pub(super) fn classify_boundary(
    session: &RunControlSession,
    stop: &RunControlAutoStopV1,
) -> BranchStatus {
    if let Some(result) = terminal_outcome(session) {
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

pub(super) fn terminal_outcome(session: &RunControlSession) -> Option<TerminalOutcome> {
    match &session.engine_state {
        EngineState::GameOver(RunResult::Victory) => Some(TerminalOutcome::Victory),
        EngineState::GameOver(RunResult::Defeat) => Some(TerminalOutcome::Defeat),
        _ => None,
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
            DomainEventSource::Event(
                event_id @ (sts_simulator::state::events::EventId::Designer
                | sts_simulator::state::events::EventId::LivingWall),
            ) => Some(Owner::Event(event_id)),
            _ if super::run_choice_owner::can_handle(choice.reason) => Some(Owner::RunChoice),
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
