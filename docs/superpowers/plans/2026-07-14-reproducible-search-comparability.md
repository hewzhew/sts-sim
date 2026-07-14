# Reproducible Search Comparability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve deterministic combat-search budget provenance and fail closed when owner-audit trajectory comparisons contain wall-clock-limited or otherwise insufficient search evidence.

**Architecture:** `CombatSearchV2Report` continues to own raw search facts, run-control traces preserve those facts, and a new pure owner-audit classifier converts retained trace history into generic trajectory comparability evidence. The generic trajectory comparison layer knows only the compact evidence vocabulary and forces excluded pairs to `Inconclusive`; no scheduler, owner, searcher, or strategy scorer consumes the result.

**Tech Stack:** Rust 2021, serde/serde_json, existing `CombatSearchV2Report`, run-control trace annotations, owner-audit branch history and capsule artifacts, Cargo test suites.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator` on the existing local feature branch; do not create a Git worktree.
- Begin every implementation task from a clean status and make one focused local commit per independently reviewable task.
- Never run `cargo clean`.
- Write durable experiment evidence only below `artifacts/runs`.
- This delivery classifies evidence only: do not change search ordering, line acceptance, strategy scores, scheduler behavior, owner policy, or game policy.
- Missing legacy comparability fields must deserialize conservatively as insufficient/excluded rather than comparable.
- Unknown coverage labels and replay failures must never panic or manufacture comparable evidence.
- Fixed node limits define experiment work; wall limits remain safety stops whose activation invalidates the affected arm.
- Use focused tests during red/green work; finish with formatting, the full library suite, and `architecture_runtime_boundaries`.

## File Map

```text
src/eval/run_control/trace_annotation.rs
  Preserve node-budget termination in persisted run-control summaries.
src/eval/run_control/combat_line_trace.rs
  Copy node-budget termination from CombatSearchV2Report and test legacy JSON.
src/ai/strategy/trajectory_comparison.rs
  Define generic comparability/eligibility types and fail-closed pair verdicts.
src/eval/combat_case.rs
  Keep the existing test fixture compiling with the new snapshot field.
src/runtime/branch/owner_audit.rs
  Register the owner-audit classifier module.
src/runtime/branch/owner_audit/search_comparability.rs
  Classify retained run-control attempts and aggregate branch-arm provenance.
src/runtime/branch/owner_audit/trajectory_snapshot.rs
  Attach branch history classification to every durable trajectory snapshot.
src/runtime/branch/owner_audit/trajectory_evidence_store.rs
  Prove old trajectory-state JSON remains readable and fails closed on refresh.
src/runtime/branch/owner_audit/run_capsule_format.rs
  Prove capsule JSON exposes snapshot comparability and pair eligibility.
```

---

### Task 1: Preserve Node-Budget Provenance Across Run-Control

**Files:**

- Modify: `src/eval/run_control/trace_annotation.rs:97-130,188-225,456-500`
- Modify: `src/eval/run_control/combat_line_trace.rs:120-170,306-455`

**Interfaces:**

- Consumes: `CombatSearchV2Report::stats.node_budget_hit: bool`.
- Produces: serde-defaulted `CombatSearchPerformanceSnapshotV1::node_budget_hit: bool` and `CombatSearchTraceSummary::node_budget_hit: bool`.

- [ ] **Step 1: Write a failing propagation and compatibility test**

Add these tests to `combat_line_trace.rs`'s existing `tests` module. The first test exercises both report-to-snapshot and snapshot-to-summary conversion. The second removes the new fields from serialized values to model old trace JSON.

```rust
use crate::eval::run_control::{combat_search_trace_summaries, CombatSearchTraceSummary};

#[test]
fn combat_search_trace_preserves_node_budget_hit() {
    let mut combat = crate::test_support::blank_test_combat();
    combat.entities.monsters.clear();
    let start = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
    let mut report = run_combat_search_v2(
        &start.engine,
        &start.combat,
        CombatSearchV2Config {
            max_nodes: 1,
            ..CombatSearchV2Config::default()
        },
    );
    report.stats.node_budget_hit = true;
    let session = RunControlSession::new(Default::default());
    let annotations = vec![combat_search_performance_trace_annotation(
        "search_combat",
        &session,
        &start,
        &report,
    )];

    let RunControlTraceAnnotationV1::CombatSearchPerformance { snapshot } = &annotations[0]
    else {
        panic!("expected combat search performance annotation")
    };
    let summary = combat_search_trace_summaries(&annotations)
        .next()
        .expect("combat search summary");

    assert!(snapshot.node_budget_hit);
    assert!(summary.node_budget_hit);
}

#[test]
fn legacy_combat_search_trace_defaults_node_budget_hit_to_false() {
    let mut combat = crate::test_support::blank_test_combat();
    combat.entities.monsters.clear();
    let start = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
    let mut report = run_combat_search_v2(
        &start.engine,
        &start.combat,
        CombatSearchV2Config {
            max_nodes: 1,
            ..CombatSearchV2Config::default()
        },
    );
    report.stats.node_budget_hit = true;
    let session = RunControlSession::new(Default::default());
    let annotation = combat_search_performance_trace_annotation(
        "legacy",
        &session,
        &start,
        &report,
    );
    let RunControlTraceAnnotationV1::CombatSearchPerformance { snapshot } = annotation else {
        panic!("expected combat search performance annotation")
    };
    let mut snapshot_value = serde_json::to_value(snapshot).expect("serialize snapshot");
    snapshot_value
        .as_object_mut()
        .expect("snapshot object")
        .remove("node_budget_hit");
    let restored_snapshot: CombatSearchPerformanceSnapshotV1 =
        serde_json::from_value(snapshot_value).expect("legacy snapshot");

    let mut summary_value = serde_json::to_value(CombatSearchTraceSummary {
        coverage_status: "NodeBudgetLimited".to_string(),
        node_budget_hit: true,
        ..CombatSearchTraceSummary::default()
    })
    .expect("serialize summary");
    summary_value
        .as_object_mut()
        .expect("summary object")
        .remove("node_budget_hit");
    let restored_summary: CombatSearchTraceSummary =
        serde_json::from_value(summary_value).expect("legacy summary");

    assert!(!restored_snapshot.node_budget_hit);
    assert!(!restored_summary.node_budget_hit);
}
```

- [ ] **Step 2: Run the tests and confirm RED**

Run:

```powershell
cargo test --lib combat_search_trace_preserves_node_budget_hit -- --nocapture
cargo test --lib legacy_combat_search_trace_defaults_node_budget_hit_to_false -- --nocapture
```

Expected: compilation fails because neither run-control type has a `node_budget_hit` field.

- [ ] **Step 3: Add the minimal fields and mappings**

Add this field immediately after `deadline_hit` in both trace structs:

```rust
#[serde(default)]
pub node_budget_hit: bool,
```

In `combat_search_performance_snapshot`, copy the report fact:

```rust
nodes_to_first_win: report.stats.nodes_to_first_win,
deadline_hit: report.stats.deadline_hit,
node_budget_hit: report.stats.node_budget_hit,
nodes_expanded: report.stats.nodes_expanded,
```

In `combat_search_trace_summaries`, preserve it again:

```rust
nodes_to_first_win: snapshot.nodes_to_first_win,
deadline_hit: snapshot.deadline_hit,
node_budget_hit: snapshot.node_budget_hit,
nodes_expanded: snapshot.nodes_expanded,
```

- [ ] **Step 4: Run focused GREEN verification**

Run:

```powershell
cargo test --lib combat_search_trace_preserves_node_budget_hit -- --nocapture
cargo test --lib legacy_combat_search_trace_defaults_node_budget_hit_to_false -- --nocapture
cargo test --lib eval::run_control::combat_line_trace::tests -- --nocapture
```

Expected: all selected tests pass.

- [ ] **Step 5: Commit the provenance boundary**

```powershell
git diff --check
git add src/eval/run_control/trace_annotation.rs src/eval/run_control/combat_line_trace.rs
git commit -m "feat: preserve combat node budget provenance"
```

---

### Task 2: Make Generic Trajectory Eligibility Explicit

**Files:**

- Modify: `src/ai/strategy/trajectory_comparison.rs:1-140,306-400`
- Modify: `src/eval/combat_case.rs:238-270`

**Interfaces:**

- Consumes: compact arm-level search comparability supplied later by owner-audit.
- Produces: `TrajectorySearchComparabilityStatus`, `TrajectorySearchComparability`, `TrajectoryPairEligibility`, `TrajectorySnapshot::search_comparability`, and `TrajectoryComparison::eligibility`.

- [ ] **Step 1: Write failing pair-eligibility and legacy-JSON tests**

Extend the existing `snapshot` helper with `search_comparability: TrajectorySearchComparability::comparable_without_attempts()`, then add:

```rust
#[test]
fn comparable_pair_keeps_existing_terminal_verdict() {
    let baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    challenger.terminal = TrajectoryTerminal::Victory;

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(comparison.eligibility, TrajectoryPairEligibility::Comparable);
    assert_eq!(comparison.verdict, TrajectoryVerdict::ChallengerBetter);
}

#[test]
fn wall_limited_pair_is_explicitly_excluded() {
    let baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    challenger.terminal = TrajectoryTerminal::Victory;
    challenger.search_comparability = TrajectorySearchComparability {
        status: TrajectorySearchComparabilityStatus::WallSafetyLimited,
        total_attempts: 1,
        exact_accepted_attempts: 0,
        node_bounded_attempts: 0,
        exhaustive_attempts: 0,
        wall_limited_attempts: 1,
        insufficient_attempts: 0,
    };

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(
        comparison.eligibility,
        TrajectoryPairEligibility::ExcludedWallSafetyLimited
    );
    assert_eq!(comparison.verdict, TrajectoryVerdict::Inconclusive);
    assert_eq!(comparison.progression, LayerComparison::ChallengerBetter);
}

#[test]
fn insufficient_pair_is_explicitly_excluded() {
    let baseline = snapshot("baseline");
    let mut challenger = snapshot("challenger-1");
    challenger.search_comparability = TrajectorySearchComparability::default();

    let comparison = compare_trajectories(&baseline, &challenger);

    assert_eq!(
        comparison.eligibility,
        TrajectoryPairEligibility::ExcludedInsufficientEvidence
    );
    assert_eq!(comparison.verdict, TrajectoryVerdict::Inconclusive);
}

#[test]
fn legacy_trajectory_json_defaults_to_excluded_search_evidence() {
    let baseline = snapshot("baseline");
    let challenger = snapshot("challenger-1");
    let comparison = compare_trajectories(&baseline, &challenger);

    let mut snapshot_value = serde_json::to_value(&baseline).expect("serialize snapshot");
    snapshot_value
        .as_object_mut()
        .expect("snapshot object")
        .remove("search_comparability");
    let restored_snapshot: TrajectorySnapshot =
        serde_json::from_value(snapshot_value).expect("legacy snapshot");

    let mut comparison_value =
        serde_json::to_value(comparison).expect("serialize comparison");
    comparison_value
        .as_object_mut()
        .expect("comparison object")
        .remove("eligibility");
    let restored_comparison: TrajectoryComparison =
        serde_json::from_value(comparison_value).expect("legacy comparison");

    assert_eq!(
        restored_snapshot.search_comparability.status,
        TrajectorySearchComparabilityStatus::InsufficientEvidence
    );
    assert_eq!(
        restored_comparison.eligibility,
        TrajectoryPairEligibility::ExcludedInsufficientEvidence
    );
}
```

- [ ] **Step 2: Run the generic tests and confirm RED**

Run:

```powershell
cargo test --lib ai::strategy::trajectory_comparison::tests -- --nocapture
```

Expected: compilation fails on the new missing types and fields.

- [ ] **Step 3: Define the fail-closed generic vocabulary**

Insert these definitions before `TrajectorySnapshot`:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectorySearchComparabilityStatus {
    Comparable,
    WallSafetyLimited,
    InsufficientEvidence,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrajectorySearchComparability {
    pub status: TrajectorySearchComparabilityStatus,
    pub total_attempts: u32,
    pub exact_accepted_attempts: u32,
    pub node_bounded_attempts: u32,
    pub exhaustive_attempts: u32,
    pub wall_limited_attempts: u32,
    pub insufficient_attempts: u32,
}

impl TrajectorySearchComparability {
    pub const fn comparable_without_attempts() -> Self {
        Self {
            status: TrajectorySearchComparabilityStatus::Comparable,
            total_attempts: 0,
            exact_accepted_attempts: 0,
            node_bounded_attempts: 0,
            exhaustive_attempts: 0,
            wall_limited_attempts: 0,
            insufficient_attempts: 0,
        }
    }
}

impl Default for TrajectorySearchComparability {
    fn default() -> Self {
        Self {
            status: TrajectorySearchComparabilityStatus::InsufficientEvidence,
            total_attempts: 0,
            exact_accepted_attempts: 0,
            node_bounded_attempts: 0,
            exhaustive_attempts: 0,
            wall_limited_attempts: 0,
            insufficient_attempts: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryPairEligibility {
    Comparable,
    ExcludedWallSafetyLimited,
    ExcludedInsufficientEvidence,
}

impl Default for TrajectoryPairEligibility {
    fn default() -> Self {
        Self::ExcludedInsufficientEvidence
    }
}
```

Add the serde-defaulted fields:

```rust
pub struct TrajectorySnapshot {
    pub lane: String,
    pub terminal: TrajectoryTerminal,
    pub progress: TrajectoryProgress,
    pub pressure: TrajectoryPressureEvidence,
    pub deployability: TrajectoryDeployabilityEvidence,
    pub resources: TrajectoryResources,
    pub construction: TrajectoryConstruction,
    #[serde(default)]
    pub search_comparability: TrajectorySearchComparability,
}

pub struct TrajectoryComparison {
    pub baseline_lane: String,
    pub challenger_lane: String,
    pub progression: LayerComparison,
    pub pressure: LayerComparison,
    pub deployability: LayerComparison,
    pub resources: LayerComparison,
    pub construction: LayerComparison,
    #[serde(default)]
    pub eligibility: TrajectoryPairEligibility,
    pub verdict: TrajectoryVerdict,
}
```

- [ ] **Step 4: Gate only the final verdict, retaining diagnostic layers**

Add this helper:

```rust
fn pair_eligibility(
    baseline: TrajectorySearchComparabilityStatus,
    challenger: TrajectorySearchComparabilityStatus,
) -> TrajectoryPairEligibility {
    use TrajectorySearchComparabilityStatus::{
        Comparable, InsufficientEvidence, WallSafetyLimited,
    };
    match (baseline, challenger) {
        (WallSafetyLimited, _) | (_, WallSafetyLimited) => {
            TrajectoryPairEligibility::ExcludedWallSafetyLimited
        }
        (InsufficientEvidence, _) | (_, InsufficientEvidence) => {
            TrajectoryPairEligibility::ExcludedInsufficientEvidence
        }
        (Comparable, Comparable) => TrajectoryPairEligibility::Comparable,
    }
}
```

In `compare_trajectories`, calculate the old verdict first and then gate it:

```rust
let eligibility = pair_eligibility(
    baseline.search_comparability.status,
    challenger.search_comparability.status,
);
let computed_verdict = terminal_verdict.unwrap_or_else(|| aggregate_nonterminal(&layers));
let verdict = if eligibility == TrajectoryPairEligibility::Comparable {
    computed_verdict
} else {
    TrajectoryVerdict::Inconclusive
};
```

Include `eligibility` in the returned `TrajectoryComparison`. In `src/eval/combat_case.rs`, add `search_comparability: TrajectorySearchComparability::default()` to `sample_snapshot` and import that type.

- [ ] **Step 5: Run focused GREEN verification and commit**

Run:

```powershell
cargo test --lib ai::strategy::trajectory_comparison::tests -- --nocapture
cargo test --lib eval::combat_case::tests -- --nocapture
git diff --check
git add src/ai/strategy/trajectory_comparison.rs src/eval/combat_case.rs
git commit -m "feat: make trajectory comparison eligibility explicit"
```

Expected: all selected tests pass and legacy JSON receives conservative defaults.

---

### Task 3: Classify Retained Search History in Owner-Audit

**Files:**

- Create: `src/runtime/branch/owner_audit/search_comparability.rs`
- Modify: `src/runtime/branch/owner_audit.rs:150-180`
- Modify: `src/runtime/branch/owner_audit/trajectory_snapshot.rs:1-75,130-230`

**Interfaces:**

- Consumes: `classify_search_comparability(attempts: &[CombatSearchTraceSummary])` input from `Branch::combat_search_history`.
- Produces: `TrajectorySearchComparability` with fail-closed arm-level precedence and auditable counts.

- [ ] **Step 1: Create classifier RED tests**

Create `search_comparability.rs` with a test module that constructs compact summaries. Use this exact helper for accepted adjudication:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId;
    use sts_simulator::eval::run_control::{
        CombatLineCleanlinessV1, CombatLineObservedOutcomeV1,
        CombatLineRejectionReasonV1,
    };
    use sts_simulator::sim::combat::CombatTerminal;

    fn attempt(coverage: &str) -> CombatSearchTraceSummary {
        CombatSearchTraceSummary {
            source: "test".to_string(),
            coverage_status: coverage.to_string(),
            ..CombatSearchTraceSummary::default()
        }
    }

    fn accepted() -> CombatLineAdjudicationV1 {
        CombatLineAdjudicationV1::Accepted {
            policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            cleanliness: CombatLineCleanlinessV1::Clean,
            observed_outcome: CombatLineObservedOutcomeV1 {
                terminal: CombatTerminal::Win,
                final_hp: 40,
                hp_loss: 10,
                potions_used: 0,
                action_count: 8,
                gold_delta: 0,
                ritual_dagger_growth: 0,
                gained_curses: Vec::new(),
            },
        }
    }

    fn rejected() -> CombatLineAdjudicationV1 {
        CombatLineAdjudicationV1::Rejected {
            policy: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            reason: CombatLineRejectionReasonV1::NewCurse { cards: Vec::new() },
            observed_outcome: CombatLineObservedOutcomeV1 {
                terminal: CombatTerminal::Win,
                final_hp: 40,
                hp_loss: 10,
                potions_used: 0,
                action_count: 8,
                gold_delta: 0,
                ritual_dagger_growth: 0,
                gained_curses: Vec::new(),
            },
        }
    }

    #[test]
    fn exact_accepted_attempt_remains_comparable_after_safety_deadline() {
        let mut item = attempt("TimeBudgetLimited");
        item.deadline_hit = true;
        item.execution_adjudication = Some(accepted());

        let result = classify_search_comparability(&[item]);

        assert_eq!(result.status, TrajectorySearchComparabilityStatus::Comparable);
        assert_eq!(result.exact_accepted_attempts, 1);
        assert_eq!(result.wall_limited_attempts, 0);
    }

    #[test]
    fn wall_limited_primary_is_not_erased_by_accepted_rescue() {
        let mut primary = attempt("TimeBudgetLimited");
        primary.deadline_hit = true;
        let mut rescue = attempt("AcceptedCompleteCandidate");
        rescue.execution_adjudication = Some(accepted());

        let result = classify_search_comparability(&[primary, rescue]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::WallSafetyLimited
        );
        assert_eq!(result.wall_limited_attempts, 1);
        assert_eq!(result.exact_accepted_attempts, 1);
    }

    #[test]
    fn node_bounded_negative_evidence_is_comparable() {
        let mut item = attempt("NodeBudgetLimited");
        item.node_budget_hit = true;
        item.execution_adjudication = Some(rejected());

        let result = classify_search_comparability(&[item]);

        assert_eq!(result.status, TrajectorySearchComparabilityStatus::Comparable);
        assert_eq!(result.node_bounded_attempts, 1);
    }

    #[test]
    fn exhaustive_negative_evidence_is_comparable() {
        let mut item = attempt("exhaustive");
        item.execution_adjudication = Some(rejected());

        let result = classify_search_comparability(&[item]);

        assert_eq!(result.status, TrajectorySearchComparabilityStatus::Comparable);
        assert_eq!(result.exhaustive_attempts, 1);
    }

    #[test]
    fn unadjudicated_winning_candidate_is_insufficient() {
        let mut item = attempt("AcceptedCompleteCandidate");
        item.complete_win_found = true;

        let result = classify_search_comparability(&[item]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::InsufficientEvidence
        );
        assert_eq!(result.insufficient_attempts, 1);
    }

    #[test]
    fn replay_failure_and_unknown_coverage_are_insufficient() {
        let mut replay_failed = attempt("AcceptedCompleteCandidate");
        replay_failed.execution_adjudication = Some(CombatLineAdjudicationV1::ReplayFailed {
            policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            error: "replay drift".to_string(),
        });

        let result = classify_search_comparability(&[
            replay_failed,
            attempt("FutureCoverageVocabulary"),
        ]);

        assert_eq!(
            result.status,
            TrajectorySearchComparabilityStatus::InsufficientEvidence
        );
        assert_eq!(result.insufficient_attempts, 2);
    }

    #[test]
    fn no_search_attempts_is_comparable() {
        assert_eq!(
            classify_search_comparability(&[]),
            TrajectorySearchComparability::comparable_without_attempts()
        );
    }
}
```

- [ ] **Step 2: Register the empty module and confirm RED**

Add to `owner_audit.rs` beside the other `#[path]` declarations:

```rust
#[path = "owner_audit/search_comparability.rs"]
mod search_comparability;
```

Run:

```powershell
cargo test --lib search_comparability::tests -- --nocapture
```

Expected: compilation fails because `classify_search_comparability` is not defined.

- [ ] **Step 3: Implement the pure attempt and arm classifier**

Place this production code above the test module:

```rust
use sts_simulator::ai::strategy::trajectory_comparison::{
    TrajectorySearchComparability, TrajectorySearchComparabilityStatus,
};
use sts_simulator::eval::run_control::{
    CombatLineAdjudicationV1, CombatSearchTraceSummary,
};

#[derive(Clone, Copy)]
enum AttemptComparability {
    ExactAccepted,
    NodeBounded,
    Exhaustive,
    WallLimited,
    Insufficient,
}

pub(super) fn classify_search_comparability(
    attempts: &[CombatSearchTraceSummary],
) -> TrajectorySearchComparability {
    let mut result = TrajectorySearchComparability::comparable_without_attempts();
    for attempt in attempts {
        result.total_attempts = result.total_attempts.saturating_add(1);
        match classify_attempt(attempt) {
            AttemptComparability::ExactAccepted => {
                result.exact_accepted_attempts =
                    result.exact_accepted_attempts.saturating_add(1);
            }
            AttemptComparability::NodeBounded => {
                result.node_bounded_attempts =
                    result.node_bounded_attempts.saturating_add(1);
            }
            AttemptComparability::Exhaustive => {
                result.exhaustive_attempts = result.exhaustive_attempts.saturating_add(1);
            }
            AttemptComparability::WallLimited => {
                result.wall_limited_attempts =
                    result.wall_limited_attempts.saturating_add(1);
            }
            AttemptComparability::Insufficient => {
                result.insufficient_attempts =
                    result.insufficient_attempts.saturating_add(1);
            }
        }
    }
    result.status = if result.wall_limited_attempts > 0 {
        TrajectorySearchComparabilityStatus::WallSafetyLimited
    } else if result.insufficient_attempts > 0 {
        TrajectorySearchComparabilityStatus::InsufficientEvidence
    } else {
        TrajectorySearchComparabilityStatus::Comparable
    };
    result
}

fn classify_attempt(attempt: &CombatSearchTraceSummary) -> AttemptComparability {
    if matches!(
        attempt.execution_adjudication.as_ref(),
        Some(CombatLineAdjudicationV1::Accepted { .. })
    ) {
        return AttemptComparability::ExactAccepted;
    }
    if attempt.deadline_hit || coverage_is(&attempt.coverage_status, "timebudgetlimited") {
        return AttemptComparability::WallLimited;
    }
    if matches!(
        attempt.execution_adjudication.as_ref(),
        Some(CombatLineAdjudicationV1::ReplayFailed { .. })
    ) {
        return AttemptComparability::Insufficient;
    }
    let unadjudicated_candidate = attempt.execution_adjudication.is_none()
        && (attempt.complete_win_found
            || attempt.best_win.is_some()
            || coverage_is(&attempt.coverage_status, "acceptedcompletecandidate"));
    if unadjudicated_candidate {
        return AttemptComparability::Insufficient;
    }
    if attempt.node_budget_hit || coverage_is(&attempt.coverage_status, "nodebudgetlimited") {
        return AttemptComparability::NodeBounded;
    }
    if coverage_is(&attempt.coverage_status, "exhaustive") {
        return AttemptComparability::Exhaustive;
    }
    AttemptComparability::Insufficient
}

fn coverage_is(actual: &str, expected: &str) -> bool {
    actual
        .chars()
        .filter(|character| *character != '_')
        .flat_map(char::to_lowercase)
        .eq(expected.chars())
}
```

- [ ] **Step 4: Attach classification to branch snapshots**

Import the function in `trajectory_snapshot.rs`:

```rust
use super::search_comparability::classify_search_comparability;
```

Set the new field in `trajectory_snapshot`:

```rust
construction: TrajectoryConstruction {
    burden,
    completed_commitments,
    active_commitments,
    failed_commitments,
},
search_comparability: classify_search_comparability(&branch.combat_search_history),
```

Add this integration test to `trajectory_snapshot.rs`:

```rust
#[test]
fn snapshot_classifies_retained_node_bounded_search_history() {
    let mut branch = test_branch(BranchPolicyLane::default());
    branch.combat_search_history = vec![CombatSearchTraceSummary {
        source: "primary".to_string(),
        coverage_status: "NodeBudgetLimited".to_string(),
        node_budget_hit: true,
        ..CombatSearchTraceSummary::default()
    }];

    let snapshot = trajectory_snapshot(&branch);

    assert_eq!(
        snapshot.search_comparability.status,
        TrajectorySearchComparabilityStatus::Comparable
    );
    assert_eq!(snapshot.search_comparability.node_bounded_attempts, 1);
}
```

Import `CombatSearchTraceSummary` and `TrajectorySearchComparabilityStatus` in the test module.

- [ ] **Step 5: Run focused GREEN verification and commit**

Run:

```powershell
cargo test --lib search_comparability::tests -- --nocapture
cargo test --lib trajectory_snapshot::tests -- --nocapture
git diff --check
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/search_comparability.rs src/runtime/branch/owner_audit/trajectory_snapshot.rs
git commit -m "feat: classify branch search comparability"
```

Expected: accepted witnesses override their own safety deadline, any separate wall-limited attempt invalidates its arm, deterministic node/exhaustive negatives remain usable, and empty histories remain comparable.

---

### Task 4: Lock Durable Artifact Compatibility and Visibility

**Files:**

- Modify: `src/runtime/branch/owner_audit/trajectory_evidence_store.rs:130-230`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs:370-500`

**Interfaces:**

- Consumes: serde defaults from Task 2 and branch snapshot classification from Task 3.
- Produces: regression proof that legacy `branch_tiny_trajectory_state_v0` loads fail closed and current capsule JSON exposes audit fields.

- [ ] **Step 1: Write the legacy trajectory-state regression test**

Add imports for `TrajectoryPairEligibility` and `TrajectorySearchComparabilityStatus`, then add:

```rust
#[test]
fn legacy_trajectory_state_loads_and_refreshes_fail_closed() {
    let root = std::env::temp_dir().join("trajectory_state_legacy_comparability");
    let path = root.join("trajectory_state.json");
    let _ = std::fs::remove_dir_all(&root);
    let frontier = VecDeque::from([
        test_branch(1, BranchPolicyLane::default()),
        test_branch(
            2,
            BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
        ),
    ]);
    record_frontier(&path, 10, &frontier).expect("record current state");

    let mut value: Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("read current state"),
    )
    .expect("parse current state");
    for observation in value["observations"]
        .as_array_mut()
        .expect("observations")
    {
        observation["snapshot"]
            .as_object_mut()
            .expect("observation snapshot")
            .remove("search_comparability");
    }
    for snapshot in value["evaluation"]["snapshots"]
        .as_array_mut()
        .expect("evaluation snapshots")
    {
        snapshot
            .as_object_mut()
            .expect("evaluation snapshot")
            .remove("search_comparability");
    }
    for comparison in value["evaluation"]["comparisons"]
        .as_array_mut()
        .expect("evaluation comparisons")
    {
        comparison
            .as_object_mut()
            .expect("comparison")
            .remove("eligibility");
    }
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&value).expect("serialize legacy state"),
    )
    .expect("write legacy state");

    let restored = read_state(&path).expect("load legacy state");

    assert!(restored.evaluation.snapshots.iter().all(|snapshot| {
        snapshot.search_comparability.status
            == TrajectorySearchComparabilityStatus::InsufficientEvidence
    }));
    assert_eq!(
        restored.evaluation.comparisons[0].eligibility,
        TrajectoryPairEligibility::ExcludedInsufficientEvidence
    );
}
```

- [ ] **Step 2: Write the current capsule visibility regression test**

Add these imports in `run_capsule_format.rs`'s test module:

```rust
use sts_simulator::ai::strategy::trajectory_comparison::{
    TrajectoryPairEligibility, TrajectorySearchComparabilityStatus,
};
```

Then add:

```rust
#[test]
fn capsule_exposes_search_comparability_and_pair_eligibility() {
    let mut baseline = sample_branch();
    baseline.combat_search_history = vec![CombatSearchTraceSummary {
        source: "primary".to_string(),
        coverage_status: "NodeBudgetLimited".to_string(),
        node_budget_hit: true,
        ..CombatSearchTraceSummary::default()
    }];
    let mut challenger = challenger_sample_branch(1);
    challenger.combat_search_history = baseline.combat_search_history.clone();
    let trajectory_evaluation = evaluation(vec![baseline.clone(), challenger]);
    let summary = branch_summary_value(
        Path::new("target/test-capsule"),
        sample_args(),
        1,
        &baseline,
        &Value::Null,
        &json!([]),
        &trajectory_evaluation,
        "gap",
        None,
        None,
    );

    assert_eq!(
        summary["trajectory_snapshot"]["search_comparability"]["status"],
        json!(TrajectorySearchComparabilityStatus::Comparable)
    );
    assert_eq!(
        summary["trajectory_evaluation"]["comparisons"][0]["eligibility"],
        json!(TrajectoryPairEligibility::Comparable)
    );
}
```

- [ ] **Step 3: Run artifact tests and commit**

Run:

```powershell
cargo test --lib legacy_trajectory_state_loads_and_refreshes_fail_closed -- --nocapture
cargo test --lib capsule_exposes_search_comparability_and_pair_eligibility -- --nocapture
cargo test --lib trajectory_evidence_store::tests -- --nocapture
cargo test --lib run_capsule_format::tests -- --nocapture
git diff --check
git add src/runtime/branch/owner_audit/trajectory_evidence_store.rs src/runtime/branch/owner_audit/run_capsule_format.rs
git commit -m "test: lock search comparability artifacts"
```

Expected: current artifacts expose snake-case status/eligibility values; legacy artifacts load but regenerate excluded comparisons.

---

### Task 5: Verify the Delivery and Run the Seed006 Relic Fork

> Execution correction: the first exact-cutpoint run proved that Boss Relic
> choices were not assigned to challenger lanes and that shared-prefix search
> evidence polluted suffix eligibility. Before resuming the long fork, add the
> three-relic lane expansion and a persisted comparison horizon. Preserve and
> report full-history comparability separately.

**Files:**

- Verify: all maintained Rust sources and tests
- Create runtime evidence: `artifacts/runs/seed006-comparable-mainline-20260714/`
- Create runtime evidence: `artifacts/runs/seed006-comparable-boss-relic-fork-20260714/`

**Interfaces:**

- Consumes: Tasks 1-4, exact boss-relic cutpoint persistence, seed `20260713006`, fixed search node budgets.
- Produces: completion evidence and one resumable Black Blood / Coffee Dripper / Philosopher's Stone comparison that is rankable only when every relevant pair is eligible.

- [x] **Step 1: Run repository completion verification**

Run from a clean status after Task 4's commit:

```powershell
cargo fmt --all -- --check
git diff --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: formatting succeeds, no whitespace errors are reported, all library tests pass, and all architecture boundary tests pass.

- [x] **Step 2: Run the bounded seed006 prefix once and retain its capsule**

Use the historical mainline budget that reached this branch, with node limits fixed and the wall acting only as a one-hour outer safety stop:

```powershell
cargo run --release --bin branch_tiny -- --seed 20260713006 --generations 29 --max-branches 1 --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 2000000 --rescue-search-ms 20000 --boss-search-nodes 2000000 --boss-search-ms 20000 --wall-ms 3600000 --run-capsule artifacts/runs/seed006-comparable-mainline-20260714
```

Expected: the run writes `artifacts/runs/seed006-comparable-mainline-20260714/cutpoints/a2f32_boss_relic.frontier.json` and its matching manifest. If it stops earlier, retain the capsule and report the first real blocker; do not rerun the prefix speculatively.

- [x] **Step 3: Verify exact cutpoint identity before forking**

Run:

```powershell
$root = 'artifacts/runs/seed006-comparable-mainline-20260714/cutpoints'
$manifest = Get-Content -Raw "$root/a2f32_boss_relic.manifest.json" | ConvertFrom-Json
$checkpoint = Get-Content -Raw "$root/a2f32_boss_relic.frontier.json" | ConvertFrom-Json
$branch = $checkpoint.frontier[0]
$run = $branch.session[1]
$extras = $run[11]
if ($manifest.schema -ne 'branch_tiny_run_cutpoint_v1') { throw 'wrong cutpoint schema' }
if ($manifest.artifact_trust -ne 'exact_run_control_checkpoint_v1') { throw 'wrong cutpoint trust' }
if ($manifest.act -ne 2 -or $manifest.floor -ne 32 -or $manifest.boundary -ne 'Boss Relic') { throw 'wrong cutpoint boundary' }
if ($manifest.candidate_count -ne 4) { throw 'wrong boss relic candidate count' }
if ($run[4] -ne 26 -or $run[5] -ne 106 -or $run[6] -ne 167) { throw 'unexpected HP or gold' }
if ($extras.master_deck.Count -ne 15) { throw 'unexpected deck size' }
$branch.session[0] | ConvertTo-Json -Depth 12
$extras.relics | ConvertTo-Json -Depth 12
```

Expected: the assertions pass; the current exact cutpoint is 26/106 HP with 167 gold, and the printed boss-relic selection is ordered Black Blood, Coffee Dripper, Philosopher's Stone (plus the normal skip candidate). The older 13/101 artifact remains historical evidence but is not substituted for this checkpoint. The retained relic/RNG payload matches the exact session fingerprint in the manifest. The subsequent resume command revalidates all fingerprints before executing any choice.

- [ ] **Step 4: Resume the same cutpoint into three retained relic branches**

Run:

```powershell
cargo run --release --bin branch_tiny -- --resume-frontier artifacts/runs/seed006-comparable-mainline-20260714/cutpoints/a2f32_boss_relic.frontier.json --objective exhaust-frontier --generations 24 --max-branches 3 --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 2000000 --rescue-search-ms 20000 --boss-search-nodes 2000000 --boss-search-ms 20000 --wall-ms 3600000 --run-capsule artifacts/runs/seed006-comparable-boss-relic-fork-20260714
```

Expected: resume validation succeeds before expansion, and the capsule retains Black Blood, Coffee Dripper, and Philosopher's Stone as baseline plus two challenger lanes. Skip remains visible in candidate evidence but is not a fourth experimental arm. A wall safety stop is recorded rather than hidden.

- [ ] **Step 5: Enforce eligibility before interpreting the relic result**

Run:

```powershell
$trajectory = Get-Content -Raw 'artifacts/runs/seed006-comparable-boss-relic-fork-20260714/trajectory_state.json' | ConvertFrom-Json
$trajectory.evaluation.snapshots | Select-Object lane,terminal,@{n='suffix_status';e={$_.search_comparability.status}},@{n='full_status';e={$_.full_search_comparability.status}},@{n='suffix_attempts';e={$_.search_comparability.total_attempts}},@{n='suffix_wall';e={$_.search_comparability.wall_limited_attempts}},@{n='suffix_insufficient';e={$_.search_comparability.insufficient_attempts}} | Format-Table -AutoSize
$trajectory.evaluation.comparisons | Select-Object baseline_lane,challenger_lane,eligibility,verdict | Format-Table -AutoSize
$excluded = @($trajectory.evaluation.comparisons | Where-Object { $_.eligibility -ne 'comparable' })
if ($excluded.Count -gt 0) { Write-Warning 'Boss relic counterfactual is inconclusive; keep the cutpoint and resume the affected work instead of ranking arms.' }
```

Expected: rank the three relic arms only when all relevant comparisons print `eligibility = comparable`. If any row is excluded, report the experiment as inconclusive and preserve both capsules for continuation.

- [ ] **Step 6: Record the final clean state**

```powershell
git status --short --branch
git log -5 --oneline
```

Expected: the feature branch has no uncommitted source changes. Runtime capsules may be ignored durable evidence under `artifacts/runs`; do not add them to the source commit unless repository policy explicitly tracks that artifact family.
