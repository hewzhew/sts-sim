# Act 3 Monster Source Audit

Date: 2026-05-17

This audit is Java-source-driven. The Java source root is `D:/rust/cardcrawl/monsters/beyond`,
and the Rust implementation root is `src/content/monsters/beyond`.

The goal is source parity for combat mechanics, not UI parity. Animation, VFX, SFX, dialogue,
screen shake, and `MathUtils` render randomness are omitted unless they mutate combat state or
consume gameplay RNG. `AbstractDungeon.aiRng`, queued action order, monster private fields, summon
slot state, ascension thresholds, death/escape flags, and combat RNG consumption are treated as
mechanics.

## Current Pass Scope

Covered in this pass:

- Exploder
- Repulsor
- Spiker
- Orb Walker
- Maw
- Spire Growth
- Writhing Mass / Reactive
- Darkling
- Time Eater
- Donu / Deca
- Reptomancer / Snake Dagger
- Giant Head
- Nemesis
- Transient
- Awakened One

## Source Findings

### Exploder

Java uses a private `turnCount` that makes the first two turns attacks and the third turn the
explosion intent. Rust derives this from the planned move history. In the normal decision-boundary
flow this matches Java timing: initial roll is an attack, after the first executed turn the next roll
is still attack, and after the second executed turn the next roll is explosion.

### Repulsor

Java `Repulsor.takeTurn()` queues `MakeTempCardInDrawPileAction(new Dazed(), amount, true, true)`.
Rust maps this to `CardDestination::DrawPileRandom`, through the shared Java-index-aware draw-pile
helper. No change was needed.

### Spiker

Java keeps private `thornsCount` and increments it only inside `takeTurn()` when `BUFF_THORNS`
executes. Rust now carries this as explicit `SpikerRuntimeState` instead of deriving it from move
history. This matters because Java `setMove` appends to move history when a move is planned; a
planned-but-unexecuted thorns buff must not count as an executed buff for the `thornsCount > 5`
attack-forcing rule.
`CommunicationMod` exports Java's private `thornsCount` as `monster.runtime_state.thorns_count`,
and Rust state sync treats it as strict protocol truth.

### Shapes Encounter Construction

Java `MonsterHelper.spawnShapes(weak)` builds its draw-without-replacement pool in this exact order:

```text
Repulsor, Repulsor, Exploder, Exploder, Spiker, Spiker
```

This is not UI-only: the pool index is rolled with `AbstractDungeon.miscRng`, so the initial pool
order changes the actual monster composition for the same run seed. Rust now uses the Java order for
both `ThreeShapes` and `FourShapes`. `randomXOffset/randomYOffset` in nearby helper code uses
LibGDX `MathUtils.random`, not `AbstractDungeon.miscRng`, so those render-position rolls remain
intentionally omitted from simulator RNG.

### Orb Walker

Java Laser queues one `MakeTempCardInDiscardAndDeckAction(new Burn())`. That action creates a
draw-pile Burn copy first and then a discard-pile Burn copy.

Rust previously expanded Laser into two separate `AddCard` actions in discard-then-draw order. That
could change generated-card UUID order, Master Reality hooks, and random draw-pile insertion order.
Rust now emits the canonical `Action::MakeTempCardInDiscardAndDeck { card_id: Burn, amount: 1 }`
for Laser.

### Maw

Java starts `turnCount` at 1, increments it inside `getMove`, and uses it to scale Nom. Rust derives
the planned and executed Nom hit counts from move history. In normal decision-boundary flow this
matches the Java increment timing.

### Spire Growth

Spire Growth matches the Java source:

- Constrict amount is 12 at A17+, otherwise 10.
- A17 prioritizes Constrict when the player lacks Constricted and the previous move was not
  Constrict.
- Quick Tackle and Smash damage thresholds match the Java ascension checks.

### Writhing Mass / Reactive

Java has private `usedMegaDebuff`, initially false, and sets it to true only inside
`WrithingMass.takeTurn()` when the Parasite move actually executes.

Rust previously inferred "used Mega Debuff" from move history. That is wrong because Writhing Mass
has `ReactivePower`: player attacks can roll the visible intent into Mega Debuff and then roll it
away before the monster executes. Java does not consume `usedMegaDebuff` in that case.

Rust now carries explicit `WrithingMassRuntimeState`:

- `used_mega_debuff`
- `protocol_seeded`

The runtime flag is set only by the Mega Debuff take-turn action, before adding Parasite to the
master deck. Search/memo state keys include this runtime field, so branches where Parasite has been
executed are not merged with branches where Mega Debuff was only a transient Reactive intent.
`CommunicationMod` exports Java's private `usedMegaDebuff` as
`monster.runtime_state.used_mega_debuff`, and Rust state sync treats it as strict protocol truth.

### Darkling

Darkling already carries explicit runtime state for Java private fields:

- `first_move`
- `nip_dmg`

Rust now treats these as strict runtime truth. Factory/test construction and live state import mark
the slice as seeded, `roll_move` asserts that the slice is seeded, and the opening `firstMove`
transition is emitted through `Action::UpdateMonsterRuntime` only when Java `getMove()` would
actually consume the opening branch. Generic `SetMonsterMove` and `RollMonsterMove` no longer clear
`first_move` as a side effect.

The half-death / revive flow is intentionally modeled as runtime combat state rather than UI state.
`Regrow` uses the existing sentinel power amount convention in Rust.

### Time Eater

Java private fields:

- `usedHaste`
- `firstTurn`

`firstTurn` only controls dialogue and is omitted as non-mechanical. `usedHaste` is set during
`getMove()` when Haste is selected. Rust reconstructs this from move history; because Time Eater has
no Reactive-style reroll of its own intent, this matches the normal roll boundary. Haste heal amount
is read at take-turn queue time, matching Java `HealAction(this.maxHealth / 2 - this.currentHealth)`.

### Donu / Deca

Donu and Deca alternate via private `isAttacking` booleans. Rust derives the same alternation from
move history:

- Donu starts with Circle of Protection, then Beam.
- Deca starts with Beam, then Square of Protection.

Pre-battle Artifact amounts match A19, and Deca's Square action preserves Java's per-monster
block-then-Plated-Armor ordering at A19+.

### Reptomancer / Snake Dagger

Reptomancer source-backed details preserved:

- first move is always Spawn Dagger and comes from Java private `firstMove`;
- A18+ spawns two daggers, otherwise one;
- `canSpawn()` counts all non-self monsters that are not `isDying`, matching Java's flag check;
- dagger slot reuse uses Java private `daggers[4]`, not draw-position inference; a slot is free
  when its stored dagger is null or `isDeadOrEscaped()`;
- death cleanup queues suicide for every non-dying ally.

`CommunicationMod` exports `runtime_state.first_move` and `runtime_state.dagger_slots` from the
Java Reptomancer object. Rust imports those slots by Java monster instance id, stores them in
`ReptomancerRuntimeState`, and updates the slot array through `Action::SpawnReptomancerDagger`
when Spawn Dagger executes.

Snake Dagger source-backed details preserved:

- first move Wound attack, second move Explode;
- Explode queues player damage and `LoseHPAction(this, this, currentHealth)`, not `SuicideAction`;
- Wound is added to discard.

### Giant Head

Java has private `count`, starting at 5 and reduced to 4 at A18+ during pre-battle. It decrements
inside `getMove()` before selecting/counting the attack. Rust derives count from move history and
the A18 starting offset. This matches Java roll timing in normal decision-boundary flow.

### Nemesis

Java has private `scytheCooldown`, decremented at the start of every `getMove()` and reset to 2
when Scythe is selected. Rust reconstructs this cooldown from the last Scythe in move history. This
matches the Java timing at roll boundaries.

Nemesis also reapplies Intangible after every turn only when it does not already have Intangible;
Rust preserves that guard.

### Transient

Java Transient does not queue `RollMoveAction`. It directly increments its attack counter and sets
the next move inside `takeTurn()`. Rust preserves this with `SetMonsterMove` rather than a normal
roll. Fading amount is 6 at A17+, otherwise 5; Shifting is applied pre-battle.

### Awakened One

Awakened One runtime truth is explicit:

- `form1`
- `first_turn`

The first-form death path is split between damage/death power hooks and the later Rebirth move, as
in Java:

- `Unawakened` queues `ClearCardQueue`, clears the appropriate powers, sets Rebirth, and switches
  runtime to form 2 first turn.
- Rebirth revives and heals to max.
- Final death makes non-dying Cultists escape.

## Follow-Up Watch Points

- Act 3 live/protocol runtime truth for `WrithingMass.usedMegaDebuff` and `Spiker.thornsCount` is
  now exported and strictly imported. Future Act 3 monster fixes should continue using this pattern:
  Java private field -> protocol `runtime_state` -> Rust runtime field, not move-history inference
  when planning and execution can diverge.
