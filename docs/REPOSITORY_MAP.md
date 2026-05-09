# Repository Map

This file is the current structure blueprint for the repo.

## Tags

- `core`
  - directly implements simulator/runtime truth
- `integration`
  - protocol sync, replay, CLI, and verification around runtime truth
- `tooling`
  - offline analysis, extraction, DecisionRecord capture, and dev utilities
- `experiment`
  - workbenches and topic-specific probes
- `artifact`
  - generated outputs, captures, reports, datasets, and logs
- `legacy`
  - preserved but not primary sources of truth
- `generated`
  - machine-built code or schemas that should not own business rules

## Active Paths

### Engine Truth Path

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

These are consumers, not sources of engine truth:

- `src/bot/`
- `src/cli/`
- `src/bin/`

### AI / Eval Infrastructure Path

The active AI-facing path is infrastructure only:

1. line-protocol full-run driver
   - `src/bin/full_run_env_driver/`
2. legal observation, action candidates, transition records
   - `src/verification/decision_env.rs`
3. collection and replay checks
   - `tools/learning/collect_decision_records.py`
   - `tools/learning/collect_decision_records_batch.py`
   - `tools/learning/audit_decision_record_contract.py`
   - `tools/learning/verify_decision_records_replay.py`

This path does not contain a trusted policy learner.

## Top-Level Layout

- `src/` - `core` plus downstream app/workbench consumers
- `tests/` - `integration`
- `tools/` - `tooling`
- `docs/` - `tooling`
- `logs/` - `artifact`
- `tmp/` - `artifact`
- `data/` - `artifact`

## `src/` Ownership

- `src/runtime/` - `core`
- `src/core/` - `core`
- `src/engine/` - `core`
- `src/content/` - `core`
- `src/state/` - `core`
- `src/semantics/` - `core`
- `src/projection/` - `core`
- `src/diff/` - `integration`
- `src/protocol/` - `integration`
- `src/testing/` - `integration`
- `src/verification/` - `integration`
- `src/bin/` - `integration` entrypoints and workbenches
- `src/bot/` - combat diagnostics/search experiment only
- `src/bot/harness/` - combat/eval experiment
- `src/cli/coverage_tools/` - `experiment`

## Current Notes

- `bot` and `cli` are downstream of protocol/importer truth.
- Hand-written macro-policy modules have been removed from the active bot tree.
- Remaining bot code is not a teacher for reward/shop/event/path/campfire/boss
  relic choices.
- Older learning docs may describe removed paths. Current entrypoints win.

## Root Rules

- no loose live-comm captures in the repo root
- no root-level historical implementation snapshots
- no root-level generated audits if they belong in `tools/artifacts/` or `logs/`
- root should hold only build/config files and primary project docs
