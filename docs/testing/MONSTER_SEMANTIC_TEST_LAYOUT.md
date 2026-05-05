# Monster Semantic Test Layout

This note defines the target testing shape for monster semantic work.

The immediate trigger is practical: monster files such as
[spheric_guardian.rs](/d:/rust/sts_simulator/src/content/monsters/city/spheric_guardian.rs)
are starting to accumulate large `#[cfg(test)]` blocks whose main job is
reconstructing generic `CombatState` scaffolding rather than checking monster
logic.

That is a code-organization problem, not a reason to remove tests.

## Decision

Yes, this should be fixed now.

The project has already crossed the threshold where inline monster tests are
slowing down semantic migration:

- monster files are mixing production logic, temporary compatibility code, and
  integration-grade test fixtures
- the same `CombatState` boilerplate is being copied across files
- local test setup is now visually larger than the logic being tested
- this makes semantic monster work harder to write and harder to review

The correct response is to move test construction out of content files, not to
lower test coverage.

## Goals

- Keep monster production files focused on monster behavior.
- Keep semantic regression coverage strong.
- Make new monster semantic ports cheap to test.
- Prevent each monster file from inventing its own mini test harness.

## Non-Goals

- Do not add a second semantic layer.
- Do not move protocol or parity tests into content modules.
- Do not delete behavior tests just because they are large.

## Final Layout

### 1. Content Files

Files under `src/content/monsters/**` should contain:

- production monster logic
- at most very small local tests for pure local helpers
- examples:
  - `plan_for(move_id, asc)`
  - `decode_turn(plan)`
  - very small move-shape assertions

These local tests should not manually build a full `CombatState` unless there is
no simpler option.

### 2. Shared Monster Test Support

`src/testing/support/` should own reusable test-only builders for monster
semantic tests.

This layer should provide small helpers such as:

- `minimal_combat()`
- `combat_with_player(class, hp, energy)`
- `monster(enemy_id)`
- `monster_with_move(enemy_id, move_id)`
- `monster_with_history(enemy_id, &[u8])`
- `combat_with_monsters(vec![...])`

The support layer exists to remove duplicated setup, not to hide behavior.

### 3. Integration-Grade Monster Behavior Tests

Monster semantic behavior tests should live under `tests/`, not inside the
monster implementation file.

Recommended shape:

- `tests/monster_semantics/<monster>.rs`

or, if the repository keeps flat test files:

- `tests/<monster>_behavior.rs`

These tests should cover:

- `turn_plan`
- `roll_move_plan`
- `take_turn_plan`
- move-history-sensitive sequences
- hidden/runtime-state-sensitive behavior when applicable

### 4. Live / Protocol / Regression Tests

Tests that validate:

- split snapshot import
- live-comm panic regressions
- protocol truth import
- replay/minimized captures

must remain outside content modules.

These belong in existing integration surfaces such as:

- `tests/live_comm_*`
- `tests/protocol_*`
- `tests/state_sync_*`

## What Stays Inline

Inline `#[cfg(test)]` is still appropriate when all of the following are true:

- the test exercises only local helper logic
- it does not need shared state construction
- it is short enough that the test improves readability of the file

As a rough rule:

- good inline test: 5 to 20 lines
- bad inline test: 50 lines of generic combat scaffolding to assert one move id

## What Moves Out

Move the test out of the monster file when any of the following are true:

- it builds `CombatState`, `EntityState`, `PlayerEntity`, or `CardZones` manually
- it validates a real turn sequence rather than a pure helper
- it needs move history, runtime state, or protocol-derived truth
- the test setup is longer than the assertion logic

## Migration Rule

For each monster semantic port:

1. Add the production semantic entry points.
2. Add or update one integration-grade behavior test under `tests/`.
3. Keep only tiny local helper tests inline.
4. If inline tests need a full combat scaffold, move that scaffold into
   `testing::support` immediately instead of copying it again.

## Support API Direction

The current repository already reserves this place:

- [src/testing/mod.rs](/d:/rust/sts_simulator/src/testing/mod.rs)
  - `testing::support` is already named as the owner of test-only helpers

That should become the canonical home for monster-semantic builders.

Initial useful support modules would be:

- `src/testing/support/combat_builders.rs`
- `src/testing/support/monster_builders.rs`

Initial useful helper functions would be:

- `blank_test_combat()`
- `test_player_ironclad()`
- `test_monster(enemy_id)`
- `planned_monster(enemy_id, move_id)`
- `monster_with_history(enemy_id, history)`

## Why This Is The Right Boundary

This keeps responsibilities clean:

- content files own monster rules
- `testing::support` owns generic test scaffolding
- `tests/` owns behavior validation
- live/protocol tests stay in their existing system-level locations

This also matches the larger architecture direction:

- one semantic execution path
- one protocol adapter path
- one place for reusable test construction

## Immediate Application

The current large inline test blocks in:

- [cultist.rs](/d:/rust/sts_simulator/src/content/monsters/exordium/cultist.rs)
- [spheric_guardian.rs](/d:/rust/sts_simulator/src/content/monsters/city/spheric_guardian.rs)

should be treated as transitional, not final.

`spheric_guardian.rs` is the current example of why this document exists:

- the semantic port itself is reasonable in size
- the inline scaffolding is not

## Done Criteria

This migration is in good shape when:

- new monster semantic ports do not need to hand-write full `CombatState` setup
- most monster files have either no inline tests or only tiny helper tests
- behavior-heavy monster tests live under `tests/`
- `testing::support` is actually used instead of remaining an empty placeholder

