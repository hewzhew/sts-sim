# Pre-Battle API Redesign

This document defines the next-step redesign for monster pre-battle behavior.

It exists because the current API is too narrow:

- it can only see the current monster
- it cannot inspect or initialize other monsters in the encounter
- it cannot express encounter-scoped pre-battle truth cleanly
- it encourages workarounds in factory/build code and engine side effects

`GremlinLeader` exposed this directly, but it is not a one-off problem.

Relevant future monsters in act 2 and later will hit the same wall:

- `GremlinLeader`
- `BronzeAutomaton`
- `TheCollector`
- any summon leader that initializes allies
- any monster whose Java `usePreBattleAction()` depends on full encounter state

## Current Problem

Today the trait shape is:

```rust
fn use_pre_battle_action(
    entity: &MonsterEntity,
    hp_rng: &mut StsRng,
    ascension_level: u8,
) -> Vec<Action>
```

And it is called from:

- [cards.rs](D:\rust\sts_simulator\src\engine\action_handlers\cards.rs)
- [spawning.rs](D:\rust\sts_simulator\src\engine\action_handlers\spawning.rs)

That API is sufficient only for self-contained effects like:

- `CurlUp`
- `Malleable`
- `Mode Shift`
- `Flight`

It is **not** sufficient for monsters whose Java pre-battle logic:

- reads other monsters
- modifies other monsters
- assigns runtime relationships between monsters
- depends on stable encounter slot/identity information

## Concrete Failure Mode

`GremlinLeader.java` does this in `usePreBattleAction()`:

1. captures the two existing gremlins into `this.gremlins[0]` and `this.gremlins[1]`
2. applies `MinionPower` to those allies

The current Rust API cannot do that honestly, because:

- it has no combat-state view
- it has no ally iteration
- it has no encounter-scoped context object

So any current workaround must leak into:

- factory initialization
- spawn handlers
- monster-specific engine hacks

That is exactly the kind of drift we have been trying to remove.

## Design Goal

Pre-battle logic should follow the same rule as take-turn logic:

- semantic monsters should receive enough context to execute truthfully
- monster files should own monster-specific semantics
- factory and engine should not carry monster-specific initialization hacks

This redesign is not about making pre-battle "more flexible".

It is about restoring a single semantic ownership boundary.

## Hard Rules

### Rule 1: Pre-battle semantics may inspect the encounter

If Java `usePreBattleAction()` depends on the encounter, the Rust semantic entry point must also be able to inspect the encounter.

Do not force that logic back into:

- factory
- state_sync
- generic engine handlers

### Rule 2: Pre-battle semantics may update other monsters only through actions

Even with full encounter access, pre-battle logic should not directly mutate combat state in monster files.

It should still emit actions.

That keeps:

- replayability
- action logging
- execution ordering
- semantic/debug visibility

aligned with the rest of the engine.

### Rule 3: Encounter-owned truth stays in combat state, not in local monster scratch fields

If pre-battle logic needs to remember encounter relationships, the remembered truth must live in:

- combat state
- monster runtime state
- protocol identity/runtime truth

Not in temporary local variables hidden inside a one-shot pre-battle function.

### Rule 4: Factory should only build initial encounter shape, not semantic pre-battle effects

Factory should still decide:

- which monsters exist
- initial HP
- initial positions

Factory should **not** be the long-term home for:

- leader marks allies as minions
- leader registers summon slots
- leader assigns runtime bindings

Those belong to semantic pre-battle logic.

## Implemented Phase 1 API Shape

The codebase now routes all pre-battle execution through a single public resolver:

```rust
fn resolve_pre_battle_actions(
    state: &mut CombatState,
    id: EnemyId,
    entity: &MonsterEntity,
    legacy_rng: PreBattleLegacyRng,
) -> Vec<Action>
```

And monster files now target this semantic hook:

```rust
fn use_pre_battle_actions(
    state: &mut CombatState,
    entity: &MonsterEntity,
    legacy_rng: PreBattleLegacyRng,
) -> Vec<Action>
```

Notes:

- `state` is mutable because the migration shim may need to consume the correct combat RNG.
- `entity` remains the semantic owner.
- the function still returns actions only.
- the old scalar hook survives only as an internal compatibility adapter.

This is the current preferred API for new monsters.

## Why The Migration Hook Still Carries `legacy_rng`

Most Java `usePreBattleAction()` logic does not roll AI randomness.

But several migrated and unmigrated monsters still rely on the older scalar hook:

- `fn use_pre_battle_action(entity, rng, ascension_level)`

To avoid keeping two public resolvers alive, phase 1 internalizes that fallback inside the default implementation of `use_pre_battle_actions(...)`.

That means:

- new monsters can ignore `legacy_rng`
- old monsters keep compiling through the trait default
- the engine only needs one pre-battle entry point

The long-term goal is still to retire the scalar hook entirely.

## Dispatch Shape

Old resolver:

```rust
resolve_pre_battle_action(id, entity, hp_rng, ascension_level)
```

Current replacement:

```rust
resolve_pre_battle_actions(state, id, entity, legacy_rng)
```

Called from:

- `Action::PreBattleTrigger`
- spawn path for newly spawned monsters, if Java would call `usePreBattleAction()` there

The central change is:

- pre-battle becomes **state-aware semantic dispatch**
- not a narrow helper with scalar parameters

## Action Requirements

This redesign assumes the existing action model remains the main execution boundary.

That means pre-battle logic should use:

- `Action::ApplyPower`
- `Action::SetMonsterMove`
- `Action::UpdateMonsterRuntime`
- `Action::SpawnMonsterSmart`
- existing generic actions where possible

If a monster needs a pre-battle effect that cannot be expressed by current actions,
the right response is:

- add a missing action or runtime patch
- not leak monster-specific mutation into resolver/factory

## Migration Plan

### Phase 0: Design freeze

Do this first:

- define the new API
- keep old API untouched
- do not migrate every monster immediately

This document is that freeze.

### Phase 1: Internalize the old scalar fallback

Done:

- `MonsterBehavior::use_pre_battle_actions(state, entity, legacy_rng) -> Vec<Action>`
- `resolve_pre_battle_actions(state, id, entity, legacy_rng)`

The old scalar hook now exists only behind the trait default implementation.

### Phase 2: Migrate known encounter-aware monsters first

High-priority targets:

1. `GremlinLeader`
2. `BronzeAutomaton`
3. `TheCollector`
4. `Reptomancer` if needed

Reason:

- these are the monsters most likely to need encounter-aware pre-battle truth
- they are also the monsters most likely to regress if semantics stay split

### Phase 3: Migrate simple self-only monsters opportunistically

Examples:

- `Byrd`
- `SphericGuardian`
- `Louse`
- `FungiBeast`

These are not urgent, because the old API is already expressive enough for them.

But once the new API exists, they can be moved for consistency.

### Phase 4: Delete the old scalar pre-battle hook

Only after:

- all semantic monsters are on the new hook
- unsupported monsters are either migrated or intentionally still legacy

Then remove:

- old trait hook
- old resolver
- scalar argument plumbing

## Required Engine Changes

Minimal required changes:

1. `handle_pre_battle_trigger(state)` should call:
   - `resolve_pre_battle_actions(state, enemy_id, entity)`

2. spawn-time pre-battle in [spawning.rs](D:\rust\sts_simulator\src\engine\action_handlers\spawning.rs) should also call the new resolver

3. all call sites should pass a read-only snapshot or immutable view of `CombatState`

The important constraint is:

- monster files still return actions
- execution still happens centrally through the engine

## Interaction With Runtime-State Framework

This redesign complements, not replaces:

- [MONSTER_RUNTIME_STATE_FRAMEWORK.md](D:\rust\sts_simulator\docs\design\MONSTER_RUNTIME_STATE_FRAMEWORK.md)

Relationship:

- runtime-state framework governs hidden truth import and execution updates
- pre-battle redesign governs encounter-aware initialization semantics

Together they define:

- who owns pre-battle encounter initialization
- where hidden monster truth may be established
- how that truth is maintained after combat begins

## What This Design Explicitly Avoids

### Not a generic "context bag"

Do not invent a giant mutable pre-battle context object first.

That tends to become an untyped side channel.

Start with:

- `&CombatState`
- `&MonsterEntity`
- returned `Vec<Action>`

Only add more if a real monster forces it.

### Not direct in-place mutation from monster files

Do not move to:

```rust
fn use_pre_battle_action(state: &mut CombatState, ...)
```

as the default shape.

That would immediately weaken:

- action traceability
- replay semantics
- debugging visibility

### Not factory-driven semantic initialization

Do not respond to this problem by teaching factory about:

- leader/minion bindings
- ally initialization powers
- monster-specific pre-battle semantics

That is short-term convenient and long-term wrong.

## Initial Success Criteria

This redesign is successful when all of the following are true:

1. `GremlinLeader` can apply initial `MinionPower` to existing gremlins through semantic pre-battle logic
2. no new monster-specific pre-battle hacks are added to factory
3. no new monster-specific pre-battle hacks are added to generic engine handlers
4. the old scalar pre-battle API can be retired after migration

## Immediate Recommendation

Do **not** implement the full migration everywhere now.

Do this next:

1. add the new encounter-aware pre-battle hook and resolver
2. migrate `GremlinLeader` first
3. use that implementation to validate the pattern
4. then apply the same pattern to act 2 summon leaders and boss setups

That is the smallest move that prevents larger future rework.
