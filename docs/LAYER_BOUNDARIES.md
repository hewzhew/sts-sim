# Layer Boundaries

This file defines the hard dependency direction for `src/`.

## Layers

- `core`
  - runtime truth and RL-facing semantics
  - `src/action.rs`
  - `src/combat.rs`
  - `src/content/`
  - `src/core/`
  - `src/engine/`
  - `src/map/`
  - `src/rewards/`
  - `src/rng.rs`
  - `src/state/`
- `integration`
  - protocol mapping, replay, sync, fixtures, and analysis helpers around the runtime
  - `src/diff/`
  - `src/testing/`
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

- `fixtures`
  - integration-only fixture/spec assembly
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

## Enforcement

- `tests/layer_boundaries.rs`
  - blocks `core -> integration/app`
  - blocks `integration -> app`
- Any new exception should be treated as a structural regression, not as a casual import choice.
