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

## RL Main Path

The RL-facing path through the repo is:

1. state build / sync
   - `src/state/`
   - `src/state/semantics.rs`
   - `src/combat.rs`
   - `src/diff/state_sync/`
2. engine progression
   - `src/engine/`
   - `src/action.rs`
3. content semantics
   - `src/content/cards/`
   - `src/content/powers/`
   - `src/content/relics/`
   - `src/content/monsters/`
4. rewards / terminal / run flow
   - `src/rewards/`
   - `src/events/`
   - `src/shop/`
   - `src/engine/run_loop.rs`
5. observation / validation / replay surfaces
   - `src/diff/`
   - `src/testing/`
   - selected `src/bin/*`

Anything outside that path should be treated as support infrastructure, not as the source of engine truth.

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
- `tools/legacy/` — `legacy`
  - preserved old scripts or implementation snapshots

## `src/` Ownership

- `src/engine/` — `core`
  - turn progression, queue driving, action dispatch, room handlers
- `src/content/` — `core`
  - per-entity behavior and hook logic
- `src/state/` — `core`
  - structured run / combat / pending-choice state
- `src/diff/` — `integration`
  - protocol mapping in `diff::protocol`, replay/verification in `diff::replay`, sync support in `diff::state_sync`
- `src/testing/` — `integration`
  - fixtures in `testing::fixtures`, harnesses in `testing::harness`, test-only helpers in `testing::support`
- `src/bin/` — `integration`
  - explicit binary entrypoints, now one directory per binary
- `src/bot/` — `experiment`
  - search, policy, and sidecar logic
- `src/generated/` — `generated`
  - generated tables and protocol-adapter support

## Root Rules

- no loose live-comm captures in the repo root
- no root-level historical implementation snapshots
- no root-level generated audits if they belong in `tools/artifacts/` or `logs/`
- root should hold only build/config files and primary project docs
