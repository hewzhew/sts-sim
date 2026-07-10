# Run-Control First-Round Narrowing Design

## Status

Approved direction from the 2026-07-10 investigation. This design limits the
first implementation round to retiring stale run-play-only behavior and
centralizing the combat-search report schema contract. It does not move the
run-control session or redesign combat portfolio orchestration.

## Problem

`eval::run_control` began as the execution and inspection surface behind
`run_play_driver`, then became a shared kernel for branch experiments and the
owner-audit runtime. It now mixes four responsibilities:

1. simulator session state and typed input application;
2. decision surfaces, transitions, traces, and inspection views;
3. combat-search execution and exact-line application;
4. legacy `FullPlanner` non-combat policy selection for the semi-automatic
   run-play REPL.

Two failing tests expose stale contracts at the edge of that mixture:

- `CombatSearchV2Report` is produced as schema version 11, while the standalone
  `CombatSearchEvidenceV1` loader accepts only nested report versions 7 through
  9;
- the legacy run-play campfire planner still executes an action but no longer
  emits the typed non-combat decision record expected by its old test.

The owner-audit mainline already bypasses the legacy non-combat planner and
stops at owner boundaries. The safest first narrowing round is therefore to
remove the stale run-play-only strategy behavior while preserving the shared
execution kernel.

## Considered Approaches

### 1. Patch the two failing assertions only

Raise the SearchEvidence report-version ceiling to 11 and reconnect the old
campfire annotation. This is the smallest diff, but it preserves two unused or
superseded contracts and leaves future schema synchronization manual.

### 2. Retire SearchEvidence only

Delete the standalone evidence artifact while keeping `FullPlanner`. This
removes one stale format but leaves run-control selecting strategic non-combat
actions through a path the owner-audit runtime deliberately avoids.

### 3. First-round system narrowing (selected)

Centralize report schema identity, retire standalone SearchEvidence, and make
run-play automatic advancement stop at strategic non-combat boundaries. This
aligns run-play with the owner-boundary model without moving the heavily shared
session kernel in the same change.

## Scope

### In scope

- Give `CombatSearchV2Report` one schema-name constant and one current-version
  constant owned next to the report type.
- Use those constants when finalizing reports and asserting the current report
  contract.
- Remove creation, loading, validation, command parsing, help text, outcome
  plumbing, and new trace recording for `CombatSearchEvidenceV1`.
- Preserve the historical `SessionTraceArtifactKind::CombatSearchEvidence`
  enum variant so older traces containing it can still deserialize.
- Make `AutoStep` and `AutoRun` use routine-only non-combat advancement:
  routine/forced transitions may run automatically, while campfire, shop,
  event, boss-relic, card-reward, and run-choice strategy decisions stop at a
  typed human/owner boundary.
- Remove production functions and tests that exist only to support the retired
  `FullPlanner` strategy chain.
- Retain explicit manual commands such as recorded card-reward selection and
  Singing Bowl actions.
- Retain helpers used by branch campaign inspection or owner-audit execution,
  even when they currently live in a broadly named policy module.

### Out of scope

- Moving `RunControlSession` from `eval` to `runtime`.
- Redesigning `RunControlCommand` or the decision-surface representation.
- Splitting the outer owner-audit combat portfolio from run-control's internal
  fallback pipeline.
- Retiring `branch_experiment` or `branch_campaign`.
- Changing AI policy quality, combat-search heuristics, route planning, reward
  valuation, or owner decisions.
- Migrating old standalone SearchEvidence JSON files. No such artifact was
  found in the workspace, and no in-repository reader consumes them.

## Architecture

### Combat-search report schema ownership

`types/report/core.rs` owns:

```rust
pub const COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME: &str = "CombatSearchV2Report";
pub const COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION: u32 = 11;
```

`search/finalize.rs` consumes those constants. Tests compare emitted reports to
the constants rather than repeating numeric literals. Future report changes
therefore update one producer-owned declaration.

### Search evidence retirement

The supported combat investigation flow becomes:

```text
runner or run-play boundary
  -> CombatCase / CombatCapture for reproducible input
  -> combat search or combat-case review
  -> session trace / capsule summary for bounded attempt facts
```

There is no longer a second envelope containing a full serialized
`CombatSearchV2Report`. The `sc` and auto-step option structures no longer carry
an evidence target. Combat search outcomes no longer carry a saved evidence
path. Session traces continue to deserialize the historical artifact-kind enum
variant, but new commands cannot create that artifact.

### Routine-only run-play automation

`AutoStep` and `AutoRun` use one non-combat behavior:

```text
reward housekeeping
  -> active combat handoff
  -> optional route planner move
  -> routine or forced visible candidate
  -> stop at strategic boundary
```

The implementation does not ask campfire, shop, event, boss-relic,
run-choice, or card-reward policy modules to choose an action. The stop path
continues to emit the existing hidden-free human-boundary record.

`BranchExperimentBoundary` remains separate because the older branch
experiment still owns Match and Keep and Note For Yourself compatibility
behavior. `OwnerAuditRoutineOnly` remains the mainline owner-audit mode. If the
three modes become behaviorally identical after dead-code removal, naming may
be simplified only where tests demonstrate no compatibility change.

## Compatibility Boundaries

- Mainline owner-audit behavior must not change.
- Manual typed run-control commands must continue to apply through the existing
  input gate and transition reporting.
- `SessionTraceV1` versions currently accepted by `trace_replay` must still
  deserialize a historical `CombatSearchEvidence` artifact reference.
- Search command aliases and non-evidence search options remain supported.
- Removed `save=case|path` search options must fail as unknown options instead
  of being silently ignored.
- Strategic run-play auto boundaries must stop without mutating the strategic
  choice.

## Error Handling

- Parsing a retired SearchEvidence option returns the existing unknown-option
  error shape.
- Routine-only automation uses the existing typed `HumanBoundary` stop with a
  boundary-specific reason.
- Combat search itself continues to report rejection and exact-line execution
  failures through `RunControlCommandOutcome`; only evidence-path side effects
  are removed.

## Test Strategy

### Schema contract

- A combat-search report test asserts schema name and version through the
  producer-owned constants.

### SearchEvidence removal

- Command parser tests assert that `sc save=case` is rejected.
- Existing search-combat execution tests assert normal search behavior without
  evidence-path output.
- A trace deserialization test preserves compatibility with the historical
  `CombatSearchEvidence` enum variant.

### Routine-only automation

- Campfire auto-run at low HP stops at the campfire and records a typed human
  boundary without healing or smithing.
- Representative shop/event/card-reward strategic boundaries stop without
  applying the retired policy chain.
- Routine and forced transitions still advance.
- Owner-audit and branch-experiment focused tests remain green.

### Verification

- Run focused schema, command, auto-step, owner-audit, and trace tests during
  each red-green cycle.
- Run `cargo fmt --check`.
- Run the full library test suite and report any failures exactly.

## Implementation Sequence

1. Centralize the report schema constants.
2. Remove SearchEvidence from command and combat-search data flow while
   preserving trace enum compatibility.
3. Add routine-only run-play behavior tests, then retire `FullPlanner` policy
   selection and its stale tests/functions.
4. Remove newly unreachable modules or extract still-used narrow helpers.
5. Run focused and full verification.

## Success Criteria

- The two known library failures are gone for the intended reason: the obsolete
  SearchEvidence contract and FullPlanner campfire behavior no longer exist.
- No new standalone combat-search evidence artifact can be created.
- Run-play auto advancement does not make strategic non-combat decisions.
- Owner-audit mainline behavior and historical trace deserialization remain
  intact.
- Combat-search report schema identity has one producer-owned source of truth.
