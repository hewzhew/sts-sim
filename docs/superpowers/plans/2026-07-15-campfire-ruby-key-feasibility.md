# Campfire Ruby Key Feasibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Promote the Ruby Key sub-obligation from `Unsupported` to typed, route-bounded evidence for every Campfire candidate without ranking candidates or entering production run control.

**Architecture:** Keep public evaluation assembly in `campfire_evaluation.rs` and put Ruby Key reasoning in a focused nested module. The module consumes the already-shared evaluation context and authoritative candidate projection, returns a typed obligation plus field evidence, and never reads hidden RNG or future queues.

**Tech Stack:** Rust, serde, existing `CampfireProjection`, existing `RouteWindowFacts`, Cargo unit tests.

## Global Constraints

- Work in the stable checkout; do not create a worktree.
- Use only the explicit `CampfireRunGoal` from the evaluation specification.
- Treat only complete Act 3 route evidence as a proven missed Ruby Key deadline.
- Earlier-act bosses, unavailable maps, partial route coverage, and unmodeled chance mobility remain unresolved.
- Do not rank candidates, call `campfire_policy_v1`, or add a production reader.
- Run focused tests first, then the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Add Typed Immediate Ruby Key Obligations

**Files:**

- Create: `src/eval/campfire_evaluation/run_feasibility.rs`
- Modify: `src/eval/campfire_evaluation.rs`

**Interfaces:**

- Consumes: `CampfireRunGoal`, `CampfireCandidate`, `CampfireProjection`, and root key state.
- Produces: `CampfireRunFeasibility`, `CampfireRubyKeyObligation`, and `assess_run_feasibility(...) -> CampfireRunFeasibilityAssessment`.

- [ ] **Step 1: Write failing tests for immediate obligations**

Add tests proving:

```rust
assert_eq!(
    act3_candidate.run_feasibility.ruby_key,
    CampfireRubyKeyObligation::NotRequired
);
assert_eq!(
    recall.run_feasibility.ruby_key,
    CampfireRubyKeyObligation::SatisfiedByCandidate
);
assert_eq!(
    already_held.run_feasibility.ruby_key,
    CampfireRubyKeyObligation::AlreadySatisfied
);
```

Also assert that the projected Recall state actually holds `keys[0]`, so the fact is grounded in the authoritative transition.

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test -p sts_simulator campfire_evaluation --lib
```

Expected: compilation fails because `run_feasibility` and the Ruby Key obligation types do not exist.

- [ ] **Step 3: Implement the immediate obligation types and assembly**

Define:

```rust
pub struct CampfireRunFeasibility {
    pub declared_goal: CampfireRunGoal,
    pub ruby_key: CampfireRubyKeyObligation,
    pub sapphire_key_held: bool,
    pub emerald_key_held: bool,
}

pub enum CampfireRubyKeyObligation {
    NotRequired,
    AlreadySatisfied,
    SatisfiedByCandidate,
    DeferrableOnEveryVisiblePath,
    DeferrableOnSomeVisiblePath,
    ViolatedAtVisibleAct3BossDeadline,
    UnresolvedBeyondVisibleWindow,
    UnresolvedByChanceOutcome,
}
```

Add `run_feasibility` to `CampfireCandidateEvaluation`. For `Act3Victory`, emit exact `NotRequired`. For Heart goals, use projected key state to distinguish `AlreadySatisfied` from `SatisfiedByCandidate`; keep other Heart-key coverage explicit rather than claiming overall victory feasibility.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the same focused command. Expected: the immediate-obligation tests pass and existing Campfire evaluation tests remain green.

- [ ] **Step 5: Commit the immediate obligation boundary**

```powershell
git add src/eval/campfire_evaluation.rs src/eval/campfire_evaluation/run_feasibility.rs
git commit -m "feat: model campfire ruby key obligations"
```

---

### Task 2: Add Visible Route Deadline Evidence

**Files:**

- Modify: `src/eval/campfire_evaluation/run_feasibility.rs`
- Modify: `src/eval/campfire_evaluation.rs`

**Interfaces:**

- Consumes: `RouteWindowPredicate::PresentInWindow`, `RouteWindowPredicate::OccursBefore`, and `RouteWindowModality` from the shared context.
- Produces: partial or exact `CampfireFieldEvidence` for `RunFeasibility` with machine-readable limitations.

- [ ] **Step 1: Write failing route-boundary tests**

Build real map graphs and assert:

```rust
assert_eq!(
    future_fire.run_feasibility.ruby_key,
    CampfireRubyKeyObligation::DeferrableOnEveryVisiblePath
);
assert_eq!(
    last_act3_fire.run_feasibility.ruby_key,
    CampfireRubyKeyObligation::ViolatedAtVisibleAct3BossDeadline
);
assert_eq!(
    missing_map.run_feasibility.ruby_key,
    CampfireRubyKeyObligation::UnresolvedBeyondVisibleWindow
);
```

For a Dig candidate at an otherwise proven deadline, assert `UnresolvedByChanceOutcome` rather than a false exact violation because the relic outcome may change route mobility.

- [ ] **Step 2: Run the focused test and verify RED**

Run the Campfire evaluation filter. Expected: assertions fail because all non-Recall Heart candidates are still unresolved.

- [ ] **Step 3: Implement route-bounded classification**

Use the shared `RouteWindowFacts` in this order:

1. exact already-held/projected Recall state;
2. on Act 3, `Boss occurs before Campfire == Must` proves the deadline, except for unmodeled Dig mobility;
3. `Campfire present == Must` means deferrable on every covered path;
4. `Campfire present == Can` means deferrable on some covered path;
5. otherwise remain unresolved.

Emit `Exact` for a proven violation or a fully satisfied key set. Emit `Partial` for deferral, incomplete other Heart keys, or unresolved windows. Add `OtherHeartKeysNotEvaluated`, `FutureRecallDecisionNotEvaluated`, `VisibleDeadlineNotProven`, and `ChanceOutcomeCouldChangeRouteAccess` limitations only where they apply.

- [ ] **Step 4: Run focused Campfire tests and verify GREEN**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator campfire --lib
```

Expected: all Campfire evaluation, projection, engine, and existing policy tests pass.

- [ ] **Step 5: Commit route-bounded evidence**

```powershell
git add src/eval/campfire_evaluation.rs src/eval/campfire_evaluation/run_feasibility.rs
git commit -m "feat: bound ruby key deadlines by visible routes"
```

---

### Task 3: Verify The Slice

**Files:** verification only.

**Interfaces:** Verifies the offline evaluation layer remains hidden-information safe and does not cross runtime architecture boundaries.

- [ ] **Step 1: Run completion suites**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries
```

Expected: formatting succeeds and both test suites report zero failures.

- [ ] **Step 2: Confirm clean local history**

```powershell
git status --short --branch
git log -6 --oneline
```

Expected: no uncommitted changes and the Ruby Key commits appear above the evaluation-batch commits.

