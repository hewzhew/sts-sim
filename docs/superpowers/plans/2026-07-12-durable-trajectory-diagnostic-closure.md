# Durable Trajectory Diagnostic Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the latest baseline/challenger trajectory comparison through stopped branches and resumed slices, and retain complete owner decision evidence in combat-gap cases.

**Architecture:** A new owner-audit evidence store merges the latest observation for each lane into `trajectory_state.json`; capsule writes refresh it from live frontiers and stopped branches, while final JSON only reads it for diagnostics. Combat cases gain additive optional branch and path evidence, populated from existing branch records without changing replay state or search configuration.

**Tech Stack:** Rust, serde/serde_json, existing owner-audit capsule IO, existing typed trajectory comparator, red-green Cargo tests.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree or use subagents.
- Start implementation from a clean Git status and make frequent local commits.
- Never run `cargo clean`.
- Write durable evidence only below `artifacts/runs` during manual runs; automated tests use temporary directories.
- Do not keep stopped branches in the frontier or change branch scheduling, retention, expansion, policy choice, combat search, or promotion behavior.
- `trajectory_state.json` is diagnostic output, never resumable execution state.
- Missing trajectory state is valid; malformed existing state is an error and must not be overwritten silently.
- Pressure and deployability remain `Unknown` in this delivery.
- Existing combat cases without additive evidence must continue to load and replay.
- Use focused red-green tests for every behavior; run the full library and architecture suites only at the completion checkpoint.
- Do not rerun the bounded seed as part of implementation verification.

---

### Task 1: Durable Latest-Lane Trajectory Evidence Store

**Files:**
- Create: `src/runtime/branch/owner_audit/trajectory_evidence_store.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/trajectory_snapshot.rs`

**Interfaces:**
- Consumes: `trajectory_snapshot(&Branch) -> TrajectorySnapshot`, `compare_trajectories`, and the existing structured `BranchStatus` serializer.
- Produces: `TrajectoryObservation`, `TrajectoryEvidenceState`, `record_frontier`, `record_branch`, and `read_state` for capsule integration.
- Boundary: the store accepts branch facts and writes evidence; it exposes no scheduler or owner decision API.

- [ ] **Step 1: Add a failing snapshot-list evaluation test**

In `trajectory_snapshot.rs`, add a test for a wished-for helper:

```rust
#[test]
fn snapshot_evaluation_pairs_latest_lanes_without_live_branches() {
    let baseline = trajectory_snapshot(&test_branch(BranchPolicyLane::default()));
    let challenger = trajectory_snapshot(&test_branch(BranchPolicyLane::challenger(
        ChallengerPolicyState::new(1),
    )));

    let evaluation = trajectory_evaluation_from_snapshots(vec![
        challenger.clone(),
        baseline.clone(),
    ]);

    assert_eq!(evaluation.snapshots[0].lane, "baseline");
    assert_eq!(evaluation.snapshots[1].lane, "challenger-1");
    assert_eq!(evaluation.comparisons.len(), 1);
    assert_eq!(evaluation.comparisons[0].baseline_lane, "baseline");
}
```

- [ ] **Step 2: Verify the helper test is red**

Run:

```powershell
cargo test --lib snapshot_evaluation_pairs_latest_lanes_without_live_branches
```

Expected: compilation fails because `trajectory_evaluation_from_snapshots` does not exist.

- [ ] **Step 3: Implement snapshot-list evaluation and reuse it for frontiers**

Derive `Deserialize` as well as `Serialize` for `FrontierTrajectoryEvaluation`, then add:

```rust
pub(super) fn trajectory_evaluation_from_snapshots(
    mut snapshots: Vec<TrajectorySnapshot>,
) -> FrontierTrajectoryEvaluation {
    snapshots.sort_by(|left, right| lane_sort_key(&left.lane).cmp(&lane_sort_key(&right.lane)));
    let baseline = snapshots.iter().find(|snapshot| snapshot.lane == "baseline");
    let comparisons = baseline
        .map(|baseline| {
            snapshots
                .iter()
                .filter(|snapshot| snapshot.lane != "baseline")
                .map(|challenger| compare_trajectories(baseline, challenger))
                .collect()
        })
        .unwrap_or_default();
    FrontierTrajectoryEvaluation {
        snapshots,
        comparisons,
    }
}

fn lane_sort_key(lane: &str) -> (u8, u16, &str) {
    if lane == "baseline" {
        return (0, 0, lane);
    }
    let number = lane
        .strip_prefix("challenger-")
        .and_then(|value| value.parse().ok())
        .unwrap_or(u16::MAX);
    (1, number, lane)
}
```

Make `frontier_trajectory_evaluation` collect snapshots and delegate to this helper.

- [ ] **Step 4: Verify the snapshot helper is green**

Run:

```powershell
cargo test --lib trajectory_snapshot::tests
```

Expected: all trajectory snapshot tests pass.

- [ ] **Step 5: Add failing evidence-store tests**

Register `trajectory_evidence_store.rs` in `owner_audit.rs`. Define tests first with a local branch fixture:

```rust
#[test]
fn stopped_lanes_survive_reopen_and_form_final_comparison() {
    let root = std::env::temp_dir().join("trajectory_state_cross_slice");
    let path = root.join("trajectory_state.json");
    let _ = std::fs::remove_dir_all(&root);

    let mut baseline = test_branch(1, BranchPolicyLane::default());
    let mut challenger = test_branch(
        2,
        BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
    );
    let frontier = VecDeque::from([baseline.clone(), challenger.clone()]);
    record_frontier(&path, 10, &frontier).unwrap();

    challenger.status = BranchStatus::CombatGap {
        boundary: "boss".to_string(),
        reason: "no win".to_string(),
    };
    challenger.session.run_state.current_hp = 47;
    record_branch(&path, 40, &challenger).unwrap();

    baseline.status = BranchStatus::CombatGap {
        boundary: "boss".to_string(),
        reason: "no win".to_string(),
    };
    baseline.session.run_state.current_hp = 42;
    record_branch(&path, 42, &baseline).unwrap();

    let state = read_state(&path).unwrap();
    assert_eq!(state.observations.len(), 2);
    assert_eq!(state.evaluation.snapshots.len(), 2);
    assert_eq!(state.evaluation.comparisons.len(), 1);
    assert_eq!(state.evaluation.snapshots[0].resources.hp, 42);
    assert_eq!(state.evaluation.snapshots[1].resources.hp, 47);
    assert!(state
        .evaluation
        .snapshots
        .iter()
        .all(|snapshot| snapshot.terminal == TrajectoryTerminal::CoverageLimited));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn malformed_existing_state_is_not_silently_replaced() {
    let root = std::env::temp_dir().join("trajectory_state_malformed");
    let path = root.join("trajectory_state.json");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&path, "not-json").unwrap();

    let error = record_branch(&path, 1, &test_branch(1, BranchPolicyLane::default()))
        .unwrap_err();

    assert!(error.contains("trajectory_state.json"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "not-json");
    let _ = std::fs::remove_dir_all(root);
}
```

- [ ] **Step 6: Verify evidence-store tests are red**

Run:

```powershell
cargo test --lib trajectory_evidence_store::tests
```

Expected: compilation fails because the state types and store functions do not exist.

- [ ] **Step 7: Implement the evidence state and atomic merge/write**

Use these exact data shapes:

```rust
const TRAJECTORY_STATE_SCHEMA: &str = "branch_tiny_trajectory_state_v0";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TrajectoryObservation {
    pub(super) generation: usize,
    pub(super) branch_id: usize,
    pub(super) parent_id: Option<usize>,
    pub(super) status: Value,
    pub(super) snapshot: TrajectorySnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TrajectoryEvidenceState {
    pub(super) schema: String,
    pub(super) observations: Vec<TrajectoryObservation>,
    pub(super) evaluation: FrontierTrajectoryEvaluation,
}
```

Implement:

```rust
pub(super) fn read_state(path: &Path) -> Result<TrajectoryEvidenceState, String>;
pub(super) fn record_frontier(
    path: &Path,
    generation: usize,
    frontier: &VecDeque<Branch>,
) -> Result<TrajectoryEvidenceState, String>;
pub(super) fn record_branch(
    path: &Path,
    generation: usize,
    branch: &Branch,
) -> Result<TrajectoryEvidenceState, String>;
```

`read_state` returns an empty valid state on `NotFound`, rejects malformed JSON or a schema other
than `branch_tiny_trajectory_state_v0`, and includes the path in every error. Merge by lane label;
replace only when `(generation, branch_id)` is not older. Recompute the evaluation after every
merge. Write with `run_capsule_io::write_json` so the existing temporary-file replacement contract
is reused.

- [ ] **Step 8: Run focused tests and commit Task 1**

Run:

```powershell
cargo fmt --all
cargo test --lib trajectory_snapshot::tests
cargo test --lib trajectory_evidence_store::tests
git diff --check
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/trajectory_snapshot.rs src/runtime/branch/owner_audit/trajectory_evidence_store.rs
git commit -m "feat: persist policy trajectory evidence"
```

Expected: focused tests pass and the first implementation commit is created.

---

### Task 2: Record Stopped Lanes and Expose Final Paired Evaluation

**Files:**
- Modify: `src/runtime/branch/slice_result.rs`
- Modify: `src/runtime/branch/owner_audit/capsule_artifact_store.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule.rs`
- Modify: `src/runtime/branch/owner_audit/branch_observer.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`

**Interfaces:**
- Consumes: Task 1 `record_frontier`, `record_branch`, and `read_state`.
- Produces: a tracked `TrajectoryEvidence` artifact reference and state-derived
  `trajectory_evaluation` in final result/summary JSON.
- Boundary: stopped-branch observation occurs beside persistence; runtime behavior never reads the
  resulting verdict.

- [ ] **Step 1: Add a failing artifact-kind round-trip test**

Extend `slice_result.rs` tests:

```rust
#[test]
fn artifact_summary_tracks_trajectory_evidence() {
    let artifact = ArtifactRef::new(
        ArtifactKind::TrajectoryEvidence,
        "target/trajectory_state.json",
        "branch_tiny_trajectory_state_v0",
        "owner_audit_runtime",
    );
    let summary = ArtifactWriteSummary::single_ref(artifact.clone());

    assert!(summary.trajectory_evidence_written);
    assert_eq!(summary.trajectory_evidence_ref, Some(artifact.clone()));
    assert!(summary.refs().contains(&artifact));
}
```

- [ ] **Step 2: Verify the artifact-kind test is red**

Run:

```powershell
cargo test --lib artifact_summary_tracks_trajectory_evidence
```

Expected: compilation fails because `TrajectoryEvidence` and its summary fields do not exist.

- [ ] **Step 3: Implement tracked trajectory artifacts**

Add `ArtifactKind::TrajectoryEvidence`, plus:

```rust
pub trajectory_evidence_written: bool,
pub trajectory_evidence_ref: Option<ArtifactRef>,
```

Merge, record, and return this reference through the existing `ArtifactWriteSummary` methods. Use
schema `branch_tiny_trajectory_state_v0` and creator `owner_audit_runtime`.

- [ ] **Step 4: Add failing capsule lifecycle tests**

In `capsule_artifact_store.rs`, extend the trajectory artifact test fixture:

```rust
#[test]
fn final_result_keeps_challenger_that_stopped_in_an_earlier_slice() {
    let root = std::env::temp_dir().join("final_cross_lane_trajectory_evidence");
    let _ = std::fs::remove_dir_all(&root);
    let store = CapsuleArtifactStore::new(root.clone());
    let baseline = test_branch(1, BranchPolicyLane::default());
    let mut challenger = test_branch(
        2,
        BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
    );
    let frontier = VecDeque::from([baseline.clone(), challenger.clone()]);
    store
        .write_frontier(test_args(), 10, 3, &frontier, "paused", Some("wall_deadline"))
        .unwrap();

    challenger.status = BranchStatus::CombatGap {
        boundary: "boss".to_string(),
        reason: "no win".to_string(),
    };
    challenger.session.run_state.current_hp = 47;
    store.record_stopped_trajectory(40, &challenger).unwrap();

    let mut final_baseline = baseline;
    final_baseline.status = BranchStatus::CombatGap {
        boundary: "boss".to_string(),
        reason: "no win".to_string(),
    };
    final_baseline.session.run_state.current_hp = 42;
    store
        .write_result(test_args(), 42, &final_baseline)
        .unwrap();

    let result: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("result.json")).unwrap(),
    )
    .unwrap();
    let summary: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("summary.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(result["trajectory_evaluation"]["snapshots"].as_array().unwrap().len(), 2);
    assert_eq!(summary["trajectory_evaluation"]["comparisons"].as_array().unwrap().len(), 1);
    assert!(root.join("trajectory_state.json").exists());

    let _ = std::fs::remove_dir_all(root);
}
```

Add a branch-observer test using a fake capsule root that records a non-resumable challenger and
asserts that `trajectory_state.json` contains `challenger-1`. This proves the lifecycle call is made,
not only the store helper.

- [ ] **Step 5: Verify capsule lifecycle tests are red**

Run:

```powershell
cargo test --lib final_result_keeps_challenger_that_stopped_in_an_earlier_slice
cargo test --lib stopped_branch_records_trajectory_before_discard
```

Expected: failures because stopped trajectory recording and final evaluation parameters are absent.

- [ ] **Step 6: Integrate trajectory evidence with capsule persistence**

Add to `CapsuleArtifactStore`:

```rust
pub(super) fn trajectory_state_path(&self) -> PathBuf;
pub(super) fn record_stopped_trajectory(
    &self,
    generation: usize,
    branch: &Branch,
) -> Result<(), String>;
fn current_trajectory_evaluation(&self) -> Result<FrontierTrajectoryEvaluation, String>;
```

Before writing a frontier summary, call `record_frontier`. Before writing any selected result or
terminal entry, call `record_branch`. Pass the state-derived evaluation to `branch_summary_value`
and `result_value`, which add a top-level `trajectory_evaluation` field.

Expose through `RunCapsule`:

```rust
pub(super) fn record_stopped_trajectory(
    &self,
    generation: usize,
    branch: &Branch,
) -> Result<ArtifactWriteSummary, String>;
```

In `branch_observer::record_terminal_and_objective`, call this method when
`!branch.status.is_resumable()` before terminal/result persistence. Merge the returned trajectory
artifact reference into the existing artifact summary. Do not alter `GenerationAdvance`, frontier
membership, or objective handling.

- [ ] **Step 7: Run focused tests and commit Task 2**

Run:

```powershell
cargo fmt --all
cargo test --lib trajectory_artifact_tests
cargo test --lib branch_observer
cargo test --lib artifact_summary_tracks_trajectory_evidence
git diff --check
git add src/runtime/branch/slice_result.rs src/runtime/branch/owner_audit/capsule_artifact_store.rs src/runtime/branch/owner_audit/run_capsule.rs src/runtime/branch/owner_audit/branch_observer.rs src/runtime/branch/owner_audit/run_capsule_format.rs
git commit -m "fix: retain final paired trajectory evidence"
```

Expected: stopped lanes survive into final JSON and artifact references include the durable state.

---

### Task 3: Backward-Compatible Combat-Case Decision Evidence

**Files:**
- Modify: `src/eval/combat_case.rs`
- Modify: `src/runtime/branch/owner_audit/combat_gap_case.rs`

**Interfaces:**
- Consumes: typed `TrajectorySnapshot`, serialized private `BranchPolicyLane`, and the already
  recorded `BranchPathStep`.
- Produces: optional `CombatCaseBranchEvidence` and optional full path-step `decision_evidence`.
- Boundary: evidence fields do not alter `CombatPosition`, RNG summaries, saved search, or review
  search configuration.

- [ ] **Step 1: Add failing legacy and branch-evidence round-trip tests**

In `eval/combat_case.rs`, add tests:

```rust
fn sample_snapshot() -> TrajectorySnapshot {
    TrajectorySnapshot {
        lane: "challenger-1".to_string(),
        terminal: TrajectoryTerminal::CoverageLimited,
        progress: TrajectoryProgress { act: 3, floor: 48 },
        pressure: TrajectoryPressureEvidence::Unknown,
        deployability: TrajectoryDeployabilityEvidence::Unknown,
        resources: TrajectoryResources {
            hp: 47,
            max_hp: 81,
            gold: 595,
            potion_count: 2,
        },
        construction: TrajectoryConstruction {
            burden: DeckBurdenBand::Clean,
            completed_commitments: 0,
            active_commitments: 0,
            failed_commitments: 0,
        },
    }
}

fn sample_case() -> CombatCase {
    let run = crate::state::run::RunState::new(7, 0, false, "IRONCLAD");
    let position = CombatPosition::new(
        EngineState::CombatPlayerTurn,
        crate::test_support::blank_test_combat(),
    );
    CombatCase::new(
        CombatCaseSource {
            seed: 7,
            ascension: 0,
            generation: 4,
            branch_id: 2,
            parent_id: Some(1),
        },
        CombatCaseGap {
            boundary: "Combat".to_string(),
            reason: "no win".to_string(),
            search_nodes: 100,
            search_ms: 10,
            rescue_search_nodes: 200,
            rescue_search_ms: 20,
        },
        CombatCaseRunSummary {
            act: 3,
            floor: 48,
            hp: 47,
            max_hp: 81,
            gold: 595,
            deck_size: 14,
            relic_count: 11,
            potion_slots: 3,
        },
        Vec::new(),
        None,
        vec![CombatCasePathStep {
            key: Value::Null,
            label: "Skip card reward".to_string(),
            state_before: None,
            decision_evidence: None,
        }],
        CombatCaseRngSummary::from_pool(&run.rng_pool),
        position,
    )
}

fn sample_branch_evidence() -> CombatCaseBranchEvidence {
    CombatCaseBranchEvidence {
        schema: "branch_policy_combat_evidence_v0".to_string(),
        policy_lane: json!({"kind": "challenger", "policy": {"lane_id": 1}}),
        trajectory_snapshot: sample_snapshot(),
    }
}

#[test]
fn legacy_case_without_branch_evidence_still_deserializes() {
    let value = serde_json::to_value(sample_case()).unwrap();
    let mut object = value.as_object().unwrap().clone();
    object.remove("branch_evidence");
    for step in object["path"].as_array_mut().unwrap() {
        step.as_object_mut().unwrap().remove("decision_evidence");
    }

    let restored: CombatCase = serde_json::from_value(Value::Object(object)).unwrap();

    assert!(restored.branch_evidence.is_none());
    assert!(restored
        .path
        .iter()
        .all(|step| step.decision_evidence.is_none()));
}

#[test]
fn branch_and_decision_evidence_round_trip_without_changing_position() {
    let mut case = sample_case();
    let original_position = serde_json::to_value(&case.position).unwrap();
    case.branch_evidence = Some(sample_branch_evidence());
    case.path[0].decision_evidence = Some(json!({
        "policy_lane": "challenger-1",
        "candidate_pool": [{"rank": 1, "selected": true}],
        "annotation": {"kind": "candidate"},
        "decision_delta": {"gold_delta": -50},
        "shop_boss_preview_candidates": [{"rank": 1}],
        "shop_boss_preview_bundles": [{"rank": 1}]
    }));

    let restored: CombatCase =
        serde_json::from_value(serde_json::to_value(&case).unwrap()).unwrap();

    assert_eq!(serde_json::to_value(&restored.position).unwrap(), original_position);
    assert_eq!(restored.branch_evidence.unwrap().trajectory_snapshot.lane, "challenger-1");
    assert_eq!(restored.path[0].decision_evidence.as_ref().unwrap()["candidate_pool"][0]["selected"], true);
}
```

- [ ] **Step 2: Verify combat-case type tests are red**

Run:

```powershell
cargo test --lib eval::combat_case::tests
```

Expected: compilation fails because `branch_evidence` and `decision_evidence` do not exist.

- [ ] **Step 3: Add additive evidence types**

In `eval/combat_case.rs`, add:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseBranchEvidence {
    pub schema: String,
    pub policy_lane: Value,
    pub trajectory_snapshot: TrajectorySnapshot,
}
```

Add to `CombatCase`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub branch_evidence: Option<CombatCaseBranchEvidence>,
```

Add to `CombatCasePathStep`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub decision_evidence: Option<Value>,
```

`CombatCase::new` initializes `branch_evidence` to `None`; existing constructor call sites remain
source-compatible.

- [ ] **Step 4: Add a failing owner-audit projection test**

In `combat_gap_case.rs`, construct a `BranchPathStep` with one ordinary candidate and one shop
preview candidate/bundle, then assert:

```rust
#[test]
fn path_projection_keeps_complete_recorded_decision_evidence() {
    let step = branch_path_step_with_all_evidence();

    let projected = path_step(&step);
    let evidence = projected.decision_evidence.unwrap();

    assert_eq!(evidence["policy_lane"], "challenger-1");
    assert_eq!(evidence["candidate_pool"].as_array().unwrap().len(), 1);
    assert_eq!(evidence["shop_boss_preview_candidates"].as_array().unwrap().len(), 1);
    assert_eq!(evidence["shop_boss_preview_bundles"].as_array().unwrap().len(), 1);
    assert!(!evidence["annotation"].is_null());
    assert!(!evidence["decision_delta"].is_null());
}
```

Add a saved-case test that builds a challenger branch at a stable combat choice, calls
`save_combat_gap_case`, reloads it, and asserts the top-level policy lane is `challenger` and the
trajectory lane is `challenger-1`.

- [ ] **Step 5: Verify owner-audit projection tests are red**

Run:

```powershell
cargo test --lib path_projection_keeps_complete_recorded_decision_evidence
cargo test --lib saved_challenger_case_keeps_policy_and_trajectory_identity
```

Expected: assertions fail because the current projection drops those fields.

- [ ] **Step 6: Populate branch and full decision evidence**

In `save_combat_gap_case`, set:

```rust
case.branch_evidence = Some(CombatCaseBranchEvidence {
    schema: "branch_policy_combat_evidence_v0".to_string(),
    policy_lane: to_value(&branch.policy_lane).unwrap_or(Value::Null),
    trajectory_snapshot: trajectory_snapshot::trajectory_snapshot(branch),
});
```

In `path_step`, set `decision_evidence` to a JSON object serialized directly from the existing
recorded fields:

```rust
decision_evidence: Some(json!({
    "policy_lane": step.policy_lane,
    "annotation": &step.annotation,
    "decision_delta": &step.decision_delta,
    "candidate_pool": &step.candidate_pool,
    "shop_boss_preview_candidates": &step.shop_boss_preview_candidates,
    "shop_boss_preview_bundles": &step.shop_boss_preview_bundles,
})),
```

Do not derive candidates from current run state and do not feed these fields into replay or search.

- [ ] **Step 7: Run focused tests and commit Task 3**

Run:

```powershell
cargo fmt --all
cargo test --lib eval::combat_case::tests
cargo test --lib combat_gap_case::tests
git diff --check
git add src/eval/combat_case.rs src/runtime/branch/owner_audit/combat_gap_case.rs
git commit -m "feat: retain combat case decision evidence"
```

Expected: new and legacy combat cases pass round-trip tests.

---

### Task 4: Completion Verification and Boundary Audit

**Files:**
- Verify only; no planned production changes.

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: fresh evidence that diagnostics persist and behavior boundaries remain unchanged.

- [ ] **Step 1: Format and run every focused suite**

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo test --lib trajectory_snapshot::tests
cargo test --lib trajectory_evidence_store::tests
cargo test --lib trajectory_artifact_tests
cargo test --lib eval::combat_case::tests
cargo test --lib combat_gap_case::tests
cargo test --lib artifact_summary_tracks_trajectory_evidence
git diff --check
```

Expected: every command exits zero. If a filter runs zero tests, replace it with the exact module or
test name and rerun; zero tests is not verification.

- [ ] **Step 2: Run full project verification once**

```powershell
cargo test --lib
cargo test --bin branch_tiny
cargo test --test architecture_runtime_boundaries
```

Expected: all library and architecture tests pass and `branch_tiny` compiles.

- [ ] **Step 3: Prove the evidence remains behavior-free**

```powershell
$runControl = rg -n "TrajectoryEvidenceState|trajectory_state|trajectory_evaluation" src/eval/run_control
if ($LASTEXITCODE -eq 0) { $runControl; throw "trajectory evidence leaked into run-control" }
if ($LASTEXITCODE -ne 1) { throw "run-control boundary scan failed" }

$behavior = rg -n "TrajectoryEvidenceState|trajectory_state|trajectory_evaluation" `
  src/runtime/branch/owner_audit/branch_frontier.rs `
  src/runtime/branch/owner_audit/branch_generation.rs `
  src/runtime/branch/owner_audit/policy_expansion_plan.rs `
  src/runtime/branch/owner_audit/owners.rs
if ($LASTEXITCODE -eq 0) { $behavior; throw "trajectory evidence affects behavior" }
if ($LASTEXITCODE -ne 1) { throw "behavior boundary scan failed" }
```

Expected: both scans return no matches.

- [ ] **Step 4: Inspect final repository state**

```powershell
git status --short --branch
git log -8 --oneline
```

Expected: clean local branch with the specification, plan, and three focused implementation
commits; no push and no seed rerun.
