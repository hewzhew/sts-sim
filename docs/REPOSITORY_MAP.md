# Repository Map

This file is the current structure blueprint for the repo.

## Tags

- `core`
  - directly implements simulator/runtime truth
- `integration`
  - fixtures, importer helpers, eval surfaces, and binaries around runtime truth
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
4. truth-side move-plan and preview layers
   - `src/runtime/monster_move.rs`
   - `src/sim/`
5. run and reward flow
   - `src/rewards/`
   - `src/events/`
   - `src/state/shop/`
   - `src/map/`

### Fixture / Import Path

1. fixtures and scenario tests
   - `src/testing/`
   - `tests/`
2. private Java/protocol fixture import helpers
   - `src/testing/protocol/`
   - `src/testing/state_sync/`
   - `src/testing/replay_support.rs`

### App / Workbench Path

These are consumers, not sources of engine truth:

- `src/bin/`
- `src/app/`

### AI / Eval Infrastructure Path

The active AI-facing path is infrastructure only:

1. decision environment contract
   - `src/app/decision_env.rs`
2. combat search and env surfaces
   - `src/eval/`
   - `src/bin/combat_env_driver/`
   - `src/bin/combat_search_v2_driver/`
3. historical collection and replay checks
   - `tools/learning/`

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
- `src/state/shop/` - `core`
- `src/runtime/monster_move.rs` - `core`
- `src/sim/` - `core` and AI-facing simulator views
- `src/testing/` - `integration`
- `src/bin/` - `integration` entrypoints and workbenches
- `src/app/` - app contracts and downstream integration surfaces
- `src/eval/` - AI/eval experiments over simulator state

## Current Notes

- Removed top-level live-comm/protocol modules are not active architecture.
- Hand-written macro-policy modules have been removed from the active bot tree.
- `live_comm` is fixture-only legacy unless rebuilt under the documented adapter
  boundary.
- Older learning docs may describe removed paths. Current entrypoints win.

## Root Rules

- no loose live-comm captures in the repo root
- no root-level historical implementation snapshots
- no root-level generated audits if they belong in `tools/artifacts/` or `logs/`
- root should hold only build/config files and primary project docs
