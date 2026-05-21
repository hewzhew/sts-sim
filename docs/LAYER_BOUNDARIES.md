# Layer Boundaries

This file defines the hard dependency direction for `src/`.

## Layers

- `core`
  - runtime truth and game semantics
  - `src/runtime/`
  - `src/content/`
  - `src/core/`
  - `src/engine/`
  - `src/map/`
  - `src/rewards/`
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
- `runtime::monster_move`
  - runtime monster move-plan structs used by content, action execution, and
    AI-facing projections
- `sim`
  - AI-facing simulator views, legal action helpers, and projection helpers
- `fixtures`
  - integration-only fixture/spec assembly
  - exported from `lib.rs` as `sts_simulator::fixtures`
- `testing::harness`
  - integration-side analysis helpers
  - currently `hexaghost_value`
- `testing::protocol`
  - private Java/protocol fixture metadata parser
- `testing::state_sync`
  - private fixture importer from protocol/live snapshots into runtime combat state
- `testing::replay_support`
  - compatibility helpers for old fixture imports only
- `live_comm`
  - legacy external bridge tooling; fixture capture only unless rebuilt under
    `docs/live_comm/LEGACY_FIXTURE_ONLY.md`

## Enforcement

- no active boundary test is currently checked in
- a future boundary test should block `core -> integration/entrypoints` and
  `integration -> entrypoints`
- Any new exception should be treated as a structural regression, not as a casual import choice.
