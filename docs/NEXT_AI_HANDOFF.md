# Next AI Handoff

Date: 2026-05-18
Branch: `codex/evidence-path-cleanup-20260509`
Workspace: `D:\rust\sts_simulator`
Java source reference: `D:\rust\cardcrawl`
CommunicationMod reference: `D:\rust\CommunicationMod`

## Purpose

This file is the durable working memory for context compaction. At the start of
any resumed turn, read only:

1. `git status --short`
2. `git log --oneline -5`
3. this file

Do not re-read broad source trees just to rediscover recent state. Use this file
to choose the next narrow Java/Rust evidence packet.

## Current Rule

Continue Java-source-backed mechanics cleanup for a Rust simulator intended for
AI use.

Allowed:

- Preserve Java gameplay semantics from `D:\rust\cardcrawl`.
- Change Rust architecture when the current one hides or distorts Java state.
- Omit UI/VFX only when it is truly presentation-only.
- Keep UI-tied Java behavior only when it mutates gameplay state, consumes
  gameplay RNG, gates choices, changes visibility, or affects replay.
- Encode resolved source comparisons as tests, audit notes, and commits.

Forbidden:

- Strategy heuristics, seed patches, bot compatibility layers, CleanRL/Gym-first
  constraints, or policy logic.
- Simulating UI effects for their own sake.
- Treating Java private mechanical fields as inferable from `move_history`
  unless Java itself only uses history.
- Re-reading large trees after compaction without first checking this file.

## Latest Pushed Checkpoint

Branch tip:

- `a8e467e Add champ move parity tests`

Recent commits:

- `a8e467e Add champ move parity tests`
- `ad7747d Update handoff after bronze audit`
- `8385df0 Fix stasis selection parity`
- `35b4dc4 Update handoff after collector audit`
- `5232ea9 Add collector move parity tests`

`a8e467e` summary:

- `Champ` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - crossing below half HP selects `ANGER` and mutates `threshold_reached`
    inside the Java `getMove()` equivalent;
  - threshold mode forces `EXECUTE` unless `lastMove(EXECUTE)` or
    `lastMoveBefore(EXECUTE)` blocks it;
  - the fourth pre-threshold roll forces `TAUNT` and resets `num_turns`;
  - A19 expands the Defensive Stance roll cap to `num <= 30` and increments
    `forge_times`;
  - `ANGER` queues first-turn runtime update, debuff cleanup, Shackled removal,
    Strength gain, then `RollMonsterMove`;
  - `FACE_SLAP` and `TAUNT` queue their debuffs in Java order.
- Java `TalkAction`, `ShoutAction`, VFX/SFX, and `MathUtils` dialogue/death
  quote rolls remain presentation-only for the Rust simulator.

Verification for `a8e467e`:

- `cargo test champ --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1257 passed`

`8385df0` summary:

- `BronzeAutomaton`, `BronzeOrb`, and Java `ApplyStasisAction` behavior were
  checked.
- Fixed `handle_apply_stasis` candidate selection: Java
  `CardGroup.getRandomCard(rng, rarity)` sorts matching cards by `cardID`
  before applying the RNG index. Rust now sorts rarity candidates by
  `cards::java_id(...)` before removal.
- Added tests for:
  - Stasis rarity-candidate ordering before `cardRandomRng` selection;
  - BronzeAutomaton first turn, Hyper Beam counter reset, post-Hyper no-counter
    increment, and normal Flail/Boost counter increments;
  - BronzeOrb usedStasis update, Support/Beam `lastTwoMoves` gates, and Stasis
    take-turn queue order.

Verification for `8385df0`:

- `cargo test bronze_automaton --all-targets` -> `6 passed`
- `cargo test bronze_orb --all-targets` -> `5 passed`
- `cargo test apply_stasis --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1251 passed`

`5232ea9` summary:

- `TheCollector` and `TorchHead` Java/Rust behavior were checked.
- No business logic change was needed.
- Added tests proving:
  - initial spawn queues two TorchHead spawns, then runtime update, then
    `RollMonsterMove`;
  - initial spawn is forced regardless of random roll;
  - turn-three `MEGA_DEBUFF` is forced until `ult_used` becomes true;
  - Fireball is blocked only by Java `lastTwoMoves(FIREBALL)`;
  - Mega Debuff queues Weak, Vulnerable, Frail, runtime update, then roll.
- Existing tests already covered Collector buff targeting, death cleanup, and
  enemy-slot-based revive behavior.

Verification for `5232ea9`:

- `cargo test collector --all-targets` -> `10 passed`
- `cargo test --all-targets` -> `1244 passed`

`6e9a4d6` summary:

- `GremlinLeader` Java/Rust behavior was checked.
- Fixed `GremlinLeader` and `Reptomancer` pre-battle Minion applications to use
  Java `AbstractPower.amount` sentinel `-1`.
- Fixed generic spawned-minion handling in `SpawnMonsterAction` /
  `SummonGremlinAction` equivalent code to queue Minion with `amount: -1`.
- Added GremlinLeader tests for Minion sentinel, Encourage queue order, STAB
  three-hit queue before `RollMonsterMove`, and existing slot-truth behavior.
- Added Reptomancer and generic spawned-minion sentinel tests.
- Confirmed GremlinLeader slot truth is already factory-seeded for authored
  encounters and state-sync-seeded for live truth import; Rally should continue
  to use `gremlin_slots`, not draw-position inference.

Verification for `6e9a4d6`:

- `cargo test gremlin_leader --all-targets` -> `8 passed`
- `cargo test reptomancer --all-targets` -> `5 passed`
- `cargo test spawned_minion_power_uses_java_sentinel_amount --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1240 passed`

`f511731` summary:

- `Taskmaster` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving Java's constant `SCOURING_WHIP` roll, wound-count
  ascension thresholds, below-A18 no-Strength path, and A18 queue order:
  damage, Wounds, Strength, then `RollMonsterMove`.
- Java `playSfx()` burns `MathUtils` only for voice selection and remains
  presentation-only for the Rust simulator.

Verification for `f511731`:

- `cargo test taskmaster --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1235 passed`

`0b984ca` summary:

- `Chosen` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests for the below-A17 second-roll Hex transition, Drain order
  (Weak then Strength), Debilitate order (attack then Vulnerable), and Poke
  two-hit execution before `RollMonsterMove`.

Verification for `0b984ca`:

- `cargo test chosen --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1231 passed`

`dc4622d` summary:

- `BookOfStabbing` Java/Rust behavior was checked.
- Fixed pre-battle `PainfulStabsPower` to use Java sentinel amount `-1`.
- Added tests for Painful Stabs pre-battle application, `stabCount` growth
  before visible hit count, A18 Big Stab incrementing future `stabCount`, and
  STAB take-turn multi-hit execution before `RollMonsterMove`.

Verification for `dc4622d`:

- `cargo test book_of_stabbing --all-targets` -> `5 passed`
- `cargo test painful_stabs --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1227 passed`

`aa55e3d` summary:

- Corrected sentinel-power action amounts to follow Java `AbstractPower.amount`
  truth: `ConfusionPower` and `BarricadePower` use `-1`, not synthetic `0` or
  `1`.
- `Snecko` Glare and `SneckoEye` now emit Confusion with `amount: -1`.
- `SphericGuardian` pre-battle Barricade now emits `amount: -1`, followed by
  Artifact `3` and block `40`.
- Added a focused SphericGuardian pre-battle queue-order test.

Verification for `aa55e3d`:

- `cargo test snecko --all-targets` -> `7 passed`
- `cargo test spheric_guardian --all-targets` -> `6 passed`
- `cargo test barricade --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1223 passed`

`632492c` summary:

- `Snecko` Java/Rust behavior was checked.
- Added tests for Glare, A17 Tail queuing Weak before Vulnerable, and Java
  `lastTwoMoves(BITE)` forcing Tail. The initial Confusion amount from this
  commit was corrected to Java sentinel `-1` in `aa55e3d`.

Verification for `632492c`:

- `cargo test snecko --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1222 passed`

`1ad40f2` summary:

- `SnakePlant` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests for the A17+ `lastMoveBefore(SPORES)` rule versus the lower
  ascension `lastMove(SPORES)` rule.
- Added a queue-order test for three Chompy Chomps damage actions before
  `RollMonsterMove`.

Verification for `1ad40f2`:

- `cargo test snake_plant --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1219 passed`

`8d16e69` summary:

- `Centurion` + `Healer` Java/Rust behavior was checked as a pair because both
  depend on ally state.
- No business logic change was needed.
- Existing Centurion tests already cover zero-HP non-dying ally counting for
  Protect rolls and `GainBlockRandomMonsterAction`.
- Added Healer tests proving Java-style loops count/target zero-HP non-dying
  allies for heal selection and heal execution.

Verification for `8d16e69`:

- `cargo test healer --all-targets` -> `2 passed`
- `cargo test centurion --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1217 passed`

`a4d74f4` summary:

- `ShelledParasite` Java/Rust timing was checked; no code change was needed.
  Existing tests already cover `firstMove`, STUN writing a FELL move before the
  roll, live truth import, and Plated Armor break triggering STUN.
- `Byrd` Java/Rust timing exposed a real issue: Java Headbutt queues damage but
  synchronously calls `setMove(GO_AIRBORNE)` before queued damage can execute.
- Rust Byrd Headbutt now records the next move before the queued attack, matching
  Java's synchronous `setMove(...)` timing.
- Added a focused Byrd Headbutt timing test.

Verification for `a4d74f4`:

- `cargo test shelled_parasite --all-targets` -> `4 passed`
- `cargo test byrd --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1215 passed`

`5ad39bc` summary:

- `TorchHead` Java source was checked against Rust.
- No business logic change was needed: Rust already emits one `MonsterAttack`
  followed by queued `SetMonsterMove`, matching Java's `DamageAction` followed
  by `SetMoveAction`.
- Java `update()` only emits `TorchHeadFireEffect` VFX and was not modeled.
- Added a focused parity test to lock that queue order.

Verification for `5ad39bc`:

- `cargo test torch_head --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1214 passed`

`0b0eec3` summary:

- `BanditPointy` Java source was checked against Rust.
- No business logic change was needed: Rust already emits two separate
  `MonsterAttack` actions followed by queued `SetMonsterMove`, matching Java's
  two `DamageAction`s followed by `SetMoveAction`.
- Added a focused parity test to lock that queue order.

Verification for `0b0eec3`:

- `cargo test bandit_pointy --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1213 passed`

`1ac61f2` summary:

- Gremlin escape turns now preserve Java's queued post-escape
  `SetMoveAction(ESCAPE)` for Fat Gremlin, Gremlin Warrior, Gremlin Thief,
  Gremlin Wizard, and Gremlin Tsundere.
- Gremlin Tsundere Protect now models Java timing: queued
  `GainBlockRandomMonsterAction` is preceded by the synchronous next-move
  update from `setMove(...)`, so the visible next intent changes before the
  queued block action can be interrupted.
- Gremlin Wizard Dope Magic now models Java timing: reset `currentCharge`, then
  record the synchronous next-move update, then execute queued damage.
- Added focused tests for the escape follow-up move and timing-sensitive Wizard
  / Tsundere branches.

Verification for `1ac61f2`:

- `cargo test gremlin --all-targets` -> `34 passed`
- `cargo test --all-targets` -> `1212 passed`

`874605d` summary:

- `Looter` and `Mugger` now distinguish Java synchronous `setMove(...)`
  mutations from queued `SetMoveAction(...)`.
- Looter/Mugger lunge-style attacks place the next Smoke Bomb move update
  before queued steal/damage actions so later queue cleanup cannot erase a Java
  immediate move mutation.
- Looter/Mugger escape turns now include the Java post-escape
  `SetMoveAction(ESCAPE)`.
- `Mugger.die()` burns one `aiRng.random(2)` for Java death voice selection,
  even when there is no stolen gold reward.

Verification for `874605d`:

- `cargo test looter --all-targets` -> `4 passed`
- `cargo test mugger --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1207 passed`

`d0adc3b` summary:

- `BanditBear.getMove(int)` in Java always sets `BEAR_HUG`; Rust
  `roll_move_plan` now always returns the Bear Hug plan. Maul/Lunge remain a
  `take_turn` `SetMonsterMove` chain.
- `BanditLeader.getMove(int)` in Java always sets `MOCK`; Rust
  `roll_move_plan` now always returns the Mock plan. Attack chain remains in
  `take_turn`.
- `Lagavulin` no longer uses an empty-history special branch as private state.
- `Red Slaver` tests now set explicit runtime fields (`first_turn`,
  `used_entangle`) rather than deriving them from history.
- Audit note updated in
  `docs/audits/MONSTER_RUNTIME_TRUTH_AUDIT_2026-04-18.md`.

Verification for `d0adc3b`:

- `cargo test bandit_bear --all-targets`
- `cargo test bandit_leader --all-targets`
- `cargo test lagavulin --all-targets`
- `cargo test slaver_red --all-targets`
- `cargo test --all-targets` -> `1202 passed`

## Current Audit Position

We are in monster/runtime parity work after broad card parity work.

The current monster architecture is still usable if these rules are followed:

- Java private gameplay fields become explicit Rust runtime fields, protocol
  imports, or factory-seeded state. They are not reconstructed from history.
- Java `lastMove`, `lastTwoMoves`, `lastMoveBefore` map to Rust
  `move_history`.
- Java `takeTurn()` chains that queue `SetMoveAction` become Rust queued
  `SetMonsterMove`, not `roll_move_plan`.
- Java `RollMoveAction` after a turn consumes monster AI RNG and records a move
  when Java does so, even if the next move is deterministic.
- UI/VFX classes are ignored only after checking that they do not mutate combat
  state, RNG, room state, map state, or visible choices.

Current text scans after `1ad40f2`:

- `src/content/monsters` has no remaining direct `move_history().is_empty`
  private-state pattern from the recent search.
- The obvious "private flags from history" smell was cleaned in the audited
  Red Slaver/Lagavulin/Bandit cases.

No uncommitted changes were present after `a8e467e`.

## Recent Source Findings Not Yet Needing Edits

Mixed `SetMoveAction` / `RollMoveAction` audit:

- `SlimeBoss`: Java split path does not queue `RollMoveAction`; Rust split path
  does not roll.
- `AcidSlime_L`: Java split path does not queue `RollMoveAction`; Rust guards
  roll with `if plan.move_id != SPLIT`.
- `SpikeSlime_L`: Java queues `RollMoveAction` after the switch, including the
  split path; Rust always pushes the post-turn roll after `execute_steps`.
- `Looter` / `Mugger`: fixed in `874605d`. Java contains both synchronous
  `setMove(...)` branches and queued `SetMoveAction(...)` branches; Rust now
  preserves the meaningful timing split for lunge/smoke/escape paths.
- Gremlin packet: fixed in `1ac61f2`. Java Gremlin escape paths queue
  `SetMoveAction(ESCAPE)` after `EscapeAction`; Rust now mirrors that for the
  audited Exordium Gremlins. Timing-sensitive synchronous `setMove(...)`
  branches in Gremlin Wizard and Gremlin Tsundere were preserved before queued
  actions.
- `BanditPointy`: checked in `0b0eec3`. No logic change needed; added a test
  for the two-hit damage queue before queued `SetMoveAction`.
- `TorchHead`: checked in `5ad39bc`. No logic change needed; added a test for
  damage before queued `SetMoveAction`; Java fire effect update is VFX-only.
- `ShelledParasite`: checked before `a4d74f4`; no code change needed. Existing
  tests cover first-move runtime state, STUN + roll timing, state import, and
  Plated Armor break.
- `Byrd`: fixed in `a4d74f4`. Headbutt now applies synchronous Java
  `setMove(GO_AIRBORNE)` timing before queued damage.
- `Centurion` + `Healer`: checked in `8d16e69`. No business logic change
  needed; added Healer tests for zero-HP non-dying ally inclusion.
- `SnakePlant`: checked in `1ad40f2`. No business logic change needed; added
  A17 `lastMoveBefore` and triple-hit queue tests.
- `Snecko`: fixed across `632492c` and `aa55e3d`. Glare now emits Confusion
  with Java sentinel amount `-1`, and tests lock Glare, A17 Tail debuff
  ordering, and the `lastTwoMoves(BITE)` Tail rule.
- `SphericGuardian`: fixed in `aa55e3d`. Pre-battle Barricade now uses Java
  sentinel amount `-1`; tests lock Barricade, Artifact, and opening block order.
- `BookOfStabbing`: fixed in `dc4622d`. Pre-battle Painful Stabs now uses Java
  sentinel amount `-1`; tests lock `stabCount` roll-time growth and STAB
  multi-hit execution.
- `Chosen`: checked in `0b984ca`. No business logic change was needed; tests
  lock below-A17 Hex transition, Drain/Debilitate ordering, and Poke two-hit
  execution.
- `Taskmaster`: checked in `f511731`. No business logic change was needed;
  tests lock constant Scouring Whip roll, wound thresholds, A18 Strength
  ordering, and below-A18 no-Strength behavior.
- `GremlinLeader`: fixed in `6e9a4d6`. Pre-battle Minion and spawned Minion
  applications now use Java sentinel `-1`; tests lock Encourage queue order,
  STAB three-hit scheduling, and slot-truth behavior.
- `Reptomancer`: touched in `6e9a4d6` only for shared Minion sentinel parity.
  Its broader move/slot behavior still deserves a later dedicated packet if
  needed.
- `TheCollector` + `TorchHead`: checked in `5232ea9`. No business logic change
  was needed; tests lock initial spawn, Mega Debuff forcing, Fireball
  lastTwoMoves gate, debuff queue order, and existing enemy-slot revive truth.
- `BronzeAutomaton` + `BronzeOrb`: fixed in `8385df0`. `ApplyStasisAction`
  rarity candidate selection now sorts by Java `cardID` before RNG; tests lock
  Automaton runtime counters, Hyper Beam timing, BronzeOrb usedStasis, and
  Support/Beam history gates.
- `Champ`: checked in `a8e467e`. No business logic change was needed; tests
  lock half-HP Anger, Execute gating, fourth-turn Taunt reset, A19 Defensive
  Stance cap/forge counter, Anger cleanup queue order, and Face Slap/Taunt
  debuff order.

Split / victory timing:

- Java split uses `CannotLoseAction`, `SuicideAction`, `SpawnMonsterAction`,
  then `CanLoseAction`.
- Rust drains the action queue and settles victory only after pending actions
  drain, so the checked Slime split paths do not need UI/global CannotLose
  modeling just for premature reward prevention.

Random target audit:

- `src/engine/targeting.rs` has tests for manual target filtering and random
  target behavior.
- Random monster targeting includes zero-HP monsters when they are not dying,
  escaped, or half-dead, matching Java `MonsterGroup.getRandomMonster(true)`.
- `GainBlockRandomMonsterAction` is special: Java excludes source, `intent ==
  ESCAPE`, and `isDying`, but does not exclude `isEscaping`; Rust has dedicated
  tests for this behavior.
- Naming caveat: Rust `is_escaped` currently represents Java
  `isEscaping || escaped`. In normal Java escape flow this is usually safe
  because `escape()` sets `isEscaping = true` before `escaped = true`, but the
  lifecycle mapping should remain on the watch list.

## High-Risk Evergreen List

Keep these on the short list and revisit with narrow source packets:

1. Draw pile API and top/bottom conventions.
2. Generated cards entering draw/discard/hand, including random spot behavior.
3. Random target selection and monster lifecycle flags.
4. Pending choices, selection order, cancel/confirm behavior, and replay.
5. Post-combat cleanup and retained queued actions.
6. Card instance copying, UUID/misc propagation, and battle-instance mutation.
7. Potion discard/use affordances outside combat and during phase boundaries.
8. Map/boss/event/shop/chest/campfire visibility and room transition state.
9. Relic counters, relic hooks, and hidden vs public state.
10. Monster pools, event pools, and act/floor/ascension gates.
11. Java synchronous `setMove(...)` vs queued `SetMoveAction(...)`; do not
    collapse these when queued damage, death, or cleanup can intervene.

## Next Work Queue

Continue monster audit before jumping back to machine learning.

Recommended next packets:

1. Finish the mixed `SetMoveAction` / `RollMoveAction` monster sweep:
   - `AwakenedOne` and `Darkling` were read; no immediate code change was made.
     Keep Java duplicate move-history behavior from immediate `setMove(...)`
     plus later `SetMoveAction(...)` on the watch list.
   - `Looter` and `Mugger` were fixed in `874605d`.
   - Exordium Gremlins were fixed in `1ac61f2`.
   - `BanditPointy` was checked in `0b0eec3`.
   - `TorchHead` was checked in `5ad39bc`.
   - `ShelledParasite` was checked; no code change needed.
   - `Byrd` was fixed in `a4d74f4`.
   - `Centurion` + `Healer` were checked in `8d16e69`.
   - `SnakePlant` was checked in `1ad40f2`.
   - `Snecko` was fixed across `632492c` and `aa55e3d`.
   - `SphericGuardian` was fixed in `aa55e3d`.
   - `BookOfStabbing` was fixed in `dc4622d`.
   - `Chosen` was checked in `0b984ca`.
   - `Taskmaster` was checked in `f511731`.
   - `GremlinLeader` was fixed in `6e9a4d6`.
   - `TheCollector` was checked in `5232ea9`.
   - `BronzeAutomaton` + `BronzeOrb` were fixed in `8385df0`.
   - `Champ` was checked in `a8e467e`.
   - Next narrow packet: `AwakenedOne`
     (`D:\rust\cardcrawl\monsters\beyond\AwakenedOne.java` and
     `src/content/monsters/beyond/awakened_one.rs`). It exercises phase
     transition state, death/revive semantics, Cultist spawning, Curiosity,
     and the previously noted immediate `setMove(...)` plus queued
     `SetMoveAction(...)` move-history duplication risk.
2. For each monster packet, inspect only:
   - Java monster file.
   - Rust monster file.
   - Relevant action files if `takeTurn()` queues custom actions.
   - Existing test file or nearest module tests.
3. If source comparison is resolved, add or adjust a focused test, run the
   narrow tests, then commit.
4. If a source comparison exposes an architectural issue, write the issue here
   first before changing broad modules.

## Compression Control Protocol

Every meaningful chunk must end with:

- Latest commit hash or `uncommitted` status.
- Files changed.
- Tests run and result.
- Exact next source packet.
- Any unresolved suspicion moved into this file.

If context compacts, do not infer from memory. Resume from this file and the
latest five commits.
