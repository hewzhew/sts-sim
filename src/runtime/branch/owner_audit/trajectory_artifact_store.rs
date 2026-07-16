use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sts_simulator::ai::planner_core::{LegalCandidateSet, PlannerObservation};
use sts_simulator::runtime::branch::{
    validate_run_trajectory_segment_id_v1, ArtifactKind, ArtifactRef, ArtifactWriteSummary,
    RunTrajectoryHeadV1, RunTrajectoryReconstructionV1, RunTrajectorySegmentV1,
    RUN_TRAJECTORY_RECONSTRUCTION_SCHEMA_NAME, RUN_TRAJECTORY_RECONSTRUCTION_SCHEMA_VERSION,
    RUN_TRAJECTORY_SEGMENT_SCHEMA_NAME,
};

use super::branch_model::Branch;
use super::run_capsule_io::{ensure_dir, write_json};

pub(super) struct TrajectoryArtifactStore {
    capsule_root: PathBuf,
}

impl TrajectoryArtifactStore {
    pub(super) fn new(capsule_root: PathBuf) -> Self {
        Self { capsule_root }
    }

    pub(super) fn commit_branch(
        &self,
        branch: &mut Branch,
    ) -> Result<ArtifactWriteSummary, String> {
        let mut summary = ArtifactWriteSummary::default();
        while let Some(draft) = branch.trajectory.pending_front().cloned() {
            let run_id = branch
                .trajectory
                .run_id()
                .ok_or_else(|| "pending trajectory segment has no bound run id".to_string())?;
            if draft.segment.run_id != run_id {
                return Err(format!(
                    "pending trajectory segment belongs to {}, expected {run_id}",
                    draft.segment.run_id
                ));
            }
            let expected_parent = branch
                .trajectory
                .committed_head()
                .map(|head| head.segment_id.as_str());
            if draft.segment.parent_segment_id.as_deref() != expected_parent {
                return Err(format!(
                    "trajectory parent mismatch for {}: expected {:?}, got {:?}",
                    draft.segment.segment_id, expected_parent, draft.segment.parent_segment_id
                ));
            }
            validate_run_trajectory_segment_id_v1(&draft.segment)
                .map_err(|gap| format!("trajectory segment integrity gap: {gap}"))?;

            let mut refs = Vec::new();
            for observation in &draft.observations {
                let path = self.observation_path(&observation.observation_id)?;
                if write_immutable(&path, observation)? {
                    let artifact = trajectory_ref(
                        ArtifactKind::TrajectoryObservation,
                        path,
                        "PlannerObservation",
                    );
                    summary.record_ref(artifact.clone());
                    refs.push(artifact);
                }
            }
            for candidate_set in &draft.legal_candidate_sets {
                let path = self.candidate_set_path(&candidate_set.candidate_set_id)?;
                if write_immutable(&path, candidate_set)? {
                    let artifact = trajectory_ref(
                        ArtifactKind::TrajectoryCandidateSet,
                        path,
                        "LegalCandidateSet",
                    );
                    summary.record_ref(artifact.clone());
                    refs.push(artifact);
                }
            }
            let segment_path = self.segment_path(&draft.segment.segment_id)?;
            write_immutable(&segment_path, &draft.segment)?;
            let segment_ref = trajectory_ref(
                ArtifactKind::TrajectorySegment,
                segment_path,
                RUN_TRAJECTORY_SEGMENT_SCHEMA_NAME,
            );
            refs.push(segment_ref.clone());
            self.append_segment_ledger_once(&draft.segment, &refs)?;
            summary.record_ref(segment_ref);
            branch
                .trajectory
                .mark_front_committed(&draft.segment.segment_id)?;
        }
        Ok(summary)
    }

    pub(super) fn verify_head(
        &self,
        run_id: &str,
        head: Option<&RunTrajectoryHeadV1>,
    ) -> Result<(), String> {
        let Some(head) = head else {
            return Ok(());
        };
        let mut next_id = Some(head.segment_id.clone());
        let mut expected_depth = head.depth;
        let mut seen = BTreeSet::new();
        while let Some(segment_id) = next_id {
            if !seen.insert(segment_id.clone()) {
                return Err(format!("trajectory segment cycle at {segment_id}"));
            }
            let segment = self.read_segment(&segment_id)?;
            validate_run_trajectory_segment_id_v1(&segment)
                .map_err(|gap| format!("trajectory segment integrity gap: {gap}"))?;
            if segment.run_id != run_id {
                return Err(format!(
                    "trajectory segment {} belongs to {}, expected {run_id}",
                    segment.segment_id, segment.run_id
                ));
            }
            if segment.depth != expected_depth {
                return Err(format!(
                    "trajectory depth mismatch for {}: expected {expected_depth}, got {}",
                    segment.segment_id, segment.depth
                ));
            }
            next_id = segment.parent_segment_id;
            if next_id.is_some() {
                expected_depth = expected_depth.checked_sub(1).ok_or_else(|| {
                    format!(
                        "root trajectory segment {} has a parent",
                        segment.segment_id
                    )
                })?;
            } else if expected_depth != 0 {
                return Err(format!(
                    "trajectory chain for {} ended at nonzero depth {expected_depth}",
                    head.segment_id
                ));
            }
        }
        Ok(())
    }

    pub(super) fn read_segment(&self, segment_id: &str) -> Result<RunTrajectorySegmentV1, String> {
        read_json(&self.segment_path(segment_id)?)
    }

    pub(super) fn read_observation(&self, id: &str) -> Result<PlannerObservation, String> {
        read_json(&self.observation_path(id)?)
    }

    pub(super) fn read_candidate_set(&self, id: &str) -> Result<LegalCandidateSet, String> {
        read_json(&self.candidate_set_path(id)?)
    }

    pub(super) fn reconstruct(
        &self,
        run_id: &str,
        head: &RunTrajectoryHeadV1,
    ) -> Result<RunTrajectoryReconstructionV1, String> {
        self.verify_head(run_id, Some(head))?;
        let mut reversed = Vec::with_capacity(head.depth.saturating_add(1) as usize);
        let mut next_id = Some(head.segment_id.clone());
        while let Some(segment_id) = next_id {
            let segment = self.read_segment(&segment_id)?;
            next_id = segment.parent_segment_id.clone();
            reversed.push(segment);
        }
        reversed.reverse();
        Ok(RunTrajectoryReconstructionV1 {
            schema_name: RUN_TRAJECTORY_RECONSTRUCTION_SCHEMA_NAME.to_string(),
            schema_version: RUN_TRAJECTORY_RECONSTRUCTION_SCHEMA_VERSION,
            run_id: run_id.to_string(),
            head: head.clone(),
            segments: reversed,
        })
    }

    fn observation_path(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self
            .capsule_root
            .join("trajectory")
            .join("observations")
            .join(content_file_name(id, "observation")?))
    }

    fn candidate_set_path(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self
            .capsule_root
            .join("trajectory")
            .join("candidate_sets")
            .join(content_file_name(id, "candidate_set")?))
    }

    fn segment_path(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self
            .capsule_root
            .join("trajectory")
            .join("segments")
            .join(content_file_name(id, "trajectory_segment")?))
    }

    fn append_segment_ledger_once(
        &self,
        segment: &RunTrajectorySegmentV1,
        artifact_refs: &[ArtifactRef],
    ) -> Result<(), String> {
        ensure_dir(&self.capsule_root)?;
        let path = self.capsule_root.join("capsule_ledger.jsonl");
        if ledger_contains_segment(&path, &segment.segment_id)? {
            return Ok(());
        }
        let event = TrajectorySegmentCommittedLedgerEventV1 {
            schema: "trajectory_segment_committed_ledger_event_v1",
            event: "trajectory_segment_committed",
            run_id: &segment.run_id,
            branch_id: segment.branch_id,
            segment_id: &segment.segment_id,
            parent_segment_id: segment.parent_segment_id.as_deref(),
            depth: segment.depth,
            artifact_refs,
        };
        let encoded = serde_json::to_string(&event)
            .map_err(|error| format!("serialize trajectory ledger event: {error}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("open {}: {error}", path.display()))?;
        writeln!(file, "{encoded}").map_err(|error| format!("write {}: {error}", path.display()))
    }
}

#[derive(Serialize)]
struct TrajectorySegmentCommittedLedgerEventV1<'a> {
    schema: &'static str,
    event: &'static str,
    run_id: &'a str,
    branch_id: u64,
    segment_id: &'a str,
    parent_segment_id: Option<&'a str>,
    depth: u64,
    artifact_refs: &'a [ArtifactRef],
}

#[derive(Deserialize)]
struct LedgerSegmentIdentity {
    event: Option<String>,
    segment_id: Option<String>,
}

fn ledger_contains_segment(path: &Path, segment_id: &str) -> Result<bool, String> {
    let payload = match fs::read_to_string(path) {
        Ok(payload) => payload,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    for line in payload.lines().filter(|line| !line.trim().is_empty()) {
        let identity: LedgerSegmentIdentity = serde_json::from_str(line)
            .map_err(|error| format!("parse {} ledger line: {error}", path.display()))?;
        if identity.event.as_deref() == Some("trajectory_segment_committed")
            && identity.segment_id.as_deref() == Some(segment_id)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn content_file_name(id: &str, namespace: &str) -> Result<String, String> {
    let suffix = id
        .strip_prefix(namespace)
        .and_then(|value| value.strip_prefix(':'))
        .ok_or_else(|| format!("expected {namespace} content id, got {id}"))?;
    if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("invalid {namespace} content id: {id}"));
    }
    Ok(format!("{suffix}.json"))
}

fn trajectory_ref(kind: ArtifactKind, path: PathBuf, schema: &'static str) -> ArtifactRef {
    ArtifactRef::new(kind, path, schema, "trajectory_artifact_store_v1")
}

fn write_immutable<T>(path: &Path, value: &T) -> Result<bool, String>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    match fs::read_to_string(path) {
        Ok(payload) => {
            let existing: T = serde_json::from_str(&payload)
                .map_err(|error| format!("parse immutable {}: {error}", path.display()))?;
            if existing != *value {
                return Err(format!(
                    "immutable trajectory payload collision at {}",
                    path.display()
                ));
            }
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let encoded = serde_json::to_value(value)
                .map_err(|error| format!("serialize immutable {}: {error}", path.display()))?;
            write_json(path, encoded)?;
            Ok(true)
        }
        Err(error) => Err(format!("read immutable {}: {error}", path.display())),
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let payload = fs::read_to_string(path)
        .map_err(|error| format!("read trajectory artifact {}: {error}", path.display()))?;
    serde_json::from_str(&payload)
        .map_err(|error| format!("parse trajectory artifact {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::{
        build_decision_surface, capture_planner_boundary_ticket_v1, RunControlSession,
        RunProgressJournalV1,
    };

    use crate::runtime::branch::owner_audit::branch_model::{BranchStatus, Owner};
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;

    fn evidence_branch() -> Branch {
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
        let mut branch = Branch {
            id: 4,
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
        };
        branch
            .bind_trajectory_run("trajectory_run:test", 0)
            .unwrap();
        branch
    }

    #[test]
    fn commit_is_immutable_idempotent_and_head_is_verifiable() {
        let root = std::env::temp_dir().join(format!(
            "trajectory_artifact_store_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = TrajectoryArtifactStore::new(root.clone());
        let mut branch = evidence_branch();

        let first = store.commit_branch(&mut branch).unwrap();
        assert_eq!(first.trajectory_segment_refs.len(), 1);
        assert_eq!(first.trajectory_observation_refs.len(), 1);
        assert_eq!(first.trajectory_candidate_set_refs.len(), 1);
        assert_eq!(branch.trajectory.pending_len(), 0);
        store
            .verify_head("trajectory_run:test", branch.trajectory.committed_head())
            .unwrap();

        let second = store.commit_branch(&mut branch).unwrap();
        assert!(second.refs().is_empty());
        let ledger = fs::read_to_string(root.join("capsule_ledger.jsonl")).unwrap();
        assert_eq!(
            ledger
                .lines()
                .filter(|line| line.contains("trajectory_segment_committed"))
                .count(),
            1
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn forked_branches_share_the_committed_parent_without_copying_it() {
        let root = std::env::temp_dir().join(format!(
            "trajectory_artifact_fork_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = TrajectoryArtifactStore::new(root.clone());
        let mut parent = evidence_branch();
        store.commit_branch(&mut parent).unwrap();
        let parent_id = parent
            .trajectory
            .committed_head()
            .unwrap()
            .segment_id
            .clone();

        let mut left = parent.clone();
        left.id = 5;
        left.capture_recent_trajectory(1).unwrap();
        store.commit_branch(&mut left).unwrap();
        let mut right = parent.clone();
        right.id = 6;
        right.capture_recent_trajectory(1).unwrap();
        store.commit_branch(&mut right).unwrap();

        let left_segment = store
            .read_segment(&left.trajectory.committed_head().unwrap().segment_id)
            .unwrap();
        let right_segment = store
            .read_segment(&right.trajectory.committed_head().unwrap().segment_id)
            .unwrap();
        assert_eq!(left_segment.parent_segment_id.as_deref(), Some(&*parent_id));
        assert_eq!(
            right_segment.parent_segment_id.as_deref(),
            Some(&*parent_id)
        );
        assert_ne!(left_segment.segment_id, right_segment.segment_id);
        assert_eq!(
            fs::read_dir(root.join("trajectory").join("observations"))
                .unwrap()
                .count(),
            1,
            "content-addressed observations must be shared across the fork"
        );
        let _ = fs::remove_dir_all(root);
    }
}
