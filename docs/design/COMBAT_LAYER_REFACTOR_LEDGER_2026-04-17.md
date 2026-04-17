# Combat Layer Refactor Ledger

This file is the first migration ledger for the combat architecture reset.

The goal is not incremental bugfixing around the current snapshot-driven model.
The goal is to separate:

- engine truth
- content semantics
- projection
- protocol adapters
- verification
- bot policy

## Keep As Core

- `src/runtime/`
  - base combat state, action queue, RNG, and entity storage stay in core
  - this remains the host for executable truth, not protocol repair
- `src/engine/`
  - keep the scheduling and rule execution path
  - move it toward consuming explicit specs instead of preview-oriented fields
- `src/content/monsters/`
  - keep monster-specific move rules and execution logic
  - this should migrate from `Intent + next_move_byte` toward `MonsterMoveSpec + move selection state`
- `src/content/powers/`
  - keep as engine/content truth
  - runtime-only latches still belong here or in adjacent runtime state, not in replay code

## New Core Slices

- `src/semantics/`
  - new home for explicit truth-side action and move specs
  - first slice landed: `combat::MonsterMoveSpec`, `AttackSpec`, `MonsterTurnPlan`
- `src/projection/`
  - new home for preview and audit derivations
  - first slice landed: `combat::MonsterMovePreview`

## Move Behind Adapters

- `src/diff/protocol/`
  - keep as Java schema and adapter surface only
  - it should not define runtime semantics
- `src/diff/state_sync/`
  - narrow to importer duties
  - current `build::monster` still writes snapshot intent, preview damage, protocol identity, and monster runtime latches directly into `MonsterEntity`
  - long term this should split into:
    - truth adapter input
    - observation adapter input
- `src/testing/scenario.rs`
  - should stop being a primary constructor for runtime truth
  - replay fixtures belong under verification once truth import is explicit

## Rewrite

- `src/runtime/combat.rs`
  - `MonsterEntity` currently mixes truth, observation, preview, and protocol identity
  - target end state:
    - executable truth fields
    - explicit turn plan/spec
    - no preview cache on the truth object
- `src/content/monsters/mod.rs`
  - `MonsterBehavior::roll_move` and `take_turn` still orbit `Intent` and `next_move_byte`
  - target end state should be explicit semantic move specs
- `src/bot/combat/monster_belief.rs`
  - current logic mixes visible intent handling with protocol-seeded hidden-state reconstruction
  - bot should consume truth exports and projection outputs, not runtime/protocol hybrids
- `src/cli/live_comm/combat.rs`
  - currently mixes live protocol handling, bot diagnostics, parity inspection, and belief logging
  - split toward:
    - protocol IO
    - verification hooks
    - bot diagnostics on top of projected truth
- `src/diff/live_comm_replay.rs`
  - keep the verification role, but strip any remaining architectural pressure on runtime types

## Retire

- `MonsterEntity.intent_preview_damage`
  - compatibility-only field after the first semantics/projection slice
  - target is a derived preview object, not a truth field
- direct bot reads of `current_intent`, `next_move_byte`, and protocol-seeded runtime flags as a combined truth source
- any replay/live sync code that needs to silently guess missing truth from prior Rust state

## First Concrete Targets

1. Replace visible intent consumers with `MonsterTurnPlan` + `MonsterMovePreview`.
2. Stop adding new features to `Intent` as a truth model.
3. Split `diff/state_sync` monster import into truth inputs vs observation inputs.
4. Move replay/live parity reporting under an explicit verification namespace.
5. Rewrite monster execution entry points to take semantic specs rather than preview-shaped fields.

## Files To Watch Closely

- `src/runtime/combat.rs`
- `src/content/monsters/mod.rs`
- `src/content/monsters/factory.rs`
- `src/engine/action_handlers/spawning.rs`
- `src/diff/state_sync/build/monster.rs`
- `src/diff/live_comm_replay.rs`
- `src/testing/scenario.rs`
- `src/bot/combat/monster_belief.rs`
- `src/cli/live_comm/combat.rs`
