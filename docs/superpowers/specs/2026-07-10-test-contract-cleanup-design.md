# Test Contract Cleanup Design

## Goal

Align the test suite with the current single-mainline policy direction without
changing production behavior.

## Scope

Delete these four tests because they protect the retired branch-first shop
strategy:

- `compiled_shop_branch_topk_preserves_distinct_card_purchase_lanes`
- `compiled_shop_branch_frontier_can_admit_non_rollout_thesis_candidate`
- `current_boundary_includes_three_purchase_combo_for_high_gold_shop_pressure`
- `branch_experiment_executes_shop_combo_purchase_branch`

Remove these three stale assertions while keeping the surrounding behavioral
tests:

- the old `DeckMutationCompilerV1` render-title assertion;
- the old unopened Singing Bowl command-hint assertion;
- the old `turn_plan_policy=diagnostic_only` default-policy assertion.

## Non-Goals

- Do not change shop policy, acquisition gates, or branch scheduling.
- Do not change combat-search report schemas or evidence validation.
- Do not refactor run-control or campfire automation in this change.
- Do not replace stale prose assertions with new prose assertions.

## Verification

Run the previously failing focused tests that remain, then run:

```powershell
cargo fmt --check
cargo test --lib
cargo test --bins
cargo test --test architecture_runtime_boundaries
python -m unittest discover -s tests -p 'test_*.py'
git diff --check
```

The intended result is that the seven retired assertions no longer fail. Any
remaining failure, especially combat-search evidence schema validation or the
missing campfire noncombat record, remains visible and is reported separately.
