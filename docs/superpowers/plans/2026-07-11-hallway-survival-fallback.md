# Hallway Survival Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the non-boss HP reserve as a quality target while allowing a bounded clean complete win to keep a pressured hallway run alive.

**Architecture:** Append one typed fallback lane to the existing pressured-hallway portfolio. It reuses the hallway quality search profile and curse acceptance rule, but overrides the HP-loss gate to unlimited only for that final lane.

**Tech Stack:** Rust, Cargo library tests, existing owner-audit combat portfolio and saved combat-case review CLI.

## Global Constraints

- Work on `fix/hallway-survival-fallback` in the stable checkout; do not create a worktree.
- Do not run `cargo clean`.
- Do not change elite or boss portfolio behavior.
- Do not weaken the strict lane's quarter-max-HP reserve.
- Do not lock a particular A2F26 card sequence in a unit test.

---

### Task 1: Add the final pressured-hallway lane

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lanes.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_spec.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs`

**Interfaces:**
- Consumes: `CombatSearchPortfolioContext::nonboss_potion_rescue_signal`, `quality_profile`, and `CleanAcceptedLineNoNewCurse`.
- Produces: `CombatSearchLaneKind::HallwaySurvivalFallback` with label `hallway_survival_fallback`.

- [ ] **Step 1: Write a failing portfolio-order test**

Add a test-only `lane_labels` helper and require these labels:

```rust
assert_eq!(
    plan.lane_labels(),
    vec![
        "primary_immediate_escalation",
        "hallway_quality_potion_rescue",
        "hallway_survival_fallback",
    ]
);
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_plan::tests::pressured_hallway_plan_adds_explicit_quality_potion_rescue
```

Expected: assertion failure because the current plan has only two labels.

- [ ] **Step 3: Add the typed lane with the existing strict HP gate**

Add the enum variant, map it to:

```rust
dirty_rejecting_spec("hallway_survival_fallback")
```

Append it after `HallwayQualityPotionRescue` only when the rescue signal is
present. Give it the same bounded `quality_profile` used by the hallway quality
lane. Leave the shared `owner_audit_hp_loss_limit` assignment unchanged during
this step.

- [ ] **Step 4: Run the test and verify GREEN**

Run the same focused command. Expected: one test passes.

### Task 2: Separate the survival HP gate from the quality gate

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`

**Interfaces:**
- Consumes: `CombatSearchLane::kind()` and `RunControlHpLossLimit`.
- Produces: lane-specific `max_hp_loss` where only `HallwaySurvivalFallback` is unlimited.

- [ ] **Step 1: Write a failing lane-options test**

For a 20/74 HP hallway session, assert that the quality lane has `Limit(2)`,
while the fallback has `Unlimited`, a semantic potion policy, and a two-potion
cap.

- [ ] **Step 2: Run the test and verify RED**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_lane_options::tests::hallway_survival_fallback_relaxes_only_hp_reserve
```

Expected: fallback reports `Limit(2)` instead of `Unlimited`.

- [ ] **Step 3: Add the minimal typed override**

```rust
options.search.max_hp_loss = Some(match lane.kind() {
    CombatSearchLaneKind::HallwaySurvivalFallback => RunControlHpLossLimit::Unlimited,
    _ => owner_audit_hp_loss_limit(session),
});
```

- [ ] **Step 4: Run the test and verify GREEN**

Run the same focused command. Expected: one test passes.

### Task 3: Verify behavior and repository boundaries

**Files:**
- No production changes expected.
- Read: `target/route-reliability-seed-20260711004/combat_cases/seed20260711004_g20_b0020_a2f26_shelledparasite_fungibeast.json`

**Interfaces:**
- Consumes: the saved A2F26 case and repository test suites.
- Produces: fresh verification evidence for the search case and owner boundary.

- [ ] **Step 1: Run the focused owner tests together**

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_
```

- [ ] **Step 2: Review the saved A2F26 case**

```powershell
cargo run --quiet --bin combat_case_review -- --case "target/route-reliability-seed-20260711004/combat_cases/seed20260711004_g20_b0020_a2f26_shelledparasite_fungibeast.json" --ladder --compact
```

Expected: at least one complete winning candidate remains discoverable.

- [ ] **Step 3: Run completion suites**

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: all library and architecture tests pass.

- [ ] **Step 4: Inspect the diff and commit locally**

```powershell
git diff --check
git status --short
git add docs/superpowers/specs/2026-07-11-hallway-survival-fallback-design.md docs/superpowers/plans/2026-07-11-hallway-survival-fallback.md src/runtime/branch/owner_audit/combat_search_lanes.rs src/runtime/branch/owner_audit/combat_search_lane_spec.rs src/runtime/branch/owner_audit/combat_search_lane_options.rs src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs
git commit -m "fix: add hallway survival fallback"
```
