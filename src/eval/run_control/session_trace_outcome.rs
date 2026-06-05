use crate::ai::noncombat_decision_v1::{
    attach_noncombat_outcome_with_card_reward_observation_v1, CardRewardOutcomeObservationV1,
    DecisionSiteKindV1, NonCombatDecisionRecordV1, NonCombatOutcomeAttachmentV1,
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
    card_reward_observation: CardRewardOutcomeObservationV1,
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
            card_reward_observation: CardRewardOutcomeObservationV1::default(),
        });
        queued = true;
    }
    queued
}

pub(super) fn update_outcome_counters(
    counters: &mut SessionTraceOutcomeCounters,
    action_result: &ActionResult,
    session_after: &RunControlSession,
    annotations: &[RunControlTraceAnnotationV1],
) {
    let combat_ended_changes = action_result
        .changes
        .iter()
        .filter(|change| matches!(change, ActionResultChange::CombatEnded))
        .count() as u32;
    let combat_automation_trajectories = annotations
        .iter()
        .filter(|annotation| {
            matches!(
                annotation,
                RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                    action_count,
                    ..
                } if *action_count > 0
            )
        })
        .count() as u32;
    let completed_combats = combat_ended_changes.max(combat_automation_trajectories);
    if completed_combats == 0 {
        return;
    }

    counters.combats_completed = counters.combats_completed.saturating_add(completed_combats);
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

pub(super) fn update_pending_outcome_observations(
    pending_outcomes: &mut [SessionTracePendingOutcome],
    action_result: &ActionResult,
) {
    if pending_outcomes.is_empty() {
        return;
    }
    for change in &action_result.changes {
        match change {
            ActionResultChange::CardUpgraded { before, .. } => {
                for pending in pending_outcomes.iter_mut() {
                    if selected_card_reward_matches_card(pending, before) {
                        pending
                            .card_reward_observation
                            .picked_card_upgraded_before_boss = Some(true);
                    }
                }
            }
            ActionResultChange::CardRemoved { card } => {
                for pending in pending_outcomes.iter_mut() {
                    if selected_card_reward_matches_card(pending, card) {
                        pending.card_reward_observation.picked_card_removed_later = Some(true);
                    }
                }
            }
            _ => {}
        }
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
            let attachment = attach_noncombat_outcome_with_card_reward_observation_v1(
                &pending.record,
                pending.window,
                pending.before,
                after.clone(),
                pending.card_reward_observation,
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
            | RunControlTraceAnnotationV1::NonCombatPolicyDecision { record, .. }
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

fn selected_card_reward_matches_card(
    pending: &SessionTracePendingOutcome,
    card: &crate::eval::run_control::transition_report::CardSnapshot,
) -> bool {
    if pending.record.site != DecisionSiteKindV1::CardReward {
        return false;
    }
    let Some(selected_candidate_id) = pending.record.selection.selected_candidate_id.as_deref()
    else {
        return false;
    };
    selected_candidate_id
        .rsplit(':')
        .next()
        .is_some_and(|card_id| card_id == format!("{:?}", card.id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::noncombat_decision_v1::{
        CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1,
        InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1, PolicyProvenanceV1,
        PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
        NONCOMBAT_DECISION_RECORD_SCHEMA_NAME, NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
    };
    use crate::content::cards::CardId;
    use crate::eval::run_control::transition_report::{
        ActionResult, ActionResultChange, CardSnapshot, RunApplyStatus,
    };
    use crate::eval::run_control::{RunControlConfig, RunControlSession};
    use crate::state::core::{ClientInput, EngineState};

    #[test]
    fn pending_card_reward_outcome_records_selected_card_upgrade_and_removal() {
        let mut pending_outcomes = Vec::new();
        let before = NonCombatOutcomeSnapshotV1 {
            act: 1,
            floor: 1,
            current_hp: 80,
            max_hp: 80,
            gold: 99,
            deck_size: 10,
            relic_count: 1,
            potion_count: 0,
            combats_completed: 0,
            elites_completed: 0,
            bosses_completed: 0,
            run_terminal: None,
        };
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record: selected_card_reward_record(CardId::TwinStrike),
                card_reward_packet: None,
            }],
            before,
        );

        update_pending_outcome_observations(
            &mut pending_outcomes,
            &ActionResult {
                chosen_label: "smith Twin Strike".to_string(),
                status: RunApplyStatus::Running,
                changes: vec![ActionResultChange::CardUpgraded {
                    before: CardSnapshot {
                        id: CardId::TwinStrike,
                        uuid: 100,
                        upgrades: 0,
                    },
                    after: CardSnapshot {
                        id: CardId::TwinStrike,
                        uuid: 100,
                        upgrades: 1,
                    },
                }],
            },
        );
        update_pending_outcome_observations(
            &mut pending_outcomes,
            &ActionResult {
                chosen_label: "remove Twin Strike".to_string(),
                status: RunApplyStatus::Running,
                changes: vec![ActionResultChange::CardRemoved {
                    card: CardSnapshot {
                        id: CardId::TwinStrike,
                        uuid: 100,
                        upgrades: 1,
                    },
                }],
            },
        );

        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_upgraded_before_boss,
            Some(true)
        );
        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_removed_later,
            Some(true)
        );
    }

    #[test]
    fn combat_automation_trajectory_counts_as_completed_combat_for_card_reward_outcome() {
        let mut pending_outcomes = Vec::new();
        let mut attachments = Vec::new();
        let mut counters = SessionTraceOutcomeCounters::default();
        let before = NonCombatOutcomeSnapshotV1 {
            act: 1,
            floor: 1,
            current_hp: 80,
            max_hp: 80,
            gold: 99,
            deck_size: 11,
            relic_count: 1,
            potion_count: 0,
            combats_completed: 0,
            elites_completed: 0,
            bosses_completed: 0,
            run_terminal: None,
        };
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record: selected_card_reward_record(CardId::TwinStrike),
                card_reward_packet: None,
            }],
            before,
        );

        let annotations = vec![RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source: "test_combat_search".to_string(),
            action_count: 2,
            actions: vec![super::super::trace_annotation::CombatAutomationActionV1 {
                step_index: 0,
                action_key: "combat/end_turn".to_string(),
                input: ClientInput::EndTurn,
            }],
            label_role: "behavior_policy_not_teacher".to_string(),
        }];
        let action_result = ActionResult {
            chosen_label: "advance-to-human-boundary applied 1 operation(s)".to_string(),
            status: RunApplyStatus::Running,
            changes: vec![ActionResultChange::LocationChanged {
                before_act: 1,
                before_floor: 1,
                after_act: 1,
                after_floor: 2,
            }],
        };
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 2;
        session.run_state.current_hp = 72;
        session.engine_state = EngineState::RewardScreen(crate::state::rewards::RewardState::new());

        update_outcome_counters(&mut counters, &action_result, &session, &annotations);
        resolve_pending_outcomes(&mut pending_outcomes, &mut attachments, &session, counters)
            .expect("pending card reward outcome should resolve");

        assert_eq!(attachments.len(), 1);
        let card_reward = attachments[0]
            .card_reward
            .as_ref()
            .expect("selected card reward should receive card reward outcome");
        assert_eq!(card_reward.next_combat_hp_loss, Some(8));
        assert_eq!(attachments[0].metrics.combats_completed_delta, 1);
    }

    fn selected_card_reward_record(card_id: CardId) -> NonCombatDecisionRecordV1 {
        let candidate_id = format!("card_reward:0:{card_id:?}");
        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::CardReward,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "test".to_string(),
                source_schema_name: "TestPolicy".to_string(),
                source_schema_version: 1,
            },
            candidates: vec![CandidateDescriptorV1 {
                candidate_id: candidate_id.clone(),
                site: DecisionSiteKindV1::CardReward,
                label: "Twin Strike".to_string(),
                action_plan: PublicActionPlanV1 {
                    summary: "pick Twin Strike".to_string(),
                    command: Some("pick 0".to_string()),
                },
                information_classes: vec![InformationClassV1::PublicObservation],
                uncertainty_notes: Vec::new(),
            }],
            evidence: EvidenceBundleV1::default(),
            values: Vec::new(),
            selection: PolicySelectionV1 {
                status: PolicySelectionStatusV1::Selected,
                selected_candidate_id: Some(candidate_id),
                reason: "test selected card reward".to_string(),
                confidence: 1.0,
                selection_mode: "test".to_string(),
            },
        }
    }
}
