use sts_simulator::ai::planner_core::{
    stable_planner_id, LegalCandidateSet, PlannerObservation, PlannerOutcomeSnapshot,
    PlannerTerminalKind, SelectionProbability,
};
use sts_simulator::eval::run_control::{
    CombatAutomationActionV1, CombatAutomationAnswerSourceV1, PlannerBoundaryVisitOutcomeV1,
    RunCombatResolutionKindV1, RunDecisionSelectionSourceV1, RunProgressStepV1,
};
use sts_simulator::runtime::branch::{
    RunTrajectoryAnswerDeploymentV1, RunTrajectoryBehaviorEventV1,
    RunTrajectoryBehaviorLabelRoleV1, RunTrajectoryBehaviorProjectionV1,
    RunTrajectoryCensorReasonV1, RunTrajectoryDeploymentCensorReasonV1,
    RunTrajectoryDeploymentProjectionV1, RunTrajectoryDeploymentResultV1,
    RunTrajectoryDeploymentStageV1, RunTrajectoryDeploymentSummaryV1, RunTrajectoryHeadV1,
    RunTrajectoryOutcomeAttachmentV1, RunTrajectoryOutcomeFactV1, RunTrajectoryOutcomeHorizonV1,
    RunTrajectoryOutcomeProjectionV1, RunTrajectoryOutcomeResultV1, RunTrajectoryReconstructionV1,
    RunTrajectorySegmentDispositionV1, RunTrajectoryTerminalOutcomeV1,
    RunTrajectoryVisitOccurrenceV1, RUN_TRAJECTORY_BEHAVIOR_PROJECTION_SCHEMA_NAME,
    RUN_TRAJECTORY_BEHAVIOR_PROJECTION_SCHEMA_VERSION,
    RUN_TRAJECTORY_DEPLOYMENT_PROJECTION_SCHEMA_NAME,
    RUN_TRAJECTORY_DEPLOYMENT_PROJECTION_SCHEMA_VERSION,
    RUN_TRAJECTORY_OUTCOME_PROJECTION_SCHEMA_NAME,
    RUN_TRAJECTORY_OUTCOME_PROJECTION_SCHEMA_VERSION,
};
use sts_simulator::state::core::ClientInput;

use std::collections::BTreeMap;

use super::trajectory_artifact_store::TrajectoryArtifactStore;

pub(super) struct RunTrajectoryProjectionBundleV1 {
    pub(super) reconstruction: RunTrajectoryReconstructionV1,
    pub(super) behavior: RunTrajectoryBehaviorProjectionV1,
    pub(super) outcomes: RunTrajectoryOutcomeProjectionV1,
    pub(super) deployment: RunTrajectoryDeploymentProjectionV1,
}

pub(super) fn project_trajectory(
    store: &TrajectoryArtifactStore,
    run_id: &str,
    head: &RunTrajectoryHeadV1,
) -> Result<RunTrajectoryProjectionBundleV1, String> {
    let reconstruction = store.reconstruct(run_id, head)?;
    let occurrences = load_occurrences(store, &reconstruction)?;
    let projected = project_behavior_events(&reconstruction, &occurrences)?;
    let behavior = RunTrajectoryBehaviorProjectionV1 {
        schema_name: RUN_TRAJECTORY_BEHAVIOR_PROJECTION_SCHEMA_NAME.to_string(),
        schema_version: RUN_TRAJECTORY_BEHAVIOR_PROJECTION_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        head: head.clone(),
        events: projected.iter().map(|item| item.event.clone()).collect(),
    };
    let outcomes = project_outcomes(&reconstruction, &occurrences, &projected)?;
    let deployment = project_deployment(&reconstruction)?;
    Ok(RunTrajectoryProjectionBundleV1 {
        reconstruction,
        behavior,
        outcomes,
        deployment,
    })
}

fn project_deployment(
    reconstruction: &RunTrajectoryReconstructionV1,
) -> Result<RunTrajectoryDeploymentProjectionV1, String> {
    let mut records = Vec::new();
    for segment in &reconstruction.segments {
        for (journal_ordinal, step) in segment.progress_journal.entries().iter().enumerate() {
            let RunProgressStepV1::CombatResolution(resolution) = step else {
                continue;
            };
            if resolution.kind == RunCombatResolutionKindV1::SmokeBombEscape {
                continue;
            }
            let has_opportunity = resolution
                .trajectory
                .actions
                .iter()
                .any(|action| action.opportunity_before.is_some());
            for claim in &resolution.trajectory.answer_claims {
                let stage = deployment_stage(&claim.source, &resolution.trajectory.actions);
                let result = deployment_result(resolution.kind, has_opportunity, stage);
                for axis in &claim.axes {
                    let mut record = RunTrajectoryAnswerDeploymentV1 {
                        deployment_id: String::new(),
                        segment_id: segment.segment_id.clone(),
                        journal_ordinal,
                        combat_sequence: resolution.before.combat_sequence,
                        resolution_kind: resolution.kind,
                        source: claim.source.clone(),
                        axis: *axis,
                        result: result.clone(),
                    };
                    record.deployment_id = stable_planner_id("trajectory_deployment", &record)?;
                    records.push(record);
                }
            }
        }
    }
    let summary = deployment_summary(&records);
    Ok(RunTrajectoryDeploymentProjectionV1 {
        schema_name: RUN_TRAJECTORY_DEPLOYMENT_PROJECTION_SCHEMA_NAME.to_string(),
        schema_version: RUN_TRAJECTORY_DEPLOYMENT_PROJECTION_SCHEMA_VERSION,
        run_id: reconstruction.run_id.clone(),
        head: reconstruction.head.clone(),
        records,
        summary,
    })
}

fn deployment_result(
    resolution_kind: RunCombatResolutionKindV1,
    has_opportunity: bool,
    stage: RunTrajectoryDeploymentStageV1,
) -> RunTrajectoryDeploymentResultV1 {
    if stage == RunTrajectoryDeploymentStageV1::Applied {
        RunTrajectoryDeploymentResultV1::Observed { stage }
    } else if !has_opportunity {
        RunTrajectoryDeploymentResultV1::Censored {
            highest_observed_stage: stage,
            reason: RunTrajectoryDeploymentCensorReasonV1::OpportunityObservationUnavailable,
        }
    } else if resolution_kind == RunCombatResolutionKindV1::TurnSegment {
        RunTrajectoryDeploymentResultV1::Censored {
            highest_observed_stage: stage,
            reason: RunTrajectoryDeploymentCensorReasonV1::CombatResolutionContinues,
        }
    } else {
        RunTrajectoryDeploymentResultV1::Observed { stage }
    }
}

fn deployment_stage(
    source: &CombatAutomationAnswerSourceV1,
    actions: &[CombatAutomationActionV1],
) -> RunTrajectoryDeploymentStageV1 {
    let mut stage = RunTrajectoryDeploymentStageV1::Claimed;
    for action in actions {
        let Some(opportunity) = action.opportunity_before.as_ref() else {
            continue;
        };
        match source {
            CombatAutomationAnswerSourceV1::Card { uuid, .. } => {
                if opportunity.hand.iter().any(|card| card.uuid == *uuid) {
                    stage = stage.max(RunTrajectoryDeploymentStageV1::Reached);
                }
                if opportunity.playable_card_uuids.contains(uuid) {
                    stage = stage.max(RunTrajectoryDeploymentStageV1::Playable);
                }
                if matches!(
                    action.input,
                    ClientInput::PlayCard { card_index, .. }
                        if opportunity.hand.get(card_index).is_some_and(|card| card.uuid == *uuid)
                ) {
                    stage = RunTrajectoryDeploymentStageV1::Applied;
                }
            }
            CombatAutomationAnswerSourceV1::Potion { uuid, .. } => {
                if opportunity
                    .potions
                    .iter()
                    .flatten()
                    .any(|potion| potion.uuid == *uuid)
                {
                    stage = stage.max(RunTrajectoryDeploymentStageV1::Reached);
                }
                if opportunity.usable_potion_uuids.contains(uuid) {
                    stage = stage.max(RunTrajectoryDeploymentStageV1::Playable);
                }
                if matches!(
                    action.input,
                    ClientInput::UsePotion { potion_index, .. }
                        if opportunity
                            .potions
                            .get(potion_index)
                            .and_then(Option::as_ref)
                            .is_some_and(|potion| potion.uuid == *uuid)
                ) {
                    stage = RunTrajectoryDeploymentStageV1::Applied;
                }
            }
        }
    }
    stage
}

fn deployment_summary(
    records: &[RunTrajectoryAnswerDeploymentV1],
) -> RunTrajectoryDeploymentSummaryV1 {
    let mut instances = BTreeMap::new();
    for record in records {
        let (stage, censored) = match record.result {
            RunTrajectoryDeploymentResultV1::Observed { stage } => (stage, false),
            RunTrajectoryDeploymentResultV1::Censored {
                highest_observed_stage,
                ..
            } => (highest_observed_stage, true),
        };
        let (source_kind, source_uuid) = match record.source {
            CombatAutomationAnswerSourceV1::Card { uuid, .. } => (0_u8, uuid),
            CombatAutomationAnswerSourceV1::Potion { uuid, .. } => (1_u8, uuid),
        };
        instances
            .entry((
                record.combat_sequence,
                source_kind,
                source_uuid,
                record.axis,
            ))
            .and_modify(
                |(highest_stage, latest_censored): &mut (RunTrajectoryDeploymentStageV1, bool)| {
                    *highest_stage = (*highest_stage).max(stage);
                    *latest_censored = censored;
                },
            )
            .or_insert((stage, censored));
    }
    let mut summary = RunTrajectoryDeploymentSummaryV1::default();
    for (stage, censored) in instances.into_values() {
        summary.claimed_answers = summary.claimed_answers.saturating_add(1);
        if stage >= RunTrajectoryDeploymentStageV1::Reached {
            summary.reached_answers = summary.reached_answers.saturating_add(1);
        }
        if stage >= RunTrajectoryDeploymentStageV1::Playable {
            summary.playable_answers = summary.playable_answers.saturating_add(1);
        }
        if stage >= RunTrajectoryDeploymentStageV1::Applied {
            summary.applied_answers = summary.applied_answers.saturating_add(1);
        }
        if censored {
            summary.censored_answers = summary.censored_answers.saturating_add(1);
        }
    }
    summary
}

struct LoadedOccurrence {
    segment_index: usize,
    segment_id: String,
    segment_depth: u64,
    occurrence: RunTrajectoryVisitOccurrenceV1,
    observation: PlannerObservation,
    candidate_set: LegalCandidateSet,
}

#[derive(Clone)]
struct ProjectedBehavior {
    event: RunTrajectoryBehaviorEventV1,
    occurrence_position: usize,
    segment_index: usize,
    journal_ordinal: usize,
    before: PlannerOutcomeSnapshot,
}

fn load_occurrences(
    store: &TrajectoryArtifactStore,
    reconstruction: &RunTrajectoryReconstructionV1,
) -> Result<Vec<LoadedOccurrence>, String> {
    let mut loaded = Vec::new();
    for (segment_index, segment) in reconstruction.segments.iter().enumerate() {
        for occurrence in &segment.boundary_visit_occurrences {
            let observation = store.read_observation(&occurrence.observation_id)?;
            let candidate_set = store.read_candidate_set(&occurrence.legal_candidate_set_id)?;
            if observation.observation_id != occurrence.observation_id {
                return Err(format!(
                    "trajectory occurrence {} observation payload mismatch",
                    occurrence.occurrence_id
                ));
            }
            if candidate_set.candidate_set_id != occurrence.legal_candidate_set_id
                || candidate_set.observation_id != observation.observation_id
            {
                return Err(format!(
                    "trajectory occurrence {} candidate-set payload mismatch",
                    occurrence.occurrence_id
                ));
            }
            loaded.push(LoadedOccurrence {
                segment_index,
                segment_id: segment.segment_id.clone(),
                segment_depth: segment.depth,
                occurrence: occurrence.clone(),
                observation,
                candidate_set,
            });
        }
    }
    Ok(loaded)
}

fn project_behavior_events(
    reconstruction: &RunTrajectoryReconstructionV1,
    occurrences: &[LoadedOccurrence],
) -> Result<Vec<ProjectedBehavior>, String> {
    let mut projected = Vec::new();
    for (occurrence_position, loaded) in occurrences.iter().enumerate() {
        let PlannerBoundaryVisitOutcomeV1::Selected {
            selection_source,
            run_candidate_id,
            planner_candidate_id,
        } = &loaded.occurrence.outcome
        else {
            continue;
        };
        let segment = &reconstruction.segments[loaded.segment_index];
        let matching = segment
            .progress_journal
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(ordinal, step)| step.as_decision().map(|decision| (ordinal, decision)))
            .filter(|(_, decision)| {
                decision.before.decision_step == loaded.occurrence.decision_step
                    && decision.selection.source == *selection_source
                    && decision.selection.candidate_id == *run_candidate_id
            })
            .collect::<Vec<_>>();
        let [(journal_ordinal, transaction)] = matching.as_slice() else {
            return Err(format!(
                "selected occurrence {} matched {} transactions during projection",
                loaded.occurrence.occurrence_id,
                matching.len()
            ));
        };
        if !loaded
            .candidate_set
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_id == *planner_candidate_id)
        {
            return Err(format!(
                "selected occurrence {} references absent planner candidate {}",
                loaded.occurrence.occurrence_id, planner_candidate_id
            ));
        }
        if !loaded.occurrence.candidate_links.iter().any(|link| {
            link.run_candidate_id == *run_candidate_id
                && link.planner_candidate_id == *planner_candidate_id
        }) {
            return Err(format!(
                "selected occurrence {} lacks its run/planner candidate link",
                loaded.occurrence.occurrence_id
            ));
        }
        let sequence = projected.len() as u64;
        let mut event = RunTrajectoryBehaviorEventV1 {
            behavior_id: String::new(),
            segment_id: loaded.segment_id.clone(),
            occurrence_id: loaded.occurrence.occurrence_id.clone(),
            segment_depth: loaded.segment_depth,
            journal_ordinal: *journal_ordinal,
            sequence,
            policy_lane: segment.policy_lane,
            decision_step: loaded.occurrence.decision_step,
            decision_id: loaded.candidate_set.decision_id.clone(),
            observation_id: loaded.observation.observation_id.clone(),
            legal_candidate_set_id: loaded.candidate_set.candidate_set_id.clone(),
            run_candidate_id: transaction.selection.candidate_id.clone(),
            planner_candidate_id: planner_candidate_id.clone(),
            selection_source: *selection_source,
            selection_probability: selection_probability(*selection_source),
            mechanics: loaded.observation.mechanics.clone(),
            label_role: RunTrajectoryBehaviorLabelRoleV1::ObservedBehaviorNotTeacher,
        };
        event.behavior_id = stable_planner_id("trajectory_behavior", &event)?;
        projected.push(ProjectedBehavior {
            event,
            occurrence_position,
            segment_index: loaded.segment_index,
            journal_ordinal: *journal_ordinal,
            before: outcome_snapshot(&loaded.observation, None),
        });
    }
    Ok(projected)
}

fn selection_probability(source: RunDecisionSelectionSourceV1) -> SelectionProbability {
    match source {
        RunDecisionSelectionSourceV1::OnlyVisibleCandidate => {
            SelectionProbability::KnownDeterministic
        }
        RunDecisionSelectionSourceV1::ExplicitCandidate
        | RunDecisionSelectionSourceV1::RoutinePolicy
        | RunDecisionSelectionSourceV1::RoutePolicy
        | RunDecisionSelectionSourceV1::OwnerPolicy
        | RunDecisionSelectionSourceV1::RewardPolicy => SelectionProbability::Unknown,
    }
}

fn project_outcomes(
    reconstruction: &RunTrajectoryReconstructionV1,
    occurrences: &[LoadedOccurrence],
    behaviors: &[ProjectedBehavior],
) -> Result<RunTrajectoryOutcomeProjectionV1, String> {
    let final_segment = reconstruction
        .segments
        .last()
        .ok_or_else(|| "trajectory reconstruction has no segments".to_string())?;
    let censor = censor_reason(final_segment.disposition);
    let terminal = RunTrajectoryTerminalOutcomeV1::from_disposition(final_segment.disposition).map(
        |outcome| RunTrajectoryOutcomeFactV1::Terminal {
            segment_id: final_segment.segment_id.clone(),
            outcome,
        },
    );
    let mut attachments = Vec::with_capacity(behaviors.len().saturating_mul(4));
    for behavior in behaviors {
        let immediate = occurrences
            .get(behavior.occurrence_position.saturating_add(1))
            .map(observation_fact)
            .or_else(|| terminal.clone())
            .map(observed)
            .unwrap_or_else(|| censored(censor));
        attachments.push(attachment(
            behavior,
            RunTrajectoryOutcomeHorizonV1::ImmediateCommittedSuccessor,
            immediate,
        )?);

        let next_combat = next_combat_fact(reconstruction, behavior)
            .map(observed)
            .unwrap_or_else(|| censored(censor));
        attachments.push(attachment(
            behavior,
            RunTrajectoryOutcomeHorizonV1::NextCombatResolution,
            next_combat,
        )?);

        let act_terminal = occurrences
            .iter()
            .skip(behavior.occurrence_position.saturating_add(1))
            .find(|loaded| loaded.observation.run.act > behavior.before.act)
            .map(observation_fact)
            .or_else(|| terminal.clone())
            .map(observed)
            .unwrap_or_else(|| censored(censor));
        attachments.push(attachment(
            behavior,
            RunTrajectoryOutcomeHorizonV1::ActTerminal,
            act_terminal,
        )?);

        let run_terminal = terminal
            .clone()
            .map(observed)
            .unwrap_or_else(|| censored(censor));
        attachments.push(attachment(
            behavior,
            RunTrajectoryOutcomeHorizonV1::RunTerminal,
            run_terminal,
        )?);
    }
    Ok(RunTrajectoryOutcomeProjectionV1 {
        schema_name: RUN_TRAJECTORY_OUTCOME_PROJECTION_SCHEMA_NAME.to_string(),
        schema_version: RUN_TRAJECTORY_OUTCOME_PROJECTION_SCHEMA_VERSION,
        run_id: reconstruction.run_id.clone(),
        head: reconstruction.head.clone(),
        attachments,
    })
}

fn next_combat_fact(
    reconstruction: &RunTrajectoryReconstructionV1,
    behavior: &ProjectedBehavior,
) -> Option<RunTrajectoryOutcomeFactV1> {
    for (segment_index, segment) in reconstruction
        .segments
        .iter()
        .enumerate()
        .skip(behavior.segment_index)
    {
        let start = if segment_index == behavior.segment_index {
            behavior.journal_ordinal.saturating_add(1)
        } else {
            0
        };
        for (journal_ordinal, step) in segment
            .progress_journal
            .entries()
            .iter()
            .enumerate()
            .skip(start)
        {
            if let RunProgressStepV1::CombatResolution(resolution) = step {
                return Some(RunTrajectoryOutcomeFactV1::CombatResolution {
                    segment_id: segment.segment_id.clone(),
                    journal_ordinal,
                    resolution_kind: resolution.kind,
                });
            }
        }
    }
    None
}

fn observation_fact(loaded: &LoadedOccurrence) -> RunTrajectoryOutcomeFactV1 {
    RunTrajectoryOutcomeFactV1::Observation {
        segment_id: loaded.segment_id.clone(),
        occurrence_id: loaded.occurrence.occurrence_id.clone(),
        observation_id: loaded.observation.observation_id.clone(),
        snapshot: outcome_snapshot(&loaded.observation, None),
    }
}

fn outcome_snapshot(
    observation: &PlannerObservation,
    terminal: Option<PlannerTerminalKind>,
) -> PlannerOutcomeSnapshot {
    PlannerOutcomeSnapshot {
        act: observation.run.act,
        floor: observation.run.floor,
        current_hp: observation.run.current_hp,
        max_hp: observation.run.max_hp,
        gold: observation.run.gold,
        deck_size: observation.cards.len(),
        relic_count: observation.relics.len(),
        potion_count: observation
            .potions
            .iter()
            .filter(|slot| slot.potion.is_some())
            .count(),
        terminal,
    }
}

fn attachment(
    behavior: &ProjectedBehavior,
    horizon: RunTrajectoryOutcomeHorizonV1,
    result: RunTrajectoryOutcomeResultV1,
) -> Result<RunTrajectoryOutcomeAttachmentV1, String> {
    let mut attachment = RunTrajectoryOutcomeAttachmentV1 {
        attachment_id: String::new(),
        behavior_id: behavior.event.behavior_id.clone(),
        horizon,
        before: behavior.before.clone(),
        result,
    };
    attachment.attachment_id = stable_planner_id("trajectory_outcome", &attachment)?;
    Ok(attachment)
}

fn observed(fact: RunTrajectoryOutcomeFactV1) -> RunTrajectoryOutcomeResultV1 {
    RunTrajectoryOutcomeResultV1::Observed { fact }
}

fn censored(reason: RunTrajectoryCensorReasonV1) -> RunTrajectoryOutcomeResultV1 {
    RunTrajectoryOutcomeResultV1::Censored { reason }
}

fn censor_reason(disposition: RunTrajectorySegmentDispositionV1) -> RunTrajectoryCensorReasonV1 {
    match disposition {
        RunTrajectorySegmentDispositionV1::Resumable => {
            RunTrajectoryCensorReasonV1::TrajectoryHeadResumable
        }
        RunTrajectorySegmentDispositionV1::Stopped
        | RunTrajectorySegmentDispositionV1::TerminalVictory
        | RunTrajectorySegmentDispositionV1::TerminalDefeat => {
            RunTrajectoryCensorReasonV1::TrajectoryStoppedBeforeHorizon
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::{
        build_decision_surface, capture_planner_boundary_ticket_v1, RunControlSession,
        RunProgressJournalV1,
    };

    use crate::runtime::branch::owner_audit::branch_model::{
        Branch, BranchStatus, Owner, TerminalOutcome,
    };
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;

    fn selected_branch() -> Branch {
        let mut session = RunControlSession::new(Default::default());
        let ticket = capture_planner_boundary_ticket_v1(&session)
            .unwrap()
            .expect("planner-visible boundary");
        let candidate_id = build_decision_surface(&session).view.candidates[0]
            .id
            .clone();
        let outcome = session.apply_candidate_id(&candidate_id).unwrap();
        let journal = RunProgressJournalV1::from_committed_steps(outcome.progress_steps.clone())
            .expect("decision journal");
        let capture = ticket.finish_for_progress(&outcome.progress_steps);
        Branch {
            id: 1,
            parent_id: None,
            path: Vec::new(),
            session,
            status: BranchStatus::Running {
                owner: Owner::NeowStart,
                boundary: "Neow".to_string(),
            },
            policy_lane: BranchPolicyLane::default(),
            combat_portfolio: None,
            recent_progress_journal: journal,
            recent_planner_capture: capture,
            trajectory: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    #[test]
    fn durable_chain_projects_stable_behavior_and_censored_raw_horizons() {
        let root = std::env::temp_dir().join(format!(
            "trajectory_projector_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = TrajectoryArtifactStore::new(root.clone());
        let mut parent = selected_branch();
        parent
            .bind_trajectory_run("trajectory_run:projection", 0)
            .unwrap();
        store.commit_branch(&mut parent).unwrap();
        let mut child = parent.clone();
        child.id = 2;
        child.parent_id = Some(parent.id);
        child.status = BranchStatus::Terminal(TerminalOutcome::Victory);
        child.capture_recent_trajectory(1).unwrap();
        store.commit_branch(&mut child).unwrap();
        let head = child.trajectory.committed_head().unwrap();

        let first = project_trajectory(&store, "trajectory_run:projection", head).unwrap();
        let second = project_trajectory(&store, "trajectory_run:projection", head).unwrap();

        assert_eq!(first.behavior, second.behavior);
        assert_eq!(first.outcomes, second.outcomes);
        assert_eq!(first.deployment, second.deployment);
        assert_eq!(first.reconstruction.segments.len(), 2);
        assert_eq!(first.behavior.events.len(), 2);
        assert_eq!(first.behavior.events[0].sequence, 0);
        assert_eq!(first.behavior.events[1].sequence, 1);
        assert_eq!(first.behavior.events[0].journal_ordinal, 0);
        assert_eq!(
            first.behavior.events[0].selection_probability,
            SelectionProbability::Unknown
        );
        assert_eq!(first.outcomes.attachments.len(), 8);
        assert!(matches!(
            first.outcomes.attachments[0].result,
            RunTrajectoryOutcomeResultV1::Observed {
                fact: RunTrajectoryOutcomeFactV1::Observation { .. }
            }
        ));
        assert!(first.outcomes.attachments.iter().any(|attachment| {
            attachment.horizon == RunTrajectoryOutcomeHorizonV1::NextCombatResolution
                && matches!(
                    attachment.result,
                    RunTrajectoryOutcomeResultV1::Censored {
                        reason: RunTrajectoryCensorReasonV1::TrajectoryStoppedBeforeHorizon
                    }
                )
        }));
        assert!(first.outcomes.attachments.iter().all(|attachment| {
            attachment.horizon != RunTrajectoryOutcomeHorizonV1::RunTerminal
                || matches!(
                    attachment.result,
                    RunTrajectoryOutcomeResultV1::Observed {
                        fact: RunTrajectoryOutcomeFactV1::Terminal {
                            outcome: RunTrajectoryTerminalOutcomeV1::Victory,
                            ..
                        }
                    }
                )
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn deployment_stage_uses_instance_legal_mask_and_committed_input() {
        let action = CombatAutomationActionV1 {
            step_index: 0,
            action_key: "combat/play".to_string(),
            input: ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            opportunity_before: Some(
                sts_simulator::eval::run_control::CombatAutomationOpportunityStateV1 {
                    turn: 1,
                    energy: 1,
                    hand: vec![sts_simulator::eval::run_control::RunActionCardSnapshotV1 {
                        id: sts_simulator::content::cards::CardId::Anger,
                        uuid: 10,
                        upgrades: 0,
                    }],
                    potions: vec![Some(
                        sts_simulator::eval::run_control::CombatAutomationPotionStateV1 {
                            id: sts_simulator::content::potions::PotionId::BlockPotion,
                            uuid: 20,
                        },
                    )],
                    playable_card_uuids: vec![10],
                    usable_potion_uuids: vec![20],
                },
            ),
            drawn_cards: Vec::new(),
            combat_after: None,
        };

        assert_eq!(
            deployment_stage(
                &CombatAutomationAnswerSourceV1::Card {
                    id: sts_simulator::content::cards::CardId::Anger,
                    uuid: 10,
                    upgrades: 0,
                    origin:
                        sts_simulator::eval::run_control::CombatAutomationCardOriginV1::MasterDeck,
                },
                std::slice::from_ref(&action),
            ),
            RunTrajectoryDeploymentStageV1::Applied
        );
        assert_eq!(
            deployment_stage(
                &CombatAutomationAnswerSourceV1::Potion {
                    id: sts_simulator::content::potions::PotionId::BlockPotion,
                    uuid: 20,
                },
                std::slice::from_ref(&action),
            ),
            RunTrajectoryDeploymentStageV1::Playable
        );
        assert_eq!(
            deployment_stage(
                &CombatAutomationAnswerSourceV1::Card {
                    id: sts_simulator::content::cards::CardId::Defend,
                    uuid: 30,
                    upgrades: 0,
                    origin:
                        sts_simulator::eval::run_control::CombatAutomationCardOriginV1::MasterDeck,
                },
                &[action],
            ),
            RunTrajectoryDeploymentStageV1::Claimed
        );
    }

    #[test]
    fn unfinished_combat_censors_unapplied_answers_but_not_applied_facts() {
        assert_eq!(
            deployment_result(
                RunCombatResolutionKindV1::TurnSegment,
                true,
                RunTrajectoryDeploymentStageV1::Playable,
            ),
            RunTrajectoryDeploymentResultV1::Censored {
                highest_observed_stage: RunTrajectoryDeploymentStageV1::Playable,
                reason: RunTrajectoryDeploymentCensorReasonV1::CombatResolutionContinues,
            }
        );
        assert_eq!(
            deployment_result(
                RunCombatResolutionKindV1::TurnSegment,
                true,
                RunTrajectoryDeploymentStageV1::Applied,
            ),
            RunTrajectoryDeploymentResultV1::Observed {
                stage: RunTrajectoryDeploymentStageV1::Applied,
            }
        );
        assert_eq!(
            deployment_result(
                RunCombatResolutionKindV1::CompleteVictory,
                false,
                RunTrajectoryDeploymentStageV1::Claimed,
            ),
            RunTrajectoryDeploymentResultV1::Censored {
                highest_observed_stage: RunTrajectoryDeploymentStageV1::Claimed,
                reason: RunTrajectoryDeploymentCensorReasonV1::OpportunityObservationUnavailable,
            }
        );
    }

    #[test]
    fn deployment_summary_merges_turn_segments_into_the_closed_combat_window() {
        let source = CombatAutomationAnswerSourceV1::Card {
            id: sts_simulator::content::cards::CardId::Shockwave,
            uuid: 10,
            upgrades: 0,
            origin: sts_simulator::eval::run_control::CombatAutomationCardOriginV1::MasterDeck,
        };
        let records = vec![
            RunTrajectoryAnswerDeploymentV1 {
                deployment_id: "turn-segment".to_string(),
                segment_id: "segment-1".to_string(),
                journal_ordinal: 0,
                combat_sequence: 7,
                resolution_kind: RunCombatResolutionKindV1::TurnSegment,
                source: source.clone(),
                axis: sts_simulator::ai::strategy::pressure_assessment::PressureAxis::DelayCapacity,
                result: RunTrajectoryDeploymentResultV1::Censored {
                    highest_observed_stage: RunTrajectoryDeploymentStageV1::Playable,
                    reason: RunTrajectoryDeploymentCensorReasonV1::CombatResolutionContinues,
                },
            },
            RunTrajectoryAnswerDeploymentV1 {
                deployment_id: "complete".to_string(),
                segment_id: "segment-2".to_string(),
                journal_ordinal: 0,
                combat_sequence: 7,
                resolution_kind: RunCombatResolutionKindV1::CompleteVictory,
                source,
                axis: sts_simulator::ai::strategy::pressure_assessment::PressureAxis::DelayCapacity,
                result: RunTrajectoryDeploymentResultV1::Observed {
                    stage: RunTrajectoryDeploymentStageV1::Claimed,
                },
            },
        ];

        let summary = deployment_summary(&records);

        assert_eq!(summary.claimed_answers, 1);
        assert_eq!(summary.reached_answers, 1);
        assert_eq!(summary.playable_answers, 1);
        assert_eq!(summary.applied_answers, 0);
        assert_eq!(summary.censored_answers, 0);
    }

    #[test]
    fn resumable_head_censors_every_unobserved_horizon_without_fabricating_defeat() {
        let root = std::env::temp_dir().join(format!(
            "trajectory_projector_censor_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = TrajectoryArtifactStore::new(root.clone());
        let mut branch = selected_branch();
        branch
            .bind_trajectory_run("trajectory_run:censor", 0)
            .unwrap();
        store.commit_branch(&mut branch).unwrap();

        let projection = project_trajectory(
            &store,
            "trajectory_run:censor",
            branch.trajectory.committed_head().unwrap(),
        )
        .unwrap();

        assert_eq!(projection.behavior.events.len(), 1);
        assert_eq!(projection.outcomes.attachments.len(), 4);
        assert!(projection.outcomes.attachments.iter().all(|attachment| {
            matches!(
                attachment.result,
                RunTrajectoryOutcomeResultV1::Censored {
                    reason: RunTrajectoryCensorReasonV1::TrajectoryHeadResumable
                }
            )
        }));
        let _ = std::fs::remove_dir_all(root);
    }
}
