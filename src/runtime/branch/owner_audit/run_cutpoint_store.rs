use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::run_cutpoint::{RunCutpointManifestV1, RunCutpointSnapshot};
use super::{frontier_checkpoint, run_capsule_io, Args, Branch};

#[derive(Clone)]
pub(super) struct RunCutpointHandle {
    pub(super) frontier_path: PathBuf,
    pub(super) manifest_path: PathBuf,
    pub(super) snapshot: RunCutpointSnapshot,
    args: Args,
    next_branch_id: usize,
}

pub(super) struct RunCutpointStore {
    root: PathBuf,
}

impl RunCutpointStore {
    pub(super) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(super) fn latest_pre_combat_frontier_path(&self) -> PathBuf {
        self.root.join("latest_pre_combat_search.frontier.json")
    }

    pub(super) fn latest_pre_combat_manifest_path(&self) -> PathBuf {
        self.root.join("latest_pre_combat_search.manifest.json")
    }

    pub(super) fn inflight_pre_combat_frontier_path(&self, branch_id: usize) -> PathBuf {
        self.root
            .join(format!("inflight_pre_combat_b{branch_id:04}.frontier.json"))
    }

    pub(super) fn inflight_pre_combat_manifest_path(&self, branch_id: usize) -> PathBuf {
        self.root
            .join(format!("inflight_pre_combat_b{branch_id:04}.manifest.json"))
    }

    pub(super) fn boss_relic_frontier_path(&self, act: u8, floor: i32) -> PathBuf {
        self.root
            .join(format!("a{act}f{floor}_boss_relic.frontier.json"))
    }

    pub(super) fn boss_relic_manifest_path(&self, act: u8, floor: i32) -> PathBuf {
        self.root
            .join(format!("a{act}f{floor}_boss_relic.manifest.json"))
    }

    pub(super) fn write_pre_combat_inflight(
        &self,
        args: Args,
        next_branch_id: usize,
        snapshot: RunCutpointSnapshot,
    ) -> Result<RunCutpointHandle, String> {
        let handle = RunCutpointHandle {
            frontier_path: self.inflight_pre_combat_frontier_path(snapshot.branch.id),
            manifest_path: self.inflight_pre_combat_manifest_path(snapshot.branch.id),
            snapshot,
            args,
            next_branch_id,
        };
        write_pair(
            &handle.frontier_path,
            &handle.manifest_path,
            handle.args,
            handle.next_branch_id,
            &handle.snapshot,
        )?;
        Ok(handle)
    }

    pub(super) fn retain_pre_combat_gap(&self, handle: RunCutpointHandle) -> Result<(), String> {
        write_pair(
            &self.latest_pre_combat_frontier_path(),
            &self.latest_pre_combat_manifest_path(),
            handle.args,
            handle.next_branch_id,
            &handle.snapshot,
        )?;
        remove_inflight_pair(&handle)
    }

    pub(super) fn discard_pre_combat(&self, handle: RunCutpointHandle) -> Result<(), String> {
        remove_inflight_pair(&handle)?;
        self.remove_latest_if_manifest_matches(&handle.snapshot.manifest)
    }

    pub(super) fn write_boss_relic(
        &self,
        args: Args,
        next_branch_id: usize,
        snapshot: RunCutpointSnapshot,
    ) -> Result<PathBuf, String> {
        let frontier_path =
            self.boss_relic_frontier_path(snapshot.manifest.act, snapshot.manifest.floor);
        let manifest_path =
            self.boss_relic_manifest_path(snapshot.manifest.act, snapshot.manifest.floor);
        write_pair(
            &frontier_path,
            &manifest_path,
            args,
            next_branch_id,
            &snapshot,
        )?;
        Ok(frontier_path)
    }

    pub(super) fn validate_resume_path(
        frontier_path: &Path,
        frontier: &VecDeque<Branch>,
    ) -> Result<(), String> {
        let is_cutpoint = frontier_path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "cutpoints");
        let manifest_path = match manifest_path_for_frontier(frontier_path) {
            Ok(path) => path,
            Err(error) if is_cutpoint => return Err(error),
            Err(_) => return Ok(()),
        };
        if !manifest_path.exists() {
            return if is_cutpoint {
                Err(format!(
                    "cutpoint manifest missing: {}",
                    manifest_path.display()
                ))
            } else {
                Ok(())
            };
        }
        if frontier.len() != 1 {
            return Err(format!(
                "manifested cutpoint requires one frontier branch, got {}",
                frontier.len()
            ));
        }
        let payload = std::fs::read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
        let manifest: RunCutpointManifestV1 = serde_json::from_str(&payload)
            .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
        let branch = frontier.front().expect("frontier length was checked");
        RunCutpointSnapshot {
            manifest,
            branch: branch.clone(),
        }
        .validate_branch(branch)
    }

    fn remove_latest_if_manifest_matches(
        &self,
        expected: &RunCutpointManifestV1,
    ) -> Result<(), String> {
        let path = self.latest_pre_combat_manifest_path();
        let Ok(payload) = std::fs::read_to_string(&path) else {
            return Ok(());
        };
        let actual: RunCutpointManifestV1 = serde_json::from_str(&payload)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        if actual.branch_id != expected.branch_id
            || actual.session_checkpoint_hash != expected.session_checkpoint_hash
        {
            return Ok(());
        }
        run_capsule_io::remove_if_exists(&self.latest_pre_combat_frontier_path())?;
        run_capsule_io::remove_if_exists(&path)
    }
}

fn remove_inflight_pair(handle: &RunCutpointHandle) -> Result<(), String> {
    run_capsule_io::remove_if_exists(&handle.frontier_path)?;
    run_capsule_io::remove_if_exists(&handle.manifest_path)
}

fn manifest_path_for_frontier(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid cutpoint frontier path: {}", path.display()))?;
    let stem = file_name.strip_suffix(".frontier.json").ok_or_else(|| {
        format!(
            "cutpoint frontier must end in .frontier.json: {}",
            path.display()
        )
    })?;
    Ok(path.with_file_name(format!("{stem}.manifest.json")))
}

fn write_pair(
    frontier_path: &Path,
    manifest_path: &Path,
    args: Args,
    next_branch_id: usize,
    snapshot: &RunCutpointSnapshot,
) -> Result<(), String> {
    let frontier = VecDeque::from([snapshot.branch.clone()]);
    frontier_checkpoint::save(
        frontier_path,
        args,
        snapshot.manifest.generation,
        next_branch_id,
        &frontier,
    )?;
    let value = serde_json::to_value(&snapshot.manifest).map_err(|error| error.to_string())?;
    run_capsule_io::write_json(manifest_path, value)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::run_cutpoint::{RunCutpointKind, RunCutpointSnapshot};
    use super::super::{Args, BranchStatus};
    use super::*;

    #[test]
    fn successful_search_discards_only_its_inflight_pair() {
        let (store, args, next_id, first, second) = fixture_two_precombat_snapshots();
        let first_handle = store
            .write_pre_combat_inflight(args, next_id, first)
            .unwrap();
        let second_handle = store
            .write_pre_combat_inflight(args, next_id, second)
            .unwrap();

        store.discard_pre_combat(second_handle.clone()).unwrap();

        assert!(first_handle.frontier_path.exists());
        assert!(first_handle.manifest_path.exists());
        assert!(!second_handle.frontier_path.exists());
        assert!(!second_handle.manifest_path.exists());
    }

    #[test]
    fn gap_promotes_one_exact_pair_and_removes_inflight_files() {
        let (store, args, next_id, snapshot) = fixture_precombat_snapshot();
        let handle = store
            .write_pre_combat_inflight(args, next_id, snapshot)
            .unwrap();

        store.retain_pre_combat_gap(handle.clone()).unwrap();

        assert!(store.latest_pre_combat_frontier_path().exists());
        assert!(store.latest_pre_combat_manifest_path().exists());
        assert!(!handle.frontier_path.exists());
        assert!(!handle.manifest_path.exists());
    }

    #[test]
    fn manifested_resume_rejects_payload_tampering() {
        let (store, args, next_id, snapshot) = fixture_precombat_snapshot();
        let handle = store
            .write_pre_combat_inflight(args, next_id, snapshot)
            .unwrap();
        store.retain_pre_combat_gap(handle).unwrap();
        let path = store.latest_pre_combat_frontier_path();
        let checkpoint = super::super::frontier_checkpoint::load(&path).unwrap();
        let (mut frontier, _) = checkpoint.into_frontier().unwrap();
        frontier.front_mut().unwrap().session.run_state.gold += 1;

        let error = RunCutpointStore::validate_resume_path(&path, &frontier).unwrap_err();
        assert!(error.contains("session checkpoint fingerprint mismatch"));
    }

    #[test]
    fn cutpoint_frontier_without_manifest_fails_closed() {
        let (store, args, next_id, snapshot) = fixture_precombat_snapshot();
        let handle = store
            .write_pre_combat_inflight(args, next_id, snapshot)
            .unwrap();
        store.retain_pre_combat_gap(handle).unwrap();
        let path = store.latest_pre_combat_frontier_path();
        std::fs::remove_file(store.latest_pre_combat_manifest_path()).unwrap();
        let checkpoint = super::super::frontier_checkpoint::load(&path).unwrap();
        let (frontier, _) = checkpoint.into_frontier().unwrap();

        let error = RunCutpointStore::validate_resume_path(&path, &frontier).unwrap_err();
        assert!(error.contains("cutpoint manifest missing"));
    }

    fn unique_root(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "sts_run_cutpoint_{label}_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ))
            .join("cutpoints")
    }

    fn precombat_snapshot(branch_id: usize, gold: i32) -> (Args, usize, RunCutpointSnapshot) {
        let args = crate::runtime::branch::default_branch_args(20260713006);
        let (mut frontier, next_id) = super::super::branch_runtime::BranchRuntime::initial_frontier(
            args,
            std::time::Instant::now(),
        );
        let mut branch = frontier.pop_front().unwrap();
        branch.id = branch_id;
        branch.session.run_state.gold = gold;
        branch.status = BranchStatus::AwaitingAuto {
            boundary: "Combat".to_string(),
            reason: "resume pre-combat search cutpoint".to_string(),
        };
        let snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::PreCombatSearch, 17, &branch).unwrap();
        (args, next_id.max(branch_id + 1), snapshot)
    }

    fn fixture_precombat_snapshot() -> (RunCutpointStore, Args, usize, RunCutpointSnapshot) {
        let (args, next_id, snapshot) = precombat_snapshot(17, 61);
        (
            RunCutpointStore::new(unique_root("single")),
            args,
            next_id,
            snapshot,
        )
    }

    fn fixture_two_precombat_snapshots() -> (
        RunCutpointStore,
        Args,
        usize,
        RunCutpointSnapshot,
        RunCutpointSnapshot,
    ) {
        let (args, next_id, first) = precombat_snapshot(17, 61);
        let (_, second_next_id, second) = precombat_snapshot(18, 62);
        (
            RunCutpointStore::new(unique_root("two")),
            args,
            next_id.max(second_next_id),
            first,
            second,
        )
    }
}
