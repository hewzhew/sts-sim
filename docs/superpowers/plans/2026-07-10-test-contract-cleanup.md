# Test Contract Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove tests and assertions that protect retired branch-first shop behavior or obsolete human-facing output while preserving current typed behavior coverage.

**Architecture:** This is a test-only cleanup. Delete four complete shop-strategy tests, remove three obsolete assertions from otherwise useful tests, and leave production policy, schemas, and run-control behavior unchanged.

**Tech Stack:** Rust 2021, Cargo test harness, Python `unittest`, Git.

## Global Constraints

- Do not modify production Rust code.
- Do not modify shop policy, acquisition gates, branch scheduling, combat-search schemas, or run-control behavior.
- Do not replace removed prose/default assertions with new prose/default assertions.
- Preserve the two known failures for combat-search schema validation and missing campfire noncombat records so they remain separately visible.

---

### Task 1: Remove Retired Branch-First Shop Tests

**Files:**
- Modify: `src/ai/shop_policy_v1/tests.rs`
- Modify: `src/eval/branch_experiment_boundary/tests.rs`
- Modify: `src/eval/branch_experiment/tests.rs`

**Interfaces:**
- Consumes: current acquisition gates and shop compiler behavior.
- Produces: no production interface; removes obsolete test contracts only.

- [ ] **Step 1: Delete the four complete test items**

Delete the full `#[test] fn ... { ... }` items with these exact names:

```text
compiled_shop_branch_topk_preserves_distinct_card_purchase_lanes
compiled_shop_branch_frontier_can_admit_non_rollout_thesis_candidate
current_boundary_includes_three_purchase_combo_for_high_gold_shop_pressure
branch_experiment_executes_shop_combo_purchase_branch
```

- [ ] **Step 2: Verify the retired contracts are absent**

Run:

```powershell
rg -n "compiled_shop_branch_topk_preserves_distinct_card_purchase_lanes|compiled_shop_branch_frontier_can_admit_non_rollout_thesis_candidate|current_boundary_includes_three_purchase_combo_for_high_gold_shop_pressure|branch_experiment_executes_shop_combo_purchase_branch" src
```

Expected: exit code 1 with no matches.

- [ ] **Step 3: Check formatting**

Run:

```powershell
cargo fmt --check
```

Expected: exit code 0.

### Task 2: Remove Three Obsolete Assertions

**Files:**
- Modify: `src/eval/run_control/session/tests.rs`
- Modify: `src/eval/run_control/view_model/candidates.rs`

**Interfaces:**
- Consumes: existing behavioral test setup and typed candidate identity.
- Produces: retained tests that no longer pin obsolete render text, command text, or old defaults.

- [ ] **Step 1: Remove the old combat turn-plan default assertion**

From `run_control_search_combat_applies_complete_winning_trajectory`, delete only:

```rust
assert!(outcome.message.contains("turn_plan_policy=diagnostic_only"));
```

- [ ] **Step 2: Remove the old deck-mutation render-title assertion**

From `run_control_details_include_deck_mutation_compiler_groups`, delete only:

```rust
assert!(rendered.contains("DeckMutationCompilerV1"));
```

- [ ] **Step 3: Remove the old unopened Singing Bowl command assertion**

From `singing_bowl_unopened_card_reward_candidate_is_visible_as_command`, delete only:

```rust
assert_eq!(bowl.action.command_hint(), "bowl");
```

- [ ] **Step 4: Run the three retained focused tests**

Run each command:

```powershell
cargo test --lib eval::run_control::session::tests::run_control_search_combat_applies_complete_winning_trajectory -- --exact
cargo test --lib eval::run_control::session::tests::run_control_details_include_deck_mutation_compiler_groups -- --exact
cargo test --lib eval::run_control::view_model::candidates::tests::singing_bowl_unopened_card_reward_candidate_is_visible_as_command -- --exact
```

Expected: each command reports `1 passed; 0 failed`.

### Task 3: Verify The Narrow Cleanup

**Files:**
- Verify only; no additional modifications.

**Interfaces:**
- Consumes: the test-only edits from Tasks 1 and 2.
- Produces: fresh evidence that the intended seven retired contracts are gone and unrelated targets remain healthy.

- [ ] **Step 1: Run the library suite**

Run:

```powershell
cargo test --lib
```

Expected: exactly two known failures remain:

```text
eval::run_control::session::tests::run_control_auto_run_uses_recovery_route_package_to_rest_at_low_hp_campfire
eval::run_control::session::tests::run_control_search_combat_can_save_search_evidence_for_capture_case
```

- [ ] **Step 2: Run unaffected target suites**

Run:

```powershell
cargo test --bins
cargo test --test architecture_runtime_boundaries
python -m unittest discover -s tests -p 'test_*.py'
```

Expected: all commands exit 0.

- [ ] **Step 3: Check the final diff**

Run:

```powershell
git diff --check
git status --short
```

Expected: no whitespace errors; only the approved test files and plan document are changed.

- [ ] **Step 4: Commit the test cleanup**

Run:

```powershell
git add src/ai/shop_policy_v1/tests.rs src/eval/branch_experiment_boundary/tests.rs src/eval/branch_experiment/tests.rs src/eval/run_control/session/tests.rs src/eval/run_control/view_model/candidates.rs docs/superpowers/plans/2026-07-10-test-contract-cleanup.md
git commit -m "Remove retired test contracts"
```

Expected: one commit containing only the approved cleanup and implementation plan.

## Self-Review

- Spec coverage: all four retired shop tests and all three obsolete assertions are named explicitly.
- Scope: production behavior, schemas, and run-control implementation are excluded.
- Verification: focused retained tests, full library classification, binary tests, integration tests, Python tests, formatting, and diff checks are covered.
- Type consistency: no interfaces or production types are changed.
