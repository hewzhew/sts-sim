# Layer Boundaries

This file defines the hard dependency direction for `src/`.

## Layers

- `core`
  - runtime truth and RL-facing semantics
  - `src/runtime/`
  - `src/semantics/`
  - `src/projection/`
  - `src/content/`
  - `src/core/`
  - `src/engine/`
  - `src/map/`
  - `src/rewards/`
  - `src/state/`
- `integration`
  - protocol mapping, replay, sync, fixtures, and analysis helpers around the runtime
  - `src/diff/`
  - `src/protocol/`
  - `src/testing/`
  - `src/verification/`
- `app`
  - search, policy, CLI, coverage, and higher-level workbenches that consume core/integration
  - `src/bot/`
  - `src/cli/`
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
- `projection`
  - preview/audit views derived from truth-side specs
- `fixtures`
  - integration-only fixture/spec assembly
  - exported from `lib.rs` as `sts_simulator::fixtures`
- `testing::harness`
  - integration-side analysis helpers
  - currently `hexaghost_value`
- `bot::harness`
  - app-layer workbenches and bot-coupled validation surfaces
  - `boss_validation`
  - `combat_env`
  - `combat_lab`
  - `combat_policy`
- `bot::coverage_signatures`
  - bot-side shared signature extraction for coverage/curiosity and live combat logging
- `cli::coverage_tools`
  - offline replay/live-comm coverage record extraction and report output for devtool flows
- `diff::protocol`
  - thin protocol-facing facade over mapping, parsing, and snapshot shaping
- `diff::replay`
  - thin facade over replay execution, inspection, and comparator surfaces
- `diff::state_sync`
  - thin facade over protocol -> runtime state construction and sync
  - must behave as a strict importer for migrated `runtime_state` slices, not as
    a shadow-state repair layer
- `verification`
  - integration-side facade for replay/reconstruction consumers that should not
    reach into `diff::*` internals directly
  - current first slice: `verification::combat`
- `protocol`
  - integration-side Java/protocol adapter facade
  - `protocol::java`

## Enforcement

- `tests/layer_boundaries.rs`
  - blocks `core -> integration/app`
  - blocks `integration -> app`
- Any new exception should be treated as a structural regression, not as a casual import choice.
