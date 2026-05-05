# Repository Map

This file is the current structure blueprint for the repo.

## Tags

- `core`
  - directly implements the simulator and RL-facing runtime path
- `integration`
  - sync, replay, CLI, and verification layers around the runtime
- `tooling`
  - offline analysis, extraction, dataset building, and dev utilities
- `experiment`
  - workbenches, topic-specific probes, and temporary validation surfaces
- `artifact`
  - generated outputs, captures, reports, datasets, and logs
- `legacy`
  - preserved but not primary sources of truth
- `generated`
  - machine-built code or schemas that should not own business rules

## Active Paths

### Engine Truth Path

Treat these as the truth-bearing core:

1. runtime and state
   - `src/runtime/`
   - `src/state/`
   - `src/core/`
2. engine progression
   - `src/engine/`
3. content semantics
   - `src/content/`
4. truth-side semantic and preview layers
   - `src/semantics/`
   - `src/projection/`
5. run and reward flow
   - `src/rewards/`
   - `src/events/`
   - `src/shop/`
   - `src/map/`

### Protocol / Verification Path

Treat these as the integration layer around engine truth:

1. Java/protocol adapter
   - `src/protocol/`
2. importer, replay, and sync
   - `src/diff/`
3. fixtures and scenario tests
   - `src/testing/`
   - `tests/`
4. verification facades
   - `src/verification/`

### App / Workbench Path

These are consumers, not the source of engine truth:

- `src/bot/`
- `src/cli/`
- `src/bin/`

### Current Learning Path

The current RL-facing experiment path is:

1. Rust combat environment truth
   - `src/bot/harness/combat_env.rs`
2. bridge binary
   - `src/bin/combat_env_driver/`
3. Python transport and policy experiments
   - `tools/learning/structured_combat_env.py`
   - `tools/learning/train_structured_combat_ppo.py`
   - `tools/learning/build_combat_rl_datasets.py`
   - `tools/learning/build_macro_counterfactual_dataset.py`

This path is active, but still experimental. Do not treat it as a stable runtime policy stack.

## Top-Level Layout

- `src/` — `core`
  - long-lived Rust code
- `tests/` — `integration`
  - external test drivers and scenario suites
- `tools/` — `tooling`
  - offline analysis, schema building, live-comm helpers, learning datasets, and artifacts
- `docs/` — `tooling`
  - architecture, workflows, protocol rules, and archived investigations
- `logs/` — `artifact`
  - live-comm captures and other loose runtime logs
- `tmp/` — `artifact`
  - temporary local workspace
- `data/` — `artifact`
  - user- or run-specific generated data

## `src/` Ownership

- `src/runtime/` — `core`
  - base runtime primitives: `action`, `combat`, `rng`
- `src/core/` — `core`
  - shared engine-side utility types that are still core truth, not tooling
- `src/engine/` — `core`
  - turn progression, queue driving, action dispatch, room handlers
- `src/content/` — `core`
  - per-entity behavior and hook logic
- `src/state/` — `core`
  - structured run / combat / pending-choice state
- `src/semantics/` — `core`
  - explicit rule-critical semantic views derived from engine truth
- `src/projection/` — `core`
  - preview and presentation views derived from semantic truth
- `src/diff/` — `integration`
  - protocol mapping in `diff::protocol`, replay/verification in `diff::replay`, sync support in `diff::state_sync`
- `src/protocol/` — `integration`
  - Java/protocol-facing facade and adapter surface
- `src/testing/` — `integration`
  - fixtures in `testing::fixtures`, integration analysis in `testing::harness`
- `src/verification/` — `integration`
  - narrower facades for replay/reconstruction consumers
- `src/bot/harness/` — `experiment`
  - bot-coupled workbenches and validation harnesses promoted out of `testing`
- `src/bin/` — `integration`
  - explicit binary entrypoints, now one directory per binary
- `src/bot/` — `experiment`
  - search, policy, and sidecar logic
- `src/cli/coverage_tools/` — `experiment`
  - offline replay/live-comm coverage record extraction and report output

## Current Notes

- `fixtures` is exported from `lib.rs`, but its implementation still lives under `src/testing/`
- `bot` and `cli` are important working surfaces, but they are downstream of protocol/importer truth
- older design notes may still describe `diff::state_sync` as if it were a repair layer; current docs treat it as a strict importer

## Root Rules

- no loose live-comm captures in the repo root
- no root-level historical implementation snapshots
- no root-level generated audits if they belong in `tools/artifacts/` or `logs/`
- root should hold only build/config files and primary project docs
