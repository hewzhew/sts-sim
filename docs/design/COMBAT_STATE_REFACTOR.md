# `CombatState` Refactor Plan

## Status

Phase 1 is already complete:

- `CombatState` is already a composition root
- `turn`, `engine`, `zones`, `entities`, `meta`, and `rng` already exist as sub-objects

The current problem is no longer “introduce sub-structs”. It is:

- too many call sites still mutate those sub-objects like a semi-flat bag of fields
- ownership is still implicit even though the layout is no longer flat

This document therefore describes the ongoing **phase 2 ownership cleanup**:

- keep the current nested layout
- narrow mutation surfaces
- move common queue/turn operations behind subsystem-owned APIs

## Problem

[`CombatState`](D:/rust/sts_simulator/src/runtime/combat.rs) is currently the default landing zone for almost every new combat concern:

- turn flow
- card zones
- entities
- powers
- action queue
- RNG
- encounter metadata
- small runtime counters
- new derived state such as `turn_start_draw_modifier`

This is not just a size problem. It is a **state ownership** problem.

The main failure mode is:

- a new mechanic appears
- there is no clear subsystem boundary
- the quickest move is to add one more top-level field or one more helper on `CombatState`

That keeps momentum in the short term, but it steadily makes:

- invariants harder to define
- state sync harder to reason about
- test fixtures more expensive to construct
- live parity bugs harder to localize

## Current State Buckets Inside `CombatState`

The current nested fields already imply several different ownership domains:

### 1. Turn / player-turn runtime

- `turn_count`
- `current_phase`
- `energy`
- `counters`
- `turn_start_draw_modifier`

### 2. Card zone state

- `draw_pile`
- `hand`
- `discard_pile`
- `exhaust_pile`
- `limbo`
- `queued_cards`
- `card_uuid_counter`

### 3. Entity state

- `player`
- `monsters`
- `potions`
- `power_db`

### 4. Engine runtime

- `action_queue`

### 5. Combat metadata

- `ascension_level`
- `is_boss_fight`
- `is_elite_fight`
- `meta_changes`

### 6. RNG state

- `rng`

These should not all continue to evolve as siblings.

## Root Design Smells

### Flat top-level growth

Every concern looks equally “native” to `CombatState`, even when it is really only owned by one subsystem.

### Cross-layer mutation

Files in very different domains directly mutate unrelated fields:

- engine core
- action handlers
- content powers
- relic hooks
- state sync
- tests

That means ownership is implicit and unenforced.

### Weak invariants

Examples:

- When `power_db` changes, what derived state must be recomputed?
- What is the canonical owner of queue-sensitive card residency?
- Which fields are runtime-only and should never be part of snapshot truth?

### Expensive fixture construction

Because `CombatState` is flat, tests and builders must know too much about unrelated fields just to create a legal value.

## Refactor Goal

Do **not** delete `CombatState` or flatten it again.

Keep it as a composition root with explicit sub-objects:

```rust
pub struct CombatState {
    pub meta: CombatMeta,
    pub turn: TurnRuntime,
    pub zones: CardZones,
    pub entities: EntityState,
    pub engine: EngineRuntime,
    pub rng: CombatRng,
}
```

That structural split is already present. The current key idea is:

- `CombatState` remains the external handle
- routine operations should stop reaching through it into raw nested fields
- future mechanics must first choose a subsystem-owned operation, not a random nested slot

## Proposed Target Structure

### `CombatMeta`

Owns encounter-level information that is stable for the fight and not part of the tactical runtime loop.

Fields:

- `ascension_level`
- `is_boss_fight`
- `is_elite_fight`
- `meta_changes`

### `TurnRuntime`

Owns the player-turn state machine and per-turn combat bookkeeping.

Fields:

- `turn_count`
- `current_phase`
- `energy`
- `counters`
- `turn_start_draw_modifier`

Why:

- `turn_start_draw_modifier` does not belong with powers directly; it is a turn-start consumption state
- per-turn counters and phase transitions are strongly coupled already

### `CardZones`

Owns all combat card residency and queue-adjacent zone semantics.

Fields:

- `draw_pile`
- `hand`
- `discard_pile`
- `exhaust_pile`
- `limbo`
- `queued_cards`
- `card_uuid_counter`

Why:

- queue-sensitive mechanics and pile movement need one ownership domain
- future full cardQueue support should land here, not at top level

### `EntityState`

Owns persistent combat entities and their attached powers.

Fields:

- `player`
- `monsters`
- `potions`
- `power_db`

Why:

- powers are entity-attached state, not engine runtime
- long-term, `power_db` may itself deserve further splitting, but it belongs here first

### `EngineRuntime`

Owns execution machinery, not game truth.

Fields:

- `action_queue`

Future candidates:

- pending animation/runtime queues
- deferred cleanup queues
- execution diagnostics

### `CombatRng`

Owns combat RNG streams.

Fields:

- `rng`

Why:

- RNG is runtime infrastructure, not entity state or turn state
- keeping it isolated makes future replay/debug tooling cleaner

## What This Refactor Is Trying To Prevent

### Bad pattern

“Need a new mechanic, add one more top-level field.”

Examples of risky future variants:

- `next_turn_energy_bonus`
- `draw_denial_override`
- `queued_trigger_mask`
- `pending_retarget_choice`
- `combat_trace_flags`

### Better pattern

“Need a new mechanic, identify its owning subsystem first.”

Examples:

- `Machine Learning` support: `turn.turn_start_draw_modifier`
- future full card queue semantics: `zones`
- execution-only debug queue: `engine`

## Migration Strategy

This should be done in phases. Do **not** switch every callsite in one shot.

### Phase 0: design freeze

Before moving fields, freeze the grouping plan in this document. New state additions should reference the intended subsystem here.

### Phase 1: introduce sub-structs with facade access

Completed.

`TurnRuntime` and `EngineRuntime` already exist, along with the other major sub-objects.

The new migration rule is:

- it is acceptable to add narrow subsystem operations during the cleanup
- it is **not** acceptable to reintroduce flattened convenience access or duplicate truth

### Phase 2: move `CardZones`

Second target:

- `draw_pile`
- `hand`
- `discard_pile`
- `exhaust_pile`
- `limbo`
- `queued_cards`
- `card_uuid_counter`

Reason:

- this is where future queue work will land
- it reduces the temptation to spread card residency logic across random files

### Phase 3: move `EntityState`

Third target:

- `player`
- `monsters`
- `potions`
- `power_db`

This is higher risk because the reference surface is huge.

Only do this after the previous phases prove the migration style is workable.

### Phase 4: move `CombatMeta` and `CombatRng`

These are mostly cleanup phases once the main structural ownership is in place.

## Strict Rules During Migration

### 1. No duplicated truth

If a field moves into `turn`, do not keep a stale mirror at top level “for convenience”.

### 2. No broad helper dumping

Avoid replacing field moves with a large pile of unrelated `CombatState::*` helpers.

Helpers are acceptable only when they:

- preserve invariants during migration
- encode a real subsystem operation

Not when they merely hide continued flat ownership.

### 3. Update builders and tests early

The following should be updated as first-class consumers, not as afterthoughts:

- state sync builders
- engine test support builders
- scenario builders
- `play.rs` combat constructors

These files reveal whether the new ownership split is actually reducing construction complexity.

### 4. Document workflow-impacting changes

Any phase that changes:

- how `CombatState` is constructed
- where turn state lives
- where queue state lives

must update the relevant docs:

- [DRAW_HAND_SIZE_DESIGN.md](DRAW_HAND_SIZE_DESIGN.md) when draw-target semantics move
- [LIVE_COMM_RUNBOOK.md](../live_comm/LIVE_COMM_RUNBOOK.md) only if live/debug workflow changes
- this file when subsystem boundaries or migration order change

## Recommended Next Real Refactor

Start with a bounded ownership cleanup:

### Step 1

Add narrow helper APIs around:

- queue enqueue/dequeue/order-preserving batching
- turn phase transitions
- next-player-turn setup/reset

### Step 2

Update only:

- [core.rs](D:/rust/sts_simulator/src/engine/core.rs)
- selected action handlers that directly participate in queue/turn flow
- state sync build/sync
- test builders

### Step 3

Do not broaden the pass into `CardZones` or `EntityState` ownership cleanup yet.

That keeps the first cleanup bounded enough to validate the ownership direction without turning into a repo-wide churn bomb.

## Expected Benefits

If the plan is followed, the likely wins are:

- new runtime state must declare an owner
- turn/draw/energy work becomes less invasive
- queue work has a clear future home
- test builders become more legible
- live parity bugs become easier to localize by subsystem

## Expected Costs

This is not free. The main costs are:

- broad mechanical edits
- temporary access friction during migration
- many constructor/test updates
- some helper churn while old callsites are being moved

These costs are acceptable because the current cost curve is worse: every new mechanic keeps making the flat `CombatState` more implicit and more fragile.

## Bottom Line

The right question is no longer:

- “Is `CombatState` too big?”

It is:

- “Does `CombatState` still enforce meaningful ownership?”

Right now, it does not.

The correct fix is not a giant rewrite, and not more random top-level fields.

It is a staged ownership refactor with `CombatState` kept as the outer shell and subsystem state moved underneath it in a deliberate order.
