# Combat Attrition Accounting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a factual accepted-combat HP ledger that separates observed combat drawdown, terminal rebound, and persistent net loss without changing gameplay policy.

**Architecture:** A new pure owner-audit module derives `AcceptedCombatAttritionV1` from the captured start, selected terminal summary, and executed trajectory. The existing accepted-high-loss diagnostic owns retention and v2 evidence serialization, while the capsule store discovers each evidence file's declared schema so v1 and v2 artifacts coexist.

**Tech Stack:** Rust 2021, Serde/serde_json, existing run-control trajectory types, Cargo unit and architecture tests.

## Global Constraints

- Do not change combat search ordering, budgets, acceptance, repair, or potion policy.
- Do not change reward, shop, route, campfire, deck-deficit, or owner policy.
- Do not label a combat avoidable, unavoidable, or defense-deficient.
- Treat observed drawdown as a lower bound whenever any action HP snapshot is absent.
- Preserve deserialization of checkpoints created before the attrition field existed.
- Discover both `accepted_high_loss_combat_evidence_v1` and `accepted_high_loss_combat_evidence_v2` sidecars.
- Do not rerun a full seed; inspect the already preserved Snake Plant artifact after implementation.

---

### Task 1: Derive the typed attrition ledger

**Files:**
- Create: `src/runtime/branch/owner_audit/accepted_combat_attrition.rs`
- Modify: `src/runtime/branch/owner_audit.rs`

**Interfaces:**
- Consumes: `CombatSearchTerminalLineSummary` and `CombatAutomationTrajectoryRecordV1`.
- Produces: `AcceptedCombatAttritionV1` and `accepted_combat_attrition_v1(start_hp, selected, trajectory)` for the diagnostic builder.

- [ ] **Step 1: Register the module and write failing table-driven tests**

Add to `src/runtime/branch/owner_audit.rs` beside the accepted-high-loss module:

```rust
#[path = "owner_audit/accepted_combat_attrition.rs"]
mod accepted_combat_attrition;
```

Create `accepted_combat_attrition.rs` with tests that construct step snapshots and assert the exact ledger:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attrition_separates_observed_drawdown_rebound_and_persistent_loss() {
        let selected = terminal_win(20, 24);
        let trajectory = trajectory(&[Some(44), Some(23), Some(8), None]);

        assert_eq!(
            accepted_combat_attrition_v1(44, &selected, &trajectory),
            AcceptedCombatAttritionV1 {
                start_hp: 44,
                lowest_observed_hp: 8,
                observed_combat_drawdown: 36,
                terminal_hp: 20,
                terminal_rebound_from_observed_low: 12,
                persistent_net_hp_loss: 24,
                observation_complete: false,
            }
        );
    }

    #[test]
    fn attrition_without_healing_has_equal_observed_and_net_loss() {
        let selected = terminal_win(61, 13);
        let trajectory = trajectory(&[Some(70), Some(61)]);
        let attrition = accepted_combat_attrition_v1(74, &selected, &trajectory);

        assert_eq!(attrition.observed_combat_drawdown, 13);
        assert_eq!(attrition.terminal_rebound_from_observed_low, 0);
        assert_eq!(attrition.persistent_net_hp_loss, 13);
        assert!(attrition.observation_complete);
    }
}
```

Test helpers must build real `CombatAutomationStepStateV1` values with empty monster lists and real `CombatAutomationActionV1` entries; do not introduce test-only production branches.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib runtime::branch::owner_audit::accepted_combat_attrition::tests -- --nocapture
```

Expected: compilation fails because `AcceptedCombatAttritionV1` and `accepted_combat_attrition_v1` are not defined.

- [ ] **Step 3: Implement the pure ledger**

Add the production type and function above the tests:

```rust
use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    CombatAutomationTrajectoryRecordV1, CombatSearchTerminalLineSummary,
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedCombatAttritionV1 {
    pub(super) start_hp: i32,
    pub(super) lowest_observed_hp: i32,
    pub(super) observed_combat_drawdown: i32,
    pub(super) terminal_hp: i32,
    pub(super) terminal_rebound_from_observed_low: i32,
    pub(super) persistent_net_hp_loss: i32,
    pub(super) observation_complete: bool,
}

pub(super) fn accepted_combat_attrition_v1(
    start_hp: i32,
    selected: &CombatSearchTerminalLineSummary,
    trajectory: &CombatAutomationTrajectoryRecordV1,
) -> AcceptedCombatAttritionV1 {
    let lowest_observed_hp = trajectory
        .actions
        .iter()
        .filter_map(|action| action.combat_after.as_ref().map(|state| state.player_hp))
        .fold(start_hp, i32::min);
    AcceptedCombatAttritionV1 {
        start_hp,
        lowest_observed_hp,
        observed_combat_drawdown: start_hp.saturating_sub(lowest_observed_hp).max(0),
        terminal_hp: selected.final_hp,
        terminal_rebound_from_observed_low: selected
            .final_hp
            .saturating_sub(lowest_observed_hp)
            .max(0),
        persistent_net_hp_loss: start_hp.saturating_sub(selected.final_hp).max(0),
        observation_complete: trajectory
            .actions
            .iter()
            .all(|action| action.combat_after.is_some()),
    }
}
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the Task 1 focused command again.

Expected: both attrition tests pass.

- [ ] **Step 5: Commit the pure ledger**

```powershell
git add -- src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/accepted_combat_attrition.rs
git commit -m "feat: derive accepted combat attrition"
```

---

### Task 2: Retain gross-pressure diagnostics and write v2 evidence

**Files:**
- Modify: `src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs`

**Interfaces:**
- Consumes: `accepted_combat_attrition_v1` and `AcceptedCombatAttritionV1` from Task 1.
- Produces: optional, backward-compatible `attrition` on `AcceptedHighLossDiagnosticDraft`; v2 sidecars and result projections containing the ledger.

- [ ] **Step 1: Write failing diagnostic and serialization tests**

Extend the existing test helpers so `capture()` can set start HP and `annotations()` can accept selected HP loss plus action snapshot HP values. Add assertions equivalent to:

```rust
#[test]
fn observed_drawdown_can_retain_recovered_combat() {
    let draft = accepted_high_loss_diagnostic(
        capture_with_hp(44),
        "primary",
        &annotations_with_hp(terminal_win(38, 6), &[Some(44), Some(20), None]),
        true,
        Some(26),
    )
    .expect("proven 24 HP drawdown should retain diagnostic");

    let attrition = draft.attrition.as_ref().unwrap();
    assert_eq!(attrition.observed_combat_drawdown, 24);
    assert_eq!(attrition.terminal_rebound_from_observed_low, 18);
    assert_eq!(attrition.persistent_net_hp_loss, 6);
}
```

Update `write_diagnostic_pair_emits_replayable_capture_and_evidence` to require:

```rust
assert_eq!(evidence["schema"], "accepted_high_loss_combat_evidence_v2");
assert_eq!(evidence["attrition"]["start_hp"], 44);
assert_eq!(evidence["attrition"]["lowest_observed_hp"], 8);
assert_eq!(written.attrition.as_ref().unwrap().persistent_net_hp_loss, 24);
```

Add a serde compatibility test that removes `attrition` from a serialized draft and successfully deserializes it with `attrition == None`.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib runtime::branch::owner_audit::accepted_high_loss_diagnostic::tests -- --nocapture
```

Expected: compilation fails because the draft and written result do not expose `attrition`, and the old three-input trigger cannot consider observed drawdown.

- [ ] **Step 3: Thread attrition through retention and output**

Import Task 1 and add optional fields for checkpoint compatibility:

```rust
use super::accepted_combat_attrition::{
    accepted_combat_attrition_v1, AcceptedCombatAttritionV1,
};

#[serde(default, skip_serializing_if = "Option::is_none")]
pub(super) attrition: Option<AcceptedCombatAttritionV1>,
```

Add this field to both `AcceptedHighLossDiagnosticDraft` and
`WrittenAcceptedHighLossDiagnostic`. Change the trigger to:

```rust
pub(super) fn high_loss_trigger(
    max_hp: i32,
    original_hp_loss: i32,
    selected_hp_loss: i32,
    observed_combat_drawdown: i32,
) -> bool {
    let max_hp = i64::from(max_hp.max(1));
    [original_hp_loss, selected_hp_loss, observed_combat_drawdown]
        .into_iter()
        .any(|loss| i64::from(loss.max(0)).saturating_mul(4) >= max_hp)
}
```

In `accepted_high_loss_diagnostic`, extract the selected trajectory before the trigger, derive attrition from `capture.summary.player_hp`, and pass its observed drawdown into the trigger. New drafts always store `Some(attrition)`.

In `write_diagnostic_pair`, derive a fallback ledger when a legacy checkpoint supplied `None`, serialize schema v2 with an `attrition` object, and return the same ledger in the written summary. Extend `WrittenAcceptedHighLossDiagnostic::value()` with `"attrition"`.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the Task 2 focused command again.

Expected: all accepted-high-loss diagnostic tests pass, including legacy draft deserialization and gross-pressure retention.

- [ ] **Step 5: Commit diagnostic integration**

```powershell
git add -- src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs
git commit -m "feat: account for accepted combat attrition"
```

---

### Task 3: Discover v1 and v2 capsule artifacts accurately

**Files:**
- Modify: `src/runtime/branch/owner_audit/capsule_artifact_store.rs`
- Modify: `src/runtime/branch/slice_result.rs`

**Interfaces:**
- Consumes: evidence sidecars whose top-level `schema` is v1 or v2.
- Produces: `ArtifactRef.schema` matching the file's declared schema; existing v1 references remain valid.

- [ ] **Step 1: Write failing schema-discovery tests**

Extract a pure helper in `capsule_artifact_store.rs` and first add tests for the intended interface:

```rust
#[test]
fn accepted_diagnostic_schema_reads_v1_and_v2_sidecars() {
    let root = std::env::temp_dir().join("accepted_diagnostic_schema_versions");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let v1 = root.join("old.evidence.json");
    let v2 = root.join("new.evidence.json");
    std::fs::write(&v1, r#"{"schema":"accepted_high_loss_combat_evidence_v1"}"#).unwrap();
    std::fs::write(&v2, r#"{"schema":"accepted_high_loss_combat_evidence_v2"}"#).unwrap();

    assert_eq!(accepted_combat_diagnostic_schema(&v1), "accepted_high_loss_combat_evidence_v1");
    assert_eq!(accepted_combat_diagnostic_schema(&v2), "accepted_high_loss_combat_evidence_v2");
    let _ = std::fs::remove_dir_all(root);
}
```

Update the slice-result artifact test to use v2 for a new sidecar while retaining a separate v1 ref assertion, proving the generic artifact collection accepts both schema strings.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib accepted_diagnostic_schema -- --nocapture
cargo test --lib accepted_combat_diagnostic_tests -- --nocapture
```

Expected: the first command fails because `accepted_combat_diagnostic_schema` is undefined.

- [ ] **Step 3: Implement schema-aware discovery**

Add:

```rust
fn accepted_combat_diagnostic_schema(path: &std::path::Path) -> String {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".capture.json"))
    {
        return "CombatCaptureV1".to_string();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| value.get("schema")?.as_str().map(str::to_string))
        .filter(|schema| {
            matches!(
                schema.as_str(),
                "accepted_high_loss_combat_evidence_v1"
                    | "accepted_high_loss_combat_evidence_v2"
            )
        })
        .unwrap_or_else(|| "accepted_high_loss_combat_evidence_v1".to_string())
}
```

Replace the hard-coded v1 branch in `record_accepted_combat_diagnostic_refs` with this helper. The v1 fallback preserves prior behavior for malformed or legacy untagged evidence instead of making artifact discovery fatal.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run both Task 3 commands again.

Expected: all schema-discovery and artifact-summary tests pass.

- [ ] **Step 5: Commit capsule compatibility**

```powershell
git add -- src/runtime/branch/owner_audit/capsule_artifact_store.rs src/runtime/branch/slice_result.rs
git commit -m "feat: discover combat attrition evidence versions"
```

---

### Task 4: Verify the integrated feature and inspect the preserved case

**Files:**
- Modify only if formatting requires it: files from Tasks 1-3.
- Inspect without modifying: `target/bounded-mainline-20260711004-high-loss-diagnostics/accepted_high_loss_combat/seed20260711004_g22_b0022_a2f21t0_snakeplant.evidence.json`

**Interfaces:**
- Consumes: all Task 1-3 commits and the preserved Snake Plant evidence.
- Produces: final verification evidence; no full-seed rerun and no policy mutation.

- [ ] **Step 1: Run formatting and focused owner-audit tests**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo fmt --all -- --check
cargo test --lib runtime::branch::owner_audit::accepted_combat_attrition::tests
cargo test --lib runtime::branch::owner_audit::accepted_high_loss_diagnostic::tests
cargo test --lib accepted_diagnostic_schema
```

Expected: formatting and all focused tests pass.

- [ ] **Step 2: Run the full library and architecture suites**

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: zero failures in both suites.

- [ ] **Step 3: Compute the expected preserved-case ledger without rerunning the seed**

Read the v1 evidence's `start_hp`, selected final HP, and action snapshots. Confirm the same production formula yields:

```text
start_hp=44
lowest_observed_hp=8
observed_combat_drawdown=36
terminal_hp=20
terminal_rebound_from_observed_low=12
persistent_net_hp_loss=24
observation_complete=false
```

Do not rewrite the v1 artifact; compatibility requires old evidence to remain unchanged.

- [ ] **Step 4: Check diff scope and commit formatting only if necessary**

```powershell
git status --short
git diff --check
git diff --stat master...HEAD
```

Expected: only the planned source files differ from the execution base, and the worktree is clean after the three feature commits. If `cargo fmt` changed a planned file, commit only that formatting with `git commit -m "style: format combat attrition accounting"`.

