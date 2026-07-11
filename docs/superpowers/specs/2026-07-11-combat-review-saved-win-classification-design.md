# Combat Review Saved-Win Classification Design

## Problem

`combat_case_review` currently serializes the captured `failed_search` as
`saved_search`, but its classification reads only the newly executed generic
review ladder. The ladder does not use the same profile as the owner lane.
Consequently, a case can contain authoritative evidence that the owner found a
complete win while the review is classified as `StillNoWinAfterReview`.

This happened in both recent saved cases:

- the A2F26 hallway owner found a 13 HP complete win using two potions;
- the A2F23 Book of Stabbing owner found a 13 HP complete win using one potion.

In both cases the run stopped because policy rejected the HP loss, not because
combat search failed to find a win. The current classification erases that
distinction and can feed false no-win observations into strategic feedback.

## Decision

Treat the saved owner search as the primary fact about why the case was
captured. The generic ladder remains a supplemental review experiment.

`classify_gap_review` will receive the optional saved search in addition to the
ladder and review focus. Its first classification rule will be:

- when `saved_search.complete_win_found` is true or `saved_search.best_win` is
  present, return `SavedCompleteWinRejectedByPolicy`;
- use reason `saved_complete_win_present_in_case`;
- use `saved_search` as the classification basis.

This rule intentionally says "policy" rather than "HP limit". The current
combat-case schema does not retain the typed run-control rejection, so deriving
an exact rejection kind from a display reason would recreate schema drift.
The case's existing `gap.reason` remains available beside the classification.

When no saved complete win exists, the current ladder classification order and
labels remain unchanged.

## Strategic Feedback

A saved complete win rejected by policy is not evidence that the deck could not
win the combat. `combat_strategic_feedback` will therefore return no strategic
feedback report for `SavedCompleteWinRejectedByPolicy`.

The ladder, focus, and all raw search measurements remain in the output. This
suppresses only the derived deck-failure interpretation; it does not hide the
supplemental experiment.

## Non-goals

- Do not rerun the exact owner profile from `combat_case_review`.
- Do not expose owner-audit private profile builders as a public API.
- Do not copy owner plugin stacks into the review binary.
- Do not change combat search, owner acceptance, or elite survival behavior.
- Do not change the combat-case schema in this slice.

Capturing a typed rejection and a complete owner profile in future case schemas
can be designed separately. It is not required to stop the current diagnostic
contradiction.

## Verification

Focused tests will prove that:

1. a saved complete win wins classification precedence even when the ladder has
   no win;
2. legacy saved summaries with `best_win` but a default-false
   `complete_win_found` receive the same classification;
3. cases without a saved win retain the existing ladder classifications;
4. the new classification suppresses derived strategic feedback.

A cheap CLI check will review the saved Book of Stabbing case with deliberately
tiny ladder budgets. The output must retain the ladder results while reporting
`SavedCompleteWinRejectedByPolicy` and no combat strategic feedback. Completion
verification will run the `combat_case_review` binary tests, the full library
suite, and `architecture_runtime_boundaries`.
