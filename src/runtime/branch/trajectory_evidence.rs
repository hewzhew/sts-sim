use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ai::planner_core::{stable_planner_id, LegalCandidateSet, PlannerObservation};
use crate::eval::run_control::{
    PlannerBoundaryCandidateLinkV1, PlannerBoundaryCaptureSegmentV1, PlannerBoundaryMutationKindV1,
    PlannerBoundaryVisitOutcomeV1, RunProgressJournalV1, RunProgressStepV1,
};

pub const RUN_TRAJECTORY_SEGMENT_SCHEMA_NAME: &str = "RunTrajectorySegment";
pub const RUN_TRAJECTORY_SEGMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrajectorySegmentDispositionV1 {
    Resumable,
    TerminalVictory,
    TerminalDefeat,
    Stopped,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunTrajectoryPolicyLaneV1 {
    Baseline,
    Challenger { lane_id: u8 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryHeadV1 {
    pub segment_id: String,
    pub depth: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryVisitOccurrenceV1 {
    pub occurrence_id: String,
    pub visit_id: String,
    pub decision_step: u64,
    pub observation_id: String,
    pub legal_candidate_set_id: String,
    pub candidate_links: Vec<PlannerBoundaryCandidateLinkV1>,
    pub outcome: PlannerBoundaryVisitOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectorySegmentV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub segment_id: String,
    pub run_id: String,
    pub branch_id: u64,
    pub policy_lane: RunTrajectoryPolicyLaneV1,
    pub parent_segment_id: Option<String>,
    pub generation: u64,
    pub depth: u64,
    pub disposition: RunTrajectorySegmentDispositionV1,
    pub progress_journal: RunProgressJournalV1,
    pub boundary_visit_occurrences: Vec<RunTrajectoryVisitOccurrenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunTrajectorySegmentDraftV1 {
    pub segment: RunTrajectorySegmentV1,
    pub observations: Vec<PlannerObservation>,
    pub legal_candidate_sets: Vec<LegalCandidateSet>,
}

impl RunTrajectorySegmentDraftV1 {
    pub fn head(&self) -> RunTrajectoryHeadV1 {
        RunTrajectoryHeadV1 {
            segment_id: self.segment.segment_id.clone(),
            depth: self.segment.depth,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunTrajectoryIntegrityGapV1 {
    ObservationIdMismatch {
        visit_id: String,
        expected: String,
        actual: String,
    },
    CandidateSetIdMismatch {
        visit_id: String,
        expected: String,
        actual: String,
    },
    CandidateSetObservationMismatch {
        visit_id: String,
        observation_id: String,
        candidate_set_observation_id: String,
    },
    ConflictingObservationPayload {
        observation_id: String,
    },
    ConflictingCandidateSetPayload {
        candidate_set_id: String,
    },
    DuplicateOccurrenceId {
        occurrence_id: String,
    },
    SelectedOccurrenceTransactionCount {
        occurrence_id: String,
        match_count: usize,
    },
    UnrepresentedSelectionTransactionCount {
        occurrence_id: String,
        match_count: usize,
    },
    DecisionTransactionOccurrenceCount {
        decision_step: u64,
        run_candidate_id: String,
        match_count: usize,
    },
    ForcedMutationOccurrenceCount {
        decision_step: u64,
        match_count: usize,
    },
    ForcedTransitionOccurrenceCount {
        decision_step: u64,
        match_count: usize,
    },
    StopInsideCommittedJournal,
    SegmentIdMismatch {
        expected: String,
        actual: String,
    },
    IdEncodingFailed {
        subject: String,
        message: String,
    },
}

impl std::fmt::Display for RunTrajectoryIntegrityGapV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RunTrajectoryIntegrityGapV1 {}

#[allow(clippy::too_many_arguments)]
pub fn build_run_trajectory_segment_v1(
    run_id: &str,
    branch_id: u64,
    policy_lane: RunTrajectoryPolicyLaneV1,
    generation: u64,
    parent_head: Option<&RunTrajectoryHeadV1>,
    disposition: RunTrajectorySegmentDispositionV1,
    progress_journal: &RunProgressJournalV1,
    planner_capture: &PlannerBoundaryCaptureSegmentV1,
) -> Result<Option<RunTrajectorySegmentDraftV1>, RunTrajectoryIntegrityGapV1> {
    if progress_journal.is_empty() && planner_capture.visits.is_empty() {
        return Ok(None);
    }
    if progress_journal
        .entries()
        .iter()
        .any(|entry| matches!(entry, RunProgressStepV1::Stop(_)))
    {
        return Err(RunTrajectoryIntegrityGapV1::StopInsideCommittedJournal);
    }

    let mut observations = BTreeMap::<String, PlannerObservation>::new();
    let mut candidate_sets = BTreeMap::<String, LegalCandidateSet>::new();
    let mut occurrence_ids = BTreeSet::new();
    let mut occurrences = Vec::with_capacity(planner_capture.visits.len());
    for (index, visit) in planner_capture.visits.iter().enumerate() {
        validate_observation_id(&visit.visit_id, &visit.observation)?;
        validate_candidate_set_id(&visit.visit_id, &visit.legal_candidate_set)?;
        if visit.observation.observation_id != visit.legal_candidate_set.observation_id {
            return Err(
                RunTrajectoryIntegrityGapV1::CandidateSetObservationMismatch {
                    visit_id: visit.visit_id.clone(),
                    observation_id: visit.observation.observation_id.clone(),
                    candidate_set_observation_id: visit.legal_candidate_set.observation_id.clone(),
                },
            );
        }
        insert_observation(&mut observations, &visit.observation)?;
        insert_candidate_set(&mut candidate_sets, &visit.legal_candidate_set)?;
        let occurrence_id = stable_planner_id(
            "visit_occurrence",
            &(
                run_id,
                branch_id,
                policy_lane,
                generation,
                parent_head.map(|head| head.segment_id.as_str()),
                index,
                visit.visit_id.as_str(),
                &visit.outcome,
            ),
        )
        .map_err(|message| RunTrajectoryIntegrityGapV1::IdEncodingFailed {
            subject: "visit_occurrence".to_string(),
            message,
        })?;
        if !occurrence_ids.insert(occurrence_id.clone()) {
            return Err(RunTrajectoryIntegrityGapV1::DuplicateOccurrenceId { occurrence_id });
        }
        occurrences.push(RunTrajectoryVisitOccurrenceV1 {
            occurrence_id,
            visit_id: visit.visit_id.clone(),
            decision_step: visit.decision_step,
            observation_id: visit.observation.observation_id.clone(),
            legal_candidate_set_id: visit.legal_candidate_set.candidate_set_id.clone(),
            candidate_links: visit.candidate_links.clone(),
            outcome: visit.outcome.clone(),
        });
    }

    validate_progress_pairing(progress_journal, &occurrences)?;
    let depth = parent_head.map_or(0, |head| head.depth.saturating_add(1));
    let mut segment = RunTrajectorySegmentV1 {
        schema_name: RUN_TRAJECTORY_SEGMENT_SCHEMA_NAME.to_string(),
        schema_version: RUN_TRAJECTORY_SEGMENT_SCHEMA_VERSION,
        segment_id: String::new(),
        run_id: run_id.to_string(),
        branch_id,
        policy_lane,
        parent_segment_id: parent_head.map(|head| head.segment_id.clone()),
        generation,
        depth,
        disposition,
        progress_journal: progress_journal.clone(),
        boundary_visit_occurrences: occurrences,
    };
    segment.segment_id = expected_segment_id(&segment)?;
    Ok(Some(RunTrajectorySegmentDraftV1 {
        segment,
        observations: observations.into_values().collect(),
        legal_candidate_sets: candidate_sets.into_values().collect(),
    }))
}

pub fn validate_run_trajectory_segment_id_v1(
    segment: &RunTrajectorySegmentV1,
) -> Result<(), RunTrajectoryIntegrityGapV1> {
    let expected = expected_segment_id(segment)?;
    if expected != segment.segment_id {
        return Err(RunTrajectoryIntegrityGapV1::SegmentIdMismatch {
            expected,
            actual: segment.segment_id.clone(),
        });
    }
    Ok(())
}

fn expected_segment_id(
    segment: &RunTrajectorySegmentV1,
) -> Result<String, RunTrajectoryIntegrityGapV1> {
    let mut hash_input = segment.clone();
    hash_input.segment_id.clear();
    stable_planner_id("trajectory_segment", &hash_input).map_err(|message| {
        RunTrajectoryIntegrityGapV1::IdEncodingFailed {
            subject: "trajectory_segment".to_string(),
            message,
        }
    })
}

fn validate_observation_id(
    visit_id: &str,
    observation: &PlannerObservation,
) -> Result<(), RunTrajectoryIntegrityGapV1> {
    let mut hash_input = observation.clone();
    let actual = hash_input.observation_id.clone();
    hash_input.observation_id.clear();
    let expected = stable_planner_id("observation", &hash_input).map_err(|message| {
        RunTrajectoryIntegrityGapV1::IdEncodingFailed {
            subject: "observation".to_string(),
            message,
        }
    })?;
    if expected != actual {
        return Err(RunTrajectoryIntegrityGapV1::ObservationIdMismatch {
            visit_id: visit_id.to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn validate_candidate_set_id(
    visit_id: &str,
    candidate_set: &LegalCandidateSet,
) -> Result<(), RunTrajectoryIntegrityGapV1> {
    let mut hash_input = candidate_set.clone();
    let actual = hash_input.candidate_set_id.clone();
    hash_input.candidate_set_id.clear();
    let expected = stable_planner_id("candidate_set", &hash_input).map_err(|message| {
        RunTrajectoryIntegrityGapV1::IdEncodingFailed {
            subject: "candidate_set".to_string(),
            message,
        }
    })?;
    if expected != actual {
        return Err(RunTrajectoryIntegrityGapV1::CandidateSetIdMismatch {
            visit_id: visit_id.to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn insert_observation(
    payloads: &mut BTreeMap<String, PlannerObservation>,
    observation: &PlannerObservation,
) -> Result<(), RunTrajectoryIntegrityGapV1> {
    match payloads.get(&observation.observation_id) {
        Some(existing) if existing != observation => {
            Err(RunTrajectoryIntegrityGapV1::ConflictingObservationPayload {
                observation_id: observation.observation_id.clone(),
            })
        }
        Some(_) => Ok(()),
        None => {
            payloads.insert(observation.observation_id.clone(), observation.clone());
            Ok(())
        }
    }
}

fn insert_candidate_set(
    payloads: &mut BTreeMap<String, LegalCandidateSet>,
    candidate_set: &LegalCandidateSet,
) -> Result<(), RunTrajectoryIntegrityGapV1> {
    match payloads.get(&candidate_set.candidate_set_id) {
        Some(existing) if existing != candidate_set => Err(
            RunTrajectoryIntegrityGapV1::ConflictingCandidateSetPayload {
                candidate_set_id: candidate_set.candidate_set_id.clone(),
            },
        ),
        Some(_) => Ok(()),
        None => {
            payloads.insert(
                candidate_set.candidate_set_id.clone(),
                candidate_set.clone(),
            );
            Ok(())
        }
    }
}

fn validate_progress_pairing(
    journal: &RunProgressJournalV1,
    occurrences: &[RunTrajectoryVisitOccurrenceV1],
) -> Result<(), RunTrajectoryIntegrityGapV1> {
    for occurrence in occurrences {
        match &occurrence.outcome {
            PlannerBoundaryVisitOutcomeV1::Selected {
                selection_source,
                run_candidate_id,
                ..
            } => {
                let match_count = journal
                    .entries()
                    .iter()
                    .filter_map(RunProgressStepV1::as_decision)
                    .filter(|transaction| {
                        transaction.before.decision_step == occurrence.decision_step
                            && transaction.selection.source == *selection_source
                            && transaction.selection.candidate_id == *run_candidate_id
                    })
                    .count();
                if match_count != 1 {
                    return Err(
                        RunTrajectoryIntegrityGapV1::SelectedOccurrenceTransactionCount {
                            occurrence_id: occurrence.occurrence_id.clone(),
                            match_count,
                        },
                    );
                }
            }
            PlannerBoundaryVisitOutcomeV1::SelectionNotRepresented {
                selection_source,
                run_candidate_id,
            } => {
                let match_count = journal
                    .entries()
                    .iter()
                    .filter_map(RunProgressStepV1::as_decision)
                    .filter(|transaction| {
                        transaction.before.decision_step == occurrence.decision_step
                            && transaction.selection.source == *selection_source
                            && transaction.selection.candidate_id == *run_candidate_id
                    })
                    .count();
                if match_count != 1 {
                    return Err(
                        RunTrajectoryIntegrityGapV1::UnrepresentedSelectionTransactionCount {
                            occurrence_id: occurrence.occurrence_id.clone(),
                            match_count,
                        },
                    );
                }
            }
            PlannerBoundaryVisitOutcomeV1::MutationWithoutSelection {
                mutation_kind: PlannerBoundaryMutationKindV1::ForcedTransition,
            } => {
                let match_count = journal
                    .entries()
                    .iter()
                    .filter_map(RunProgressStepV1::as_forced_transition)
                    .filter(|transition| {
                        transition.before.decision_step == occurrence.decision_step
                    })
                    .count();
                if match_count != 1 {
                    return Err(RunTrajectoryIntegrityGapV1::ForcedMutationOccurrenceCount {
                        decision_step: occurrence.decision_step,
                        match_count,
                    });
                }
            }
            PlannerBoundaryVisitOutcomeV1::Yielded { .. }
            | PlannerBoundaryVisitOutcomeV1::ExecutionFailed => {}
        }
    }

    for transaction in journal
        .entries()
        .iter()
        .filter_map(RunProgressStepV1::as_decision)
    {
        let match_count = occurrences
            .iter()
            .filter(|occurrence| match &occurrence.outcome {
                PlannerBoundaryVisitOutcomeV1::Selected {
                    selection_source,
                    run_candidate_id,
                    ..
                }
                | PlannerBoundaryVisitOutcomeV1::SelectionNotRepresented {
                    selection_source,
                    run_candidate_id,
                } => {
                    occurrence.decision_step == transaction.before.decision_step
                        && *selection_source == transaction.selection.source
                        && *run_candidate_id == transaction.selection.candidate_id
                }
                _ => false,
            })
            .count();
        if match_count != 1 {
            return Err(
                RunTrajectoryIntegrityGapV1::DecisionTransactionOccurrenceCount {
                    decision_step: transaction.before.decision_step,
                    run_candidate_id: transaction.selection.candidate_id.clone(),
                    match_count,
                },
            );
        }
    }
    for transition in journal
        .entries()
        .iter()
        .filter_map(RunProgressStepV1::as_forced_transition)
    {
        let match_count = occurrences
            .iter()
            .filter(|occurrence| {
                occurrence.decision_step == transition.before.decision_step
                    && matches!(
                        occurrence.outcome,
                        PlannerBoundaryVisitOutcomeV1::MutationWithoutSelection {
                            mutation_kind: PlannerBoundaryMutationKindV1::ForcedTransition
                        }
                    )
            })
            .count();
        if match_count != 1 {
            return Err(
                RunTrajectoryIntegrityGapV1::ForcedTransitionOccurrenceCount {
                    decision_step: transition.before.decision_step,
                    match_count,
                },
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::{
        build_decision_surface, capture_planner_boundary_ticket_v1,
        capture_planner_boundary_yield_v1, PlannerBoundaryYieldKindV1, RunControlSession,
    };

    fn selected_evidence() -> (RunProgressJournalV1, PlannerBoundaryCaptureSegmentV1) {
        let mut session = RunControlSession::new(Default::default());
        let ticket = capture_planner_boundary_ticket_v1(&session)
            .expect("capture boundary")
            .expect("planner-visible boundary");
        let candidate_id = build_decision_surface(&session).view.candidates[0]
            .id
            .clone();
        let outcome = session
            .apply_candidate_id(&candidate_id)
            .expect("apply visible candidate");
        let journal = RunProgressJournalV1::from_committed_steps(outcome.progress_steps.clone())
            .expect("decision journal");
        let capture = ticket.finish_for_progress(&outcome.progress_steps);
        (journal, capture)
    }

    #[test]
    fn selected_visit_and_transaction_build_a_stable_segment() {
        let (journal, capture) = selected_evidence();
        let first = build_run_trajectory_segment_v1(
            "run:test",
            7,
            RunTrajectoryPolicyLaneV1::Baseline,
            3,
            None,
            RunTrajectorySegmentDispositionV1::Resumable,
            &journal,
            &capture,
        )
        .unwrap()
        .unwrap();
        let second = build_run_trajectory_segment_v1(
            "run:test",
            7,
            RunTrajectoryPolicyLaneV1::Baseline,
            3,
            None,
            RunTrajectorySegmentDispositionV1::Resumable,
            &journal,
            &capture,
        )
        .unwrap()
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.segment.depth, 0);
        assert_eq!(first.segment.boundary_visit_occurrences.len(), 1);
        assert_eq!(first.observations.len(), 1);
        assert_eq!(first.legal_candidate_sets.len(), 1);
        validate_run_trajectory_segment_id_v1(&first.segment).unwrap();
    }

    #[test]
    fn child_segment_uses_parent_head_without_copying_parent_payloads() {
        let (journal, capture) = selected_evidence();
        let parent = build_run_trajectory_segment_v1(
            "run:test",
            1,
            RunTrajectoryPolicyLaneV1::Baseline,
            0,
            None,
            RunTrajectorySegmentDispositionV1::Resumable,
            &journal,
            &capture,
        )
        .unwrap()
        .unwrap();
        let child = build_run_trajectory_segment_v1(
            "run:test",
            2,
            RunTrajectoryPolicyLaneV1::Baseline,
            1,
            Some(&parent.head()),
            RunTrajectorySegmentDispositionV1::Resumable,
            &journal,
            &capture,
        )
        .unwrap()
        .unwrap();

        assert_eq!(child.segment.depth, 1);
        assert_eq!(
            child.segment.parent_segment_id.as_deref(),
            Some(parent.segment.segment_id.as_str())
        );
    }

    #[test]
    fn decision_without_selected_or_unrepresented_visit_fails_closed() {
        let (journal, mut capture) = selected_evidence();
        capture.visits[0].outcome = PlannerBoundaryVisitOutcomeV1::Yielded {
            yield_kind: crate::eval::run_control::PlannerBoundaryYieldKindV1::CallbackStop,
        };

        let error = build_run_trajectory_segment_v1(
            "run:test",
            1,
            RunTrajectoryPolicyLaneV1::Baseline,
            0,
            None,
            RunTrajectorySegmentDispositionV1::Resumable,
            &journal,
            &capture,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            RunTrajectoryIntegrityGapV1::DecisionTransactionOccurrenceCount { match_count: 0, .. }
        ));
    }

    #[test]
    fn empty_evidence_does_not_create_a_segment() {
        let segment = build_run_trajectory_segment_v1(
            "run:test",
            1,
            RunTrajectoryPolicyLaneV1::Baseline,
            0,
            None,
            RunTrajectorySegmentDispositionV1::Resumable,
            &RunProgressJournalV1::default(),
            &PlannerBoundaryCaptureSegmentV1::default(),
        )
        .unwrap();

        assert!(segment.is_none());
    }

    #[test]
    fn progress_budget_yield_is_a_durable_occurrence_without_a_fake_mutation() {
        let session = RunControlSession::new(Default::default());
        let capture = capture_planner_boundary_yield_v1(
            &session,
            PlannerBoundaryYieldKindV1::ProgressBudgetExhausted,
        )
        .unwrap();
        let draft = build_run_trajectory_segment_v1(
            "run:test",
            1,
            RunTrajectoryPolicyLaneV1::Baseline,
            0,
            None,
            RunTrajectorySegmentDispositionV1::Stopped,
            &RunProgressJournalV1::default(),
            &capture,
        )
        .unwrap()
        .unwrap();

        assert!(draft.segment.progress_journal.is_empty());
        assert_eq!(draft.segment.boundary_visit_occurrences.len(), 1);
        assert!(matches!(
            draft.segment.boundary_visit_occurrences[0].outcome,
            PlannerBoundaryVisitOutcomeV1::Yielded {
                yield_kind: PlannerBoundaryYieldKindV1::ProgressBudgetExhausted
            }
        ));
    }
}
