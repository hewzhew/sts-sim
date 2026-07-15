# Campfire Lift Growth Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose Girya's authoritative combat-start Strength mechanic and add exact-projection, typed Lift growth evidence to the Campfire evaluation batch.

**Architecture:** Girya remains the sole owner of the mapping from relic counter to combat-start Strength. Campfire evaluation verifies that the exact Lift projection changes the unique Girya counter by one, copies the mechanic through Girya's pure helper, and marks aggregate growth partial because downstream outcome value remains unmodeled.

**Tech Stack:** Rust, existing relic mechanics, Campfire projection/evaluation modules, Cargo unit tests.

## Global Constraints

- Work in the stable checkout; do not create a worktree.
- Do not duplicate `counter.max(0)` or Girya Strength mechanics inside `eval`.
- Do not add a Lift score, rank candidates, add a production reader, or change existing Campfire policy behavior.
- Require exactly one Girya in both root and exact projection; missing, duplicate, or non-`+1` counter transitions are typed errors.
- Keep Dig, Rest, Recall, and other non-Smith/non-Toke/non-Lift growth `Unsupported`.
- Keep aggregate Lift growth `Partial` until downstream survival and threat outcomes are represented.
- Run focused tests first, then the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Expose Girya's Combat-Start Strength Fact

**Files:**

- Modify: `src/content/relics/girya.rs`
- Modify: `src/content/relics/tests.rs`

**Interfaces:**

- Produces: `Girya::battle_start_strength(counter: i32) -> i32`.
- Preserves: `Girya::at_battle_start(counter)` action ordering and behavior.

- [ ] **Step 1: Write the failing mechanic test**

Extend `girya_lift_counter_and_battle_start_strength_match_java` with:

```rust
assert_eq!(girya::Girya::battle_start_strength(-1), 0);
assert_eq!(girya::Girya::battle_start_strength(0), 0);
assert_eq!(girya::Girya::battle_start_strength(1), 1);
assert_eq!(girya::Girya::battle_start_strength(3), 3);
```

- [ ] **Step 2: Run the focused test and verify RED**

```powershell
cargo test -p sts_simulator girya_lift_counter_and_battle_start_strength_match_java --lib
```

Expected: compilation fails because `battle_start_strength` does not exist.

- [ ] **Step 3: Implement and reuse the pure mechanic helper**

Add:

```rust
impl Girya {
    pub fn battle_start_strength(counter: i32) -> i32 {
        counter.max(0)
    }
}
```

Change `at_battle_start` to call `Self::battle_start_strength(counter)` instead of duplicating the formula. Preserve the existing `ApplyPower` action and `AddTo::Top` order.

- [ ] **Step 4: Run the focused test and verify GREEN**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator girya_lift_counter_and_battle_start_strength_match_java --lib
```

Expected: the mechanic test passes with the unchanged action assertion.

- [ ] **Step 5: Commit the mechanic boundary**

```powershell
git add src/content/relics/girya.rs src/content/relics/tests.rs
git commit -m "refactor: expose Girya strength mechanic"
```

---

### Task 2: Add Typed Lift Growth Evidence

**Files:**

- Modify: `src/eval/campfire_evaluation/growth.rs`
- Modify: `src/eval/campfire_evaluation.rs`

**Interfaces:**

- Consumes: `CampfireCandidate::Lift`, `CampfireProjection::Exact`, and `Girya::battle_start_strength`.
- Produces: public `CampfireLiftGrowth` and `CampfireGrowth { smith, toke, lift }`.

- [ ] **Step 1: Write the failing Lift growth test**

Using the existing candidate fixture with Girya counter zero, assert:

```rust
let lift = lift.growth.lift.as_ref().unwrap();
assert_eq!(lift.girya_counter_before, 0);
assert_eq!(lift.girya_counter_after, 1);
assert_eq!(lift.combat_start_strength_before, 0);
assert_eq!(lift.combat_start_strength_after, 1);
assert_eq!(lift.combat_start_strength_delta, 1);
assert_eq!(
    candidate.evidence_for(CampfireProspectField::GrowthDistribution)
        .unwrap()
        .status,
    CampfireEvidenceStatus::Partial
);
```

Also assert the provenance is `EngineTransitionAndRelicMechanics` and the limitation remains `DownstreamGrowthDistributionNotEvaluated`.

- [ ] **Step 2: Run the evaluation filter and verify RED**

```powershell
cargo test -p sts_simulator campfire_evaluation --lib
```

Expected: compilation fails because the Lift growth contract and provenance do not exist.

- [ ] **Step 3: Implement exact Lift assessment**

Add:

```rust
pub struct CampfireLiftGrowth {
    pub girya_counter_before: i32,
    pub girya_counter_after: i32,
    pub combat_start_strength_before: i32,
    pub combat_start_strength_after: i32,
    pub combat_start_strength_delta: i32,
}
```

Add typed errors:

```rust
MissingGirya,
AmbiguousGirya,
GiryaCounterMismatch { before: i32, after: i32 },
```

For `CampfireCandidate::Lift`, require `CampfireProjection::Exact`, find exactly one Girya in root and projected relics, and require `after == before + 1`. Build Strength fields only through `Girya::battle_start_strength`. Emit `Partial` evidence with `EngineTransitionAndRelicMechanics` provenance and `DownstreamGrowthDistributionNotEvaluated` limitation. Set `lift: None` in Smith and Toke records; unsupported families use the derived default.

- [ ] **Step 4: Run focused Campfire tests and verify GREEN**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator campfire --lib
```

Expected: all Campfire evaluation, projection, engine, and legacy policy tests pass.

- [ ] **Step 5: Commit Lift evidence**

```powershell
git add src/eval/campfire_evaluation.rs src/eval/campfire_evaluation/growth.rs
git commit -m "feat: add typed campfire lift growth evidence"
```

---

### Task 3: Verify The Slice

**Files:** verification only.

**Interfaces:** Verifies Girya mechanics and typed Smith/Toke/Lift evidence under the repository completion gate.

- [ ] **Step 1: Run completion suites**

```powershell
cargo fmt --all -- --check
cargo test -p sts_simulator --lib
cargo test -p sts_simulator --test architecture_runtime_boundaries
```

Expected: formatting succeeds and both suites report zero failures.

- [ ] **Step 2: Confirm clean history**

```powershell
git status --short --branch
git log -7 --oneline
```

Expected: no uncommitted changes and both implementation commits appear above this plan.
