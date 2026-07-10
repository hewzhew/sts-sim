use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlAutoStopKind, RunControlAutoStopV1, RunControlCommandOutcome,
    RunControlSession,
};
use sts_simulator::state::core::{EngineState, RunResult};

use super::{BoundarySite, BranchStatus, Owner, TerminalOutcome};

pub(super) fn classify_auto_outcome(
    session: &RunControlSession,
    outcome: &RunControlCommandOutcome,
) -> BranchStatus {
    if let Some(result) = terminal_outcome(session) {
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
        return BranchStatus::OperationBudgetExhausted {
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

pub(super) fn owner_for_current_boundary(session: &RunControlSession) -> Option<Owner> {
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
        EngineState::RunPendingChoice(choice)
            if super::run_choice_owner::can_handle(choice.reason) =>
        {
            Some(Owner::RunChoice)
        }
        EngineState::RunPendingChoice(_) => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::RunControlConfig;
    use sts_simulator::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
    use sts_simulator::state::events::{EventId, EventState};
    use sts_simulator::state::selection::DomainEventSource;

    #[test]
    fn neow_regular_events_and_event_deck_choices_have_distinct_owners() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = Some(EventState::new(EventId::Neow));
        assert!(matches!(
            owner_for_current_boundary(&session),
            Some(Owner::NeowStart)
        ));

        session.run_state.event_state = Some(EventState::new(EventId::GoldenShrine));
        assert!(matches!(
            owner_for_current_boundary(&session),
            Some(Owner::Event(EventId::GoldenShrine))
        ));

        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Upgrade,
            source: DomainEventSource::Event(EventId::UpgradeShrine),
            return_state: Box::new(EngineState::EventRoom),
        });
        assert!(matches!(
            owner_for_current_boundary(&session),
            Some(Owner::RunChoice)
        ));
    }
}
