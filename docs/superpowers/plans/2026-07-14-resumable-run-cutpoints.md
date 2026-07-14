# Resumable Run Cutpoints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve an exact, resumable pre-search branch when combat cannot advance and an exact Boss Relic branch before owner expansion, without replaying the seed prefix.

**Architecture:** Add one internal cutpoint snapshot format around the existing `RunControlSessionCheckpointV1` and frontier serializer. A bounded artifact store writes per-branch inflight pre-combat pairs, promotes only a failed search to the single `latest_pre_combat_search` pair, and retains one durable Boss Relic pair per act/floor. The owner-audit runner supplies lifecycle hooks; ordinary frontier resume validates a sibling cutpoint manifest when present and remains backward-compatible when it is absent.

**Tech Stack:** Rust 2021, serde/serde_json, Blake2b-256, existing owner-audit runtime, existing `RunControlSessionCheckpointV1`, existing `branch_tiny_frontier_checkpoint` schema.

## Global Constraints

- Work only in `D:\rust\sts_simulator`; do not create a worktree and do not run `cargo clean`.
- Start each implementation task from a clean worktree and preserve unrelated user changes.
- Store cutpoints with the existing full `RunControlSessionCheckpointV1` and frontier schema; do not create a partial run-state checkpoint schema.
- Do not change normal run policy, combat action ordering, owner scores, or game mechanics.
- Keep only per-branch inflight pre-combat artifacts during search, one promoted `latest_pre_combat_search` pair after a gap, and the few Act Boss Relic pairs.
- When none of a run capsule, `--frontier-checkpoint`, or `--resume-frontier` supplies an artifact root, perform no implicit filesystem writes.
- Existing frontier files without cutpoint manifests must remain loadable.
- Artifact fingerprint or boundary mismatches fail closed.
- This plan does not implement the separate reproducible-search/wall-safety classification subsystem.

---

## File Map

- Create `src/runtime/branch/owner_audit/run_cutpoint.rs`: cutpoint kinds, manifest, full-session and candidate fingerprints, snapshot construction, validation.
- Create `src/runtime/branch/owner_audit/run_cutpoint_store.rs`: bounded paths, atomic pair writes, inflight promotion/removal, optional-manifest resume validation.
- Create `src/runtime/branch/owner_audit/run_cutpoint_recorder.rs`: one branch's pre-combat lifecycle and Boss Relic capture hook.
- Modify `src/runtime/branch/owner_audit.rs`: register the three focused modules.
- Modify `src/runtime/branch/owner_audit/frontier_checkpoint.rs`: make typed frontier writes atomic and expose only the data needed for strict validation.
- Modify `src/runtime/branch/owner_audit/runner.rs`: invoke pre-combat begin/finish hooks around each portfolio call.
- Modify `src/runtime/branch/owner_audit/branch_scheduler.rs`: construct a recorder per branch and persist Boss Relic before expansion.
- Modify `src/runtime/branch/owner_audit/branch_generation.rs`: pass the optional store and current next-id into scheduling.
- Modify `src/runtime/branch/owner_audit/run_loop.rs`: derive the bounded artifact root and wire the store into generation preparation.
- Modify `src/runtime/branch/owner_audit/run_capsule.rs`: expose the capsule-local `cutpoints/` directory without exposing the store internals.
- Modify `src/runtime/branch/owner_audit/run_startup.rs`: validate manifested cutpoints before returning a resumed frontier.

---

### Task 1: Exact Cutpoint Snapshot and Validation

**Files:**
- Create: `src/runtime/branch/owner_audit/run_cutpoint.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Test: inline `run_cutpoint::tests`

**Interfaces:**
- Consumes: `Branch`, `BranchStatus`, `RunControlSessionCheckpointV1`, `build_decision_surface`.
- Produces:
  - `RunCutpointKind::{PreCombatSearch, OwnerDecision}`
  - `RunCutpointManifestV1`
  - `RunCutpointSnapshot::capture(kind, generation, &Branch) -> Result<Self, String>`
  - `RunCutpointSnapshot::validate_branch(&self, &Branch) -> Result<(), String>`
  - `RunCutpointSnapshot::into_resumable_frontier(self) -> VecDeque<Branch>`

- [ ] **Step 1: Register the empty module and write failing manifest tests**

Add this module declaration in `src/runtime/branch/owner_audit.rs` beside the other run persistence modules:

```rust
#[path = "owner_audit/run_cutpoint.rs"]
mod run_cutpoint;
```

Create `run_cutpoint.rs` with tests that describe the public internal contract before the types exist:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicId;
    use crate::state::core::EngineState;
    use crate::state::rewards::BossRelicChoiceState;

    fn boss_relic_branch() -> Branch {
        let args = crate::runtime::branch::default_branch_args(20260713006);
        let (mut frontier, _) = super::super::branch_runtime::BranchRuntime::initial_frontier(
            args,
            std::time::Instant::now(),
        );
        let mut branch = frontier.pop_front().unwrap();
        branch.session.run_state.act_num = 2;
        branch.session.run_state.floor_num = 32;
        branch.session.run_state.current_hp = 13;
        branch.session.run_state.max_hp = 101;
        branch.session.run_state.gold = 167;
        branch.session.engine_state = EngineState::BossRelicSelect(
            BossRelicChoiceState::new(vec![
                RelicId::BlackBlood,
                RelicId::CoffeeDripper,
                RelicId::PhilosopherStone,
            ]),
        );
        branch.status = BranchStatus::Running {
            boundary: "Boss Relic".to_string(),
            owner: Owner::BossRelic,
        };
        branch
    }

    #[test]
    fn snapshot_fingerprints_full_session_and_candidate_order() {
        let branch = boss_relic_branch();
        let snapshot = RunCutpointSnapshot::capture(
            RunCutpointKind::OwnerDecision,
            29,
            &branch,
        )
        .unwrap();

        assert_eq!(snapshot.manifest.act, 2);
        assert_eq!(snapshot.manifest.floor, 32);
        assert_eq!(snapshot.manifest.boundary, "Boss Relic");
        assert_eq!(snapshot.manifest.candidate_count, 4);
        assert!(!snapshot.manifest.session_checkpoint_hash.is_empty());
        assert!(!snapshot.manifest.branch_control_hash.is_empty());
        assert!(!snapshot.manifest.candidate_order_hash.is_empty());
        snapshot.validate_branch(&branch).unwrap();
    }

    #[test]
    fn validation_rejects_persistent_state_drift() {
        let branch = boss_relic_branch();
        let snapshot = RunCutpointSnapshot::capture(
            RunCutpointKind::OwnerDecision,
            29,
            &branch,
        )
        .unwrap();
        let mut changed = branch.clone();
        changed.session.run_state.gold += 1;

        let error = snapshot.validate_branch(&changed).unwrap_err();
        assert!(error.contains("session checkpoint fingerprint mismatch"));
    }

    #[test]
    fn validation_rejects_candidate_order_drift() {
        let branch = boss_relic_branch();
        let mut snapshot = RunCutpointSnapshot::capture(
            RunCutpointKind::OwnerDecision,
            29,
            &branch,
        )
        .unwrap();
        snapshot.manifest.candidate_order_hash = "tampered".to_string();

        let error = snapshot.validate_branch(&branch).unwrap_err();
        assert!(error.contains("candidate order fingerprint mismatch"));
    }

    #[test]
    fn validation_rejects_branch_history_drift() {
        let branch = boss_relic_branch();
        let snapshot = RunCutpointSnapshot::capture(
            RunCutpointKind::OwnerDecision,
            29,
            &branch,
        )
        .unwrap();
        let mut changed = branch.clone();
        changed.parent_id = Some(999);

        let error = snapshot.validate_branch(&changed).unwrap_err();
        assert!(error.contains("branch control fingerprint mismatch"));
    }
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::run_cutpoint::tests -- --nocapture
```

Expected: compilation fails because `RunCutpointSnapshot` and `RunCutpointKind` do not exist.

- [ ] **Step 3: Implement the minimal typed snapshot**

Implement these types and helpers in `run_cutpoint.rs`:

```rust
use std::collections::VecDeque;

use blake2::{Blake2b512, Digest};
use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    build_decision_surface, DecisionCandidateKey, RunControlSessionCheckpointV1,
};

use super::{Branch, BranchStatus};

pub(super) const RUN_CUTPOINT_SCHEMA: &str = "branch_tiny_run_cutpoint_v1";
pub(super) const RUN_CUTPOINT_TRUST: &str = "exact_run_control_checkpoint_v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RunCutpointKind {
    PreCombatSearch,
    OwnerDecision,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunCutpointManifestV1 {
    pub(super) schema: String,
    pub(super) kind: RunCutpointKind,
    pub(super) artifact_trust: String,
    pub(super) generation: usize,
    pub(super) branch_id: usize,
    pub(super) act: u8,
    pub(super) floor: i32,
    pub(super) boundary: String,
    pub(super) session_checkpoint_hash: String,
    pub(super) branch_control_hash: String,
    pub(super) candidate_count: usize,
    pub(super) candidate_set_hash: String,
    pub(super) candidate_order_hash: String,
}

#[derive(Clone)]
pub(super) struct RunCutpointSnapshot {
    pub(super) manifest: RunCutpointManifestV1,
    pub(super) branch: Branch,
}

impl RunCutpointSnapshot {
    pub(super) fn capture(
        kind: RunCutpointKind,
        generation: usize,
        branch: &Branch,
    ) -> Result<Self, String> {
        let candidate_keys = candidate_keys(branch);
        let mut sorted_keys = candidate_keys.clone();
        sorted_keys.sort_by_key(|key| serde_json::to_string(key).unwrap_or_default());
        let session_checkpoint_hash = checkpoint_hash(branch)?;
        let branch_control_hash = hash_serializable(&(
            branch.id,
            branch.parent_id,
            &branch.path,
            &branch.status,
            &branch.policy_lane,
            &session_checkpoint_hash,
        ))?;
        Ok(Self {
            manifest: RunCutpointManifestV1 {
                schema: RUN_CUTPOINT_SCHEMA.to_string(),
                kind,
                artifact_trust: RUN_CUTPOINT_TRUST.to_string(),
                generation,
                branch_id: branch.id,
                act: branch.session.run_state.act_num,
                floor: branch.session.run_state.floor_num,
                boundary: branch_boundary(&branch.status),
                session_checkpoint_hash,
                branch_control_hash,
                candidate_count: candidate_keys.len(),
                candidate_set_hash: hash_serializable(&sorted_keys)?,
                candidate_order_hash: hash_serializable(&candidate_keys)?,
            },
            branch: branch.clone(),
        })
    }

    pub(super) fn validate_branch(&self, branch: &Branch) -> Result<(), String> {
        if self.manifest.schema != RUN_CUTPOINT_SCHEMA
            || self.manifest.artifact_trust != RUN_CUTPOINT_TRUST
        {
            return Err("unsupported run cutpoint manifest".to_string());
        }
        let actual = Self::capture(self.manifest.kind, self.manifest.generation, branch)?;
        if actual.manifest.session_checkpoint_hash != self.manifest.session_checkpoint_hash {
            return Err("session checkpoint fingerprint mismatch".to_string());
        }
        if actual.manifest.branch_control_hash != self.manifest.branch_control_hash {
            return Err("branch control fingerprint mismatch".to_string());
        }
        if actual.manifest.act != self.manifest.act
            || actual.manifest.floor != self.manifest.floor
            || actual.manifest.boundary != self.manifest.boundary
        {
            return Err("cutpoint boundary mismatch".to_string());
        }
        if actual.manifest.candidate_count != self.manifest.candidate_count
            || actual.manifest.candidate_set_hash != self.manifest.candidate_set_hash
        {
            return Err("candidate set fingerprint mismatch".to_string());
        }
        if actual.manifest.candidate_order_hash != self.manifest.candidate_order_hash {
            return Err("candidate order fingerprint mismatch".to_string());
        }
        Ok(())
    }

    pub(super) fn into_resumable_frontier(self) -> VecDeque<Branch> {
        VecDeque::from([self.branch])
    }
}

fn checkpoint_hash(branch: &Branch) -> Result<String, String> {
    let mut checkpoint = RunControlSessionCheckpointV1::from_session(&branch.session);
    checkpoint.clear_combat_diagnostics_for_external_checkpoint();
    hash_serializable(&checkpoint)
}

fn candidate_keys(branch: &Branch) -> Vec<Option<DecisionCandidateKey>> {
    build_decision_surface(&branch.session)
        .view
        .candidates
        .into_iter()
        .map(|candidate| candidate.key)
        .collect()
}

fn branch_boundary(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::OperationBudgetExhausted { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary.clone(),
        BranchStatus::Terminal(_) => "Terminal".to_string(),
        BranchStatus::ApplyFailed(_) | BranchStatus::AdvanceFailed(_) => "Failure".to_string(),
    }
}

fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    Ok(hasher.finalize()[..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run the Step 2 command again.

Expected: all three `run_cutpoint` tests pass.

- [ ] **Step 5: Commit the snapshot boundary**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/run_cutpoint.rs
git commit -m "feat: define exact run cutpoint snapshots"
```

---

### Task 2: Atomic Bounded Cutpoint Store

**Files:**
- Create: `src/runtime/branch/owner_audit/run_cutpoint_store.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/frontier_checkpoint.rs`
- Test: inline `run_cutpoint_store::tests`

**Interfaces:**
- Consumes: `RunCutpointSnapshot`, `frontier_checkpoint::save/load`, `run_capsule_io::{write_json, remove_if_exists}`.
- Produces:
  - `RunCutpointStore::new(root: PathBuf) -> Self`
  - `write_pre_combat_inflight(args, next_branch_id, snapshot) -> Result<RunCutpointHandle, String>`
  - `retain_pre_combat_gap(handle) -> Result<(), String>`
  - `discard_pre_combat(handle) -> Result<(), String>`
  - `inflight_pre_combat_frontier_path(branch_id) -> PathBuf`
  - `write_boss_relic(args, next_branch_id, snapshot) -> Result<PathBuf, String>`
  - `boss_relic_frontier_path(act, floor) -> PathBuf`
  - `validate_resume_path(path, frontier) -> Result<(), String>`

- [ ] **Step 1: Write failing retention and tamper tests**

Register `run_cutpoint_store` in `owner_audit.rs`, then add tests using a unique temp root. The tests must assert these exact behaviors:

```rust
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
    let checkpoint = frontier_checkpoint::load(&path).unwrap();
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
    let checkpoint = frontier_checkpoint::load(&path).unwrap();
    let (frontier, _) = checkpoint.into_frontier().unwrap();

    let error = RunCutpointStore::validate_resume_path(&path, &frontier).unwrap_err();
    assert!(error.contains("cutpoint manifest missing"));
}
```

Use these concrete fixture helpers above the tests:

```rust
fn unique_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sts_run_cutpoint_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ))
}

fn precombat_snapshot(
    branch_id: usize,
    gold: i32,
) -> (Args, usize, RunCutpointSnapshot) {
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
    let snapshot = RunCutpointSnapshot::capture(
        RunCutpointKind::PreCombatSearch,
        17,
        &branch,
    )
    .unwrap();
    (args, next_id.max(branch_id + 1), snapshot)
}

fn fixture_precombat_snapshot(
) -> (RunCutpointStore, Args, usize, RunCutpointSnapshot) {
    let (args, next_id, snapshot) = precombat_snapshot(17, 61);
    (
        RunCutpointStore::new(unique_root("single").join("cutpoints")),
        args,
        next_id,
        snapshot,
    )
}

fn fixture_two_precombat_snapshots(
) -> (RunCutpointStore, Args, usize, RunCutpointSnapshot, RunCutpointSnapshot) {
    let (args, next_id, first) = precombat_snapshot(17, 61);
    let (_, second_next_id, second) = precombat_snapshot(18, 62);
    (
        RunCutpointStore::new(unique_root("two").join("cutpoints")),
        args,
        next_id.max(second_next_id),
        first,
        second,
    )
}
```

- [ ] **Step 2: Run the store tests and verify RED**

```powershell
cargo test --lib runtime::branch::owner_audit::run_cutpoint_store::tests -- --nocapture
```

Expected: compilation fails because `RunCutpointStore` and `RunCutpointHandle` do not exist.

- [ ] **Step 3: Make frontier writes atomic**

Replace `fs::write(path, payload)` in `frontier_checkpoint::save` with the existing atomic JSON helper:

```rust
let value = serde_json::to_value(&checkpoint).map_err(|error| error.to_string())?;
super::run_capsule_io::write_json(path, value)
```

Keep directory creation inside `write_json`; remove the now-unused direct write setup from `frontier_checkpoint.rs`.

- [ ] **Step 4: Implement the bounded store**

Use per-branch inflight names so one branch's successful search cannot erase another branch's retained candidate:

```rust
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
        self.root.join(format!("a{act}f{floor}_boss_relic.frontier.json"))
    }

    pub(super) fn boss_relic_manifest_path(&self, act: u8, floor: i32) -> PathBuf {
        self.root.join(format!("a{act}f{floor}_boss_relic.manifest.json"))
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

    pub(super) fn retain_pre_combat_gap(
        &self,
        handle: RunCutpointHandle,
    ) -> Result<(), String> {
        write_pair(
            &self.latest_pre_combat_frontier_path(),
            &self.latest_pre_combat_manifest_path(),
            handle.args,
            handle.next_branch_id,
            &handle.snapshot,
        )?;
        remove_inflight_pair(&handle)
    }

    pub(super) fn discard_pre_combat(
        &self,
        handle: RunCutpointHandle,
    ) -> Result<(), String> {
        remove_inflight_pair(&handle)?;
        self.remove_latest_if_manifest_matches(&handle.snapshot.manifest)
    }

    pub(super) fn write_boss_relic(
        &self,
        args: Args,
        next_branch_id: usize,
        snapshot: RunCutpointSnapshot,
    ) -> Result<PathBuf, String> {
        let frontier_path = self.boss_relic_frontier_path(
            snapshot.manifest.act,
            snapshot.manifest.floor,
        );
        let manifest_path = self.boss_relic_manifest_path(
            snapshot.manifest.act,
            snapshot.manifest.floor,
        );
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
        let manifest_path = manifest_path_for_frontier(frontier_path)?;
        if !manifest_path.exists() {
            let is_cutpoint = frontier_path
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == "cutpoints");
            return if is_cutpoint {
                Err(format!("cutpoint manifest missing: {}", manifest_path.display()))
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
        let payload = std::fs::read_to_string(&manifest_path).map_err(|error| {
            format!("failed to read {}: {error}", manifest_path.display())
        })?;
        let manifest: RunCutpointManifestV1 =
            serde_json::from_str(&payload).map_err(|error| {
                format!("failed to parse {}: {error}", manifest_path.display())
            })?;
        RunCutpointSnapshot {
            manifest,
            branch: frontier.front().unwrap().clone(),
        }
        .validate_branch(frontier.front().unwrap())
    }
}

fn remove_inflight_pair(handle: &RunCutpointHandle) -> Result<(), String> {
    super::run_capsule_io::remove_if_exists(&handle.frontier_path)?;
    super::run_capsule_io::remove_if_exists(&handle.manifest_path)
}

fn manifest_path_for_frontier(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("invalid cutpoint frontier path: {}", path.display()))?;
    let stem = file_name
        .strip_suffix(".frontier.json")
        .ok_or_else(|| format!("cutpoint frontier must end in .frontier.json: {}", path.display()))?;
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
    super::run_capsule_io::write_json(manifest_path, value)
}
```

Add `remove_latest_if_manifest_matches` inside `impl RunCutpointStore`; it loads the latest manifest, compares its branch id and session checkpoint hash with the supplied manifest, and removes the latest pair only on an exact match. This clears a successfully resumed gap without allowing one branch to clear another branch's recovery point.

```rust
fn remove_latest_if_manifest_matches(
    &self,
    expected: &RunCutpointManifestV1,
) -> Result<(), String> {
    let path = self.latest_pre_combat_manifest_path();
    let Ok(payload) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let actual: RunCutpointManifestV1 =
        serde_json::from_str(&payload).map_err(|error| {
            format!("failed to parse {}: {error}", path.display())
        })?;
    if actual.branch_id != expected.branch_id
        || actual.session_checkpoint_hash != expected.session_checkpoint_hash
    {
        return Ok(());
    }
    super::run_capsule_io::remove_if_exists(
        &self.latest_pre_combat_frontier_path(),
    )?;
    super::run_capsule_io::remove_if_exists(&path)
}
```

Implement `write_boss_relic` with stem `a{act}f{floor}_boss_relic`. Derive the sibling manifest path by replacing the exact `.frontier.json` suffix with `.manifest.json`, not with `Path::with_extension`. Validation returns `Ok(())` for a missing manifest only when the frontier is outside a directory named `cutpoints`; a frontier under `cutpoints/` without its manifest fails with `cutpoint manifest missing`. When a manifest exists, deserialize `RunCutpointManifestV1`, require exactly one frontier branch, reconstruct a `RunCutpointSnapshot` from the manifest plus branch clone, and call `validate_branch`.

- [ ] **Step 5: Run store and legacy frontier tests**

```powershell
cargo test --lib runtime::branch::owner_audit::run_cutpoint_store::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::frontier_checkpoint::tests -- --nocapture
```

Expected: all focused tests pass; the legacy checkpoint-without-contract test remains green.

- [ ] **Step 6: Commit atomic bounded persistence**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/frontier_checkpoint.rs src/runtime/branch/owner_audit/run_cutpoint_store.rs
git commit -m "feat: persist bounded run cutpoints"
```

---

### Task 3: Pre-Combat Search Lifecycle

**Files:**
- Create: `src/runtime/branch/owner_audit/run_cutpoint_recorder.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/runner.rs`
- Test: inline `run_cutpoint_recorder::tests`

**Interfaces:**
- Consumes: `RunCutpointStore`, `RunCutpointSnapshot`, a cloned branch template, `CombatSearchPortfolioResult.status`.
- Produces:
  - `RunCutpointRecorder::new(store, args, generation, next_branch_id, &Branch)`
  - `before_combat_search(&mut self, &RunControlSession) -> Result<(), String>`
  - `after_combat_search(&mut self, &BranchStatus) -> Result<(), String>`
  - `retain_on_error(&mut self) -> Result<(), String>`

- [ ] **Step 1: Write failing lifecycle tests**

Register `run_cutpoint_recorder` and add tests with a temporary `RunCutpointStore`:

```rust
#[test]
fn combat_gap_promotes_the_pre_search_session_not_the_mutated_session() {
    let (store, args, mut branch) = active_combat_branch();
    let expected_gold = branch.session.run_state.gold;
    let mut recorder = RunCutpointRecorder::new(
        Some(&store),
        args,
        17,
        branch.id + 1,
        &branch,
    );
    recorder.before_combat_search(&branch.session).unwrap();
    branch.session.run_state.gold += 99;

    recorder
        .after_combat_search(&BranchStatus::CombatGap {
            boundary: "Combat".to_string(),
            reason: "no accepted win".to_string(),
        })
        .unwrap();

    let checkpoint = frontier_checkpoint::load(
        &store.latest_pre_combat_frontier_path(),
    )
    .unwrap();
    let (frontier, _) = checkpoint.into_frontier().unwrap();
    assert_eq!(frontier.front().unwrap().session.run_state.gold, expected_gold);
    assert!(matches!(
        frontier.front().unwrap().status,
        BranchStatus::AwaitingAuto { .. }
    ));
}

#[test]
fn successful_search_removes_its_inflight_pair() {
    let (store, args, branch) = active_combat_branch();
    let mut recorder = RunCutpointRecorder::new(
        Some(&store),
        args,
        17,
        branch.id + 1,
        &branch,
    );
    recorder.before_combat_search(&branch.session).unwrap();

    recorder
        .after_combat_search(&BranchStatus::Running {
            boundary: "Card Reward".to_string(),
            owner: Owner::CardReward,
        })
        .unwrap();

    assert!(!store.inflight_pre_combat_frontier_path(branch.id).exists());
    assert!(!store.latest_pre_combat_frontier_path().exists());
}
```

Build `active_combat_branch` with a real stable combat position:

```rust
fn active_combat_branch() -> (RunCutpointStore, Args, Branch) {
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;

    let args = crate::runtime::branch::default_branch_args(20260713006);
    let (mut frontier, _) = super::super::branch_runtime::BranchRuntime::initial_frontier(
        args,
        std::time::Instant::now(),
    );
    let mut branch = frontier.pop_front().unwrap();
    let combat = crate::test_support::blank_test_combat();
    branch.session.engine_state = EngineState::CombatPlayerTurn;
    branch.session.active_combat = Some(ActiveCombat::new(
        EngineState::CombatPlayerTurn,
        combat,
        CombatContext::Room(RoomCombatContext {
            room_type: RoomType::MonsterRoom,
        }),
    ));
    branch.status = BranchStatus::AwaitingAuto {
        boundary: "Combat".to_string(),
        reason: "test".to_string(),
    };
    (
        RunCutpointStore::new(unique_root("recorder").join("cutpoints")),
        args,
        branch,
    )
}

fn unique_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sts_run_cutpoint_recorder_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ))
}
```

- [ ] **Step 2: Run recorder tests and verify RED**

```powershell
cargo test --lib runtime::branch::owner_audit::run_cutpoint_recorder::tests -- --nocapture
```

Expected: compilation fails because `RunCutpointRecorder` does not exist.

- [ ] **Step 3: Implement the lifecycle recorder**

The recorder clones the supplied branch template, replaces its session with the exact pre-search session, and forces a resumable status:

```rust
pub(super) struct RunCutpointRecorder<'a> {
    store: Option<&'a RunCutpointStore>,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    branch_template: Branch,
    active_pre_combat: Option<RunCutpointHandle>,
}

impl<'a> RunCutpointRecorder<'a> {
    pub(super) fn new(
        store: Option<&'a RunCutpointStore>,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        branch_template: &Branch,
    ) -> Self {
        Self {
            store,
            args,
            generation,
            next_branch_id,
            branch_template: branch_template.clone(),
            active_pre_combat: None,
        }
    }

    pub(super) fn before_combat_search(
        &mut self,
        session: &RunControlSession,
    ) -> Result<(), String> {
        if session.active_combat.is_none() || self.active_pre_combat.is_some() {
            return Ok(());
        }
        let Some(store) = self.store else {
            return Ok(());
        };
        let mut branch = self.branch_template.clone();
        branch.session = session.clone();
        branch.status = BranchStatus::AwaitingAuto {
            boundary: "Combat".to_string(),
            reason: "resume pre-combat search cutpoint".to_string(),
        };
        let snapshot = RunCutpointSnapshot::capture(
            RunCutpointKind::PreCombatSearch,
            self.generation,
            &branch,
        )?;
        self.active_pre_combat = Some(store.write_pre_combat_inflight(
            self.args,
            self.next_branch_id,
            snapshot,
        )?);
        Ok(())
    }

    pub(super) fn after_combat_search(
        &mut self,
        status: &BranchStatus,
    ) -> Result<(), String> {
        let Some(handle) = self.active_pre_combat.take() else {
            return Ok(());
        };
        let store = self.store.expect("a handle requires a store");
        if matches!(
            status,
            BranchStatus::CombatGap { .. }
                | BranchStatus::BudgetGap { .. }
                | BranchStatus::AwaitingAuto { .. }
        ) {
            store.retain_pre_combat_gap(handle)
        } else {
            store.discard_pre_combat(handle)
        }
    }

    pub(super) fn retain_on_error(&mut self) -> Result<(), String> {
        let Some(handle) = self.active_pre_combat.take() else {
            return Ok(());
        };
        self.store
            .expect("a handle requires a store")
            .retain_pre_combat_gap(handle)
    }
}
```

- [ ] **Step 4: Invoke the recorder around the real portfolio call**

Change `advance_to_owner_or_gap` to accept `cutpoints: &mut RunCutpointRecorder<'_>`. Immediately before `run_combat_portfolio_step`, call `before_combat_search(session)`. For `Ok(portfolio)`, call `after_combat_search(&portfolio.status)` before consuming the portfolio. For `Err(error)`, call `retain_on_error`; if persistence fails, return `AdvanceFailed("cutpoint persistence failed: ...")`, otherwise preserve the original search error.

Use this exact control shape:

```rust
if let Err(error) = cutpoints.before_combat_search(session) {
    return advance_failed(format!("cutpoint persistence failed: {error}"));
}
match combat_search_orchestrator::run_combat_portfolio_step(session, run_args) {
    Ok(portfolio) => {
        if let Err(error) = cutpoints.after_combat_search(&portfolio.status) {
            return advance_failed(format!("cutpoint persistence failed: {error}"));
        }
        // existing portfolio absorption follows unchanged
    }
    Err(error) => {
        if let Err(cutpoint_error) = cutpoints.retain_on_error() {
            return advance_failed(format!(
                "combat search failed: {error}; cutpoint persistence failed: {cutpoint_error}"
            ));
        }
        return advance_failed(error);
    }
}
```

Extract only a small `advance_failed(message: String) -> AdvanceResult` helper to avoid duplicating the existing empty-vector result construction.

- [ ] **Step 5: Run recorder and runner tests**

```powershell
cargo test --lib runtime::branch::owner_audit::run_cutpoint_recorder::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::runner::tests -- --nocapture
```

Expected: lifecycle tests pass; existing runner tests remain green.

- [ ] **Step 6: Commit pre-combat lifecycle support**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/run_cutpoint_recorder.rs src/runtime/branch/owner_audit/runner.rs
git commit -m "feat: retain pre-combat search recovery"
```

---

### Task 4: Boss Relic Capture and Strict Resume Validation

**Files:**
- Modify: `src/runtime/branch/owner_audit/branch_scheduler.rs`
- Modify: `src/runtime/branch/owner_audit/branch_generation.rs`
- Modify: `src/runtime/branch/owner_audit/run_cutpoint_recorder.rs`
- Modify: `src/runtime/branch/owner_audit/run_startup.rs`
- Test: inline tests in the same modules

**Interfaces:**
- Consumes: Task 3 recorder and Task 2 optional-manifest validator.
- Produces:
  - `RunCutpointRecorder::capture_owner_boundary(&Branch) -> Result<(), String>`
  - `prepare_branch_work(..., cutpoint_store: Option<&RunCutpointStore>, next_branch_id: usize)`
  - manifested `--resume-frontier` validation before execution.

- [ ] **Step 1: Write a failing Boss Relic boundary test**

Add a scheduler test that creates a direct Boss Relic branch, a temp store, and calls `prepare_branch_work`. Assert before examining returned choices:

```rust
let frontier_path = store.boss_relic_frontier_path(2, 32);
let manifest_path = store.boss_relic_manifest_path(2, 32);
assert!(frontier_path.exists());
assert!(manifest_path.exists());

let checkpoint = frontier_checkpoint::load(&frontier_path).unwrap();
let (frontier, _) = checkpoint.into_frontier().unwrap();
let restored = frontier.front().unwrap();
assert_eq!(restored.session.run_state.current_hp, 13);
assert!(matches!(restored.status, BranchStatus::Running { owner: Owner::BossRelic, .. }));
assert_eq!(branch_owner_choices(restored).len(), 4);
```

- [ ] **Step 2: Write a failing strict-resume test**

In `run_startup.rs`, factor checkpoint restoration into a testable helper:

```rust
fn load_resume_frontier(
    path: &Path,
) -> Result<(frontier_checkpoint::FrontierCheckpoint, VecDeque<Branch>, usize), String>
```

The test writes a valid Boss Relic pair, changes the manifest's `candidate_order_hash`, and expects `load_resume_frontier` to fail with `candidate order fingerprint mismatch`.

- [ ] **Step 3: Run both tests and verify RED**

```powershell
cargo test --lib runtime::branch::owner_audit::branch_scheduler::tests::boss_relic -- --nocapture
cargo test --lib runtime::branch::owner_audit::run_startup::tests::manifested_resume -- --nocapture
```

Expected: failures because owner capture and manifested resume validation are not wired.

- [ ] **Step 4: Capture Boss Relic before expansion**

Add to `RunCutpointRecorder`:

```rust
pub(super) fn capture_owner_boundary(&self, branch: &Branch) -> Result<(), String> {
    let BranchStatus::Running { owner: Owner::BossRelic, .. } = branch.status else {
        return Ok(());
    };
    let Some(store) = self.store else {
        return Ok(());
    };
    let snapshot = RunCutpointSnapshot::capture(
        RunCutpointKind::OwnerDecision,
        self.generation,
        branch,
    )?;
    store.write_boss_relic(self.args, self.next_branch_id, snapshot)?;
    Ok(())
}
```

In `prepare_branch_work`, construct the recorder from a clone of the incoming branch metadata. Call `capture_owner_boundary(&branch)` before the generation-limit/expandability gate and again after any runner advance, always before `branch_owner_choices(&branch)`. This ensures `--generations N` can stop exactly on a Boss Relic boundary and still preserve it. Convert a persistence failure into `BranchStatus::AdvanceFailed("cutpoint persistence failed: ...")` and return no choices.

Thread `cutpoint_store` and `next_branch_id` through `prepare_generation` and `prepare_branch_work`; do not change policy expansion or retention behavior.

- [ ] **Step 5: Validate manifested resume while preserving legacy resume**

Implement `load_resume_frontier` as:

```rust
fn load_resume_frontier(
    path: &Path,
) -> Result<(frontier_checkpoint::FrontierCheckpoint, VecDeque<Branch>, usize), String> {
    let checkpoint = frontier_checkpoint::load(path)?;
    let validation_copy = checkpoint.clone();
    let (frontier, next_branch_id) = validation_copy.into_frontier()?;
    RunCutpointStore::validate_resume_path(path, &frontier)?;
    Ok((checkpoint, frontier, next_branch_id))
}
```

Derive `Clone` for `FrontierCheckpoint` and its private `BranchCheckpoint`. Use the helper only in the existing `resume_frontier` branch. `validate_resume_path` must immediately return `Ok(())` when no sibling manifest exists outside `cutpoints/`; under `cutpoints/`, a missing manifest is a hard error.

- [ ] **Step 6: Run scheduler, startup, and frontier tests**

```powershell
cargo test --lib runtime::branch::owner_audit::branch_scheduler::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::run_startup::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::frontier_checkpoint::tests -- --nocapture
```

Expected: all focused tests pass; legacy resume remains accepted.

- [ ] **Step 7: Commit owner cutpoints and validation**

```powershell
git add src/runtime/branch/owner_audit/branch_scheduler.rs src/runtime/branch/owner_audit/branch_generation.rs src/runtime/branch/owner_audit/run_cutpoint_recorder.rs src/runtime/branch/owner_audit/run_startup.rs src/runtime/branch/owner_audit/frontier_checkpoint.rs
git commit -m "feat: checkpoint boss relic decisions"
```

---

### Task 5: Artifact-Root Wiring and End-to-End Regression

**Files:**
- Modify: `src/runtime/branch/owner_audit/run_capsule.rs`
- Modify: `src/runtime/branch/owner_audit/run_loop.rs`
- Modify: `src/runtime/branch/owner_audit/branch_generation.rs`
- Modify: `src/runtime/branch/owner_audit/branch_scheduler.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Test: owner-audit runtime and cutpoint integration tests

**Interfaces:**
- Consumes: all prior tasks.
- Produces:
  - `RunCapsule::cutpoints_dir() -> PathBuf`
  - one optional `RunCutpointStore` per slice
  - no-output mode with no implicit `cutpoints/` directory.

- [ ] **Step 1: Write failing artifact-root tests**

Add two integration-level tests:

```rust
#[test]
fn explicit_frontier_path_places_cutpoints_in_sibling_directory() {
    let frontier = temp_root().join("requested.frontier.json");
    let root = cutpoint_root_for_outputs(Some(&frontier), None, None).unwrap();
    assert_eq!(root, frontier.parent().unwrap().join("cutpoints"));
}

#[test]
fn no_artifact_output_has_no_cutpoint_store() {
    assert!(cutpoint_root_for_outputs(None, None, None).is_none());
}
```

Add a capsule path test asserting `RunCapsule::new(root).cutpoints_dir() == root.join("cutpoints")`.

- [ ] **Step 2: Run artifact-root tests and verify RED**

```powershell
cargo test --lib runtime::branch::owner_audit::run_loop::tests::cutpoint -- --nocapture
cargo test --lib runtime::branch::owner_audit::run_capsule::tests::cutpoints -- --nocapture
```

Expected: compilation fails because `cutpoint_root_for_outputs` and `cutpoints_dir` do not exist.

- [ ] **Step 3: Wire one optional store per run slice**

Expose the capsule path:

```rust
pub(super) fn cutpoints_dir(&self) -> PathBuf {
    self.store.root_path().join("cutpoints")
}
```

Add `CapsuleArtifactStore::root_path(&self) -> &Path` rather than cloning or exposing the store.

In `run_loop.rs`, add:

```rust
fn cutpoint_root_for_outputs(
    frontier_checkpoint_path: Option<&PathBuf>,
    resume_frontier: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
) -> Option<PathBuf> {
    capsule
        .map(RunCapsule::cutpoints_dir)
        .or_else(|| {
            frontier_checkpoint_path
                .or(resume_frontier)
                .and_then(|path| path.parent())
                .map(|parent| {
                    if parent.file_name().is_some_and(|name| name == "cutpoints") {
                        parent.to_path_buf()
                    } else {
                        parent.join("cutpoints")
                    }
                })
        })
}
```

Construct `let cutpoint_store = cutpoint_root_for_outputs(&frontier_checkpoint_path, &resume_frontier, run_capsule.as_ref()).map(RunCutpointStore::new);` once before the generation loop. Pass `cutpoint_store.as_ref()` and the current `next_branch_id` into `prepare_generation`. Do not create the directory eagerly; the first cutpoint pair write creates it.

- [ ] **Step 4: Add an end-to-end exact-resume regression**

Use a direct Boss Relic session rather than replaying a seed. Persist through the scheduler, load through `load_resume_frontier`, then assert:

```rust
assert_eq!(restored.session.run_state, original.session.run_state);
assert_eq!(restored.session.engine_state, original.session.engine_state);
assert_eq!(
    build_decision_surface(&restored.session).view.candidates,
    build_decision_surface(&original.session).view.candidates,
);
```

Also assert that overriding `max_branches` after load does not change the restored session before expansion.

- [ ] **Step 5: Run focused owner-audit checks**

```powershell
cargo test --lib runtime::branch::owner_audit::run_cutpoint -- --nocapture
cargo test --lib runtime::branch::owner_audit::branch_scheduler -- --nocapture
cargo test --lib runtime::branch::owner_audit::run_startup -- --nocapture
cargo test --lib runtime::branch::owner_audit::run_loop -- --nocapture
```

Expected: all focused checks pass with no panic or unexpected warning.

- [ ] **Step 6: Commit the slice wiring**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/run_capsule.rs src/runtime/branch/owner_audit/capsule_artifact_store.rs src/runtime/branch/owner_audit/run_loop.rs src/runtime/branch/owner_audit/branch_generation.rs src/runtime/branch/owner_audit/branch_scheduler.rs
git commit -m "feat: wire resumable run cutpoints"
```

---

### Task 6: Completion Verification and Seed006 Handoff

**Files:**
- Modify only if verification reveals a defect covered by a new failing regression.

**Interfaces:**
- Consumes: complete cutpoint implementation.
- Produces: verified implementation ready for the separate reproducible-search plan and the seed006 Boss Relic experiment.

- [ ] **Step 1: Format and run diff checks**

```powershell
cargo fmt --all -- --check
git diff --check
```

Expected: both commands exit 0.

- [ ] **Step 2: Run the architecture boundary suite once**

```powershell
cargo test --test architecture_runtime_boundaries -- --nocapture
```

Expected: every architecture boundary test passes.

- [ ] **Step 3: Run the full library suite once**

```powershell
cargo test --lib
```

Expected: all library tests pass. Do not replace this checkpoint with repeated filtered invocations.

- [ ] **Step 4: Verify the CLI still parses checkpoint flags**

```powershell
cargo run --quiet --bin branch_tiny -- --help
```

Expected: exit 0 and help still lists `--frontier-checkpoint` and `--resume-frontier`.

- [ ] **Step 5: Inspect final history and worktree**

```powershell
git status --short
git log -6 --oneline
```

Expected: clean worktree and the task commits in order after the design/plan commits.

- [ ] **Step 6: Record the next bounded action**

Report that exact run cutpoints are complete, but do not yet rank Black Blood, Coffee Dripper, and Philosopher's Stone. The next delivery is the independent reproducible-search/wall-safety classification plan, followed by one seed006 run that creates a real Act 2 Boss Relic cutpoint and forks the three candidates from it.
