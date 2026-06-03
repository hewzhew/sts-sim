# Draw Reduction / Draw / `gameHandSize`

## Scope

This note covers the Java semantics around:

- `DrawReductionPower`
- `DrawPower`
- `GameActionManager` start-of-turn draw via `player.gameHandSize`

and how they should map into the Rust simulator.

The goal is to decide whether Rust should add an explicit `gameHandSize` runtime field now, or keep a lighter-weight design.

## Java Semantics

Relevant files:

- [DrawReductionPower.java](D:/rust/cardcrawl/powers/DrawReductionPower.java)
- [DrawPower.java](D:/rust/cardcrawl/powers/DrawPower.java)
- [GameActionManager.java](D:/rust/cardcrawl/actions/GameActionManager.java)

### `GameActionManager`

At player turn start, Java queues:

1. start-of-turn relic/card/power/orb hooks
2. `DrawCardAction(null, AbstractDungeon.player.gameHandSize, true)`
3. post-draw relic/power hooks

This means Java does not hardcode `5` at the final draw site. It consumes a mutable player-level draw target.

### `DrawReductionPower`

Java behavior:

- constructor marks it as `DEBUFF`
- `onInitialApplication()`: `--player.gameHandSize`
- `atEndOfRound()`: after the first grace round, queue `ReducePowerAction(..., 1)`
- `onRemove()`: `++player.gameHandSize`

So the power changes the player's start-of-turn draw target through apply/remove side effects, not by being queried ad hoc at draw time.

### `DrawPower`

Java behavior:

- constructor mutates `player.gameHandSize += amount`
- `onRemove()`: `player.gameHandSize -= amount`
- positive amount behaves like a `BUFF`
- negative amount behaves like a `DEBUFF`

This is a more general "modify next start-of-turn draw amount" mechanism than `DrawReductionPower`.

## Current Rust Model

Relevant files:

- [core.rs](D:/rust/sts_simulator/src/engine/core.rs)
- [cards.rs](D:/rust/sts_simulator/src/engine/action_handlers/cards.rs)
- [mod.rs](D:/rust/sts_simulator/src/content/powers/mod.rs)

Rust currently does:

- compute turn-start draw count from a hardcoded base `5`
- add `Snecko Eye`
- subtract visible `DrawReduction.amount`
- queue `Action::DrawCards(draw_count)`
- use hand cap `10` in `handle_draw_cards(...)`

Rust does **not** model a full persistent player field equivalent to Java's `gameHandSize`.

Rust now does model a narrow explicit state:

- `CombatState::turn_start_draw_modifier`

This is intentionally smaller in scope than Java's mutable `gameHandSize`.

Also:

- `DrawReduction` exists as a `PowerId`
- `DrawPower` does not yet exist as a real Rust runtime power
- `DrawReduction` removal has now been routed through `Action::RemovePower`, rather than direct `retain`, so it no longer bypasses the normal power lifecycle

## Decision

### Short-term decision

Do **not** add a full explicit `gameHandSize` runtime field.

Do add a narrow explicit start-of-turn draw-target modifier.

### Why

Because in the current simulator:

- the only live behavior we need for parity right now is "how many cards do we draw at turn start"
- that behavior is already localized in one place: [core.rs](D:/rust/sts_simulator/src/engine/core.rs)
- the rest of the engine uses actual hand length and hand cap `10`, not a mutable draw-target state
- a full Java-style field would spread invariants across:
  - player state
  - state sync
  - snapshots
  - scenario schema
  - live parity comparison
  - synthetic fixtures

That is a large surface area for a semantic that is not yet widely exercised outside `DrawReduction` / `Draw`.

At the same time, leaving the concept purely derived was becoming too weak for future `Defect` work such as `Machine Learning`.

## Recommended Rust Design

### Phase 1: narrow explicit draw-target state

Keep Rust's full state surface narrow, but formalize start-of-turn draw targeting with:

- `CombatState::turn_start_draw_modifier`
- `compute_player_turn_start_draw_count(state: &CombatState) -> i32`

Target API:

```rust
fn compute_player_turn_start_draw_count(state: &CombatState) -> i32
```

This helper should own:

- base `5`
- `Snecko Eye`
- `CombatState::turn_start_draw_modifier`

The modifier should own only effects that truly change next-turn draw target, such as:

- `DrawReduction`
- future `DrawPower`

This is the right immediate abstraction because it:

- matches the only place Rust currently consumes the concept
- avoids leaking a full Java-style `gameHandSize` field everywhere
- gives one place to add future parity fixes
- gives `Defect` a real state hook for `Machine Learning` without over-modeling everything else

### Phase 2: explicit draw-target modifiers, not full Java field

If we later implement `DrawPower` or more effects that change next-turn draw amount, extend the current narrow state instead of immediately copying Java's `player.gameHandSize` field 1:1.

Reason:

- Java's `gameHandSize` is a mutable integration point across many classes
- Rust can model the same gameplay result with a narrower, better-scoped state

### Phase 3: only add explicit `gameHandSize` if needed

Add a real persistent runtime field only if all of the following become true:

1. `DrawPower` is implemented and exercised in live parity
2. more than one independent subsystem mutates draw target outside the turn-start draw site
3. synthetic/live fixtures need to represent that value directly
4. derived computation becomes harder to reason about than a persisted field

Until then, explicit `gameHandSize` would be premature.

## Implications for `DrawReduction`

Current status after the latest engine cleanup:

- `DrawReduction` participates in `is_debuff(...)`
- turn-start consumption now flows through `CombatState::turn_start_draw_modifier`
- removal now goes through `Action::RemovePower`, not direct mutation
- build/sync recompute the modifier from visible powers

This is acceptable for now.

What remains incomplete is **not** removal ordering anymore. It is the fact that Java's apply/remove side effects target a draw-target field that Rust does not model.

That gap is acceptable as long as:

- `DrawReduction` is only used for start-of-turn draw count
- no other code path relies on observing an explicit draw-target field

## Implications for `DrawPower`

`DrawPower` is the real threshold case.

If Rust starts implementing content that uses Java `DrawPower`, we should not fake it with ad hoc one-off hooks in scattered places.

At that point:

1. represent `DrawPower` as a contribution to `turn_start_draw_modifier`
2. validate the result in live parity
3. only then evaluate whether a broader persistent field is still needed

## What To Avoid

- Do not spread `DrawReduction` cleanup through direct `retain(...)` calls
- Do not add a full Java-style `gameHandSize` field without also planning:
  - state sync
  - scenario schema
  - live comparator behavior
  - fixture authoring semantics
- Do not treat cache reports marking `DrawReduction` / `Draw` as `missing` as proof that the only missing work is a dispatch hook; the deeper issue is state modeling

## Next Recommended Step

If we continue this line, the next high-value step is:

1. keep `turn_start_draw_modifier` as the only explicit draw-target state
2. route `DrawReduction` / future `DrawPower` through it
3. revisit full `gameHandSize` only if parity demands a broader surface
