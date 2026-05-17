# Act 2 Monster Source Audit

Date: 2026-05-16

This audit is Java-source-driven. The Java source root is `D:/rust/cardcrawl/monsters/city`,
and the Rust implementation root is `src/content/monsters/city`.

The goal is source parity for combat mechanics, not UI parity. Animation, VFX, SFX, screen shake,
dialogue, and `MathUtils` render randomness are omitted unless they mutate combat state or consume
gameplay RNG. `AbstractDungeon.aiRng`, queued action order, monster private fields, summoned
monster initialization, ascension thresholds, and death/escape flags are treated as mechanics.

## Current Pass Scope

Covered in this pass:

- Spheric Guardian
- Chosen
- Byrd / Flight
- Shelled Parasite
- Snake Plant
- Snecko
- Centurion / Healer
- Mugger
- Taskmaster
- Book of Stabbing
- Gremlin Leader
- Bronze Automaton / Bronze Orb / Stasis
- Champ
- The Collector / Torch Head
- Masked Bandits: Bear, Leader, Pointy

## Source Findings

### Spheric Guardian

Java `SphericGuardian.takeTurn()` for Bash and Block queues `GainBlockAction(15)` before the
attack. Rust previously returned the attack first and block second.

Rust now emits the block action before the attack for `BASH_AND_BLOCK`. This matters when queued
effects inspect block between actions or when action-order traces are compared to Java.

Java `SphericGuardian.getMove()` also mutates private `firstMove` and `secondMove` latches. Rust
now mirrors those as `SphericGuardianRuntimeState`, `CommunicationMod` exports
`monster.runtime_state.first_move` and `second_move`, and state sync treats them as strict protocol
truth. Move history is used only after both opening latches are false, matching Java's explicit
`lastMove(BIG_ATTACK)` branch.

### Chosen

Chosen has private move truth that cannot be inferred only from public move history:

- `firstTurn`
- `usedHex`

Rust already carries this as `ChosenRuntimeState`. The A17 first-turn Hex branch intentionally keeps
`firstTurn` true, matching the Java source because that branch sets `usedHex` but does not clear
`firstTurn`.

### Byrd / Flight

Java `FlightPower` changes gameplay state when the Byrd loses enough flight stacks: it calls the
monster's grounded state and changes the next move. Rust models that as combat runtime state, not
as animation state.

Current Rust behavior matches the important Java guards:

- non-surviving damage does not reduce Flight;
- thorns / HP loss damage is not treated as an attack for Flight;
- zero Flight changes the Byrd into the grounded/stunned path.

### Shelled Parasite

Java has a `firstMove` private field and an A17 first move that always chooses Fell. Rust carries
that private truth explicitly. The recursive reroll after a previous Fell uses the same restricted
`aiRng.random(20, 99)` range as Java.

### Snake Plant

Snake Plant's Malleable pre-battle power, three-hit Chomp, Spores Frail/Weak action order, and A17
`lastMoveBefore(SPORES)` rule match the Java source. Its update-loop visual randomness is omitted.

### Snecko

Snecko carries explicit first-turn truth. Java first rolls Glare, then chooses Tail if
`num < 40` or the last two moves were Bite; otherwise it chooses Bite. Rust matches this, including
Tail applying Weak before Vulnerable at high ascension.

### Centurion / Healer

Centurion's ally-counting and defensive targeting follow Java's flag checks rather than current HP
checks. Rust already includes tests for zero-HP, non-dying entities.

`GainBlockRandomMonsterAction` itself follows the Java action filter: it excludes the source,
monsters whose current intent is `ESCAPE`, and `isDying` monsters. It does not filter Java
`isEscaping` / Rust `is_escaped`, so an already escaping monster with non-escape intent remains a
valid target, matching the source action.

Healer's total missing HP threshold, A17 attack repetition rule, heal targets, and Strength targets
match Java:

- healing threshold is `> 20` at A17, `> 15` otherwise;
- A17 blocks only a single repeated Attack, lower ascensions block two;
- Heal and Buff iterate monsters where `!isDying && !isEscaping`, without a current-HP guard.

### Mugger

Mugger is one of the rare cases where UI/dialogue code consumes gameplay RNG. Java uses
`AbstractDungeon.aiRng` for `playSfx`, optional second-slash talk, and the Smoke Bomb / Big Swipe
branch.

Rust already burns those `aiRng` calls and keeps the stolen-gold reward path on death. The escape
path marks the combat as mugged by Looter/Mugger monster identity, not by stolen-gold amount,
because Java sets `AbstractRoom.mugged = true` even if the player had no gold left to steal.

### Taskmaster

Taskmaster's Scouring Whip attack, Wound count thresholds, and A18 self-Strength follow-up match
Java. Its voice line uses `MathUtils`, not `aiRng`, so it is intentionally omitted.

### Book of Stabbing

Book of Stabbing's `stabCount` mutates inside Java `getMove()`, not inside `takeTurn()`. Rust models
that through roll-move setup actions.

The relevant Java rules are preserved:

- Stab plans with the incremented hit count.
- Big Stab at A18 increments future `stabCount`.
- Painful Stabs is applied pre-battle as the source-backed power hook.
- Per-hit stab SFX uses `MathUtils`, so it is render-only for Rust.

### Gremlin Leader

Gremlin Leader's move selection depends on the number of non-dying gremlin allies; Java does not
check current HP or escaping in `numAliveGremlins`. Rust matches that flag behavior.

Other source-backed details already modeled:

- Rally uses the Java summon pool and `aiRng` selection.
- Encourage consumes `aiRng` for its quote even though the quote is UI text.
- Encourage applies Strength to the leader and Strength/Block to non-dying allies.
- Leader death makes every non-dying ally escape.

### Bronze Automaton / Bronze Orb

Automaton uses private state for `firstTurn` and `numTurns`. Rust carries both explicitly and
matches the Java Hyper Beam calendar:

- first roll is Spawn Orbs;
- four counted turns lead to Hyper Beam and reset the counter;
- post-beam A19 uses Boost, otherwise Stunned;
- the post-beam Boost does not increment the turn counter before the following Flail.

Bronze Orb carries `usedStasis` explicitly and matches Java's move rules. Stasis selects from draw
pile if possible, otherwise discard pile, checking rarity in Rare / Uncommon / Common / fallback
order with `cardRandomRng`.

Java `SpawnMonsterAction.update()` calls `m.init()`, which rolls a first move for spawned monsters.
Rust preserves this behavior when spawning Automaton orbs and Collector torches.

### Champ

Champ private runtime truth is represented explicitly:

- `firstTurn`
- `numTurns`
- `forgeTimes`
- `thresholdReached`

Rust matches the Java `getMove()` order: increment turn count, check half-HP Anger threshold, force
Execute after threshold until it has been used recently, Taunt every fourth pre-threshold turn, then
Forge/Gloat/Face Slap/Heavy Slash selection.

The Champion Belt first-turn line and random taunt/limit-break/death quotes are UI-only and are
not modeled as mechanics.

### The Collector / Torch Head

Collector private runtime truth is represented explicitly:

- `initialSpawn`
- `ultUsed`
- `turnsTaken`
- `enemySlots`

Rust matches the Java macro rules:

- initial turn spawns two Torch Heads;
- after three turns, if the ultimate has not been used, Mega Debuff is forced;
- Revive is considered when a Torch Head currently stored in Java `enemySlots` is dying and the
  previous move was not Revive;
- Buff blocks self first, then applies Strength to every monster that is not dead/dying/escaping.

Torch Head's update-loop fire effect is UI-only. Its constructor-set move is followed by Java
`init()` rolling a move during spawn; Rust's spawn handler likewise rolls the spawned monster.

Rust now tracks Collector torch slots explicitly instead of scanning all TorchHead instances in the
monster group. This matters because Java leaves old dying minion objects in the group after revive,
while `enemySlots` is updated to point at the replacement TorchHead.

### Masked Bandits

Bear, Leader, and Pointy are not ordinary reroll-after-turn monsters. Their turns are chained by
Java `SetMoveAction`:

- Bear starts with Bear Hug, then Lunge, then Maul/Lunge loop.
- Leader starts with Mock, then Agonizing Slash / Cross Slash loop, with A17 allowing a second
  Cross Slash when the previous two moves were not both Cross Slash.
- Pointy repeatedly performs its two-hit attack.

Rust uses the same explicit next-move actions. Bear's `die()` calls other bandits'
`deathReact()`, but those reactions only queue TalkAction dialogue. No gameplay state is lost by
omitting them.

## Tests Added Or Relied On

- `bash_and_block_gains_block_before_attack_like_java`
- Existing Byrd Flight tests for grounded transition and damage-type guards
- Existing Chosen, Snecko, Shelled Parasite runtime-state tests
- Existing zero-HP/non-dying ally tests for Centurion, Gremlin Leader, Collector, and Bronze boss
  cleanup paths
- Existing Mugger `aiRng` burn tests
- Existing Stasis return-card tests

## Still Worth Rechecking

- Torch Head constructor `setMove` appends to Java move history before `init()` rolls again. Rust
  preserves the planned move and behavior but does not currently need the extra constructor-history
  entry because Torch Head has no history-dependent decisions.
- Spawned-monster pre-battle ordering should stay under source-backed tests for any future spawned
  monster whose `usePreBattleAction` has real mechanics.
