# Combat Laboratory Microscope Calibration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve exact-replayed incumbents under limited coverage, freeze executable build identity in artifacts, and produce a separate release-build `4 x 2` seed006 feasibility calibration without changing search policy or the accepted pilot.

**Architecture:** Add one optional nested candidate block to raw cells and one nested per-profile candidate summary while leaving resolved/pair/interaction semantics untouched. Add optional Cargo-profile provenance to manifests and make resume compare it exactly. After source verification and clean commits, run a new any-surviving-win release experiment in a new artifact directory.

**Tech Stack:** Rust 2021, Serde JSON, Cargo build script environment, existing Combat Search V2 exact replay, append-only Combat Lab artifacts, Clap driver.

## Global Constraints

- Work only in the stable checkout `D:\rust\sts_simulator`; do not create a worktree and never run `cargo clean`.
- Follow `docs/superpowers/specs/2026-07-14-combat-laboratory-microscope-calibration-design.md` exactly.
- Write and run each focused regression test RED before production code, then GREEN.
- Preserve `outcome_class`, coverage, resolved HP, pair, and interaction semantics.
- Do not modify search policy, rollout, action ordering, combat simulation, route, run-control, campfire, shop, or acquisition behavior.
- Do not touch or resume `artifacts/runs/combat-lab-seed006-pilot`; record and compare its hashes around the new run.
- Do not run a 10-second or larger follow-up experiment.
- Do not assert any permanent seed006 outcome, HP, or profile superiority.
- Do not push to a remote.

---

### Task 1: Retain exact-replayed candidate evidence

**Files:**

- Modify: `src/eval/combat_lab_v1/replay.rs`
- Modify: `src/eval/combat_lab_v1/runner.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`

**Interfaces:**

- Consumes: `CombatSearchV2TrajectoryReport`, `CombatSearchV2WitnessReplayV1`, and the existing strict coverage classifier.
- Produces: `CombatLabReplayedCandidateV1` and optional `CombatLabCellRecordV1::replayed_candidate` without changing resolved-only top-level fields.

- [ ] **Step 1: Add focused RED tests**

Add four tests beside the existing cell/replay tests:

- `cell_time_limited_replayed_win_retains_candidate_without_resolving` uses
  `replayable_win_sample()`, `SearchCoverageStatus::TimeBudgetLimited`, and the real
  replay function. It asserts `CoverageLimited`, `replay_validated`, candidate
  terminal `Win`, candidate HP/turn/action/draw evidence equal to the trajectory and
  replay, and all resolved-only top-level metrics/history absent.
- `cell_time_limited_replayed_loss_retains_candidate_without_resolving` constructs a
  natural one-action losing sample/trajectory with the existing combat-step helper,
  exact-replays it under `TimeBudgetLimited`, and makes the symmetric assertions for
  terminal `Loss` and terminal HP zero.
- `cell_without_complete_trajectory_has_no_replayed_candidate` passes `None` as the
  selected trajectory under `TimeBudgetLimited` and asserts coverage-limited,
  `replay_validated == false`, and no candidate.
- `cell_replay_error_has_no_candidate_and_halts` reuses the existing injected replay
  error, asserting `ExecutionError`, no candidate, error stage `ExactReplay`, and
  `halt_experiment == true`.

- [ ] **Step 2: Run RED and confirm the missing surface**

Run:

```powershell
cargo test --lib combat_lab_v1::tests::cell_time_limited_replayed -- --nocapture
cargo test --lib combat_lab_v1::tests::cell_without_complete_trajectory_has_no_replayed_candidate -- --nocapture
cargo test --lib combat_lab_v1::tests::cell_replay_error_has_no_candidate_and_halts -- --nocapture
```

Expected: compilation fails because `replayed_candidate` and `CombatLabReplayedCandidateV1` do not exist.

- [ ] **Step 3: Implement the minimal candidate block**

In `replay.rs`, define:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabReplayedCandidateV1 {
    pub terminal: SearchTerminalLabel,
    pub outcome_order_key: CombatSearchV2OutcomeOrderKeyReport,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub actions: usize,
    pub cards_played: u32,
    pub potions_used: u32,
    pub draw_history: Vec<DomainCardSnapshot>,
    pub action_history: Vec<ClientInput>,
}
```

Add to `CombatLabCellRecordV1`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub replayed_candidate: Option<CombatLabReplayedCandidateV1>,
```

Construct the candidate only from `selected.zip(replay.as_ref())` after successful exact replay. Derive draw/action histories once from replay evidence; clone them into the nested candidate and retain the existing top-level copies only for resolved outcomes. Candidate presence must not participate in `classify_combat_lab_outcome_v1`.

Set `replayed_candidate: None` in `sample_construction_error_cell_v1` in `runner.rs` and in direct test fixtures.

- [ ] **Step 4: Run GREEN and regression checks**

Run:

```powershell
cargo test --lib combat_lab_v1::tests::cell_time_limited_replayed
cargo test --lib combat_lab_v1::tests::cell_without_complete_trajectory_has_no_replayed_candidate
cargo test --lib combat_lab_v1::tests::cell_replay_error_has_no_candidate_and_halts
cargo test --lib combat_lab_v1::tests::cell
cargo fmt --all -- --check
git diff --check
```

Expected: all selected tests pass with no warnings or formatting diff.

- [ ] **Step 5: Commit Task 1**

```powershell
git add src/eval/combat_lab_v1/replay.rs src/eval/combat_lab_v1/runner.rs src/eval/combat_lab_v1/tests.rs
git commit -m "feat: retain combat lab replayed candidates"
```

---

### Task 2: Summarize candidate feasibility separately from resolution

**Files:**

- Modify: `src/eval/combat_lab_v1/summary.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`

**Interfaces:**

- Consumes: optional `CombatLabReplayedCandidateV1` values from Task 1 plus existing `nodes_to_first_win`.
- Produces: `CombatLabCandidateSummaryV1` nested under each `CombatLabProfileSummaryV1`.

- [ ] **Step 1: Write a RED aggregation test**

Add `summary_reports_replayed_candidates_without_promoting_coverage`. Build two
profiles with direct typed cells containing a coverage-limited replayed win, a
coverage-limited replayed loss, an unresolved coverage-limited cell, an execution
error, and a resolved win. Set deterministic candidate values so the expected
counts are three complete candidates, two winning candidates, one losing candidate,
an all-non-error denominator of four, and a candidate win rate of `0.5`. Assert exact
means/counts for win HP loss, terminal HP, turns, potions, and first-win nodes; assert
zero/`None` distributions on the profile without candidates. Assert the coverage
cells still contribute zero resolved wins/losses and do not enter pair or interaction
eligibility.

- [ ] **Step 2: Run RED**

```powershell
cargo test --lib combat_lab_v1::tests::summary_reports_replayed_candidates_without_promoting_coverage -- --nocapture
```

Expected: compilation fails because `CombatLabCandidateSummaryV1` and the `candidates` profile field do not exist.

- [ ] **Step 3: Implement candidate aggregation**

Define the exact structure from the design:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCandidateSummaryV1 {
    pub replayed_complete_candidates: usize,
    pub replayed_win_candidates: usize,
    pub replayed_loss_candidates: usize,
    pub replayed_win_rate_all_non_error: Option<f64>,
    pub replayed_win_rate_all_non_error_denominator: usize,
    pub win_hp_loss: CombatLabNumericSummaryV1,
    pub terminal_hp: CombatLabNumericSummaryV1,
    pub turns: CombatLabNumericSummaryV1,
    pub potions_used: CombatLabNumericSummaryV1,
    pub nodes_to_first_win: CombatLabNumericSummaryV1,
}
```

Add `pub candidates: CombatLabCandidateSummaryV1` to `CombatLabProfileSummaryV1`. Aggregate candidate metrics without changing resolved filters. Include `nodes_to_first_win` only for replayed winning candidates. Reuse `numeric_summary`; do not add candidate pair or interaction logic.

- [ ] **Step 4: Run GREEN and deterministic-summary tests**

```powershell
cargo test --lib combat_lab_v1::tests::summary_reports_replayed_candidates_without_promoting_coverage
cargo test --lib combat_lab_v1::tests::summary
cargo test --lib combat_lab_v1::tests::interaction
cargo fmt --all -- --check
git diff --check
```

Expected: candidate and existing summary tests pass; JSON regeneration remains byte-stable.

- [ ] **Step 5: Commit Task 2**

```powershell
git add src/eval/combat_lab_v1/summary.rs src/eval/combat_lab_v1/tests.rs
git commit -m "feat: summarize combat lab candidate evidence"
```

---

### Task 3: Freeze Cargo build identity at resume

**Files:**

- Modify: `build.rs`
- Modify: `src/eval/combat_lab_v1/artifact.rs`
- Modify: `src/eval/combat_lab_v1/tests.rs`
- Verify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**

- Consumes: Cargo build-script `PROFILE`, `cfg!(debug_assertions)`, and existing manifest resume identity validation.
- Produces: optional `cargo_profile` and `debug_assertions` fields that new manifests always populate and resume compares exactly.

- [ ] **Step 1: Write RED manifest compatibility and mismatch tests**

Add `artifact_manifest_records_build_identity`,
`artifact_resume_rejects_missing_or_changed_build_identity`, and
`artifact_legacy_manifest_without_build_identity_still_deserializes`. Assert new
manifests store `Some(env!("STS_CARGO_PROFILE"))` and
`Some(cfg!(debug_assertions))`. For resume, create an artifact, rewrite only one
environment field in `manifest.json`, and assert `create_or_resume` rejects with
`environment.cargo_profile` or `environment.debug_assertions` before journal bytes
change. Deserialize a JSON manifest with both fields removed and assert both become
`None`.

- [ ] **Step 2: Run RED**

```powershell
cargo test --lib combat_lab_v1::tests::artifact_manifest_records_build_identity -- --nocapture
cargo test --lib combat_lab_v1::tests::artifact_resume_rejects_missing_or_changed_build_identity -- --nocapture
cargo test --lib combat_lab_v1::tests::artifact_legacy_manifest_without_build_identity_still_deserializes -- --nocapture
```

Expected: compilation fails because the environment fields and `STS_CARGO_PROFILE` do not exist.

- [ ] **Step 3: Export and persist the build identity**

In `build.rs`, immediately after the build-script rerun directive:

```rust
let profile = env::var("PROFILE").expect("Cargo should provide PROFILE to build.rs");
println!("cargo:rustc-env=STS_CARGO_PROFILE={profile}");
```

In `CombatLabEnvironmentV1`, add:

```rust
#[serde(default)]
pub cargo_profile: Option<String>,
#[serde(default)]
pub debug_assertions: Option<bool>,
```

Populate them with `Some(env!("STS_CARGO_PROFILE").to_string())` and
`Some(cfg!(debug_assertions))`. Add exact `ensure_resume_field` checks after target
architecture validation. Do not change artifact/checkpoint schema constants.

- [ ] **Step 4: Run GREEN and architecture regression checks**

```powershell
cargo test --lib combat_lab_v1::tests::artifact_manifest_records_build_identity
cargo test --lib combat_lab_v1::tests::artifact_resume_rejects_missing_or_changed_build_identity
cargo test --lib combat_lab_v1::tests::artifact_legacy_manifest_without_build_identity_still_deserializes
cargo test --lib combat_lab_v1::tests::artifact
cargo test --test architecture_runtime_boundaries build_script_only_watches_consumed_inputs
cargo fmt --all -- --check
git diff --check
```

Expected: focused tests and the existing build-script boundary pass.

- [ ] **Step 5: Commit Task 3**

```powershell
git add build.rs src/eval/combat_lab_v1/artifact.rs src/eval/combat_lab_v1/tests.rs
git commit -m "feat: freeze combat lab build identity"
```

---

### Task 4: Add the feasibility fixture, review, verify, and run

**Files:**

- Create: `fixtures/combat_lab/seed006_reptomancer_feasibility_4x2.lab.json`
- Modify: `src/eval/combat_lab_v1/tests.rs`
- Create: `.superpowers/sdd/combat-lab-calibration-report.md` (ignored local report)

**Interfaces:**

- Consumes: Tasks 1-3, the maintained seed006-derived start, and Task 7 runner/CLI.
- Produces: a maintained release-3s feasibility fixture and local ignored 4 x 2 artifact.

- [ ] **Step 1: Write the fixture RED test before the fixture**

Add `seed006_feasibility_fixture_resolves_with_any_surviving_win_threshold`. Load
`fixtures/combat_lab/seed006_reptomancer_feasibility_4x2.lab.json`. Assert
experiment/scenario IDs, schedule, two ordered exact-state profiles, every common
budget field, and `stop_on_win_hp_loss_at_most == Some(87)`. Resolve the maintained
8 x 2 fixture alongside it, normalize only experiment ID and the threshold, and
assert the remaining resolved profile, schedule, scenario, and budget JSON values
are identical.

- [ ] **Step 2: Run RED**

```powershell
cargo test --lib combat_lab_v1::tests::seed006_feasibility_fixture_resolves_with_any_surviving_win_threshold -- --nocapture
```

Expected: failure because the fixture file does not exist.

- [ ] **Step 3: Add the exact maintained fixture**

Copy the maintained 8 x 2 contract and change only:

```json
{
  "experiment_id": "seed006_reptomancer_feasibility_release_3s_4x2",
  "common_budget": {
    "stop_on_win_hp_loss_at_most": 87
  }
}
```

Retain all omitted surrounding fields exactly from
`seed006_reptomancer_8x2.lab.json`.

- [ ] **Step 4: Run GREEN and focused calibration tests**

```powershell
cargo test --lib combat_lab_v1::tests::seed006_feasibility_fixture_resolves_with_any_surviving_win_threshold
cargo test --lib combat_lab_v1::tests::cell
cargo test --lib combat_lab_v1::tests::summary
cargo test --lib combat_lab_v1::tests::artifact
cargo test --lib combat_lab_v1::tests::runner
cargo fmt --all -- --check
git diff --check
```

- [ ] **Step 5: Self-review the complete maintained diff**

Inspect:

```powershell
git diff 141b7f5..HEAD
git diff --check
rg -n "combat_lab_v1" src/eval/run_control src/ai/route_planner_v1 src/ai/strategy/acquisition.rs
```

Confirm candidate evidence never promotes coverage, historical fields have Serde defaults, build identity blocks mismatch, no live policy imports the lab, no old artifact is changed, and no seed outcome assertion exists.

- [ ] **Step 6: Run the completion gate**

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
cargo test --bin combat_search_v2_driver
cargo fmt --all -- --check
git diff --check
```

Expected: all suites pass without warnings or failures.

- [ ] **Step 7: Commit the maintained fixture/test and require clean status**

```powershell
git add fixtures/combat_lab/seed006_reptomancer_feasibility_4x2.lab.json src/eval/combat_lab_v1/tests.rs
git commit -m "test: add combat lab feasibility calibration"
git status --short
```

Expected: empty status.

- [ ] **Step 8: Hash the accepted pilot and run the new release experiment**

Record SHA-256 for all four files under `artifacts/runs/combat-lab-seed006-pilot`.
Require `artifacts/runs/combat-lab-seed006-feasibility-release-3s` not to exist.
Then run:

```powershell
cargo run --release --bin combat_search_v2_driver -- --lab-spec fixtures/combat_lab/seed006_reptomancer_feasibility_4x2.lab.json --lab-output artifacts/runs/combat-lab-seed006-feasibility-release-3s --lab-samples 4
```

Inspect manifest, journal, checkpoint, and summary. Require final clean commit identity,
`cargo_profile = "release"`, `debug_assertions = false`, eight unique cells unless a
valid invariant halt occurs, candidate counts matching raw cells, and no old-pilot
hash change.

- [ ] **Step 9: Prove idempotence and write the local report**

Hash new journal/summary, rerun the same target, and require `cells_appended == 0`
with byte-identical hashes. Write `.superpowers/sdd/combat-lab-calibration-report.md`
with commits, test counts, raw outcomes, candidate/resolved counts, build identity,
hash comparisons, and the next-action interpretation. Do not run a follow-up budget.

- [ ] **Step 10: Final clean-state audit**

```powershell
git status --short
git log -6 --oneline
```

Compare every design requirement with source/tests/runtime evidence before closing the goal.
