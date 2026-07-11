# Accepted High-Loss Combat Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve replayable evidence for committed combat wins whose original or selected line loses at least 25% of maximum HP, while distinguishing original search quality from the line actually executed.

**Architecture:** Run-control emits typed accepted-line evidence at the line-selection boundary and reports the selected outcome in combat history while retaining original search performance. Owner-audit pairs that evidence with a pre-search `CombatCaptureV1`, propagates only committed high-loss diagnostics through the branch, and capsule persistence writes capture/evidence pairs without changing the existing combat-gap schema.

**Tech Stack:** Rust, serde/serde_json, run-control combat search, owner-audit branch runtime, capsule artifact store, Cargo tests.

## Global Constraints

- Do not change combat search ordering, budgets, acceptance gates, or repair behavior.
- Do not change reward, shop, route, campfire, or deck-deficit policy.
- Do not feed recent attrition into owners.
- Do not special-case enemies, cards, or seed `20260711004`.
- Do not save every successful combat.
- Do not reinterpret or migrate `CombatCase.gap`.
- Diagnostic artifacts are simulator evidence, not teacher labels.
- Do not use subagents; execute inline in the current session.

---

## File Structure

- `src/eval/run_control/accepted_combat_line_evidence.rs`: typed original-versus-selected evidence and annotation extraction.
- `src/eval/run_control/trace_annotation.rs`: accepted-line annotation variant.
- `src/eval/run_control/combat_search.rs`: emit evidence before moving the selected line into the executor.
- `src/eval/run_control/combat_line_trace.rs`: report the applied selected outcome while retaining original search performance.
- `src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs`: high-loss trigger, exact start capture, identity, and sidecar model.
- Owner-audit portfolio/branch/checkpoint files: propagate committed diagnostic drafts.
- Capsule store/format/slice-result files: write paired artifacts and expose their references.

---

### Task 1: Make accepted-line outcome evidence truthful and typed

**Files:**
- Create: `src/eval/run_control/accepted_combat_line_evidence.rs`
- Modify: `src/eval/run_control/trace_annotation.rs`
- Modify: `src/eval/run_control/combat_search.rs`
- Modify: `src/eval/run_control/combat_line_trace.rs`
- Modify: `src/eval/run_control/mod.rs`
- Test: inline unit tests in those modules

**Interfaces:**
- Produces: `AcceptedCombatLineEvidenceV1 { original, selected, hp_saved_by_selection, selection_summary }`.
- Produces: `accepted_combat_line_evidence_v1(&[RunControlTraceAnnotationV1]) -> Option<&AcceptedCombatLineEvidenceV1>`.

- [ ] **Step 1: Add failing evidence and selected-history tests**

```rust
#[test]
fn accepted_line_evidence_keeps_original_and_selected_losses_separate() {
    let evidence = AcceptedCombatLineEvidenceV1::new(
        line_summary(24, 35),
        line_summary(44, 15),
        Some("line_repair attempts=4 wins=2 improvements=1".to_string()),
    );
    assert_eq!(evidence.original.hp_loss, 35);
    assert_eq!(evidence.selected.hp_loss, 15);
    assert_eq!(evidence.hp_saved_by_selection, 20);
}
```

Add a `combat_line_trace` test proving the selected line overwrites outcome fields while original `nodes_expanded`, `terminal_wins`, and timing stay unchanged.

- [ ] **Step 2: Run focused tests and verify RED**

```powershell
cargo test --lib accepted_line_evidence_keeps_original_and_selected_losses_separate -- --exact
cargo test --lib selected_line_snapshot_keeps_report_performance -- --exact
```

Expected: failure because the evidence type and selected-snapshot behavior do not exist.

- [ ] **Step 3: Implement the evidence type and trace annotation**

```rust
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedCombatLineEvidenceV1 {
    pub original: CombatSearchTerminalLineSummary,
    pub selected: CombatSearchTerminalLineSummary,
    pub hp_saved_by_selection: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_summary: Option<String>,
}

impl AcceptedCombatLineEvidenceV1 {
    pub fn new(
        original: CombatSearchTerminalLineSummary,
        selected: CombatSearchTerminalLineSummary,
        selection_summary: Option<String>,
    ) -> Self {
        Self {
            hp_saved_by_selection: original.hp_loss.saturating_sub(selected.hp_loss),
            original,
            selected,
            selection_summary,
        }
    }
}
```

Add `RunControlTraceAnnotationV1::AcceptedCombatLine { evidence }`. In `apply_search_combat`, build original and selected summaries before moving the selected line, call the executor, and append the annotation. Change `combat_line_performance_trace_annotation` to overwrite selected outcome fields even when `line_performance` is absent, while leaving original report counters/timing intact.

- [ ] **Step 4: Run focused tests and verify GREEN**

```powershell
cargo test --lib accepted_combat_line_evidence -- --nocapture
cargo test --lib combat_line_trace -- --nocapture
cargo test --lib combat_search -- --nocapture
```

Expected: all matching tests pass.

- [ ] **Step 5: Commit Task 1**

```powershell
git add src/eval/run_control
git commit -m "feat: distinguish selected combat lines in traces"
```

---

### Task 2: Capture committed high-loss wins at the owner-audit boundary

**Files:**
- Create: `src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_runner.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_output.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_result.rs`
- Modify: `src/runtime/branch/owner_audit/runner.rs`
- Test: inline trigger, lane-commit, and portfolio tests

**Interfaces:**
- Produces: `AcceptedHighLossDiagnosticDraft` with exact `CombatCaptureV1`, accepted-line evidence, lane, search summary, and automation trajectory.
- Trigger: `original.hp_loss * 4 >= max_hp || selected.hp_loss * 4 >= max_hp`.

- [ ] **Step 1: Add failing trigger and commit-gate tests**

```rust
#[test]
fn high_loss_trigger_checks_original_and_selected_lines() {
    assert!(high_loss_trigger(74, 35, 15));
    assert!(high_loss_trigger(74, 10, 24));
    assert!(!high_loss_trigger(74, 15, 18));
}

#[test]
fn rejected_lane_never_produces_accepted_high_loss_diagnostic() {
    let draft = accepted_high_loss_diagnostic(
        &session_with_active_combat(),
        "primary",
        &accepted_outcome(35, 15),
        false,
    );
    assert!(draft.is_none());
}
```

Add a committed-lane test asserting capture fingerprint, selected trajectory, lane, start HP, and evidence.

- [ ] **Step 2: Run focused tests and verify RED**

```powershell
cargo test --lib high_loss_trigger_checks_original_and_selected_lines -- --exact
cargo test --lib rejected_lane_never_produces_accepted_high_loss_diagnostic -- --exact
```

Expected: failure because the diagnostic module does not exist.

- [ ] **Step 3: Implement the draft and portfolio propagation**

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct AcceptedHighLossDiagnosticDraft {
    pub(super) identity: AcceptedCombatIdentityV1,
    pub(super) lane: String,
    pub(super) capture: CombatCaptureV1,
    pub(super) evidence: AcceptedCombatLineEvidenceV1,
    pub(super) search: CombatSearchTraceSummary,
    pub(super) trajectory: CombatAutomationTrajectoryRecordV1,
    pub(super) hard_hp_loss_limit: Option<u32>,
}

pub(super) fn high_loss_trigger(max_hp: i32, original: i32, selected: i32) -> bool {
    let max_hp = max_hp.max(1) as i64;
    i64::from(original.max(0)) * 4 >= max_hp
        || i64::from(selected.max(0)) * 4 >= max_hp
}
```

Capture the stable combat position before cloning the lane trial. After classification, build a draft only if the lane committed and accepted evidence, selected automation, and a selected search summary all exist. Extend portfolio output/result and `AdvanceResult` with `Vec<AcceptedHighLossDiagnosticDraft>`.

- [ ] **Step 4: Run focused tests and verify GREEN**

```powershell
cargo test --lib accepted_high_loss_diagnostic -- --nocapture
cargo test --lib combat_search_lane_runner -- --nocapture
cargo test --lib combat_search_portfolio -- --nocapture
```

Expected: all matching tests pass.

- [ ] **Step 5: Commit Task 2**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs src/runtime/branch/owner_audit/combat_search_lane_runner.rs src/runtime/branch/owner_audit/combat_search_portfolio_output.rs src/runtime/branch/owner_audit/combat_search_portfolio_result.rs src/runtime/branch/owner_audit/runner.rs
git commit -m "feat: retain committed high-loss combat evidence"
```

---

### Task 3: Preserve diagnostics across branch creation and resume

**Files:**
- Modify: `src/runtime/branch/owner_audit/branch_model.rs`
- Modify: `src/runtime/branch/owner_audit/branch_scheduler.rs`
- Modify: `src/runtime/branch/owner_audit/branch_runtime.rs`
- Modify: `src/runtime/branch/owner_audit/owner_choice_expander.rs`
- Modify: `src/runtime/branch/owner_audit/branch_path.rs`
- Modify: `src/runtime/branch/owner_audit/frontier_checkpoint.rs`
- Test: frontier checkpoint and branch constructor tests

**Interfaces:**
- Adds: `Branch.accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>`.
- Missing checkpoint fields deserialize to an empty vector.

- [ ] **Step 1: Add a failing checkpoint round-trip test**

Create a branch with one sample diagnostic, save/load the frontier, and assert identity and selected loss survive. Extend the legacy-checkpoint test to assert an empty vector.

- [ ] **Step 2: Run checkpoint tests and verify RED**

```powershell
cargo test --lib frontier_checkpoint -- --nocapture
```

Expected: failure because branch/checkpoint models do not carry diagnostics.

- [ ] **Step 3: Thread and deduplicate the vector**

Add the field to every `Branch` constructor, initialize it from the initial advance result, extend it in `prepare_branch_work`, clone it into child branches, and persist it in `BranchCheckpoint`:

```rust
#[serde(default)]
accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
```

Deduplicate by `AcceptedCombatIdentityV1` when operation-budget chunks repeat the same committed combat.

- [ ] **Step 4: Run branch tests and verify GREEN**

```powershell
cargo test --lib frontier_checkpoint -- --nocapture
cargo test --lib owner_audit -- --nocapture
```

Expected: all matching tests pass, including legacy loading.

- [ ] **Step 5: Commit Task 3**

```powershell
git add src/runtime/branch/owner_audit/branch_model.rs src/runtime/branch/owner_audit/branch_scheduler.rs src/runtime/branch/owner_audit/branch_runtime.rs src/runtime/branch/owner_audit/owner_choice_expander.rs src/runtime/branch/owner_audit/branch_path.rs src/runtime/branch/owner_audit/frontier_checkpoint.rs
git commit -m "feat: carry high-loss diagnostics through branches"
```

---

### Task 4: Write replayable capture/evidence pairs into capsules

**Files:**
- Modify: `src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs`
- Modify: `src/runtime/branch/owner_audit/capsule_artifact_store.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`
- Modify: `src/runtime/branch/slice_result.rs`
- Test: capsule store, capsule format, and slice result tests

**Interfaces:**
- Produces: `accepted_high_loss_combat/<identity>.capture.json` as valid `CombatCaptureV1`.
- Produces: `accepted_high_loss_combat/<identity>.evidence.json` as typed evidence.
- Adds: `accepted_high_loss_combat_diagnostics` array to result/summary JSON.
- Adds: vector artifact refs without replacing `combat_case_ref`.

- [ ] **Step 1: Add a failing capsule-write test**

```rust
let result = read_json(root.join("result.json"));
let entries = result["accepted_high_loss_combat_diagnostics"]
    .as_array()
    .expect("diagnostic array");
assert_eq!(entries.len(), 1);
let capture = load_combat_capture_v1(Path::new(entries[0]["capture"].as_str().unwrap()))
    .expect("replayable capture");
assert_eq!(capture.summary.player_hp, 59);
assert_eq!(entries[0]["original_hp_loss"], 35);
assert_eq!(entries[0]["selected_hp_loss"], 15);
```

Assert `combat_case` remains null for accepted wins and existing gap tests remain unchanged.

- [ ] **Step 2: Run capsule tests and verify RED**

```powershell
cargo test --lib capsule_artifact_store -- --nocapture
cargo test --lib run_capsule_format -- --nocapture
```

Expected: failure because accepted-high-loss files are not written or surfaced.

- [ ] **Step 3: Implement paired artifact writing**

Add an accepted-high-loss directory and writer. Save captures with `save_combat_capture_v1`; write sidecars with typed evidence and capture path. Names include seed, act, floor, combat turn, and stable enemy slug, so the same identity replaces rather than duplicates a pair.

Extend `ArtifactWriteSummary` with:

```rust
#[serde(default)]
pub accepted_combat_diagnostic_refs: Vec<ArtifactRef>,
```

Merge/enumerate the vector without changing `combat_case_ref`, and pass diagnostic paths into result and summary formatting.

- [ ] **Step 4: Run persistence tests and verify GREEN**

```powershell
cargo test --lib capsule_artifact_store -- --nocapture
cargo test --lib run_capsule_format -- --nocapture
cargo test --lib slice_result -- --nocapture
```

Expected: all matching tests pass and gap compatibility stays green.

- [ ] **Step 5: Commit Task 4**

```powershell
git add src/runtime/branch/owner_audit/accepted_high_loss_diagnostic.rs src/runtime/branch/owner_audit/capsule_artifact_store.rs src/runtime/branch/owner_audit/run_capsule_format.rs src/runtime/branch/slice_result.rs
git commit -m "feat: persist replayable high-loss combat diagnostics"
```

---

### Task 5: Verify and diagnose the exact Snake Plant line

**Files:**
- No production edits expected
- Output: fresh ignored capsule under `target/`

**Interfaces:**
- Consumes: paired high-loss capture/evidence artifacts.
- Produces: evidence-backed search-versus-upstream classification; no policy change.

- [ ] **Step 1: Format and run focused suites**

```powershell
cargo fmt --all -- --check
cargo test --lib accepted_combat_line_evidence -- --nocapture
cargo test --lib accepted_high_loss_diagnostic -- --nocapture
cargo test --lib capsule_artifact_store -- --nocapture
```

Expected: exit code 0.

- [ ] **Step 2: Run the full library suite and the repository's single-build architecture suite**

```powershell
cargo test --lib
```

Expected: all tests pass with zero failures; run architecture-sensitive tests through their existing one-build command, also with zero failures.

- [ ] **Step 3: Run seed `20260711004` into a fresh capsule**

```powershell
target/fast-run/branch_tiny.exe --seed 20260711004 --ascension 0 --objective first-victory --generations 64 --max-branches 1 --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 200000 --rescue-search-ms 3000 --boss-search-nodes 800000 --boss-search-ms 10000 --wall-ms 60000 --capsule target/bounded-mainline-20260711004-high-loss-diagnostics
```

Expected: inspectable original-versus-selected evidence for Centurion and a replayable selected-high-loss Snake Plant capture.

- [ ] **Step 4: Replay Snake Plant with a quality search**

Use `combat_search_v2_driver --combat-snapshot <capture>` with a budget that continues beyond the first survivable win and retains multiple win candidates. Compare selected HP loss, terminal wins, Offering plays, Uppercut Weak timing, and attack order with the saved evidence.

Expected: classify one outcome with evidence: either a materially lower-loss win exists and search acceptance/ordering is next, or it does not appear under the bounded quality search and upstream resilience is next.

- [ ] **Step 5: Record final repository state**

```powershell
git status --short
git log -5 --oneline
```

Expected: no uncommitted source changes; capsule outputs remain ignored under `target/`.
