# Testing Charter

Tests in this project should protect stable contracts, not preserve accidental
code shape. A test is useful when a future maintainer can answer, quickly and
locally:

- What behavior or boundary does this protect?
- What kind of bug would make it fail?
- Is that bug important enough to keep the test?

If those answers are unclear, the test should be rewritten, moved to a better
layer, or removed.

## Test Compilation Boundaries

Tests are deliberately split across real Cargo packages:

- `cargo test-core <filter>` tests `sts_simulator` domain, engine, simulation,
  and stable policy code;
- `cargo test-planner <filter>` tests the exact complete-turn planner without
  linking the core crate's monolithic unit-test binary;
- `cargo test-control <filter>` tests combat search, evaluation, run-control,
  branch runtime, and their shared contracts;
- `cargo architecture <filter>` runs dependency-free source-boundary checks.

Bare `cargo test --lib` addresses only the default core package and is not a
complete repository check. Keep local unit tests beside their owning module;
do not recreate the old monolithic harness with feature gates, and do not move
private unit tests into many independently linked integration-test binaries.

New independently owned subsystems should prefer a workspace crate when that
boundary prevents unrelated unit tests from entering their edit-test loop.
Runtime filters do not reduce Rust compilation or linking work inside one test
target.

## What Good Tests Protect

Good tests protect one of these surfaces:

- **Domain semantics**: card, relic, potion, enemy, and event behavior that must
  match the game.
- **Search kernel contracts**: termination, frontier handling, rollout timing,
  win acceptance, budget accounting, and report consistency.
- **Typed interfaces**: profiles, plugin stacks, owners, run choices, capsule
  schemas, and JSON fields consumed by tools.
- **Regression cases**: a frozen combat or run case with provenance and a clear
  claim, such as "this lane must find a complete win" or "this report must not
  call a half-dead boss a win".
- **Migration guards**: short-lived tests that prevent an ongoing refactor from
  regressing. These must have an obvious deletion condition.

Bad tests usually protect one of these instead:

- Human-facing prose, adjectives, or summary wording.
- A temporary diagnostic report that no maintained consumer reads.
- A specific implementation path when the public contract is simpler.
- A single seed outcome without explaining the general contract it represents.
- A helper module that should be deleted instead of preserved.

## Test Size

Large tests are not automatically wrong. They are wrong when their size hides
the contract.

Use these thresholds as review pressure, not strict law:

- Under 20 lines: usually fine.
- 20-50 lines: acceptable for a scenario test, but the name must explain the
  contract.
- Over 50 lines: require a reason. Prefer extracting fixture setup or splitting
  separate assertions into separate tests.
- File over 500 lines: suspect mixed responsibilities. Separate fixture code,
  kernel contract tests, semantic regression tests, and report schema tests.

If a large test is truly necessary, add a short comment before the setup that
states why the scenario cannot be smaller. Do not add comments that merely
repeat the code.

## Test File Shape

A test file should have one dominant reason to exist. These shapes are healthy:

- `tests.rs` beside a small module, when it tests that module's local contract.
- `tests/<topic>.rs` when a module has multiple independent concerns.
- `fixtures.rs` or `test_support` when many tests share fake steppers, combat
  builders, or JSON builders.
- `semantic_regression/<domain>.rs` when the test protects game-mechanic
  alignment rather than a search algorithm detail.

These shapes are warning signs:

- A single `tests.rs` containing fixtures, fake engines, search contracts,
  report schema checks, and strategy assertions.
- Tests that must be read in file order to understand why they exist.
- Fixtures whose names describe how they work but not what scenario they model.
- Repeated setup that differs only in one config field.

## Adding a Test

Before adding a test, answer these questions in the test name or nearby setup:

1. What is the stable contract?
2. Which layer owns that contract?
3. Can this be tested below the full runner?
4. Is the fixture public-safe and deterministic?
5. Will this test still matter after the current experiment is deleted?

If the answer to 5 is "no", either do not add the test or mark it as a
migration guard by naming the thing that will remove it.

## Deleting or Rewriting a Test

Do not keep a test merely because it already exists. Also do not delete a test
merely because it is ugly.

Before deleting or rewriting:

1. Identify the contract it appears to protect.
2. Decide whether that contract is still real.
3. If real, move the test to the owning layer and make the fixture smaller.
4. If obsolete, delete it in the same change that removes the old behavior.
5. If uncertain, keep the test but add it to the cleanup queue with the reason.

## Combat Search Test Policy

`combat_search_v2` needs tests, but not a single giant test drawer.

Preferred split:

- **Search loop contracts**: budget, deadline, terminal handling, acceptance,
  frontier insertion, dominance pruning.
- **Plugin/profile projection**: profile ids and config fields map correctly.
- **Rollout behavior**: rollout selection, rollout cache, rollout timing, and
  estimated outcomes.
- **Action ordering**: legal action order, phase hints, setup bias, and
  root-action priors.
- **Report contracts**: fields that tools consume, not prose descriptions.
- **Semantic regression**: card/potion/enemy behavior against the simulator.

`src/ai/combat_search_v2/search/tests.rs` currently mixes several of these.
That does not mean every test is bad. It means the file is no longer a good
unit of understanding. Future cleanup should first extract shared fake
steppers and fixtures, then group tests by the contract above.

## Review Checklist

When reviewing tests, prefer these questions over "does it pass?":

- Does the test fail for exactly one meaningful reason?
- Is the name specific enough to explain the contract?
- Is the fixture smaller than the behavior being tested?
- Does the test assert typed facts instead of strings when typed facts exist?
- Does it avoid hidden futures unless the experiment explicitly declares that
  boundary?
- Would deleting the feature also make deleting this test obvious?

Passing tests are not enough. The test suite should make the system easier to
change, not harder to understand.
