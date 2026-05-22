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
   - `src/state/events/`
   - `src/state/map/`
   - `src/state/shop/`
   - `src/state/rewards/`

### Start-Spec Test Path

1. search fixture assembly
   - `src/testing/`
2. active test-only builders
   - `src/testing/support/`

### Entrypoint Path

These are thin consumers, not sources of engine truth:

- `src/bin/`

### AI / Eval Infrastructure Path

The active AI-facing path is infrastructure only:

1. combat search surfaces
   - `src/eval/`
     - exact combat capture and search benchmark adapters
   - `src/bin/combat_search_v2_driver/`
2. historical collection and replay checks
   - `tools/learning/`

This path does not contain a trusted policy learner.

## Top-Level Layout

- `src/` - `core` plus downstream app/workbench consumers
- `tests/` - `integration` when checked-in integration fixtures exist
- `tools/` - `tooling`
- `docs/` - `tooling`
- `logs/` - `artifact`
- `tmp/` - `artifact`
- `data/` - `artifact`

## `src/` Ownership

- `src/runtime/` - `core`
- `src/runtime/combat/` - `core`; combat runtime records split by state,
  entities, monster private state, cards, powers, and state methods
- `src/core/` - `core`
- `src/engine/` - `core`
- `src/content/` - `core`
- `src/state/` - `core`
- `src/state/events/` - `core`; event state, event context, event pools, and
  event-room roll generation
- `src/state/map/` - `core`; map graph, room node, map progress, and map
  generation
- `src/state/shop/` - `core`
- `src/runtime/monster_move.rs` - `core`
- `src/sim/` - `core` and AI-facing simulator views
- `src/testing/` - `integration`
- `src/bin/` - active integration entrypoints only
- `src/eval/` - AI/eval experiments over simulator state

## Current Notes

- Removed top-level live-comm/protocol modules are not active architecture.
- Hand-written macro-policy modules have been removed from the active bot tree.
- `live_comm` is fixture-only legacy unless rebuilt under the documented adapter
  boundary.
- The old `CombatCase` / `ScenarioFixture` / protocol state-sync testing stack
  has been removed from active code; current search tests start from start-specs.
- Older learning docs may describe removed paths. Current entrypoints win.

## Root Rules

- no loose live-comm captures in the repo root
- no root-level historical implementation snapshots
- no root-level generated audits if they belong in `tools/artifacts/` or `logs/`
- root should hold only build/config files and primary project docs
