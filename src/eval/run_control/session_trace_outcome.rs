use crate::ai::noncombat_decision_v1::{
    attach_noncombat_outcome_v1, NonCombatDecisionRecordV1, NonCombatOutcomeAttachmentV1,
    NonCombatOutcomeSnapshotV1, NonCombatOutcomeWindowV1, NonCombatRunTerminalV1,
    PolicySelectionStatusV1,
};
use crate::state::core::{EngineState, RunResult};
use crate::state::map::RoomType;

use super::session::RunControlSession;
use super::trace_annotation::RunControlTraceAnnotationV1;
use super::transition_report::{ActionResult, ActionResultChange};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SessionTraceOutcomeCounters {
    pub(super) combats_completed: u32,
    pub(super) elites_completed: u32,
    pub(super) bosses_completed: u32,
}

#[derive(Clone, Debug)]
pub(super) struct SessionTracePendingOutcome {
    record: NonCombatDecisionRecordV1,
    window: NonCombatOutcomeWindowV1,
    before: NonCombatOutcomeSnapshotV1,
}

pub(super) fn queue_selected_noncombat_outcomes(
    pending_outcomes: &mut Vec<SessionTracePendingOutcome>,
    annotations: &[RunControlTraceAnnotationV1],
    before: NonCombatOutcomeSnapshotV1,
) -> bool {
    let mut queued = false;
    for record in selected_noncombat_records(annotations) {
        pending_outcomes.push(SessionTracePendingOutcome {
            record,
            window: NonCombatOutcomeWindowV1::AfterOneFloor,
            before: before.clone(),
        });
        queued = true;
    }
    queued
}

pub(super) fn update_outcome_counters(
    counters: &mut SessionTraceOutcomeCounters,
    action_result: &ActionResult,
    session_after: &RunControlSession,
) {
    if !action_result
        .changes
        .iter()
        .any(|change| matches!(change, ActionResultChange::CombatEnded))
    {
        return;
    }

    counters.combats_completed = counters.combats_completed.saturating_add(1);
    match session_after.run_state.map.get_current_room_type() {
        Some(RoomType::MonsterRoomElite) => {
            counters.elites_completed = counters.elites_completed.saturating_add(1);
        }
        Some(RoomType::MonsterRoomBoss) => {
            counters.bosses_completed = counters.bosses_completed.saturating_add(1);
        }
        Some(
            RoomType::MonsterRoom
            | RoomType::EventRoom
            | RoomType::ShopRoom
            | RoomType::RestRoom
            | RoomType::TreasureRoom
            | RoomType::TrueVictoryRoom,
        )
        | None => {}
    }
}

pub(super) fn resolve_pending_outcomes(
    pending_outcomes: &mut Vec<SessionTracePendingOutcome>,
    attachments: &mut Vec<NonCombatOutcomeAttachmentV1>,
    session: &RunControlSession,
    counters: SessionTraceOutcomeCounters,
) -> Result<bool, String> {
    if pending_outcomes.is_empty() {
        return Ok(false);
    }
    let after = noncombat_outcome_snapshot(session, counters);
    if !is_noncombat_outcome_boundary(session) {
        return Ok(false);
    }

    let mut remaining = Vec::new();
    let mut resolved = false;
    for pending in std::mem::take(pending_outcomes) {
        if outcome_window_reached(&pending, &after) {
            let attachment = attach_noncombat_outcome_v1(
                &pending.record,
                pending.window,
                pending.before,
                after.clone(),
            )?;
            attachments.push(attachment);
            resolved = true;
        } else {
            remaining.push(pending);
        }
    }
    *pending_outcomes = remaining;
    Ok(resolved)
}

pub(super) fn noncombat_outcome_snapshot(
    session: &RunControlSession,
    counters: SessionTraceOutcomeCounters,
) -> NonCombatOutcomeSnapshotV1 {
    let (current_hp, max_hp) = session.visible_player_hp();
    NonCombatOutcomeSnapshotV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        current_hp,
        max_hp,
        gold: session.run_state.gold,
        deck_size: session.run_state.master_deck.len(),
        relic_count: session.run_state.relics.len(),
        potion_count: session
            .visible_potions()
            .iter()
            .filter(|slot| slot.is_some())
            .count(),
        combats_completed: counters.combats_completed,
        elites_completed: counters.elites_completed,
        bosses_completed: counters.bosses_completed,
        run_terminal: match session.engine_state {
            EngineState::GameOver(RunResult::Victory) => Some(NonCombatRunTerminalV1::Victory),
            EngineState::GameOver(RunResult::Defeat) => Some(NonCombatRunTerminalV1::Loss),
            _ => None,
        },
    }
}

fn selected_noncombat_records(
    annotations: &[RunControlTraceAnnotationV1],
) -> Vec<NonCombatDecisionRecordV1> {
    annotations
        .iter()
        .filter_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::RoutePlannerSelection {
                noncombat_record: Some(record),
                ..
            }
            | RunControlTraceAnnotationV1::NonCombatPolicyDecision { record }
            | RunControlTraceAnnotationV1::NonCombatHumanBoundary { record } => {
                (record.selection.status == PolicySelectionStatusV1::Selected)
                    .then(|| record.clone())
            }
            RunControlTraceAnnotationV1::RoutePlannerSelection {
                noncombat_record: None,
                ..
            }
            | RunControlTraceAnnotationV1::AutoCombatCapture { .. }
            | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. } => None,
        })
        .collect()
}

fn outcome_window_reached(
    pending: &SessionTracePendingOutcome,
    after: &NonCombatOutcomeSnapshotV1,
) -> bool {
    if pending.before.run_terminal != after.run_terminal && after.run_terminal.is_some() {
        return true;
    }
    match pending.window {
        NonCombatOutcomeWindowV1::AfterOneFloor => {
            after.act > pending.before.act || after.floor >= pending.before.floor + 1
        }
        NonCombatOutcomeWindowV1::AfterThreeFloors => {
            after.act > pending.before.act || after.floor >= pending.before.floor + 3
        }
        NonCombatOutcomeWindowV1::BeforeNextElite
        | NonCombatOutcomeWindowV1::AfterNextElite
        | NonCombatOutcomeWindowV1::BeforeBoss
        | NonCombatOutcomeWindowV1::AfterBoss
        | NonCombatOutcomeWindowV1::Manual => false,
    }
}

fn is_noncombat_outcome_boundary(session: &RunControlSession) -> bool {
    session.active_combat.is_none()
        && !matches!(
            session.engine_state,
            EngineState::CombatStart(_)
                | EngineState::CombatPlayerTurn
                | EngineState::CombatProcessing
                | EngineState::PendingChoice(_)
        )
}
