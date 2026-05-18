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

- `a4d74f4 Fix byrd headbutt setmove timing`

Recent commits:

- `a4d74f4 Fix byrd headbutt setmove timing`
- `5967a3c Update handoff after torch head audit`
- `5ad39bc Add torch head queue parity test`
- `5260b59 Update handoff after bandit pointy audit`
- `0b0eec3 Add bandit pointy queue parity test`

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

Current text scans after `a4d74f4`:

- `src/content/monsters` has no remaining direct `move_history().is_empty`
  private-state pattern from the recent search.
- The obvious "private flags from history" smell was cleaned in the audited
  Red Slaver/Lagavulin/Bandit cases.

No uncommitted changes were present after `a4d74f4`.

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
   - Next narrow packet: `Centurion` + `Healer`
     (`D:\rust\cardcrawl\monsters\city\Centurion.java`,
     `D:\rust\cardcrawl\monsters\city\Healer.java`,
     `src/content/monsters/city/centurion.rs`, and
     `src/content/monsters/city/healer.rs`). Audit them as a pair because their
     behavior depends on ally state and random/support targeting.
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
