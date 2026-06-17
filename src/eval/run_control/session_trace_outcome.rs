use crate::ai::noncombat_decision_v1::{
    attach_noncombat_outcome_with_card_reward_observation_v1, CardRewardOutcomeObservationV1,
    DecisionSiteKindV1, NonCombatDecisionRecordV1, NonCombatOutcomeAttachmentV1,
    NonCombatOutcomeSnapshotV1, NonCombatOutcomeWindowV1, NonCombatRunTerminalV1,
    PolicySelectionStatusV1,
};
use crate::content::cards::{java_id, CardId};
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
    selected_card_uuid: Option<u32>,
}

pub(super) fn queue_selected_noncombat_outcomes(
    pending_outcomes: &mut Vec<SessionTracePendingOutcome>,
    annotations: &[RunControlTraceAnnotationV1],
    before: NonCombatOutcomeSnapshotV1,
    action_result: Option<&ActionResult>,
) -> bool {
    let mut queued = false;
    for record in selected_noncombat_records(annotations) {
        let selected_card_uuid = selected_card_uuid_from_action_result(&record, action_result);
        for window in outcome_windows_for_record(&record) {
            pending_outcomes.push(SessionTracePendingOutcome {
                record: record.clone(),
                window,
                before: before.clone(),
                card_reward_observation: CardRewardOutcomeObservationV1::default(),
                selected_card_uuid,
            });
        }
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
    annotations: &[RunControlTraceAnnotationV1],
) {
    if pending_outcomes.is_empty() {
        return;
    }
    for change in &action_result.changes {
        match change {
            ActionResultChange::CombatCardDrawn { card } => {
                for pending in pending_outcomes.iter_mut() {
                    if selected_card_reward_matches_card(pending, card) {
                        increment_observation_count(
                            &mut pending.card_reward_observation.picked_card_drawn_count,
                            1,
                        );
                    }
                }
            }
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

    for annotation in annotations {
        let RunControlTraceAnnotationV1::CombatAutomationTrajectory { actions, .. } = annotation
        else {
            continue;
        };
        for pending in pending_outcomes.iter_mut() {
            let played_count = actions
                .iter()
                .filter(|action| {
                    selected_card_reward_matches_action_key(pending, &action.action_key)
                })
                .count() as u32;
            if played_count > 0 {
                increment_observation_count(
                    &mut pending.card_reward_observation.picked_card_played_count,
                    played_count,
                );
            }
            let drawn_count = actions
                .iter()
                .flat_map(|action| action.drawn_cards.iter())
                .filter(|card| selected_card_reward_matches_card(pending, card))
                .count() as u32;
            if drawn_count > 0 {
                increment_observation_count(
                    &mut pending.card_reward_observation.picked_card_drawn_count,
                    drawn_count,
                );
            }
        }
    }
}

pub(super) fn resolve_pending_outcomes(
    pending_outcomes: &mut Vec<SessionTracePendingOutcome>,
    attachments: &mut Vec<NonCombatOutcomeAttachmentV1>,
    session: &RunControlSession,
    counters: SessionTraceOutcomeCounters,
    action_result: Option<&ActionResult>,
) -> Result<bool, String> {
    if pending_outcomes.is_empty() {
        return Ok(false);
    }
    let after = noncombat_outcome_snapshot(session, counters);
    let is_noncombat_boundary = is_noncombat_outcome_boundary(session);
    let is_before_next_elite_boundary = is_before_next_elite_boundary(session, action_result);
    if !is_noncombat_boundary && !is_before_next_elite_boundary {
        return Ok(false);
    }

    let mut remaining = Vec::new();
    let mut resolved = false;
    for pending in std::mem::take(pending_outcomes) {
        if outcome_window_reached(
            &pending,
            &after,
            is_noncombat_boundary,
            is_before_next_elite_boundary,
        ) {
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
            | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. }
            | RunControlTraceAnnotationV1::CombatSearchPerformance { .. } => None,
        })
        .collect()
}

fn selected_card_uuid_from_action_result(
    record: &NonCombatDecisionRecordV1,
    action_result: Option<&ActionResult>,
) -> Option<u32> {
    if record.site != DecisionSiteKindV1::CardReward {
        return None;
    }
    let selected_card = selected_card_reward_card_id_from_record(record)?;
    action_result?
        .changes
        .iter()
        .find_map(|change| match change {
            ActionResultChange::CardAdded { card } if card.id == selected_card => Some(card.uuid),
            _ => None,
        })
}

fn outcome_windows_for_record(record: &NonCombatDecisionRecordV1) -> Vec<NonCombatOutcomeWindowV1> {
    if record.site == DecisionSiteKindV1::CardReward {
        vec![
            NonCombatOutcomeWindowV1::AfterOneFloor,
            NonCombatOutcomeWindowV1::BeforeNextElite,
            NonCombatOutcomeWindowV1::AfterNextElite,
            NonCombatOutcomeWindowV1::AfterBoss,
        ]
    } else {
        vec![NonCombatOutcomeWindowV1::AfterOneFloor]
    }
}

fn selected_card_reward_matches_card(
    pending: &SessionTracePendingOutcome,
    card: &crate::eval::run_control::transition_report::CardSnapshot,
) -> bool {
    if let Some(selected_card_uuid) = pending.selected_card_uuid {
        return card.uuid == selected_card_uuid
            && selected_card_reward_card_id(pending)
                .is_some_and(|selected_card| selected_card == card.id);
    }
    selected_card_reward_card_id(pending).is_some_and(|selected_card| selected_card == card.id)
}

fn selected_card_reward_matches_action_key(
    pending: &SessionTracePendingOutcome,
    action_key: &str,
) -> bool {
    let Some(selected_card) = selected_card_reward_card_id(pending) else {
        return false;
    };
    if !action_key.starts_with("combat/play_card/") {
        return false;
    }
    if let Some(selected_card_uuid) = pending.selected_card_uuid {
        return action_key.contains(&format!("#{selected_card_uuid}/target:"))
            && action_key.contains(&format!("/card:{}+", java_id(selected_card)));
    }
    action_key.contains(&format!("/card:{}+", java_id(selected_card)))
}

fn selected_card_reward_card_id(pending: &SessionTracePendingOutcome) -> Option<CardId> {
    selected_card_reward_card_id_from_record(&pending.record)
}

fn selected_card_reward_card_id_from_record(record: &NonCombatDecisionRecordV1) -> Option<CardId> {
    if record.site != DecisionSiteKindV1::CardReward {
        return None;
    }
    let selected_candidate_id = record.selection.selected_candidate_id.as_deref()?;
    let card_id = selected_candidate_id.rsplit(':').next()?;
    serde_json::from_str(&format!("\"{card_id}\"")).ok()
}

fn increment_observation_count(count: &mut Option<u32>, delta: u32) {
    *count = Some(count.unwrap_or(0).saturating_add(delta));
}

fn outcome_window_reached(
    pending: &SessionTracePendingOutcome,
    after: &NonCombatOutcomeSnapshotV1,
    is_noncombat_boundary: bool,
    is_before_next_elite_boundary: bool,
) -> bool {
    if is_noncombat_boundary
        && pending.before.run_terminal != after.run_terminal
        && after.run_terminal.is_some()
    {
        return true;
    }
    match pending.window {
        NonCombatOutcomeWindowV1::AfterOneFloor => {
            is_noncombat_boundary
                && (after.act > pending.before.act || after.floor >= pending.before.floor + 1)
        }
        NonCombatOutcomeWindowV1::AfterThreeFloors => {
            is_noncombat_boundary
                && (after.act > pending.before.act || after.floor >= pending.before.floor + 3)
        }
        NonCombatOutcomeWindowV1::BeforeNextElite => is_before_next_elite_boundary,
        NonCombatOutcomeWindowV1::AfterNextElite => {
            is_noncombat_boundary && after.elites_completed > pending.before.elites_completed
        }
        NonCombatOutcomeWindowV1::AfterBoss => {
            is_noncombat_boundary && after.bosses_completed > pending.before.bosses_completed
        }
        NonCombatOutcomeWindowV1::BeforeBoss | NonCombatOutcomeWindowV1::Manual => false,
    }
}

fn is_before_next_elite_boundary(
    session: &RunControlSession,
    action_result: Option<&ActionResult>,
) -> bool {
    let Some(action_result) = action_result else {
        return false;
    };
    action_result
        .changes
        .iter()
        .any(|change| matches!(change, ActionResultChange::CombatStarted { .. }))
        && session.run_state.map.get_current_room_type() == Some(RoomType::MonsterRoomElite)
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
    use crate::eval::run_control::trace_annotation::CombatAutomationActionV1;
    use crate::eval::run_control::transition_report::{
        ActionResult, ActionResultChange, CardSnapshot, CombatPlayerResult, RunApplyStatus,
    };
    use crate::eval::run_control::{RunControlConfig, RunControlSession};
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::map::{MapRoomNode, MapState};

    #[test]
    fn pending_card_reward_outcome_records_selected_card_upgrade_and_removal() {
        let mut pending_outcomes = Vec::new();
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[selected_card_reward_annotation(CardId::TwinStrike)],
            test_outcome_snapshot(10),
            None,
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
            &[],
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
            &[],
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
    fn pending_card_reward_outcome_counts_selected_card_played_by_combat_automation() {
        let mut pending_outcomes = queued_twin_strike_reward_pending(100);

        update_pending_outcome_observations(
            &mut pending_outcomes,
            &ActionResult {
                chosen_label: "search-combat applied 3 actions".to_string(),
                status: RunApplyStatus::Running,
                changes: Vec::new(),
            },
            &[RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source: "test_search".to_string(),
                action_count: 3,
                actions: vec![
                    combat_automation_action(
                        0,
                        "combat/play_card/hand:1/card:Twin Strike+0#100/target:monster_slot:0",
                        ClientInput::PlayCard {
                            card_index: 1,
                            target: Some(1),
                        },
                    ),
                    combat_automation_action(1, "combat/end_turn", ClientInput::EndTurn),
                    combat_automation_action(
                        2,
                        "combat/play_card/hand:0/card:Twin Strike+1#100/target:monster_slot:0",
                        ClientInput::PlayCard {
                            card_index: 0,
                            target: Some(1),
                        },
                    ),
                ],
                label_role: "simulator_generated_not_teacher_label".to_string(),
            }],
        );

        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_played_count,
            Some(2)
        );
        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_drawn_count,
            None
        );
    }

    #[test]
    fn pending_card_reward_outcome_counts_selected_card_drawn_by_uuid() {
        let mut pending_outcomes = queued_twin_strike_reward_pending(100);

        update_pending_outcome_observations(
            &mut pending_outcomes,
            &ActionResult {
                chosen_label: "draw".to_string(),
                status: RunApplyStatus::Running,
                changes: vec![
                    ActionResultChange::CombatCardDrawn {
                        card: CardSnapshot {
                            id: CardId::TwinStrike,
                            uuid: 999,
                            upgrades: 0,
                        },
                    },
                    ActionResultChange::CombatCardDrawn {
                        card: CardSnapshot {
                            id: CardId::TwinStrike,
                            uuid: 100,
                            upgrades: 0,
                        },
                    },
                ],
            },
            &[],
        );

        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_drawn_count,
            Some(1)
        );
    }

    #[test]
    fn pending_card_reward_outcome_counts_selected_card_drawn_by_combat_automation_annotation() {
        let mut pending_outcomes = queued_twin_strike_reward_pending(100);

        update_pending_outcome_observations(
            &mut pending_outcomes,
            &ActionResult {
                chosen_label: "search-combat applied 1 actions".to_string(),
                status: RunApplyStatus::Running,
                changes: Vec::new(),
            },
            &[RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source: "test_search".to_string(),
                action_count: 1,
                actions: vec![combat_automation_action_with_draws(
                    0,
                    "combat/end_turn",
                    ClientInput::EndTurn,
                    vec![CardSnapshot {
                        id: CardId::TwinStrike,
                        uuid: 100,
                        upgrades: 0,
                    }],
                )],
                label_role: "simulator_generated_not_teacher_label".to_string(),
            }],
        );

        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_drawn_count,
            Some(1)
        );
    }

    #[test]
    fn pending_card_reward_outcome_uses_selected_card_uuid_when_known() {
        let mut pending_outcomes = queued_twin_strike_reward_pending(777);

        update_pending_outcome_observations(
            &mut pending_outcomes,
            &ActionResult {
                chosen_label: "search-combat applied 2 actions".to_string(),
                status: RunApplyStatus::Running,
                changes: Vec::new(),
            },
            &[RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source: "test_search".to_string(),
                action_count: 2,
                actions: vec![
                    combat_automation_action(
                        0,
                        "combat/play_card/hand:1/card:Twin Strike+0#100/target:monster_slot:0",
                        ClientInput::PlayCard {
                            card_index: 1,
                            target: Some(1),
                        },
                    ),
                    combat_automation_action(
                        1,
                        "combat/play_card/hand:0/card:Twin Strike+0#777/target:monster_slot:0",
                        ClientInput::PlayCard {
                            card_index: 0,
                            target: Some(1),
                        },
                    ),
                ],
                label_role: "simulator_generated_not_teacher_label".to_string(),
            }],
        );

        assert_eq!(
            pending_outcomes[0]
                .card_reward_observation
                .picked_card_played_count,
            Some(1)
        );
    }

    #[test]
    fn selected_card_reward_queues_short_elite_and_boss_outcome_windows() {
        let mut pending_outcomes = Vec::new();

        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[selected_card_reward_annotation(CardId::TwinStrike)],
            test_outcome_snapshot(11),
            None,
        );

        let windows = pending_outcomes
            .iter()
            .map(|pending| pending.window)
            .collect::<Vec<_>>();
        assert!(windows.contains(&NonCombatOutcomeWindowV1::AfterOneFloor));
        assert!(windows.contains(&NonCombatOutcomeWindowV1::BeforeNextElite));
        assert!(windows.contains(&NonCombatOutcomeWindowV1::AfterNextElite));
        assert!(windows.contains(&NonCombatOutcomeWindowV1::AfterBoss));
    }

    #[test]
    fn before_next_elite_window_records_card_reward_hp_before_elite() {
        let mut pending_outcomes = Vec::new();
        let mut attachments = Vec::new();
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[selected_card_reward_annotation(CardId::TwinStrike)],
            test_outcome_snapshot(11),
            None,
        );

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 6;
        session.run_state.current_hp = 63;
        let mut elite = MapRoomNode::new(0, 0);
        elite.class = Some(RoomType::MonsterRoomElite);
        session.run_state.map = MapState::new(vec![vec![elite]]);
        session.run_state.map.current_x = 0;
        session.run_state.map.current_y = 0;
        session.engine_state = EngineState::CombatPlayerTurn;
        let action_result = ActionResult {
            chosen_label: "go elite".to_string(),
            status: RunApplyStatus::Running,
            changes: vec![ActionResultChange::CombatStarted {
                player: CombatPlayerResult {
                    hp: 63,
                    max_hp: 80,
                    block: 0,
                    energy: 3,
                },
                monsters: Vec::new(),
            }],
        };

        resolve_pending_outcomes(
            &mut pending_outcomes,
            &mut attachments,
            &session,
            SessionTraceOutcomeCounters {
                combats_completed: 3,
                elites_completed: 0,
                bosses_completed: 0,
            },
            Some(&action_result),
        )
        .expect("before-next-elite should resolve at elite combat start");

        let before_elite_attachment = attachments
            .iter()
            .find(|attachment| attachment.window == NonCombatOutcomeWindowV1::BeforeNextElite)
            .expect("before-next-elite attachment should be emitted");
        let card_reward = before_elite_attachment
            .card_reward
            .as_ref()
            .expect("card reward outcome should be present");
        assert_eq!(card_reward.hp_before_next_elite, Some(63));
        assert_eq!(card_reward.hp_after_next_elite, None);
        assert_eq!(card_reward.next_combat_hp_loss, None);
    }

    #[test]
    fn after_next_elite_window_records_card_reward_hp_after_elite() {
        let mut pending_outcomes = Vec::new();
        let mut attachments = Vec::new();
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[selected_card_reward_annotation(CardId::TwinStrike)],
            test_outcome_snapshot(11),
            None,
        );

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 6;
        session.run_state.current_hp = 57;
        session.engine_state = EngineState::RewardScreen(crate::state::rewards::RewardState::new());

        resolve_pending_outcomes(
            &mut pending_outcomes,
            &mut attachments,
            &session,
            SessionTraceOutcomeCounters {
                combats_completed: 4,
                elites_completed: 1,
                bosses_completed: 0,
            },
            None,
        )
        .expect("after-next-elite card reward outcome should resolve");

        let elite_attachment = attachments
            .iter()
            .find(|attachment| attachment.window == NonCombatOutcomeWindowV1::AfterNextElite)
            .expect("after-next-elite attachment should be emitted");
        let card_reward = elite_attachment
            .card_reward
            .as_ref()
            .expect("card reward outcome should be present");
        assert_eq!(card_reward.hp_after_next_elite, Some(57));
        assert_eq!(card_reward.next_combat_hp_loss, None);
    }

    #[test]
    fn combat_automation_trajectory_counts_as_completed_combat_for_card_reward_outcome() {
        let mut pending_outcomes = Vec::new();
        let mut attachments = Vec::new();
        let mut counters = SessionTraceOutcomeCounters::default();
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[selected_card_reward_annotation(CardId::TwinStrike)],
            test_outcome_snapshot(11),
            None,
        );

        let annotations = vec![RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source: "test_combat_search".to_string(),
            action_count: 2,
            actions: vec![combat_automation_action(
                0,
                "combat/end_turn",
                ClientInput::EndTurn,
            )],
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
        resolve_pending_outcomes(
            &mut pending_outcomes,
            &mut attachments,
            &session,
            counters,
            None,
        )
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

    fn queued_twin_strike_reward_pending(selected_uuid: u32) -> Vec<SessionTracePendingOutcome> {
        let mut pending_outcomes = Vec::new();
        queue_selected_noncombat_outcomes(
            &mut pending_outcomes,
            &[selected_card_reward_annotation(CardId::TwinStrike)],
            test_outcome_snapshot(10),
            Some(&ActionResult {
                chosen_label: "pick Twin Strike".to_string(),
                status: RunApplyStatus::Running,
                changes: vec![ActionResultChange::CardAdded {
                    card: CardSnapshot {
                        id: CardId::TwinStrike,
                        uuid: selected_uuid,
                        upgrades: 0,
                    },
                }],
            }),
        );
        pending_outcomes
    }

    fn selected_card_reward_annotation(card_id: CardId) -> RunControlTraceAnnotationV1 {
        RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record: selected_card_reward_record(card_id),
            card_reward_packet: None,
        }
    }

    fn combat_automation_action(
        step_index: usize,
        action_key: &str,
        input: ClientInput,
    ) -> CombatAutomationActionV1 {
        combat_automation_action_with_draws(step_index, action_key, input, Vec::new())
    }

    fn combat_automation_action_with_draws(
        step_index: usize,
        action_key: &str,
        input: ClientInput,
        drawn_cards: Vec<CardSnapshot>,
    ) -> CombatAutomationActionV1 {
        CombatAutomationActionV1 {
            step_index,
            action_key: action_key.to_string(),
            input,
            drawn_cards,
        }
    }

    fn test_outcome_snapshot(deck_size: usize) -> NonCombatOutcomeSnapshotV1 {
        NonCombatOutcomeSnapshotV1 {
            act: 1,
            floor: 1,
            current_hp: 80,
            max_hp: 80,
            gold: 99,
            deck_size,
            relic_count: 1,
            potion_count: 0,
            combats_completed: 0,
            elites_completed: 0,
            bosses_completed: 0,
            run_terminal: None,
        }
    }
}
