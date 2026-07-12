# Combat Candidate Adjudication Census Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Report whether each bounded combat-review search retained any clean winning trajectory, without starting another search or changing production selection.

**Architecture:** A new run-control diagnostic API owns case projection, exact replay, policy adjudication, deduplication, and aggregation. `combat_case_review` retains each ladder report long enough to call that API and adds the returned census to the matching ladder row only when `--adjudicate` is enabled.

**Tech Stack:** Rust, serde JSON, clap, existing combat-search reports, and the existing run-control exact replay and adjudication boundary.

## Global Constraints

- Work in the stable checkout on the existing local feature branch; do not create a worktree.
- Reuse only winning candidates already retained by a ladder report; do not run another search.
- Do not change search scoring, budgets, action ordering, production selection, or Writhing Mass behavior.
- Run-control remains the only owner of persistent-run outcome observation and acceptance policy.
- Existing JSON is unchanged when `--adjudicate` is absent.
- The census must not claim coverage of unretained terminal wins.
- Never run `cargo clean`.

---

### Task 1: Add the run-control candidate census owner

**Files:**
- Create: `src/eval/run_control/combat_case_candidate_census.rs`
- Modify: `src/eval/run_control/combat_case_adjudication.rs`
- Modify: `src/eval/run_control/combat_line_outcome.rs`
- Modify: `src/eval/run_control/mod.rs`

**Interfaces:**
- Consumes: `CombatCase`, `CombatSearchV2Config`, `CombatSearchV2Report`, `evaluate_combat_candidate_line_outcome`, and `CombatLineAcceptancePolicy`.
- Produces: `adjudicate_combat_case_candidates_v1(source_review: impl Into<String>, case: &CombatCase, config: &CombatSearchV2Config, report: &CombatSearchV2Report) -> CombatCaseCandidateAdjudicationCensusV1`.
- Produces public serde types for the census, conclusion, replay failure, best-clean summary, and per-curse candidate counts.

- [ ] **Step 1: Write failing aggregation tests**

Create `combat_case_candidate_census.rs` with tests that feed a private `summarize_evaluations` helper typed `CombatLineObservedOutcomeV1` values. Use one clean outcome and one outcome containing a `CardSnapshot { id: CardId::Parasite, uuid: 9001, upgrades: 0 }`.

The first test must assert:

```rust
assert_eq!(retained_candidate_count, 3);
assert_eq!(unique_candidate_count, 2);
assert_eq!(replayed_candidate_count, 2);
assert_eq!(clean_accepted_count, 1);
assert_eq!(new_curse_rejected_count, 1);
assert_eq!(gained_curse_counts[0].card, CardId::Parasite);
assert_eq!(gained_curse_counts[0].candidate_count, 1);
assert_eq!(best_clean_candidate.unwrap().retained_index, 0);
assert_eq!(
    conclusion,
    CombatCaseCandidateCensusConclusionV1::CleanCandidatePresent
);
```

The second test must combine one dirty outcome with:

```rust
CombatCaseCandidateReplayFailureV1 {
    retained_index: 1,
    action_count: 7,
    error: "drift".to_string(),
}
```

and assert the conclusion is `IncompleteDueToReplayFailures`, never `AllReplayedCandidatesDirty`. A third test must assert an empty retained list yields the typed `NoRetainedCandidates` variant.

- [ ] **Step 2: Run the tests to verify the red state**

Run: `cargo test --lib combat_case_candidate_census`

Expected: compilation fails because the census types and helpers do not exist.

- [ ] **Step 3: Implement the census data contract**

Implement these exact core types:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatCaseCandidateCensusConclusionV1 {
    CleanCandidatePresent,
    AllReplayedCandidatesDirty,
    IncompleteDueToReplayFailures,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatCaseCandidateReplayFailureV1 {
    pub retained_index: usize,
    pub action_count: usize,
    pub error: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatCaseCandidateOutcomeSummaryV1 {
    pub retained_index: usize,
    pub observed_outcome: CombatLineObservedOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatCaseGainedCurseCountV1 {
    pub card: CardId,
    pub candidate_count: usize,
}
```

Define `CombatCaseCandidateAdjudicationCensusV1` as a serde-tagged enum with:

```rust
NoRetainedCandidates {
    source_review: String,
    retained_candidate_count: usize,
}
ProjectionFailed {
    source_review: String,
    retained_candidate_count: usize,
    error: String,
}
Adjudicated {
    source_review: String,
    projection_trust: String,
    retained_candidate_count: usize,
    unique_candidate_count: usize,
    replayed_candidate_count: usize,
    replay_failures: Vec<CombatCaseCandidateReplayFailureV1>,
    clean_accepted_count: usize,
    new_curse_rejected_count: usize,
    gained_curse_counts: Vec<CombatCaseGainedCurseCountV1>,
    best_clean_candidate: Option<CombatCaseCandidateOutcomeSummaryV1>,
    conclusion: CombatCaseCandidateCensusConclusionV1,
}
```

Add `source_review(&self) -> &str`. Count each curse id at most once per evaluated candidate. Select the best clean candidate with the existing `prefer_accepted_outcome`; change that helper to `pub(super)` instead of copying its ordering.

Use this conclusion precedence:

```rust
let conclusion = if best_clean_candidate.is_some() {
    CombatCaseCandidateCensusConclusionV1::CleanCandidatePresent
} else if !replay_failures.is_empty() {
    CombatCaseCandidateCensusConclusionV1::IncompleteDueToReplayFailures
} else {
    CombatCaseCandidateCensusConclusionV1::AllReplayedCandidatesDirty
};
```

- [ ] **Step 4: Implement deduplication and exact replay**

Make `project_combat_case_session` and `adjudicate_observed_outcome` in `combat_case_adjudication.rs` visible to sibling run-control modules with `pub(super)`.

Implement the public API. Build the retained list from `best_win_trajectory` followed by `win_candidate_trajectories`, preserving report order. Record raw retained count before deduplication. Deduplicate with `HashSet<Vec<String>>`, where each fingerprint is the ordered `action_key` list. Project the case once, then evaluate every unique trajectory with:

```rust
let line = CombatCandidateLine::from_search_trajectory(trajectory);
evaluate_combat_candidate_line_outcome(&session, &case.position, config, line)
```

Feed successful outcomes and typed replay failures into `summarize_evaluations`. This module must never call `run_combat_search_v2`.

Export the API and public serde types from `src/eval/run_control/mod.rs`.

- [ ] **Step 5: Verify and commit Task 1**

Run:

```powershell
cargo test --lib combat_case_candidate_census
cargo test --lib dual_policy_results_share_one_observed_dirty_outcome
cargo test --lib acceptance_plugins_adjudicate_the_same_dirty_outcome_explicitly
```

Expected: all selected tests pass.

Commit:

```powershell
git add src/eval/run_control/combat_case_candidate_census.rs src/eval/run_control/combat_case_adjudication.rs src/eval/run_control/combat_line_outcome.rs src/eval/run_control/mod.rs
git commit -m "feat: adjudicate retained combat win candidates"
```

---

### Task 2: Attach one census to each reviewed ladder row

**Files:**
- Modify: `src/bin/combat_case_review/adjudication_probe.rs`
- Modify: `src/bin/combat_case_review/review_pipeline/ladder.rs`
- Modify: `src/bin/combat_case_review/review_pipeline.rs`
- Modify: `src/bin/combat_case_review/search_types.rs`
- Modify: `src/bin/combat_case_review/search_review.rs`
- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**
- Consumes: Task 1's census API and types.
- Produces: `SearchReview.candidate_adjudication_census: Option<CombatCaseCandidateAdjudicationCensusV1>` filled only under `--adjudicate`.

- [ ] **Step 1: Write failing CLI tests**

Replace the best-line-only transport with:

```rust
pub(super) struct ReviewAdjudicationRun {
    pub(super) source_review: &'static str,
    pub(super) config: CombatSearchV2Config,
    pub(super) report: CombatSearchV2Report,
}
```

Add `disabled_candidate_census_is_absent`, asserting:

```rust
assert_eq!(super::run_candidate_censuses(false, &[], None), None);
```

Add `candidate_adjudication_census_serialization` in `search_types.rs`. Construct a zero-valued `SearchReview`; serialize once with the census field `None` and assert the key is absent. Then set it to `Some(NoRetainedCandidates { source_review: "lane".into(), retained_candidate_count: 0 })` and assert the serialized status is `no_retained_candidates`.

Extend `combat_line_adjudication_has_one_production_owner` with:

```rust
let review_probe = std::fs::read_to_string(
    "src/bin/combat_case_review/adjudication_probe.rs",
).expect("read review adjudication probe");
assert!(!review_probe.contains("meta_changes"));
assert!(!review_probe.contains("CardType::Curse"));
assert!(!review_probe.contains("master_deck_curse_count"));
```

- [ ] **Step 2: Run the tests to verify the red state**

Run:

```powershell
cargo test --bin combat_case_review disabled_candidate_census_is_absent
cargo test --bin combat_case_review candidate_adjudication_census_serialization
cargo test --test architecture_runtime_boundaries combat_line_adjudication_has_one_production_owner -- --exact
```

Expected: the CLI tests fail to compile because the report transport, census runner, and nested field do not exist. The architecture test can pass immediately and remains a regression guard.

- [ ] **Step 3: Retain reports and run both diagnostics**

Change `ReviewLadderRun` to carry `adjudication_runs: Vec<ReviewAdjudicationRun>`. Each `LadderProfileRun` must retain its exact `config` and owned `CombatSearchV2Report`; do not rerun the profile. Preserve `line_lab_parent` before moving the slow report.

Update `run_adjudication_probe` to select a focused `ReviewAdjudicationRun` and use `report.best_win_trajectory`. Add:

```rust
pub(super) fn run_candidate_censuses(
    enabled: bool,
    runs: &[ReviewAdjudicationRun],
    case: Option<&CombatCase>,
) -> Option<Vec<CombatCaseCandidateAdjudicationCensusV1>>
```

When enabled with a case, map every run through the Task 1 API. If the case is absent, return a typed projection failure for each run; never infer a curse in the CLI.

- [ ] **Step 4: Attach results to matching ladder rows**

Add to `SearchReview` and initialize to `None` in `search_review`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub(super) candidate_adjudication_census:
    Option<CombatCaseCandidateAdjudicationCensusV1>,
```

In `build_review`, calculate the focused probe and censuses before consuming `adjudication_runs`. For every census, find the `SearchReview` whose `label == census.source_review()` and assign the field. Do not add a top-level census field: the evidence belongs beside the ladder row whose retained set it describes.

- [ ] **Step 5: Verify and commit Task 2**

Run:

```powershell
cargo test --bin combat_case_review adjudication_probe
cargo test --bin combat_case_review candidate_adjudication_census_serialization
cargo test --test architecture_runtime_boundaries combat_line_adjudication_has_one_production_owner -- --exact
```

Expected: all selected tests pass.

Commit:

```powershell
git add src/bin/combat_case_review/adjudication_probe.rs src/bin/combat_case_review/review_pipeline/ladder.rs src/bin/combat_case_review/review_pipeline.rs src/bin/combat_case_review/search_types.rs src/bin/combat_case_review/search_review.rs tests/architecture_runtime_boundaries.rs
git commit -m "feat: expose retained win candidate census"
```

---

### Task 3: Validate the real evidence and complete the branch

**Files:**
- Runtime artifact only: `artifacts/runs/writhingmass-adjudication-census-20260713.json` (ignored; do not force-add)

**Interfaces:**
- Consumes: the completed CLI and the saved Writhing Mass combat case.
- Produces: one bounded local diagnostic artifact and a fully verified feature branch.

- [ ] **Step 1: Run the saved case with the existing ladder**

```powershell
cargo run --quiet --bin combat_case_review -- --case "target\bounded-mainline-20260712002\combat_cases\seed20260712002_g34_b0034_a3f42_writhingmass.json" --adjudicate --fast-nodes 200000 --fast-ms 2000 --slow-nodes 300000 --slow-ms 5000 --compact --write-review "artifacts\runs\writhingmass-adjudication-census-20260713.json"
```

Expected: exit code 0. The census replays retained lines only and starts no search beyond the two existing ladder profiles.

- [ ] **Step 2: Print the bounded evidence**

```powershell
$review = Get-Content "artifacts\runs\writhingmass-adjudication-census-20260713.json" -Raw | ConvertFrom-Json
$review.ladder | ForEach-Object {
  [pscustomobject]@{
    Label = $_.label
    TerminalWins = $_.terminal_wins
    Retained = $_.candidate_adjudication_census.retained_candidate_count
    Unique = $_.candidate_adjudication_census.unique_candidate_count
    Clean = $_.candidate_adjudication_census.clean_accepted_count
    Dirty = $_.candidate_adjudication_census.new_curse_rejected_count
    ReplayFailures = @($_.candidate_adjudication_census.replay_failures).Count
    Conclusion = $_.candidate_adjudication_census.conclusion
  }
}
```

Expected: counts are internally consistent. Record the observed result exactly; do not require a clean candidate and do not generalize from retained candidates to all terminal wins.

- [ ] **Step 3: Audit boundaries and run completion verification**

Run:

```powershell
rg -n "meta_changes|CardType::Curse|master_deck_curse_count|run_combat_search_v2" src/bin/combat_case_review/adjudication_probe.rs src/eval/run_control/combat_case_candidate_census.rs
cargo test --lib
cargo test --bin combat_case_review
cargo test --test architecture_runtime_boundaries
git diff --check
git status -sb
```

Expected: the source audit finds no CLI curse inference and no search call in the census owner; all suites pass; `git diff --check` is silent; the feature branch is clean. The ignored runtime artifact does not appear in status. If verification exposes a source defect, correct it with TDD, repeat these exact checks, and amend the relevant Task 1 or Task 2 commit rather than creating an evidence-only code path.
