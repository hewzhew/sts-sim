# Layer Boundaries

This file defines the hard dependency direction for `src/`.

## Layers

- `core`
  - runtime truth and game semantics
  - `src/runtime/`
  - `src/content/`
  - `src/core/`
  - `src/engine/`
  - `src/state/`
- `integration`
  - fixture import, eval surfaces, and analysis helpers around the runtime
  - `src/testing/`
  - `src/eval/`
- `entrypoints`
  - thin CLI binaries that consume core/integration
  - `src/bin/`

## Allowed Dependency Direction

- `core`
- `integration -> core`
- `entrypoints -> core`
- `entrypoints -> integration`

## Forbidden Dependency Direction

- `core -> integration`
- `core -> entrypoints`
- `integration -> entrypoints`

## Current Ownership Notes

- `runtime`
  - base runtime primitives
  - `runtime::action`
  - `runtime::combat`
  - `runtime::rng`
- `runtime::combat`
  - `state`: combat container state, turn runtime, queues, zones, counters
  - `entities`: player/monster entity records and monster intent surface
  - `monster_runtime`: per-monster private runtime fields
  - `card`: combat card identity, cost, and transient stat state
  - `power`: combat-owned power payload/runtime records
  - `combat_methods`: state methods that coordinate these records
- `runtime::monster_move`
  - runtime monster move-plan structs used by content, action execution, and
    AI-facing projections
- `state::events`
  - event state, event selection context, event pools, and event-room roll
    generation
- `state::map`
  - run map graph, room nodes, map progress, and map generation
- `sim`
  - AI-facing simulator views, legal action helpers, and projection helpers
- `fixtures`
  - integration-only start-spec assembly
  - exported from `lib.rs` as `sts_simulator::fixtures`
- `testing::support`
  - test-only local combat builders
- `live_comm`
  - legacy external bridge tooling; not active unless rebuilt under
    `docs/live_comm/LEGACY_FIXTURE_ONLY.md`

## Enforcement

- no active boundary test is currently checked in
- a future boundary test should block `core -> integration/entrypoints` and
  `integration -> entrypoints`
- Any new exception should be treated as a structural regression, not as a casual import choice.
