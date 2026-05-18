# Monster Runtime Truth Audit 2026-04-18

This audit is the phase-2 follow-up to [MONSTER_RUNTIME_STATE_FRAMEWORK.md](../design/MONSTER_RUNTIME_STATE_FRAMEWORK.md).

Scope:

- already-migrated stateful semantic monsters
- protocol truth completeness for hidden runtime state
- explicit execution-time runtime patches
- retirement of `move_history` fallback for hidden runtime truth

## Rules

- Hidden runtime truth must come from protocol truth or factory/spawn initialization.
- Execution-time runtime changes must flow through `Action::UpdateMonsterRuntime`.
- `move_history` is allowed only for Java rules that explicitly depend on `lastMove` / `lastTwoMoves`.
- `move_history` must not recover hidden runtime truth such as `first_turn`, `used_hex`, or `is_flying`.

## Audit Table

| Monster | Hidden runtime truth | Protocol truth | Explicit runtime patch | Hidden-truth history fallback retired | Allowed history use | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Hexaghost | `activated`, `orb_active_count`, `burn_upgraded`, divider cache | Yes | Yes | Yes | Sequence history only | Good |
| Lagavulin | `idle_count`, `debuff_turn_count`, `is_out`, `is_out_triggered` | Yes | Yes | Yes | Sequence history only | Good |
| The Guardian | `damage_threshold`, `damage_taken`, `is_open`, `close_up_triggered` | Yes | Yes | Yes | Sequence history only | Good |
| Byrd | `first_move`, `is_flying` | Yes | Yes | Yes | `PECK`/`SWOOP`/`CAW` sequencing | Good |
| Chosen | `first_turn`, `used_hex` | Yes | Yes | Yes | `DRAIN`/`DEBILITATE` sequencing | Good |
| Looter | `slash_count`, `stolen_gold` | Yes | Yes | Yes | None | Good |
| Mugger | `slash_count`, `stolen_gold` | Yes | Yes | Yes | None | Good |
| Shelled Parasite | `first_move` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Healer | None | N/A | N/A | N/A | Heal/attack/buff sequencing only | Good |
| Centurion | None | N/A | N/A | N/A | Slash/Protect/Fury repeat rules only | Good |
| Bandit Bear | None | N/A | N/A | N/A | SetMoveAction chain only | Good |
| Bandit Leader | None | N/A | N/A | N/A | SetMoveAction chain plus Cross Slash `lastTwoMoves` guard only | Good |
| Bandit Pointy | None | N/A | N/A | N/A | Repeated attack SetMoveAction only | Good |
| Taskmaster | None | N/A | N/A | N/A | None; always Scouring Whip | Good |
| Snake Plant | None | N/A | N/A | N/A | `lastMove`/`lastMoveBefore`/`lastTwoMoves` sequencing only | Good |
| Louse | `bite_damage` | Yes | N/A | Yes | None | Good |
| Snecko | `first_turn` | Yes | Yes | Yes | `lastTwoMoves(BITE)` sequencing | Good |
| Blue Slaver | None | N/A | N/A | N/A | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Red Slaver | `first_turn`, `used_entangle` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Gremlin Nob | `used_bellow` | Yes | Yes | Yes | `lastMove`/`lastMoveBefore`/`lastTwoMoves` sequencing only | Good |
| Gremlin Leader | `gremlin_slots` | Yes | Yes | Yes | Rally/Encourage/Stab sequencing only | Good |
| Gremlin Wizard | `current_charge` | Yes | Yes | Yes | None for charge cadence | Good |
| Cultist | `first_move` | Yes | Yes | Yes | None | Good |
| Jaw Worm | `first_move`, `hard_mode` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Fungi Beast | None | N/A | N/A | N/A | `lastMove`/`lastTwoMoves` sequencing only; Spore Cloud battle-ending guard | Good |
| Slime Boss | `first_turn` | Yes | Yes | Yes | None for post-opening cycle | Good |
| Large Slimes | `split_triggered` | Yes | Yes | Yes | Attack/debuff sequencing only | Good |
| Sentry | `first_move` | Yes | Yes | Yes | Later Bolt/Beam alternation only | Good |
| Spheric Guardian | `first_move`, `second_move` | Yes | Yes | Yes | Post-opening `lastMove(BIG_ATTACK)` branch only | Good |
| Bronze Automaton | `first_turn`, `num_turns` | Yes | Yes | Yes | `lastMove(HYPER_BEAM/STUNNED/BOOST/SPAWN_ORBS)` sequencing only | Good |
| Bronze Orb | `used_stasis` | Yes | Yes | Yes | `lastTwoMoves(SUPPORT_BEAM/BEAM)` sequencing only | Good |
| Book of Stabbing | `stab_count` | Yes | Yes | Yes | `lastMove(BIG_STAB)` / `lastTwoMoves(STAB)` sequencing only | Good |
| The Collector | `initial_spawn`, `ult_used`, `turns_taken`, `enemy_slots` | Yes | Yes | Yes | `lastMove(REVIVE)` / `lastTwoMoves(FIREBALL)` sequencing only | Good |
| Champ | `first_turn`, `num_turns`, `forge_times`, `threshold_reached` | Yes | Yes | Yes | `lastMove`/`lastMoveBefore` sequencing only | Good |
| Exploder | `turn_count` | Yes | Yes | Yes | None | Good |
| Spiker | `thorns_count` | Yes | Yes | Yes | `lastMove(ATTACK)` repeat guard only | Good |
| Repulsor | None | N/A | N/A | N/A | `lastMove(ATTACK)` repeat guard only | Good |
| Orb Walker | None | N/A | N/A | N/A | Claw/Laser `lastTwoMoves` repeat rules only | Good |
| Spire Growth | None | N/A | N/A | N/A | Constrict and attack repeat rules only | Good |
| Maw | `roared`, `turn_count` | Yes | Yes | Yes | `lastMove(SLAM/NOM)` sequencing only | Good |
| Darkling | `first_move`, `nip_dmg` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Reptomancer | `first_move`, `dagger_slots` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Snake Dagger | `first_move` | Yes | Yes | Yes | None | Good |
| Nemesis | `first_move`, `scythe_cooldown` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Giant Head | `count` | Yes | Yes | Yes | `lastTwoMoves(GLARE/COUNT)` sequencing only | Good |
| Time Eater | `used_haste` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Donu | `is_attacking` | Yes | Yes | Yes | None | Good |
| Deca | `is_attacking` | Yes | Yes | Yes | None | Good |
| Transient | `count` | Yes | Yes | Yes | None | Good |
| Writhing Mass | `first_move`, `used_mega_debuff` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Awakened One | `form1`, `first_turn` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Corrupt Heart | `first_move`, `move_count`, `buff_count`, `blood_hit_count` | Yes | Yes | Yes | `lastMove(ECHO_ATTACK)` sequencing only | Good |
| Spire Spear | `move_count`, `skewer_count` | Yes | Yes | Yes | `lastMove(BURN_STRIKE)` sequencing only | Good |

## Notes

### Shared Monster Lifecycle

- `CommunicationMod` now exports `is_dying`, `is_escaping`, and `is_escaped` separately from
  the aggregate `is_gone`.
- Rust state sync maps Java `isDying` to `MonsterEntity.is_dying` and maps
  `isEscaping || escaped` to `MonsterEntity.is_escaped`.
- `is_gone` remains only a consistency check for Java `isDeadOrEscaped()`. It must not be used to
  infer death, because Java `isDeadOrEscaped()` also returns true while a monster is escaping.
- Java `deathReact()` / `escapeNext()` was reviewed separately from lifecycle import. In vanilla
  code, Gremlin `deathReact()` and `escapeNext()` have no active base-game caller, while Bandit
  `deathReact()` is reached from `BanditBear.die()` but queues only `TalkAction` on the surviving
  bandits. These are not imported as hidden runtime truth because they do not currently mutate
  mechanical state or consume game RNG. If a future bridge observes a real `nextMove=ESCAPE`, that
  remains represented by the ordinary planned move truth.
- `move_history` is recorded when Java `setMove(...)` runs, not when the monster turn finishes.
  Rust mirrors this by pushing history from `SetMonsterMove` / `RollMonsterMove`. This does not
  mean every Java `takeTurn()` SetMove chain belongs in `getMove()`:
  - `BanditBear.getMove(int)` always sets `BEAR_HUG`; Maul/Lunge are advanced only by
    `SetMoveAction` inside `takeTurn()`.
  - `BanditLeader.getMove(int)` always sets `MOCK`; Agonizing Slash / Cross Slash are advanced only
    by `SetMoveAction` inside `takeTurn()`, with Java's `lastTwoMoves(CROSS_SLASH)` check preserved
    there.
  - `BanditPointy.getMove(int)` and its `takeTurn()` follow-up both set the same repeated attack, so
    it does not need a separate SetMove-chain exception.

### Byrd

- `runtime_state.first_move` and `runtime_state.is_flying` are exported by `CommunicationMod`.
- Rust now requires these fields to be protocol-seeded or factory-seeded before semantic roll logic runs.
- Remaining `move_history` usage is intentional Java sequence logic, not hidden-state recovery.

### Chosen

- `runtime_state.first_turn` and `runtime_state.used_hex` are exported by `CommunicationMod`.
- Rust now requires these fields to be protocol-seeded or factory-seeded before semantic roll logic runs.
- Remaining `move_history` usage is limited to the Java branch that avoids repeating `DRAIN` and `DEBILITATE`.

### Louse

- `runtime_state.bite_damage` is now treated as strict protocol truth.
- Rust no longer falls back to `move_base_damage` during split truth import.
- This removes the last hidden-state recovery path for Louse parity.

### Looter and Mugger

- `runtime_state.slash_count` and `runtime_state.stolen_gold` are now exported by `CommunicationMod`.
- Rust semantic execution updates thief runtime explicitly when gold is stolen.
- Death rewards now rely on seeded thief runtime truth instead of reconstructing it from sequence history.

### Lagavulin

- `runtime_state.idle_count`, `debuff_turn_count`, `is_out`, and `is_out_triggered` are exported
  by `CommunicationMod`.
- Rust state sync treats all four fields as strict runtime truth. It no longer repairs missing
  `debuff_turn_count` as zero or infers `is_out` from the visible planned move.
- Remaining move history usage is limited to Java's explicit attack/debuff sequencing after
  Lagavulin is awake.

### Shelled Parasite

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires `first_move` to be protocol-seeded or factory-seeded.
- `move_history` is still used for Java's explicit `lastMove` / `lastTwoMoves` branching only.

### Healer

- `Healer` does not require hidden runtime truth from protocol.
- The semantic migration exposed a small framework gap, so group-heal intent is now represented explicitly as `MonsterMoveSpec::Heal(HealSpec { target: AllMonsters, ... })`.
- Execution still expands group heal and group strength buff into per-monster `Action::Heal` / `Action::ApplyPower` calls in `take_turn_plan`, which keeps the target set explicit and avoids pretending helpers already support generic group targeting.

### Bandit Bear / Bandit Leader / Bandit Pointy

- The bandit trio does not require hidden runtime truth from protocol.
- Java checked:
  - `D:\rust\cardcrawl\monsters\city\BanditBear.java`
  - `D:\rust\cardcrawl\monsters\city\BanditLeader.java`
  - `D:\rust\cardcrawl\monsters\city\BanditPointy.java`
- Rust checked:
  - `src\content\monsters\city\bandit_bear.rs`
  - `src\content\monsters\city\bandit_leader.rs`
  - `src\content\monsters\city\bandit_pointy.rs`
- `BanditBear.getMove(int)` always sets `BEAR_HUG`; Java advances Bear Hug -> Lunge -> Maul through `SetMoveAction` inside `takeTurn()`. Rust mirrors this in `take_turn_plan` and keeps `roll_move_plan` as Bear Hug only.
- `BanditLeader.getMove(int)` always sets `MOCK`; Java advances Mock -> Agonizing Slash -> Cross Slash through `SetMoveAction` inside `takeTurn()`. The only history-dependent branch is Java's Ascension 17 `lastTwoMoves(CROSS_SLASH)` guard after Cross Slash, and Rust keeps that as sequence logic instead of hidden-state recovery.
- `BanditPointy.getMove(int)` and the follow-up `SetMoveAction` both set the same repeated two-hit attack, so no separate runtime field is needed.
- `BanditBear.die()` calls surviving bandits' `deathReact()`, but the vanilla survivors queue only `TalkAction`; this remains UI/dialogue-only and is not imported as runtime truth.
- Verification:
  - `cargo test bandit --all-targets` -> `7 passed`

### Taskmaster

- `Taskmaster` does not require hidden runtime truth from protocol.
- Java checked:
  - `D:\rust\cardcrawl\monsters\city\Taskmaster.java`
- Rust checked:
  - `src\content\monsters\city\taskmaster.rs`
- Java `Taskmaster.getMove(int)` always sets Scouring Whip with `ATTACK_DEBUFF` intent and 7 base damage; it does not use the roll or move history.
- Java `takeTurn()` queues damage, Wound-to-discard, optional Ascension 18 self Strength, then `RollMoveAction(this)`. Rust mirrors the same action order and wound thresholds.
- Java voice/death sound rolls are UI/audio-only and are not imported as gameplay RNG.
- Verification:
  - `cargo test taskmaster --all-targets` -> `4 passed`

### Generic Action-Family Checks

- Action-family review aligned two generic handlers with Java:
  - `handle_heal` now ignores `is_dying` monster targets, matching `AbstractCreature.heal(...)`.
  - `handle_apply_power` now ignores escaped monster targets, matching `ApplyPowerAction.update()`.
- `RollMonsterMove` was reviewed but left unchanged in this pass; Java itself does not add an extra dead/escaped guard there, and changing it now risks interfering with half-dead/revival style monsters.

### Repulsor

- `Repulsor` does not require hidden runtime truth from protocol.
- Java checked:
  - `D:\rust\cardcrawl\monsters\beyond\Repulsor.java`
- Rust checked:
  - `src\content\monsters\beyond\repulsor.rs`
- Java `Repulsor.getMove(int)` uses only the roll and `lastMove(ATTACK)`: rolls below 20 attack unless the previous planned move was Attack; all other branches add Dazed.
- Rust mirrors the same `lastMove(ATTACK)` gate through `move_history().back()` as ordinary Java sequence logic.
- Java `takeTurn()` queues either one damage action or `MakeTempCardInDrawPileAction(new Dazed(), 2, true, true)`, then queues `RollMoveAction(this)`. Rust preserves the random draw-pile Dazed insertion and the explicit roll action.
- Verification:
  - `cargo test repulsor --all-targets` -> `3 passed`

### Centurion

- `Centurion` does not require hidden runtime truth from protocol.
- Java `Centurion.getMove(int)` uses only the roll, `lastTwoMoves(PROTECT/FURY/SLASH)`, and an
  alive-count scan that skips `isDying` and `isEscaping`.
- Rust `roll_move_plan_with_context` mirrors that scan over the current monster group and keeps
  `move_history` use limited to the same Java repeat rules.
- `GainBlockRandomMonsterAction` remains represented by a random-monster block action, and tests
  cover Java's zero-HP-but-not-dying ally edge.

### Snake Plant

- `Snake Plant` does not require hidden runtime truth from protocol.
- The semantic migration is purely a Java-sequence port:
  - `lastTwoMoves(CHOMPY_CHOMPS)`
  - `lastMove(SPORES)`
  - and at A17 specifically `lastMoveBefore(SPORES)`
- `use_pre_battle_action` is now wired through semantic dispatch so `Malleable` no longer depends on legacy paths.
- `on_death` is also routed through semantic dispatch to the default no-op implementation, which removes one more unsupported hallway-monster death edge.

### Large Slimes

- `runtime_state.split_triggered` is exported for `AcidSlime_L` and `SpikeSlime_L`.
- Rust uses the private Java latch in addition to `nextMove != SPLIT`; this covers states where a later roll temporarily changes the planned move while Java still remembers that the split interrupt already fired.
- The latch is updated immediately when the split interrupt fires. It is not queued as a Java action; only the Java `SetMoveAction` equivalent remains queued behind existing actions.

### Snecko

- `runtime_state.first_turn` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires `first_turn` to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit `lastTwoMoves(BITE)` rule.

### Blue Slaver

- `Blue Slaver` does not require hidden runtime truth from protocol.
- Java checked:
  - `D:\rust\cardcrawl\monsters\exordium\SlaverBlue.java`
- Rust checked:
  - `src\content\monsters\exordium\slaver_blue.rs`
- Java `SlaverBlue.getMove(int)` uses only the roll plus public sequence history:
  - `num >= 40 && !lastTwoMoves(STAB)` picks Stab.
  - Ascension 17+ blocks Rake with `lastMove(RAKE)`.
  - Below Ascension 17 blocks Rake with `lastTwoMoves(RAKE)`.
- Java `takeTurn()` queues damage, optional Weak from Rake, and then `RollMoveAction(this)`. Rust preserves action order and the Ascension 17 Weak amount increase.
- Java voice/death sound rolls are UI/audio-only and are not imported as gameplay RNG.
- Verification:
  - `cargo test blue_slaver --all-targets` -> `2 passed`

### Red Slaver

- `runtime_state.first_turn` and `runtime_state.used_entangle` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit repeat rules around `STAB` and `SCRAPE`.

### Fungi Beast / Spore Cloud

- `Fungi Beast` does not require hidden runtime truth from protocol.
- Java checked:
  - `D:\rust\cardcrawl\monsters\exordium\FungiBeast.java`
  - `D:\rust\cardcrawl\powers\SporeCloudPower.java`
  - `D:\rust\cardcrawl\rooms\AbstractRoom.java`
  - `D:\rust\cardcrawl\monsters\MonsterGroup.java`
- Rust checked/changed:
  - `src\content\monsters\exordium\fungi_beast.rs`
  - `src\content\powers\core\spore_cloud.rs`
  - `src\runtime\combat.rs`
- Java `FungiBeast.getMove(int)` uses only roll plus public sequence history:
  - Low rolls bite unless `lastTwoMoves(BITE)`.
  - High rolls grow unless `lastMove(GROW)`.
- Java `usePreBattleAction()` applies `SporeCloudPower(this, 2)`. Rust preserves this as a pre-battle `ApplyPower` action.
- Java `SporeCloudPower.onDeath()` returns immediately when `AbstractDungeon.getCurrRoom().isBattleEnding()` is true. `AbstractRoom.isBattleEnding()` checks `isBattleOver` or `MonsterGroup.areMonstersBasicallyDead()`, and `areMonstersBasicallyDead()` skips only `isDying` / `isEscaping` monsters. Rust now uses `are_monsters_basically_dead_java()` before queuing Vulnerable so the final dying Fungi does not poison a battle that Java considers ending.
- Verification:
  - `cargo test fungi_beast --all-targets` -> `2 passed`
  - `cargo test spore_cloud --all-targets` -> `3 passed`

### Gremlin Nob

- `runtime_state.used_bellow` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires this latch to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit `SKULL_BASH` / `BULL_RUSH` repeat rules.

### Gremlin Leader

- `runtime_state.gremlin_slots` is now exported by `CommunicationMod` from Java
  `GremlinLeader.gremlins`.
- Rust Rally execution uses the exported slot members to find the first null/dying slot, matching
  Java `SummonGremlinAction.identifySlot(...)`.
- Draw positions are still used to place the summoned monster in the same coordinate frame, but
  they are no longer the source of slot occupancy truth.

### Gremlin Wizard

- `runtime_state.current_charge` is now exported by `CommunicationMod`.
- Rust semantic turn execution requires this counter to be protocol-seeded or factory-seeded.
- Charge cadence comes from Java's private `currentCharge`, not from consecutive Charge history.
- Runtime patches are emitted during `take_turn_plan`, because Java mutates `currentCharge` inside
  `takeTurn()` rather than `getMove()`.

### Exploder

- `runtime_state.turn_count` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires this counter to be protocol-seeded or factory-seeded.
- The attack/explosion cadence comes from Java's private `turnCount`, not from move history.
- Runtime patches are emitted during `take_turn_plan`, because Java increments `turnCount` inside
  `takeTurn()` before the queued `RollMoveAction` resolves.

### Spiker

- `runtime_state.thorns_count` is exported by `CommunicationMod`.
- Rust semantic roll logic requires this counter to be protocol-seeded or factory-seeded.
- Java increments `thornsCount` only when the BUFF_THORNS turn executes, before queuing the Thorns
  `ApplyPowerAction`; Rust mirrors that ordering with `Action::UpdateMonsterRuntime`.
- Remaining history usage is limited to Java's explicit `lastMove(ATTACK)` repeat guard.

### Orb Walker

- `Orb Walker` does not require hidden runtime truth from protocol.
- Java `OrbWalker.getMove(int)` uses only `lastTwoMoves(CLAW/LASER)` repeat rules.
- Rust mirrors `usePreBattleAction()` as an ascension-gated `GenericStrengthUp` application and
  keeps `move_history` use limited to Java's Claw/Laser repeat gates.
- Laser uses Java `MakeTempCardInDiscardAndDeckAction(new Burn())`, represented by one burn in
  discard and one burn added to the draw pile through the shared card-zone action.

### Spire Growth

- `Spire Growth` does not require hidden runtime truth from protocol.
- Java `SpireGrowth.getMove(int)` uses current public player state
  (`player.hasPower("Constricted")`), ascension level, and `lastMove/lastTwoMoves` repeat rules.
- Rust passes player powers through `MonsterRollContext` instead of caching private monster state.
- Constrict amount and the A17 forced-Constrict branch are covered by focused tests.

### Maw

- `runtime_state.roared` and `runtime_state.turn_count` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- The opening Roar gate comes from Java's private `roared`, not from whether ROAR exists in history.
- Nom hit count comes from Java's private `turnCount / 2`, not from move history length.
- Runtime `turn_count` is updated during roll resolution, because Java increments it at the start of
  `getMove()`. Runtime `roared` is updated only when Roar executes.

### Transient

- `runtime_state.count` is now exported by `CommunicationMod`.
- Rust semantic planning requires this counter to be protocol-seeded or factory-seeded.
- Attack damage and next intent use Java's private `count`, not `move_history` length.
- Transient still uses direct `SetMonsterMove` after its turn instead of queuing `RollMonsterMove`,
  matching Java's `takeTurn()` shape.

### Writhing Mass

- `runtime_state.first_move` and `runtime_state.used_mega_debuff` are now exported by
  `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- The opening branch comes from Java's private `firstMove`, not empty move history.
- Mega Debuff eligibility comes from Java's private `usedMegaDebuff`, not whether move 4 appeared
  during a Reactive reroll.

### Snake Dagger

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires this field to be protocol-seeded or factory-seeded.
- The Wound/Explode switch comes from Java `SnakeDagger.firstMove`, not empty move history.

### Cultist

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires it to be protocol-seeded or factory-seeded.
- The opening Incantation gate comes from this Java private field, not from empty move history.

### Jaw Worm

- `runtime_state.first_move` and `runtime_state.hard_mode` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- The opening Chomp gate comes from Java's private `firstMove`, not from empty move history.
- Jaw Worm Horde uses Java hard mode: `hard_mode=true` and `first_move=false`; the pre-battle
  Strength/Block bonus still comes from `hard_mode`.
- Remaining history usage is limited to Java's explicit Chomp/Bellow/Thrash repeat rules.

### Slime Boss

- `runtime_state.first_turn` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires this field to be protocol-seeded or factory-seeded.
- The opening Sticky gate comes from Java's private `firstTurn`, not from empty move history.
- After `first_turn=false`, Java `getMove()` is a no-op; Rust therefore preserves the current
  planned move if a roll is requested, while the ordinary cycle remains driven by `takeTurn()`
  `SetMonsterMove` actions.

### Sentry

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires it to be protocol-seeded or factory-seeded.
- Opening Bolt/Beam parity uses the monster slot only while `first_move` is true. After that, move
  history is used only for Java's explicit Bolt/Beam alternation.

### Spheric Guardian

- `runtime_state.first_move` and `runtime_state.second_move` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- The opening Harden and Bash+Frail gates come from Java's private latches, not from move-history
  length. After both latches are false, move history is used only for Java's explicit
  `lastMove(BIG_ATTACK)` branch.

### Bronze Automaton / Bronze Orb / Book of Stabbing

- `BronzeAutomaton.firstTurn` and `numTurns`, `BronzeOrb.usedStasis`, and
  `BookOfStabbing.stabCount` are exported by `CommunicationMod`.
- Rust state sync seeds each runtime slice from `monster.runtime_state`, and
  factory/test construction seeds the same fields for authored combats.
- Roll-time mutations are emitted as `Action::UpdateMonsterRuntime` rather than
  reconstructed from truncated visible move history.
- Remaining history usage is source-backed Java sequence logic only:
  - Bronze Automaton checks the previous Hyper Beam / Stun / Boost / Spawn Orbs
    move while `numTurns` remains the private Hyper Beam calendar.
  - Bronze Orb uses `lastTwoMoves` only for Support Beam / Beam repeat guards.
  - Book of Stabbing uses `lastMove(BIG_STAB)` and `lastTwoMoves(STAB)` while
    `stabCount` remains private runtime truth.
- Verification:
  - `cargo test bronze_automaton --all-targets` -> `7 passed`
  - `cargo test bronze_orb --all-targets` -> `5 passed`
  - `cargo test book_of_stabbing --all-targets` -> `5 passed`

### The Collector

- `runtime_state.initial_spawn`, `ult_used`, `turns_taken`, and `enemy_slots` are exported by
  `CommunicationMod`.
- `enemy_slots` is mapped from Java monster instance ids to Rust entity ids during state sync.
- Rust move selection and revive execution use the current slot members, not a scan of every
  TorchHead in the monster group. This matches Java's private `enemySlots` map and avoids reviving
  stale dying TorchHead objects left behind after prior revives.

### Champ

- `runtime_state.first_turn`, `num_turns`, `forge_times`, and `threshold_reached` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires this runtime truth to be protocol-seeded or factory-seeded before branch selection runs.
- `Champ` also uses the new semantic `on_roll_move` hook to model Java's `getMove()` side effects explicitly, instead of recovering them later from `move_history`.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastMoveBefore` sequencing rules around `EXECUTE` and `GLOAT`.

### Darkling

- `runtime_state.first_move` and `runtime_state.nip_dmg` are exported by `CommunicationMod`.
- Rust state sync marks the Darkling runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java clears `firstMove` inside `getMove()` only when the opening branch is consumed. Rust now
  mirrors that through `Action::UpdateMonsterRuntime`; generic `SetMonsterMove` and
  `RollMonsterMove` no longer clear the flag as a side effect.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastTwoMoves` branching.

### Reptomancer

- `runtime_state.first_move` and `runtime_state.dagger_slots` are exported by `CommunicationMod`.
- Rust state sync maps Java dagger slot monster instance ids to Rust entity ids in a second pass,
  matching the Java private `daggers[4]` array instead of deriving occupancy from draw position.
- Spawn Dagger execution now uses `Action::SpawnReptomancerDagger`, which spawns the dagger and
  updates the corresponding runtime slot together.
- Remaining history usage is limited to Java's explicit repeat rules around Snake Strike, Spawn
  Dagger, and Big Bite.

### Nemesis

- `runtime_state.first_move` and `runtime_state.scythe_cooldown` are exported by
  `CommunicationMod`.
- Rust state sync marks the Nemesis runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java decrements `scytheCooldown` at the start of every `getMove()` and resets it to 2 only when
  Scythe is selected. Rust mirrors this through `Action::UpdateMonsterRuntime` emitted from
  `on_roll_move`.
- The opening branch is gated by Java private `firstMove`, not by empty move history.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastTwoMoves` repeat rules.

### Giant Head

- `runtime_state.count` is exported by `CommunicationMod`.
- Rust state sync marks the Giant Head runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java initializes `count=5`, decrements it once in `usePreBattleAction()` at A18+, and then
  decrements it inside every `getMove()` until it reaches -6. Rust mirrors the A18 pre-battle
  mutation immediately in the pre-battle hook and roll-time mutation through
  `Action::UpdateMonsterRuntime`.
- `It Is Time` damage is computed from the Java private count after the roll-time decrement, not
  from move-history length.
- Remaining history usage is limited to Java's explicit Glare/Count repeat rules.

### Time Eater

- `runtime_state.used_haste` is exported by `CommunicationMod`.
- Rust state sync marks the Time Eater runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java sets `usedHaste=true` inside `getMove()` when the half-HP Haste branch is selected. Rust
  mirrors that roll-time mutation through `Action::UpdateMonsterRuntime`.
- Java private `firstTurn` controls only dialogue in `takeTurn()` and is intentionally omitted.
- Haste heal amount remains execution-time state, matching Java's queued
  `HealAction(this.maxHealth / 2 - this.currentHealth)`.
- Remaining history usage is limited to Java's explicit repeat rules around Reverberate, Head Slam,
  and Ripple.

### Donu / Deca

- `runtime_state.is_attacking` is exported by `CommunicationMod` for both Donu and Deca.
- Rust state sync marks both runtime slices as protocol-seeded and semantic roll logic requires
  factory/protocol seeding before branch selection.
- Java mutates `isAttacking` inside `takeTurn()` after queueing the branch's visible actions and
  before queueing `RollMoveAction`. Rust mirrors that with `Action::UpdateMonsterRuntime`
  immediately before `Action::RollMonsterMove`.
- Rust no longer derives their alternation from `move_history`.

### Awakened One

- `runtime_state.form1` and `runtime_state.first_turn` are exported by `CommunicationMod`.
- Rust state sync marks the Awakened One runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java clears the first-form opening `firstTurn` inside `getMove()` when Slash is selected; Rust
  mirrors that through `Action::UpdateMonsterRuntime` emitted from `on_roll_move`.
- Java sets `form1=false` and `firstTurn=true` during the first death / Unawakened path; Rust
  mirrors that through `Unawakened` power death actions.
- Remaining history usage is limited to Java's explicit repeat rules around Slash, Soul Strike,
  Sludge, and Tackle.

### Corrupt Heart

- `runtime_state.first_move`, `move_count`, `buff_count`, and `blood_hit_count` are exported by
  `CommunicationMod`.
- Rust state sync marks the Heart runtime slice as protocol-seeded and semantic roll logic requires
  factory/protocol seeding before branch selection.
- `blood_hit_count` is Java constructor state derived from ascension, but it still directly affects
  Blood Shots hit count; Rust now imports and uses the Java field instead of recomputing the value
  from local ascension.
- `move_count` advances during roll resolution, while `buff_count` advances only when the buff turn
  executes, matching Java's split between `getMove()` and `takeTurn()`.

### Spire Spear

- `runtime_state.move_count` and `skewer_count` are exported by `CommunicationMod`.
- Rust state sync marks the Spire Spear runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- `skewer_count` is Java constructor state derived from ascension, but it directly controls the
  Skewer multi-hit count. Rust now imports and uses the Java field instead of recomputing the value
  from local ascension during live import.
- Remaining history usage is limited to Java's explicit `lastMove(BURN_STRIKE)` branch.

### The Guardian

- `runtime_state.guardian_threshold`, `damage_taken`, `is_open`, and `close_up_triggered` are now all treated as strict protocol truth.
- Rust split import seeds `entity.guardian` directly from those fields instead of silently falling back to the runtime default.
- This closes the live parity bug where imported Guardian state kept `damage_threshold = 0`, causing any positive hit to mis-trigger defensive mode.

## Outcome

Phase 2 is complete for the migrated hallway/stateful monsters that currently matter for live act1/act2 frontier work:

- hidden runtime truth no longer falls back to `move_history` for Byrd and Chosen
- thief runtime truth is explicit and protocol-seeded for Looter and Mugger
- Louse truth import is now strict
- remaining stateful debt is concentrated in monsters that have not completed semantic migration
