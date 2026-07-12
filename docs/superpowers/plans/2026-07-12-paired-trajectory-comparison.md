# Paired Trajectory Comparison Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce typed, layered baseline-vs-challenger trajectory comparisons in run-capsule summaries without changing lane execution, frontier retention, or production decisions.

**Architecture:** A pure strategy module owns comparison vocabulary and conservative verdict aggregation. A small owner-audit adapter converts `Branch` into typed snapshots; capsule formatting serializes the snapshots and comparisons as read-only evidence. Missing pressure/deployability instrumentation remains `Unknown`, so it cannot promote a challenger or condemn the baseline.

**Tech Stack:** Rust, serde/serde_json, existing challenger policy state, owner-audit capsule artifacts.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Do not use subagents for this repository session.
- Never run `cargo clean`.
- Follow red-green TDD with focused library tests and frequent local commits.
- Do not introduce a universal trajectory score.
- HP, gold, max HP, and potions are a Pareto resource layer; HP alone never decides the trajectory.
- A terminal victory/defeat is decisive. Non-terminal evidence with opposing directional layers is `Inconclusive`.
- Any coverage-limited or missing pressure/deployability evidence is `Unknown`, not `Equal`, `Covered`, or failure.
- Do not infer damage or defense causality from HP loss.
- Do not modify branch choice planning, expansion, retention, or baseline production ordering.
- Keep run-control free of comparison strategy.
- Write comparison evidence only through the existing capsule summary path under `artifacts/runs`.
- Run the full library and `architecture_runtime_boundaries` suites only at the completion checkpoint.

---

### Task 1: Pure Layered Trajectory Comparator

**Files:**
- Create: `src/ai/strategy/trajectory_comparison.rs`
- Modify: `src/ai/strategy/mod.rs`

**Interfaces:**
- Produces: `TrajectoryTerminal`, `TrajectoryProgress`, `TrajectoryPressureEvidence`, `TrajectoryDeployabilityEvidence`, `TrajectoryResources`, `TrajectoryConstruction`, `TrajectorySnapshot`, `LayerComparison`, `TrajectoryVerdict`, `TrajectoryComparison`, and `compare_trajectories`.
- Boundary: the module consumes typed facts only and has no dependency on runtime `Branch`.

- [ ] **Step 1: Write failing comparison tests**

Create the module with tests that use this helper:

```rust
fn snapshot(lane: &str) -> TrajectorySnapshot {
    TrajectorySnapshot {
        lane: lane.to_string(),
        terminal: TrajectoryTerminal::Running,
        progress: TrajectoryProgress { act: 2, floor: 20 },
        pressure: TrajectoryPressureEvidence::Unknown,
        deployability: TrajectoryDeployabilityEvidence::Unknown,
        resources: TrajectoryResources {
            hp: 40,
            max_hp: 80,
            gold: 100,
            potion_count: 1,
        },
        construction: TrajectoryConstruction {
            burden: DeckBurdenBand::Watch,
            completed_commitments: 0,
            active_commitments: 0,
            failed_commitments: 0,
        },
    }
}
```

Add these contracts:

```rust
#[test]
fn terminal_victory_is_decisive_even_with_fewer_resources() {
    let baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    challenger.terminal = TrajectoryTerminal::Victory;
    challenger.resources.hp = 1;
    challenger.resources.gold = 0;

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(comparison.verdict, TrajectoryVerdict::ChallengerBetter);
    assert_eq!(comparison.progression, LayerComparison::ChallengerBetter);
}

#[test]
fn more_hp_cannot_resolve_unknown_pressure_and_deployability() {
    let baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    challenger.resources.hp = 70;

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(comparison.resources, LayerComparison::ChallengerBetter);
    assert_eq!(comparison.pressure, LayerComparison::Unknown);
    assert_eq!(comparison.verdict, TrajectoryVerdict::Inconclusive);
}

#[test]
fn mixed_nonterminal_directions_are_inconclusive() {
    let mut baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    challenger.progress.floor = 21;
    baseline.resources.hp = 60;
    challenger.resources.hp = 30;

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(comparison.progression, LayerComparison::ChallengerBetter);
    assert_eq!(comparison.resources, LayerComparison::BaselineBetter);
    assert_eq!(comparison.verdict, TrajectoryVerdict::Inconclusive);
}

#[test]
fn resource_layer_uses_pareto_dominance_instead_of_a_sum() {
    let mut baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    baseline.resources.hp = 60;
    baseline.resources.gold = 20;
    challenger.resources.hp = 40;
    challenger.resources.gold = 200;

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(comparison.resources, LayerComparison::Conflict);
    assert_eq!(comparison.verdict, TrajectoryVerdict::Inconclusive);
}

#[test]
fn complete_equal_evidence_is_equivalent_but_unknown_is_not() {
    let mut baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    baseline.pressure = TrajectoryPressureEvidence::Comparable {
        open: 1,
        covered: 2,
    };
    challenger.pressure = baseline.pressure;
    baseline.deployability = TrajectoryDeployabilityEvidence::Comparable {
        claimed_answers: 2,
        timely_playable: 1,
    };
    challenger.deployability = baseline.deployability;

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(comparison.verdict, TrajectoryVerdict::Equivalent);
}
```

- [ ] **Step 2: Register the module and verify red**

```powershell
cargo test --lib trajectory_comparison::tests
```

Expected: compilation fails because the comparison types and function do not exist.

- [ ] **Step 3: Implement serializable comparison vocabulary**

```rust
use serde::{Deserialize, Serialize};
use crate::ai::strategy::challenger_signature::DeckBurdenBand;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryTerminal { Running, Victory, Defeat, CoverageLimited, Gap }

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrajectoryProgress { pub act: u8, pub floor: i32 }

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrajectoryPressureEvidence {
    Unknown,
    Comparable { open: u16, covered: u16 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrajectoryDeployabilityEvidence {
    Unknown,
    Comparable { claimed_answers: u16, timely_playable: u16 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrajectoryResources {
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub potion_count: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrajectoryConstruction {
    pub burden: DeckBurdenBand,
    pub completed_commitments: u16,
    pub active_commitments: u16,
    pub failed_commitments: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrajectorySnapshot {
    pub lane: String,
    pub terminal: TrajectoryTerminal,
    pub progress: TrajectoryProgress,
    pub pressure: TrajectoryPressureEvidence,
    pub deployability: TrajectoryDeployabilityEvidence,
    pub resources: TrajectoryResources,
    pub construction: TrajectoryConstruction,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerComparison { BaselineBetter, ChallengerBetter, Equal, Unknown, Conflict }

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryVerdict { BaselineBetter, ChallengerBetter, Equivalent, Inconclusive }

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrajectoryComparison {
    pub baseline_lane: String,
    pub challenger_lane: String,
    pub progression: LayerComparison,
    pub pressure: LayerComparison,
    pub deployability: LayerComparison,
    pub resources: LayerComparison,
    pub construction: LayerComparison,
    pub verdict: TrajectoryVerdict,
}
```

- [ ] **Step 4: Implement layer comparisons and conservative aggregation**

`compare_trajectories` follows these exact rules:

1. Victory beats every non-victory; defeat loses to every non-defeat. A terminal direction returns immediately.
2. Otherwise compare `(act, floor)` lexicographically.
3. Comparable pressure uses Pareto direction over lower `open` and higher `covered`; any `Unknown` yields `Unknown`.
4. Comparable deployability uses Pareto direction over higher `claimed_answers` and `timely_playable`; any `Unknown` yields `Unknown`.
5. Resources use Pareto direction over non-negative `(hp, max_hp, gold, potion_count)`; mixed directions yield `Conflict`.
6. Construction is `ChallengerBetter` only when it has more completed commitments, no more active/failed commitments, and no worse burden. It is `BaselineBetter` under the exact inverse. Otherwise it is `Equal`, `Unknown` for a speculative challenger with only active commitments, or `Conflict`.
7. For non-terminal evidence, ignore `Equal`; any `Unknown` or `Conflict` makes the verdict `Inconclusive`. Remaining directional layers must all agree, otherwise `Inconclusive`. No directions and no unknowns means `Equivalent`.

- [ ] **Step 5: Run focused tests and commit**

```powershell
cargo test --lib trajectory_comparison::tests
cargo fmt --all
git add src/ai/strategy/mod.rs src/ai/strategy/trajectory_comparison.rs
git commit -m "feat: compare challenger trajectories"
```

---

### Task 2: Runtime Branch-To-Trajectory Adapter

**Files:**
- Create: `src/runtime/branch/owner_audit/trajectory_snapshot.rs`
- Modify: `src/runtime/branch/owner_audit.rs`

**Interfaces:**
- Produces: `trajectory_snapshot(&Branch) -> TrajectorySnapshot` and `frontier_trajectory_evaluation(&VecDeque<Branch>) -> FrontierTrajectoryEvaluation`.
- Boundary: missing pressure and timely-play evidence remain typed `Unknown`; the adapter does not invent them from HP or static adequacy.

- [ ] **Step 1: Write failing adapter tests**

Create a local branch fixture and assert:

```rust
fn test_branch(policy_lane: BranchPolicyLane) -> Branch {
    Branch {
        id: 1,
        parent_id: Some(0),
        path: Vec::new(),
        session: RunControlSession::new(RunControlConfig::default()),
        status: BranchStatus::Running {
            owner: Owner::CardReward,
            boundary: "test".to_string(),
        },
        policy_lane,
        combat_portfolio: None,
        auto_steps: Vec::new(),
        combat_search: Vec::new(),
        combat_search_history: Vec::new(),
        accepted_high_loss_diagnostics: Vec::new(),
    }
}

fn commitment(status: CommitmentStatus) -> StrategyCommitment {
    StrategyCommitment {
        kind: StrategyCommitmentKind::ExhaustEngine,
        status,
        requirements: Vec::new(),
        horizon: CommitmentHorizon::CurrentActBoss,
        burden_units: 1,
    }
}

#[test]
fn baseline_snapshot_keeps_uninstrumented_layers_unknown() {
    let branch = test_branch(BranchPolicyLane::default());
    let snapshot = trajectory_snapshot(&branch);

    assert_eq!(snapshot.lane, "baseline");
    assert_eq!(snapshot.pressure, TrajectoryPressureEvidence::Unknown);
    assert_eq!(snapshot.deployability, TrajectoryDeployabilityEvidence::Unknown);
    assert_eq!(snapshot.resources.hp, branch.session.run_state.current_hp);
}

#[test]
fn challenger_snapshot_counts_commitment_outcomes_without_scoring_them() {
    let mut policy = ChallengerPolicyState::new(1);
    policy.commitments = vec![
        commitment(CommitmentStatus::Completed),
        commitment(CommitmentStatus::Active),
        commitment(CommitmentStatus::Expired),
    ];
    let branch = test_branch(BranchPolicyLane::challenger(policy));

    let snapshot = trajectory_snapshot(&branch);

    assert_eq!(snapshot.construction.completed_commitments, 1);
    assert_eq!(snapshot.construction.active_commitments, 1);
    assert_eq!(snapshot.construction.failed_commitments, 1);
}

#[test]
fn frontier_evaluation_pairs_every_challenger_with_baseline() {
    let frontier = VecDeque::from([
        test_branch(BranchPolicyLane::default()),
        test_branch(BranchPolicyLane::challenger(ChallengerPolicyState::new(1))),
        test_branch(BranchPolicyLane::challenger(ChallengerPolicyState::new(2))),
    ]);

    let evaluation = frontier_trajectory_evaluation(&frontier);

    assert_eq!(evaluation.snapshots.len(), 3);
    assert_eq!(evaluation.comparisons.len(), 2);
    assert!(evaluation.comparisons.iter().all(|item| item.baseline_lane == "baseline"));
}
```

- [ ] **Step 2: Verify red**

Run `cargo test --lib trajectory_snapshot::tests` and expect missing adapter failures.

- [ ] **Step 3: Implement snapshot conversion**

Define:

```rust
#[derive(Clone, Debug, Serialize)]
pub(super) struct FrontierTrajectoryEvaluation {
    pub(super) snapshots: Vec<TrajectorySnapshot>,
    pub(super) comparisons: Vec<TrajectoryComparison>,
}
```

Conversion rules:

- `BranchStatus::Terminal(Victory/Defeat)` maps to terminal states.
- operation/search budget statuses map to `CoverageLimited`; automation/apply gaps map to `Gap`; running statuses map to `Running`.
- pressure and deployability are `Unknown` in this delivery.
- potion count is the number of occupied potion slots, clamped to `u8::MAX`.
- burden maps from `DeckPlanSnapshot::from_run_state(...).strategic_deficit.deck_burden`.
- completed counts `Completed`; active counts `Active`; failed counts `Expired` and `Abandoned`.
- the baseline snapshot is first when present; each challenger is compared with it through `compare_trajectories`. Without a baseline, comparisons are empty.

- [ ] **Step 4: Run focused tests and commit**

```powershell
cargo test --lib trajectory_snapshot::tests
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/trajectory_snapshot.rs
git commit -m "feat: snapshot policy trajectories"
```

---

### Task 3: Read-Only Capsule Comparison Evidence

**Files:**
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`
- Modify: `src/runtime/branch/owner_audit/capsule_artifact_store.rs`

**Interfaces:**
- Adds: `trajectory_evaluation` to frontier `summary.json` and `trajectory_snapshot` to single-branch result/summary JSON.
- Contract: serialization only; no comparison verdict is consumed by the scheduler, planner, frontier, or owner.

- [ ] **Step 1: Write failing artifact tests**

Add a formatter test:

```rust
fn challenger_sample_branch(lane_id: u8) -> Branch {
    let mut branch = sample_branch();
    branch.id = lane_id as usize + 1;
    branch.policy_lane = super::super::branch_policy_lane::BranchPolicyLane::challenger(
        sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState::new(
            lane_id,
        ),
    );
    branch
}

#[test]
fn frontier_summary_exposes_paired_trajectory_evaluation() {
    let frontier = VecDeque::from([
        sample_branch(),
        challenger_sample_branch(1),
    ]);
    let value = frontier_trajectory_summary_value(2, 2, &frontier);

    assert_eq!(value["frontier_count"], 2);
    assert_eq!(value["trajectory_evaluation"]["snapshots"].as_array().unwrap().len(), 2);
    assert_eq!(value["trajectory_evaluation"]["comparisons"].as_array().unwrap().len(), 1);
}
```

Add a capsule-store test that writes a frontier and reads `summary.json`, asserting the same schema fields exist.

- [ ] **Step 2: Verify red**

```powershell
cargo test --lib frontier_summary_exposes_paired_trajectory_evaluation
```

Expected: failure because frontier summary accepts only counts.

- [ ] **Step 3: Serialize typed evaluation**

Replace the count-only formatter with:

```rust
pub(super) fn frontier_trajectory_summary_value(
    frontier_count: usize,
    running: usize,
    frontier: &VecDeque<Branch>,
) -> Value {
    json!({
        "frontier_count": frontier_count,
        "frontier_running_count": running,
        "trajectory_evaluation": trajectory_snapshot::frontier_trajectory_evaluation(frontier),
    })
}
```

Use it from `CapsuleArtifactStore::write_frontier_summary`. Add `trajectory_snapshot::trajectory_snapshot(branch)` to `branch_summary_value` and `result_value`.

- [ ] **Step 4: Prove no behavior consumer exists**

```powershell
$matches = rg -n "TrajectoryVerdict|trajectory_evaluation" src/runtime/branch/owner_audit/branch_frontier.rs src/runtime/branch/owner_audit/branch_generation.rs src/runtime/branch/owner_audit/policy_expansion_plan.rs src/runtime/branch/owner_audit/owners.rs
if ($LASTEXITCODE -eq 0) { $matches; throw "trajectory comparison affects runtime behavior" }
if ($LASTEXITCODE -ne 1) { throw "rg failed while checking behavior consumers" }
```

- [ ] **Step 5: Run focused tests and commit**

```powershell
cargo test --lib run_capsule_format::tests
cargo test --lib capsule_artifact_store::tests
git add src/runtime/branch/owner_audit/run_capsule_format.rs src/runtime/branch/owner_audit/capsule_artifact_store.rs
git commit -m "feat: expose paired trajectory evidence"
```

---

### Task 4: Completion Verification

- [ ] **Step 1: Format and inspect**

```powershell
cargo fmt --all
cargo fmt --all -- --check
git diff --check
git status --short
```

- [ ] **Step 2: Run all focused suites**

```powershell
cargo test --lib trajectory_comparison::tests
cargo test --lib trajectory_snapshot::tests
cargo test --lib run_capsule_format::tests
cargo test --lib capsule_artifact_store::tests
```

- [ ] **Step 3: Run full verification once**

```powershell
cargo test --lib
cargo test --bin branch_tiny
cargo test --test architecture_runtime_boundaries
```

- [ ] **Step 4: Verify architectural boundaries and final state**

```powershell
$runControlMatches = rg -n "TrajectoryVerdict|compare_trajectories|FrontierTrajectoryEvaluation" src/eval/run_control
if ($LASTEXITCODE -eq 0) { $runControlMatches; throw "trajectory comparison leaked into run-control" }
if ($LASTEXITCODE -ne 1) { throw "rg failed while checking run-control boundary" }

$behaviorMatches = rg -n "TrajectoryVerdict|trajectory_evaluation" src/runtime/branch/owner_audit/branch_frontier.rs src/runtime/branch/owner_audit/branch_generation.rs src/runtime/branch/owner_audit/policy_expansion_plan.rs
if ($LASTEXITCODE -eq 0) { $behaviorMatches; throw "trajectory comparison affects behavior" }
if ($LASTEXITCODE -ne 1) { throw "rg failed while checking behavior boundary" }

git status --short --branch
git log -8 --oneline
```

Expected: clean local `master`, no push, typed comparisons present only in strategy and capsule-evidence paths.
