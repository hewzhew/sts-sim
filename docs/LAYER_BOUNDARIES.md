# Layer Boundaries

This file defines the hard dependency direction for `src/`.

## Layers

- `core`
  - runtime truth and game semantics
  - `src/runtime/`
  - `src/semantics/`
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
- `app`
  - CLI, coverage, diagnostics, and higher-level workbenches that consume core/integration
  - `src/app/`
  - `src/bin/`

## Allowed Dependency Direction

- `core`
- `integration -> core`
- `app -> core`
- `app -> integration`

## Forbidden Dependency Direction

- `core -> integration`
- `core -> app`
- `integration -> app`

## Current Ownership Notes

- `runtime`
  - base runtime primitives
  - `runtime::action`
  - `runtime::combat`
  - `runtime::rng`
- `semantics`
  - explicit truth-side action and move specs derived from engine/runtime state
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
- `app::decision_env`
  - app-facing decision environment contract
- `live_comm`
  - legacy external bridge tooling; fixture capture only unless rebuilt under
    `docs/live_comm/LEGACY_FIXTURE_ONLY.md`

## Enforcement

- no active boundary test is currently checked in
- a future boundary test should block `core -> integration/app` and
  `integration -> app`
- Any new exception should be treated as a structural regression, not as a casual import choice.
