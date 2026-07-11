# Combat Review Saved-Win Classification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `combat_case_review` classify a captured owner win as a policy rejection instead of overwriting it with a generic ladder no-win conclusion.

**Architecture:** Pass the already captured `failed_search` into the classification boundary and give saved complete-win evidence first precedence. Keep all ladder output unchanged, but suppress derived strategic feedback when the capture proves the combat had a complete winning line.

**Tech Stack:** Rust, Serde JSON test fixtures, Cargo binary/library/architecture tests.

## Global Constraints

- Work in the stable checkout on `fix/combat-review-saved-win-classification`; do not create a worktree.
- Do not run `cargo clean`.
- Do not rerun or duplicate the exact owner profile in `combat_case_review`.
- Do not change combat search, owner acceptance, elite survival behavior, or the combat-case schema.
- Keep existing ladder classifications unchanged when no saved complete win exists.
- Keep raw ladder and focus output visible when saved evidence takes classification precedence.

---

### Task 1: Give saved complete-win evidence classification precedence

**Files:**
- Modify: `src/bin/combat_case_review/classification.rs`
- Modify: `src/bin/combat_case_review/review_pipeline.rs`

**Interfaces:**
- Consumes: `Option<&CombatSearchTraceSummary>`, `&[SearchReview]`, and `Option<&CombatReviewFocus>`.
- Produces: `classify_gap_review(saved_search, ladder, focus) -> CombatGapReviewClassification` and kind `SavedCompleteWinRejectedByPolicy`.

- [ ] **Step 1: Add a passing characterization test for the current boundary**

Before changing the function signature, add this test to `classification.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ladder_is_not_reviewed() {
        let classification = classify_gap_review(&[], None);

        assert_eq!(classification.kind, "NotReviewed");
        assert_eq!(classification.reason, "ladder_not_requested");
        assert_eq!(classification.basis_review, None);
    }
}
```

Run:

```powershell
cargo test --bin combat_case_review classification::tests::missing_ladder_is_not_reviewed
```

Expected: one test passes and characterizes the existing no-ladder behavior.

- [ ] **Step 2: Refactor the signature without changing behavior**

Import `CombatSearchTraceSummary`, add an unused saved-search argument, update
the characterization test to pass `None`, and update `review_pipeline.rs`:

```rust
pub(super) fn classify_gap_review(
    _saved_search: Option<&CombatSearchTraceSummary>,
    ladder: &[SearchReview],
    focus: Option<&CombatReviewFocus>,
) -> CombatGapReviewClassification {
```

```rust
let classification = classify_gap_review(
    case.failed_search.as_ref(),
    &ladder,
    review_focus.as_ref(),
);
```

Run the characterization test again. Expected: one test passes with unchanged
behavior.

- [ ] **Step 3: Add failing saved-win classification tests**

Extend the existing test module with a helper that deserializes the real
summary type without constructing unrelated fields:

```rust
use serde_json::json;
use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
use sts_simulator::eval::run_control::CombatSearchTraceSummary;

    fn saved_search(complete_win_found: bool, include_best_win: bool) -> CombatSearchTraceSummary {
        let best_win = include_best_win.then(|| {
            json!({
                "terminal": SearchTerminalLabel::Win,
                "final_hp": 13,
                "hp_loss": 32,
                "turns": 6,
                "cards_played": 23,
                "potions_used": 1,
                "potions_discarded": 0,
                "action_count": 32
            })
        });
        serde_json::from_value(json!({
            "source": "search_combat_rejected",
            "act": 2,
            "floor": 23,
            "turn": 0,
            "combat_kind": "elite",
            "enemies": ["Book of Stabbing"],
            "coverage_status": "TimeBudgetLimited",
            "complete_trajectory_found": true,
            "complete_win_found": complete_win_found,
            "best_win": best_win,
            "deadline_hit": true,
            "nodes_expanded": 3544,
            "terminal_wins": 84,
            "total_us": 5_067_038
        }))
        .expect("valid saved search summary")
    }

    #[test]
    fn saved_complete_win_precedes_missing_ladder_win() {
        let saved = saved_search(true, true);

        let classification = classify_gap_review(Some(&saved), &[], None);

        assert_eq!(classification.kind, "SavedCompleteWinRejectedByPolicy");
        assert_eq!(classification.reason, "saved_complete_win_present_in_case");
        assert_eq!(classification.basis_review, Some("saved_search"));
    }

    #[test]
    fn legacy_best_win_proves_saved_complete_win() {
        let saved = saved_search(false, true);

        let classification = classify_gap_review(Some(&saved), &[], None);

        assert_eq!(classification.kind, "SavedCompleteWinRejectedByPolicy");
    }

    #[test]
    fn saved_search_without_win_preserves_existing_classification() {
        let saved = saved_search(false, false);

        let classification = classify_gap_review(Some(&saved), &[], None);

        assert_eq!(classification.kind, "NotReviewed");
        assert_eq!(classification.reason, "ladder_not_requested");
    }
```

- [ ] **Step 4: Run the saved-win tests and verify RED**

Run:

```powershell
cargo test --bin combat_case_review classification::tests::
```

Expected: `saved_complete_win_precedes_missing_ladder_win` fails because the
refactored function still returns `NotReviewed`. The characterization test and
the no-win compatibility test pass.

- [ ] **Step 5: Implement the minimal evidence-first rule**

Rename `_saved_search` to `saved_search` and place this rule before the
empty-ladder rule:

```rust
if saved_search.is_some_and(|search| search.complete_win_found || search.best_win.is_some()) {
    return classification(
        "SavedCompleteWinRejectedByPolicy",
        "saved_complete_win_present_in_case",
        Some("saved_search"),
    );
}
```

- [ ] **Step 6: Run the classification tests and verify GREEN**

Run the same focused command. Expected: all three classification tests pass.

- [ ] **Step 7: Commit the classification slice**

```powershell
git add src/bin/combat_case_review/classification.rs src/bin/combat_case_review/review_pipeline.rs
git commit -m "fix: prioritize saved combat wins in review"
```

### Task 2: Suppress false deck-failure feedback for policy-rejected wins

**Files:**
- Modify: `src/bin/combat_case_review/strategic_feedback.rs`

**Interfaces:**
- Consumes: classification kind and whether the ladder contains reviews.
- Produces: `should_emit_strategic_feedback(classification_kind, has_ladder) -> bool` and no feedback report for `SavedCompleteWinRejectedByPolicy`.

- [ ] **Step 1: Extract the current feedback gate without changing behavior**

Add this helper and replace the existing empty-ladder guard with its result:

```rust
fn should_emit_strategic_feedback(_classification_kind: &str, has_ladder: bool) -> bool {
    has_ladder
}
```

```rust
if !should_emit_strategic_feedback(classification.kind, !ladder.is_empty()) {
    return None;
}
```

Run:

```powershell
cargo test --bin combat_case_review
```

Expected: the binary suite passes because the extraction preserves the current
empty-ladder behavior.

- [ ] **Step 2: Add a failing feedback-gate test**

Add this pure gate and test contract to `strategic_feedback.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_policy_rejection_does_not_emit_deck_failure_feedback() {
        assert!(!should_emit_strategic_feedback(
            "SavedCompleteWinRejectedByPolicy",
            true,
        ));
        assert!(should_emit_strategic_feedback(
            "StillNoWinAfterReview",
            true,
        ));
        assert!(!should_emit_strategic_feedback(
            "StillNoWinAfterReview",
            false,
        ));
    }
}
```

- [ ] **Step 3: Run the gate test and verify RED**

Run:

```powershell
cargo test --bin combat_case_review strategic_feedback::tests::saved_policy_rejection_does_not_emit_deck_failure_feedback
```

Expected: the assertion for `SavedCompleteWinRejectedByPolicy` fails because
the extracted gate currently returns true whenever a ladder exists.

- [ ] **Step 4: Implement the minimal policy-rejection exception**

```rust
fn should_emit_strategic_feedback(classification_kind: &str, has_ladder: bool) -> bool {
    has_ladder && classification_kind != "SavedCompleteWinRejectedByPolicy"
}
```

- [ ] **Step 5: Run the gate test and verify GREEN**

Run the same focused command. Expected: one test passes.

- [ ] **Step 6: Commit the feedback slice**

```powershell
git add src/bin/combat_case_review/strategic_feedback.rs
git commit -m "fix: suppress feedback for policy-rejected wins"
```

### Task 3: Verify the saved Book case and repository boundaries

**Files:**
- Read: `artifacts/runs/route-reliability-seed-20260711004-survival-fallback/combat_cases/seed20260711004_g18_b0018_a2f23_bookofstabbing.json`
- Modify only if verification exposes a defect in the approved scope.

**Interfaces:**
- Consumes: the new Book case, focused tests, and repository completion suites.
- Produces: fresh evidence that saved-win classification survives a nonempty losing ladder without extra search cost.

- [ ] **Step 1: Run a cheap nonempty-ladder CLI check**

```powershell
cargo run --quiet --bin combat_case_review -- --case "artifacts/runs/route-reliability-seed-20260711004-survival-fallback/combat_cases/seed20260711004_g18_b0018_a2f23_bookofstabbing.json" --ladder --fast-nodes 1 --fast-ms 1 --slow-nodes 1 --slow-ms 1 --compact --write-review artifacts/runs/route-reliability-seed-20260711004-survival-fallback/book-review-saved-win.json
```

Expected JSON assertions:

```powershell
$review = Get-Content artifacts/runs/route-reliability-seed-20260711004-survival-fallback/book-review-saved-win.json -Raw | ConvertFrom-Json
$review.ladder.Count -gt 0
$review.classification.kind -eq 'SavedCompleteWinRejectedByPolicy'
$null -eq $review.combat_strategic_feedback
```

All three expressions must print `True`.

- [ ] **Step 2: Run focused binary tests**

```powershell
cargo test --bin combat_case_review classification::tests::
cargo test --bin combat_case_review strategic_feedback::tests::
```

Expected: all focused tests pass.

- [ ] **Step 3: Run completion verification**

```powershell
cargo fmt --check
cargo test --bin combat_case_review
cargo test --lib
cargo test --test architecture_runtime_boundaries
git diff --check
```

Expected: formatting passes; all binary, library, and architecture tests pass; diff check is clean.

- [ ] **Step 4: Inspect final state**

```powershell
git status --short --branch
git log -4 --oneline
```

Expected: only the implementation plan remains uncommitted, or the worktree is clean if it was included in a final documentation commit.
