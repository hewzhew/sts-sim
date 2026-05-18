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

- `012e056 Match Java post-draw hook queue order`

Recent commits:

- `012e056 Match Java post-draw hook queue order`
- `133883b Update handoff after end-of-round timing audit`
- `ea1570c Match Java end-of-round queue timing`
- `5a568a7 Update handoff after invincible poison audit`
- `10997a8 Fix monster start-turn invincible poison timing`

`012e056` summary:

- Java `GameActionManager`, Java `AbstractCreature.atStartOfTurnPostDraw`,
  Java post-draw powers, Java `VoidCard`, Rust `PostDrawTrigger`, and Rust
  draw-card handling were checked.
- Fixed regular new-turn post-draw hook queue ordering:
  - Java calls `atStartOfTurnPostDraw` hook methods before `DrawCardAction`
    executes.
  - Those hook methods use `addToBot`, so their actions land behind the
    already-queued turn-start `DrawCardAction`, but ahead of actions generated
    while that draw action executes.
  - Rust's synthetic `PostDrawTrigger` now runs before the queued
    `DrawCards`, so it appends hook actions behind `DrawCards` and ahead of
    draw-generated actions.
- Fixed `Void` draw trigger ordering:
  - Java `VoidCard.triggerWhenDrawn()` uses `addToBot(new LoseEnergyAction(1))`.
  - Rust had modeled it as top insertion.
  - Rust now queues the energy loss to the bottom.
- Added a regression test proving `DrawCardNextTurn` post-draw actions remain
  ahead of Void's draw-generated energy loss.

Verification for `012e056`:

- `cargo test turn_start_post_draw_hooks_queue_before_draw_generated_actions_like_java --all-targets`
  -> `1 passed`
- `cargo test post_draw --all-targets` -> `3 passed`
- `cargo test draw_card_next --all-targets` -> `0 matched`
- `cargo test gambling_chip --all-targets` -> `1 passed`
- `cargo test void --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1331 passed`

`ea1570c` summary:

- Java `GameActionManager.getNextAction()`, Java
  `MonsterGroup.applyEndOfTurnPowers()`, Java `DrawReductionPower`, and Rust
  `tick_engine()` combat turn-transition logic were checked.
- Fixed Rust's round-end queue timing:
  - Java calls monster `atEndOfTurn`, player `atEndOfRound`, and monster
    `atEndOfRound` hooks as synchronous hook methods that enqueue actions.
  - Java does not drain those queued actions before it runs the following
    player start-of-turn hook methods and constructs the next-turn
    `DrawCardAction`.
  - Rust was draining monster end-of-turn actions before `atEndOfRound`, and
    draining round cleanup before the player start-of-turn setup.
  - Rust now queues the collective end-of-turn and end-of-round actions and
    leaves them in order ahead of the queued player start-of-turn actions.
- Added a regression test for `DrawReductionPower`:
  - Java queues `ReducePowerAction`, then constructs the next-turn
    `DrawCardAction` from the still-reduced `player.gameHandSize`.
  - Rust now draws 4 cards on the expiration turn, then removes
    `DrawReduction` before player control returns.

Verification for `ea1570c`:

- `cargo test draw_reduction_decay_is_queued_before_next_turn_draw_count_like_java_game_hand_size --all-targets`
  -> `1 passed`
- `cargo test blur_retains_player_block_through_next_turn_while_power_ticks_down --all-targets`
  -> `1 passed`
- `cargo test draw_reduction --all-targets` -> `2 passed`
- `cargo test end_of_round --all-targets` -> `1 passed`
- `cargo test monster_pre_turn_invincible_resets_before_poison_like_java_at_start_of_turn --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1330 passed`

`10997a8` summary:

- Java `MonsterGroup.applyPreTurnLogic()`, Java
  `AbstractCreature.applyStartOfTurnPowers()`, Java `InvinciblePower`, Java
  `PoisonPower`, Java `PoisonLoseHpAction`, Rust power start hooks, and Rust
  poison damage handling were checked.
- Fixed monster start-of-turn `Invincible` timing:
  - Java `InvinciblePower.atStartOfTurn()` immediately resets `amount` to
    `maxAmt` during the monster group's pre-turn power pass.
  - Rust was resetting `Invincible` later, just before the monster's
    `takeTurn()`.
  - Rust now resets `Invincible` in the normal `resolve_power_instance_at_turn_start`
    hook and no longer performs a second pre-`takeTurn` reset.
- Fixed monster `PoisonLoseHp` to use the normal HP_LOSS damage pipeline:
  - Java `PoisonLoseHpAction` calls `target.damage(new DamageInfo(...,
    HP_LOSS))`, so `InvinciblePower.onAttackedToChangeDamage` caps it.
  - Rust was manually subtracting monster HP and bypassing `Invincible`.
  - Rust now routes monster poison HP loss through `apply_damage_to_monster_via_pipeline`
    before decrementing/removing Poison and running post-combat cleanup.
- Added a regression test proving start-of-turn `Invincible` resets before
  Poison HP loss and is not reset again before the monster acts.

Verification for `10997a8`:

- `cargo test monster_pre_turn_invincible_resets_before_poison_like_java_at_start_of_turn --all-targets`
  -> `1 passed`
- `cargo test invincible --all-targets` -> `5 passed`
- `cargo test poison --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1329 passed`

`4fd646b` summary:

- Java `MonsterGroup.areMonstersBasicallyDead()` and Rust
  `settle_victory_if_ready` were checked after the `MonsterGroup` lifecycle
  packet exposed the filter mismatch.
- Fixed Rust victory settlement to use the same Java predicate:
  - Java `areMonstersBasicallyDead()` only treats monsters as absent when they
    are `isDying` or `isEscaping`.
  - Java does not treat `currentHealth <= 0` as basically dead by itself.
  - Rust previously inferred victory from `current_hp <= 0` unless rebirth
    powers were present.
  - Rust now delegates victory readiness to
    `CombatState::are_monsters_basically_dead_java()`.
- Added a regression test proving a zero-HP monster that is not dying/escaping
  does not settle combat victory.

Verification for `4fd646b`:

- `cargo test victory_settlement_uses_java_basically_dead_flags_not_zero_hp --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1328 passed`

`556788e` summary:

- Java `MonsterGroup.applyPreTurnLogic()`, `MonsterGroup.queueMonsters()`,
  `MonsterGroup.applyEndOfTurnPowers()`, Java `GameActionManager` monster
  queue handling, Java `AbstractCreature.applyTurnPowers()`, Java
  `FadingPower`, Java `ExplosivePower`, and Rust `engine/core.rs` were checked.
- Fixed Rust monster `duringTurn()` lifecycle timing:
  - Java calls `m.takeTurn(); m.applyTurnPowers();` for one monster, then
    drains the queued actions before dequeuing the next monster.
  - Only Java `FadingPower` and `ExplosivePower` override `duringTurn()`.
  - Rust was resolving `Fading` / `Explosive` inside the group-level
    end-of-turn power pass, after all monsters had acted.
  - Rust now has a separate `resolve_power_during_turn` hook and queues those
    actions immediately after each monster's `takeTurn` actions, before the
    current monster's queue is drained and before the next monster acts.
- Added tests proving:
  - `Fading` and `Explosive` no longer fire from
    `resolve_power_at_end_of_turn`;
  - their Java action order is preserved through the new `duringTurn` hook;
  - `Explosive` damage can kill the player before the next monster is dequeued,
    matching Java `GameActionManager`.

Verification for `556788e`:

- `cargo test monster_during_turn_powers_fire_before_next_monster_turn_like_java_apply_turn_powers --all-targets`
  -> `1 passed`
- `cargo test explosive_and_fading_countdowns_match_java_during_turn_action_order --all-targets`
  -> `1 passed`
- `cargo test explosive --all-targets` -> `2 passed`
- `cargo test fading --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1327 passed`

`894274a` summary:

- Java `MonsterGroup.usePreBattleAction()`, Java `AbstractMonster` universal
  pre-battle hook, Java Louse pre-battle code, and Rust
  `handle_pre_battle_trigger` were checked.
- Fixed Rust normal combat pre-battle RNG stream:
  - Java `MonsterGroup.usePreBattleAction()` calls each monster's
    `usePreBattleAction()` without changing RNG streams.
  - The only monster pre-battle code found in Java that consumes dungeon RNG is
    Louse Curl Up, and it explicitly uses `AbstractDungeon.monsterHpRng`.
  - Rust was passing `PreBattleLegacyRng::Misc` from the group-level
    `PreBattleTrigger`, causing Louse Curl Up to consume `misc_rng`.
  - Rust now passes `PreBattleLegacyRng::MonsterHp` for group pre-battle,
    matching Java.
- Added a handler-level test proving Louse Curl Up consumes `monster_hp_rng`,
  leaves `misc_rng` untouched, and queues `BattleStartPreDrawTrigger` after the
  monster pre-battle action.
- Java `useUniversalPreBattleAction()` contains Daily/Endless/blight mechanics
  (`Lethality`, blights, `Time Dilation`) and was not implemented in this
  packet because those global modifiers are outside the currently modeled
  normal-run mechanics.

Verification for `894274a`:

- `cargo test monster_group_pre_battle_uses_monster_hp_rng_for_louse_curl_up_like_java --all-targets`
  -> `1 passed`
- `cargo test pre_battle --all-targets` -> `24 passed`
- `cargo test --all-targets` -> `1326 passed`

`3d4805e` summary:

- Java `MonsterHelper`, Java `MonsterGroup`, Java
  `com.megacrit.cardcrawl.random.Random`, and Rust monster factory were
  checked.
- Corrected the previous fixed-HP RNG conclusion from `06e5f9f`:
  - Java `AbstractMonster.setHp(int)` calls `setHp(hp, hp)`.
  - Java `Random.random(start, end)` increments its counter even when
    `start == end`.
  - Rust `spawn_monster` therefore again consumes exactly one monster HP RNG
    roll for every monster constructor, including fixed-HP monsters such as
    Spire Shield, Spire Spear, and Corrupt Heart.
- Fixed Java `MonsterHelper.bottomHumanoid()` / `bottomWildlife()` candidate
  construction parity:
  - Java `bottomGetWeakWildlife()` constructs `getLouse()`, `SpikeSlime_M`,
    and `AcidSlime_M` before selecting one with `miscRng`.
  - Java `bottomGetStrongHumanoid()` constructs `Cultist`, `getSlaver()`, and
    `Looter` before selecting one.
  - Java `bottomGetStrongWildlife()` constructs both `FungiBeast` and
    `JawWorm` before selecting one.
  - Rust now constructs the same temporary candidates at the eventual slot and
    discards the unselected objects, preserving constructor HP RNG and louse
    bite RNG consumption.
- Confirmed by source scan that the remaining MonsterHelper random pools
  (`spawnGremlins`, `spawnManySmallSlimes`, `spawnShapes`, `getAncientShape`,
  `spawnSmallSlimes`) choose keys before constructing objects and do not need
  this discarded-candidate treatment.

Verification for `3d4805e`:

- `cargo test factory --all-targets` -> `5 passed`
- `cargo test final_act_initializes_shield_spear_and_heart_context --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1325 passed`

`06e5f9f` summary:

- Java `TheEnding`, Java `MonsterRoomBoss`, Rust `RunState` final-act setup,
  and Rust monster factory final-act encounter creation were checked.
- This commit's fixed-HP RNG conclusion was superseded by `3d4805e`; do not use
  the `06e5f9f` commit message or its old tests as source truth for
  `setHp(int)`.
- Existing final-act run test still locks Java `TheEnding` map/context:
  rest -> shop -> elite Shield/Spear -> boss Heart -> true victory, encounter
  lists with three Shield/Spear and three Heart entries, boss key visibility,
  transition heal, potion drop reset, and card RNG band alignment.

`1879996` summary:

- `CorruptHeart`, Java `BeatOfDeathPower`, Java `InvinciblePower`, and Java
  `PainfulStabsPower` were checked.
- Fixed Heart buff-turn private runtime timing:
  - Java queues the Strength and follow-up buff actions, then synchronously
    increments private `buffCount` before queued actions can execute and before
    `RollMoveAction`.
  - Rust now emits the `CorruptHeart` runtime update before the queued
    `ApplyPower` actions, matching the same synchronous-state principle used
    for Maw/Exploder/Transient-style fixes.
- Added tests proving:
  - pre-battle `Invincible` and `BeatOfDeath` use Java's A19 gate
    (`Invincible 300 / Beat 1` below A19, `Invincible 200 / Beat 2` at A19+);
  - first `getMove()` selects Debilitate and only clears private `firstMove`,
    without incrementing `moveCount`;
  - Painful Stabs follow-up uses Java sentinel amount `-1`;
  - buff turn cleanses negative Strength by adding `-Strength + 2`, picks the
    Java `buffCount == 1` Beat of Death follow-up, and updates private
    `buffCount` before queued powers execute.
- Confirmed existing tests already cover:
  - `InvinciblePower` storing its Java `maxAmt` reset amount in `extra_data`;
  - `InvinciblePower.onAttackedToChangeDamage` capping both ordinary damage and
    HP_LOSS.

Verification for `1879996`:

- `cargo test corrupt_heart --all-targets` -> `5 passed`
- `cargo test beat_of_death --all-targets` -> `1 passed`
- `cargo test invincible --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1321 passed`

`5fe09ea` summary:

- `SpireShield`, `SpireSpear`, Java `SurroundedPower`, Java
  `BackAttackPower`, and Java `AbstractMonster.calculateDamage()` BackAttack
  placement were checked.
- Fixed Shield pre-battle `Surrounded` action trace parity:
  - Java `new SurroundedPower(player)` has sentinel `amount == -1`.
  - Rust now emits `ApplyPower { power_id: Surrounded, amount: -1 }` before
    the Shield Artifact action.
- Fixed existing `BackAttack` damage behavior in the shared monster damage
  pipeline:
  - Java multiplies monster damage by `1.5` when `applyBackAttack()` /
    `BackAttackPower` is active, after player receive modifiers and before
    final receive powers such as Intangible.
  - Rust now applies the same multiplier when the source monster already has
    `PowerId::BackAttack`.
- Added Shield tests proving:
  - Surrounded sentinel then Artifact A18 pre-battle order;
  - Bash does not consume the Focus/Strength random roll when the player has
    no orbs;
  - Bash consumes `ai_rng.randomBoolean()` during `takeTurn()` when the player
    has an orb and can apply Focus;
  - Fortify loops every monster in the group, including zero-HP non-dying
    monsters.
- Added Spear tests proving:
  - Artifact uses Java's A18 gate;
  - A18 Burn Strike queues two attacks, then two Burns to draw pile top, then
    `RollMonsterMove`;
  - Piercer buffs every monster in the group, including zero-HP non-dying
    monsters;
  - Skewer uses imported/runtime `skewer_count`, not just ascension defaults.
- Important unresolved boundary:
  - Java automatic BackAttack application/removal depends on UI-tied facing
    state (`player.flipHorizontal`, `drawX`, `AbstractMonster.applyBackAttack()`).
  - Rust currently has no player facing/drawX model. This packet fixed the
    damage multiplier when `BackAttack` is already present, but did not fake
    automatic facing-based BackAttack creation. Treat that as a separate
    architecture packet if live protocol or parity work requires it.

Verification for `5fe09ea`:

- `cargo test back_attack --all-targets` -> `3 passed`
- `cargo test spire_shield --all-targets` -> `9 passed`
- `cargo test spire_spear --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1318 passed`

`24e4618` summary:

- Java `SpawnMonsterAction.update()`, Java `PhilosopherStone.onSpawnMonster`,
  and Rust spawn/relic hooks were checked.
- Fixed spawned-monster relic hook timing:
  - Java calls `r.onSpawnMonster(m)` before `m.init()`, `m.applyPowers()`, and
    `addMonster(...)`.
  - `PhilosopherStone.onSpawnMonster` directly calls `monster.addPower(...)`;
    it does not queue an `ApplyPowerAction`.
  - Rust now applies on-spawn relic effects as immediate `AbstractCreature`
    `addPower`-style state mutation before inserting the spawned monster and
    before rolling its first move.
- Fixed the same direct hook semantics for `Darkling` reincarnate:
  - Java queues Heal / ChangeState / ApplyPower(Regrow), then synchronously
    calls relic `onSpawnMonster(this)` during `takeTurn()` construction.
  - Rust now mutates the Darkling immediately instead of appending a queued
    Strength action.
- Added/updated tests proving:
  - Philosopher Stone spawn Strength is present immediately and only Minion
    remains queued to the front for spawned minions;
  - Philosopher Stone battle-start and spawn hooks both apply Strength;
  - existing Darkling reincarnate ordering tests still pass with direct hook
    mutation.

Verification for `24e4618`:

- `cargo test spawn_monster --all-targets` -> `1 passed`
- `cargo test darkling --all-targets` -> `8 passed`
- `cargo test philosopher --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1309 passed`

`bf619c7` summary:

- Dedicated `Reptomancer` / `SnakeDagger` packet was checked against Java
  source.
- Fixed the remaining Minion pre-battle source parity for both `Reptomancer`
  and `GremlinLeader`:
  - Java uses `new ApplyPowerAction(m, m, new MinionPower(this))`.
  - Rust now emits `ApplyPower { source: minion, target: minion,
    power_id: Minion, amount: -1 }` instead of using the summoner as source.
- Added Reptomancer tests proving:
  - initial dagger slots are mapped by Java monster-group index:
    daggers after Reptomancer go to `daggers[0]`, daggers before it go to
    `daggers[1]`;
  - A18 spawn turns fill the first available Java `daggers[]` slots and queue
    both spawns before `RollMonsterMove`;
  - Java `canSpawn()` counts zero-HP or escaped non-dying monsters because it
    skips only `this` and `isDying`;
  - Snake Strike queues two 16-damage hits at A3+, then Weak, then roll.
- Existing SnakeDagger tests still lock Java firstMove runtime truth and
  explode using `LoseHPAction`, not `SuicideAction`.
- Java VFX/animation/WaitAction effects remain presentation-only.

Verification for `bf619c7`:

- `cargo test reptomancer --all-targets` -> `10 passed`
- `cargo test snake_dagger --all-targets` -> `4 passed`
- `cargo test gremlin_leader --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1308 passed`

`5aa6309` summary:

- `Exploder` and Java `ExplosivePower` were checked.
- Fixed `Exploder.takeTurn()` timing:
  - Java synchronously increments private `turnCount` before the queued
    attack body and before `RollMoveAction`.
  - Rust now emits the `Exploder` runtime update first, then the attack body
    when present, then `RollMonsterMove`.
  - The Java UNKNOWN/BLOCK branch still increments `turnCount` before rolling,
    even though the switch body has no queued gameplay action.
- Confirmed pre-battle `ExplosivePower` amount is 3.
- Confirmed existing Rust `ExplosivePower` countdown order already matches
  Java: countdown reduces the power until amount 1, then queues suicide before
  the 30 THORNS player damage.
- Java `AnimateSlowAttackAction`, animation startup randomness, and explosion
  VFX were treated as presentation-only.

Verification for `5aa6309`:

- `cargo test exploder --all-targets` -> `5 passed`
- `cargo test explosive --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1304 passed`

`a8e2118` summary:

- `Repulsor` and Java `MakeTempCardInDrawPileAction` were checked.
- No business logic change was needed.
- Added tests proving:
  - low roll selects Attack only when Java `lastMove(ATTACK)` is false;
  - `num >= 20` selects Daze;
  - A2+ Attack queues one 13-damage attack before `RollMoveAction`;
  - Daze queues `MakeTempCardInDrawPileAction(new Dazed(), 2, true, true)`
    as `MakeTempCardInDrawPile { random_spot: true, to_bottom: false }`,
    then `RollMoveAction`.
- Java `AnimateSlowAttackAction`, animation startup randomness, and card-display
  effects were treated as presentation-only after confirming the gameplay
  mutation is the underlying draw-pile insertion.

Verification for `a8e2118`:

- `cargo test repulsor --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1302 passed`

`945681d` summary:

- `OrbWalker`, Java `GenericStrengthUpPower`, and Java
  `MakeTempCardInDiscardAndDeckAction` were checked.
- No business logic change was needed.
- Added tests proving:
  - pre-battle `GenericStrengthUpPower` uses Java's A17 gate: amount 3 below
    A17, amount 5 at A17+;
  - `getMove(int)` uses Java `lastTwoMoves(CLAW)` / `lastTwoMoves(LASER)`
    gates without recursive rerolling;
  - A2+ Laser queues damage 11, `MakeTempCardInDiscardAndDeckAction(Burn)`,
    then `RollMoveAction`;
  - existing Laser test keeps the shared action as one
    `MakeTempCardInDiscardAndDeck`, not two hand-expanded add-card actions.
- Java `AnimateSlowAttackAction`, `ChangeStateAction`, `WaitAction`, hit
  animation, animation startup randomness, and card-display effects were
  treated as presentation-only after confirming the gameplay mutation is the
  underlying draw/discard pile insertion.

Verification for `945681d`:

- `cargo test orb_walker --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1299 passed`

`87044fb` summary:

- `WrithingMass`, Java `ReactivePower`, Java `MalleablePower`, and Java
  `AddCardToDeckAction` were checked.
- Fixed Writhing Mass pre-battle `Reactive` amount:
  - Java `ReactivePower` does not set amount and therefore inherits
    `AbstractPower.amount == -1`.
  - Rust now applies `Reactive` with amount `-1` and treats `Reactive` as a
    sentinel amount power.
- Fixed `ReactivePower.onAttacked` queue direction:
  - Java `ReactivePower` calls `addToBot(new RollMoveAction(owner))`.
  - Rust now queues Reactive rerolls to the back, preserving existing queued
    actions ahead of the reroll.
- Confirmed existing Rust Writhing Mass runtime state already models Java
  `firstMove` and `usedMegaDebuff`, including first-move clearing and
  Mega-Debuff runtime update before adding Parasite.
- Java `FastShakeAction`, `AnimateFastAttackAction`, `AnimateSlowAttackAction`,
  `ChangeStateAction`, `WaitAction`, hit animation, and animation startup
  randomness were treated as presentation-only.

Verification for `87044fb`:

- `cargo test writhing_mass --all-targets` -> `6 passed`
- `cargo test reactive_power --all-targets` -> `2 passed`
- `cargo test sentinel_power_reapplication_matches_java_apply_power_special_cases --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1296 passed`

`9ce0e12` summary:

- `SpireGrowth` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - A17+ without player Constricted forces Constrict before the RNG Quick
    Tackle branch;
  - below A17, a low roll selects Quick Tackle before the non-constricted
    branch;
  - when player already has Constricted and the last two moves were Smash,
    Java fallback is Quick Tackle;
  - Constrict at A17 queues Constricted 12 before `RollMoveAction`;
  - Smash at A2+ queues one 25-damage attack before `RollMoveAction`.
- Java `AnimateFastAttackAction`, `AnimateSlowAttackAction`,
  `ChangeStateAction`, `WaitAction`, animation startup randomness, and Hurt
  animation were treated as presentation-only.

Verification for `9ce0e12`:

- `cargo test spire_growth --all-targets` -> `5 passed`
- `cargo test --all-targets` -> `1294 passed`

`17d05fd` summary:

- `Spiker` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - pre-battle Thorns follows Java ascension gates: 3 below A2, 4 at A2+, 7
    at A17+;
  - executed private `thornsCount > 5` forces Attack even when the RNG roll
    would otherwise allow Buff;
  - planned-but-unexecuted Buff history does not count as executed
    `thornsCount`;
  - low roll attacks only when the previous move was not Attack;
  - Attack queues one Java A2+ 9-damage attack before `RollMoveAction`;
  - Buff increments private `thornsCount` before queued Thorns
    `ApplyPowerAction`.
- Java animation and startup animation RNG remain presentation-only.

Verification for `17d05fd`:

- `cargo test spiker --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1289 passed`

`bcbd851` summary:

- `Maw` Java/Rust behavior was checked.
- Fixed `ROAR` timing:
  - Java queues Weak and Frail actions, then synchronously sets private
    `roared=true` before those queued debuffs execute, then queues
    `RollMoveAction`.
  - Rust now emits the private `roared` runtime update before queued Weak/Frail
    actions, preserving Java synchronous state mutation timing.
- Added tests proving:
  - imported private `roared=false` forces Roar even if history contains Roar;
  - imported private `turn_count` drives Nom hit count;
  - Java `lastMove(SLAM)` and `lastMove(NOMNOMNOM)` force Drool;
  - high roll after a non-attack move selects Slam and A2+ Slam damage 30;
  - RollMove increments Java private `turnCount`;
  - A17 Roar applies Weak/Frail 5 after the immediate `roared=true` update.
- Java `SFXAction`, `ShoutAction`, `AnimateSlowAttackAction`,
  `VFXAction(BiteEffect)`, animation, and death sound were treated as
  presentation-only. The Bite VFX `MathUtils` rolls are not gameplay RNG.

Verification for `bcbd851`:

- `cargo test maw --all-targets` -> `10 passed`
- `cargo test --all-targets` -> `1285 passed`

`d6a62f4` summary:

- `Transient`, Java `ShiftingPower`, and sentinel-power application were
  checked.
- Fixed `Transient.takeTurn()` timing:
  - Java queues the damage action, then synchronously increments private
    `count` and calls `setMove(...)` before queued damage can execute.
  - Rust now emits private `count` update and `SetMonsterMove` before the
    queued `MonsterAttack`, preserving Java synchronous move mutation timing.
- Fixed `ShiftingPower` amount truth:
  - Java `ShiftingPower` inherits `AbstractPower.amount == -1`.
  - Rust Transient pre-battle now applies Shifting with amount `-1`.
  - `PowerId::Shifting` is now a sentinel amount power so application/reapply
    keeps Java `-1` / stackPower(-1) behavior.
- Added tests proving:
  - Transient pre-battle applies Fading 5 below A17, Fading 6 at A17+, and
    Shifting `-1`;
  - Transient runtime count and next visible attack update happen before queued
    damage;
  - duplicate Shifting application follows Java default stackPower(-1)
    behavior.
- Java animation, `ChangeStateAction`, `WaitAction`, and achievement unlock were
  treated as presentation/meta-only.

Verification for `d6a62f4`:

- `cargo test transient --all-targets` -> `6 passed`
- `cargo test sentinel_power_reapplication_matches_java_apply_power_special_cases --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1283 passed`

`2aae03b` summary:

- `Donu` and `Deca` Java/Rust behavior were checked as a pair.
- No business logic change was needed.
- Added tests proving:
  - pre-battle Artifact uses Java's A19 gate: amount 2 below A19, amount 3
    at A19+;
  - Donu Beam at A4+ queues two 12-damage monster attacks, then updates
    private `isAttacking=false`, then rolls;
  - Donu Circle of Protection queues Strength for every monster in the current
    group, including a zero-HP non-dying ally object, before updating private
    `isAttacking=true` and rolling;
  - Deca Beam at A4+ queues two 12-damage monster attacks, then two Dazed into
    discard, then updates private `isAttacking=false`, then rolls;
  - Deca A19 Square of Protection queues block and Plated Armor interleaved per
    monster in Java loop order before updating private `isAttacking=true` and
    rolling.
- Java `ChangeStateAction`, `WaitAction`, SFX, BGM, unlock, death shake, and
  animation side effects were treated as presentation/meta-only because they do
  not mutate modeled gameplay state or gameplay RNG.

Verification for `2aae03b`:

- `cargo test donu --all-targets` -> `7 passed`
- `cargo test deca --all-targets` -> `15 passed`
- `cargo test --all-targets` -> `1282 passed`

`6c142a3` summary:

- `TimeEater` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - `lastTwoMoves(REVERBERATE)` blocks Reverberate and consumes Java
    `aiRng.random(50, 99)`;
  - `lastMove(HEAD_SLAM)` blocks Head Slam and consumes Java
    `aiRng.randomBoolean(0.66f)`;
  - `lastMove(RIPPLE)` blocks Ripple and consumes Java `aiRng.random(74)`;
  - A19 Ripple queues block, Vulnerable, Weak, Frail, then roll;
  - A19 Head Slam queues damage, Draw Reduction, two Slimed, then roll;
  - A19 Haste queues debuff removal, Shackled removal, execution-time heal,
    block, then roll.
- Existing TimeEater tests already covered execution-time Haste heal amount,
  Haste visible-spec placeholder, private `usedHaste`, and imported
  `usedHaste` not being reconstructed from history.
- Java first-turn `TalkAction`, `ChangeStateAction`, `WaitAction`, VFX/SFX,
  BGM, and unlock calls remain presentation/meta side effects outside the Rust
  combat simulator's modeled mechanics.

Verification for `6c142a3`:

- `cargo test time_eater --all-targets` -> `11 passed`
- `cargo test --all-targets` -> `1280 passed`

`9e6e73f` summary:

- `GiantHead` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - Java `lastTwoMoves(GLARE)` forces `COUNT` and decrements private `count`;
  - Java `lastTwoMoves(COUNT)` forces `GLARE` and decrements private `count`;
  - `IT_IS_TIME` stops decrementing private `count` at Java floor `-6` and
    caps the real damage table at starting damage + 30.
- Existing GiantHead tests already covered A18 pre-battle count decrement,
  SlowPower amount 0, count-driven `IT_IS_TIME`, and imported count not being
  reconstructed from move history.
- Java `ShoutAction`, SFX/death voice, animation, and MathUtils dialogue rolls
  were treated as presentation-only.

Verification for `9e6e73f`:

- `cargo test giant_head --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1274 passed`

`98ee287` summary:

- `Nemesis` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - Tri Attack queues three separate hits before self-Intangible and
    `RollMonsterMove`;
  - A18+ Tri Burn queues 5 Burns before self-Intangible and roll;
  - existing Intangible blocks the post-turn self-application, matching Java
    `hasPower("Intangible")`.
- Existing Nemesis tests already covered private `firstMove`,
  `scytheCooldown` pre-decrement, imported runtime truth, and Scythe cooldown
  reset.
- Java `ChangeStateAction`, `WaitAction`, `SFXAction`, `VFXAction`, fire
  particles, and `MathUtils` voice selection were treated as presentation-only
  because they do not mutate modeled gameplay state or gameplay RNG.

Verification for `98ee287`:

- `cargo test nemesis --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1271 passed`

`fcf0f0b` summary:

- `Reptomancer`, `SnakeDagger`, Java `SuicideAction`, Java
  `SpawnMonsterAction`, Java `LoseHPAction`, Java `FadingPower`, Java
  `ExplosivePower`, `TheCollector`, and `BronzeAutomaton` death cleanup were
  checked as one narrow packet.
- `Action::Suicide` now carries Java's `triggerRelics` flag.
- `handle_suicide(..., trigger_relics=true)` now sets HP to 0 and enters the
  central monster-death handler so power/relic death hooks run, matching
  Java `new SuicideAction(monster)`.
- Split slimes now emit `Suicide { trigger_relics: false }`, matching Java
  `new SuicideAction(this, false)`.
- Fading/Explosive and minion cleanup paths now emit
  `Suicide { trigger_relics: true }`.
- `Reptomancer`, `TheCollector`, and `BronzeAutomaton` death cleanup now emits
  minion suicides in Java `addToTop` mechanical order: while Java iterates the
  monster group forward, later minions' `SuicideAction` executes first.
- Added tests for:
  - default SuicideAction triggering The Specimen/Poison death hooks;
  - split-slime SuicideAction(false) skipping relic death hooks;
  - Reptomancer/Collector/Bronze cleanup reverse execution order;
  - updated split slime, Fading, Explosive, and SnakeDagger expectations.

Verification for `fcf0f0b`:

- `cargo test reptomancer --all-targets` -> `6 passed`
- `cargo test collector --all-targets` -> `11 passed`
- `cargo test bronze_automaton --all-targets` -> `7 passed`
- `cargo test snake_dagger --all-targets` -> `4 passed`
- `cargo test suicide --all-targets` -> `9 passed`
- `cargo test slime --all-targets` -> `10 passed`
- `cargo test fading --all-targets` -> `1 passed`
- `cargo test explosive --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1268 passed`

`c7a3546` summary:

- `Darkling`, Java `Darkling.damage()`, Java `RegrowPower`, Java
  `ApplyPowerAction`, Java `HealAction`, and the Rust death pipeline were
  checked.
- Fixed `REINCARNATE` action parity:
  - visible/spec power amount remains `Regrow 1`;
  - queued `Action::ApplyPower` now uses Java `ApplyPowerAction(..., 1)`;
  - the power handler still stores sentinel `Regrow.amount == -1`, matching the
    Java `RegrowPower` instance.
- Fixed `REINCARNATE` queue order to Java:
  `HealAction`, `ChangeStateAction("REVIVE")`, `ApplyPowerAction(Regrow, 1)`,
  relic `onSpawnMonster`, then `RollMoveAction`.
- Fixed first half-death timing:
  - Darkling is marked half-dead and not dying before power/relic death hooks;
  - powers remain visible to relic `onMonsterDeath` hooks, then are cleared;
  - `setMove(COUNT)` records an immediate move-history entry only when
    `nextMove != COUNT`;
  - queued `SetMoveAction(COUNT)` records the second Java move-history entry.
- Added tests for reincarnate queue order, duplicate COUNT move-history, the
  `nextMove != COUNT` guard, and The Specimen seeing Poison before Darkling
  powers are cleared.

Verification for `c7a3546`:

- `cargo test darkling --all-targets` -> `8 passed`
- `cargo test awakened_one --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1263 passed`

`30c73bb` summary:

- `AwakenedOne`, Java `AwakenedOne.damage()`, Java `UnawakenedPower`, and the
  Rust death pipeline were checked.
- Fixed pre-battle `Unawakened` amount to Java sentinel `-1`.
- Moved first-phase rebirth truth out of the Rust `Unawakened` power hook and
  into the central monster-death interrupt, matching Java ownership:
  `UnawakenedPower` has no `onDeath`; `AwakenedOne.damage()` mutates monster
  state immediately.
- First-phase lethal damage now immediately:
  - marks the monster half-dead and not dying;
  - removes debuffs, `Curiosity`, `Unawakened`, and `Shackled`;
  - sets runtime `form1=false`, `first_turn=true`;
  - sets planned move `REBIRTH` and writes one immediate move-history entry;
  - queues `ClearCardQueue` to the front and a later `SetMonsterMove(REBIRTH)`
    to the bottom, preserving Java's duplicate move-history behavior.
- Removed the now-dead `AwakenedRebirthClear` action variant/handler.
- Added tests for pre-battle power order/amounts, first-phase rebirth immediate
  state and queued `SetMoveAction`, and existing final-death Cultist escape.

Verification for `30c73bb`:

- `cargo test awakened_one --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1259 passed`

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

No uncommitted code changes were present after `012e056` before this handoff
update.

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
- `GremlinLeader`: fixed in `6e9a4d6` and corrected in `bf619c7`.
  Pre-battle Minion and spawned Minion applications now use Java sentinel
  `-1`; the pre-battle source is now the minion itself, matching Java
  `ApplyPowerAction(m, m, new MinionPower(this))`. Tests lock Encourage queue
  order, STAB three-hit scheduling, and slot-truth behavior.
- `Reptomancer`: shared Minion sentinel parity was touched in `6e9a4d6`, shared
  death/suicide interactions were fixed in `fcf0f0b`, and dedicated move/slot
  behavior was checked in `bf619c7`. The pre-battle Minion source now matches
  Java, dagger slot initialization is locked, A18 double-spawn order is locked,
  Java `canSpawn()` non-dying counting is locked, and Snake Strike
  damage/damage/Weak/roll order is locked.
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
- `AwakenedOne`: fixed in `30c73bb`. First-phase death now follows Java
  `AwakenedOne.damage()` ownership instead of pretending `UnawakenedPower`
  owns the transition; tests lock sentinel amount, immediate half-dead/runtime
  mutation, power clearing, top-queued card queue clear, and duplicate
  `REBIRTH` move-history from immediate `setMove` plus queued `SetMoveAction`.
- `Darkling`: fixed in `c7a3546`. Half-death now follows Java
  `Darkling.damage()` ordering: half-dead before power/relic death hooks,
  powers clear after relic hooks, COUNT immediate `setMove` only when
  `nextMove != COUNT`, queued `SetMoveAction(COUNT)` duplicate history, and
  `REINCARNATE` queues heal, revive, Regrow stackAmount `1`, spawn relic hooks,
  then roll.
- `Reptomancer` + `SnakeDagger`: fixed in `fcf0f0b` as part of the shared Java
  `SuicideAction` packet. `SuicideAction(true)` now reaches monster
  death hooks; split slimes use `false`; Fading/Explosive and minion cleanup
  use `true`; Reptomancer/Collector/Bronze cleanup follows Java `addToTop`
  reverse mechanical order.
- `Nemesis`: checked in `98ee287`. No business logic change was needed; tests
  lock Tri Attack, Tri Burn, post-turn Intangible application/skip, and
  existing private `firstMove` / `scytheCooldown` behavior.
- `GiantHead`: checked in `9e6e73f`. No business logic change was needed; tests
  lock A18 pre-battle count decrement, SlowPower amount 0, lastTwoMove gates,
  private count floor, and `IT_IS_TIME` damage.
- `TimeEater`: checked in `6c142a3`. No business logic change was needed; tests
  lock Haste private state, recursive reroll RNG consumption, A19 move queues,
  and execution-time Haste healing.
- `Donu` + `Deca`: checked in `2aae03b`. No business logic change was needed;
  tests lock Artifact amount gates, private `isAttacking`, Beam damage/add-card
  ordering, and all-monster buff/protect loop ordering.
- `Transient`: fixed in `d6a62f4`. Runtime count / next-move mutation now
  happens before queued damage, and Shifting uses Java sentinel amount `-1`.
- `Maw`: fixed in `bcbd851`. Roar private `roared` update now happens before
  queued Weak/Frail actions, and tests lock turn-count and move-history gates.
- `Spiker`: checked in `17d05fd`. No business logic change was needed; tests
  lock pre-battle Thorns gates, private `thornsCount`, low-roll/lastMove gates,
  attack damage, and Buff ordering.
- `SpireGrowth`: checked in `9ce0e12`. No business logic change was needed;
  tests lock player Constricted context, A17 branch priority, low-roll Quick
  Tackle, Smash fallback, and Constricted/Smash execution queues.
- `WrithingMass`: fixed in `87044fb`. `Reactive` now uses Java sentinel amount
  `-1`, duplicate Reactive follows default `stackPower(-1)`, and
  `ReactivePower.onAttacked` queues `RollMonsterMove` to the back like Java
  `addToBot`. Existing runtime truth tests lock `firstMove`,
  `usedMegaDebuff`, recursive reroll gating, and Mega-Debuff Parasite ordering.
- `OrbWalker`: checked in `945681d`. No business logic change was needed; tests
  lock GenericStrengthUp A17 gate, lastTwoMoves gates, Laser damage/Burn/roll
  order, and use of the shared `MakeTempCardInDiscardAndDeck` action.
- `Repulsor`: checked in `a8e2118`. No business logic change was needed; tests
  lock low-roll/lastMove Attack gating, A2+ attack damage, and Dazed random
  draw-pile insertion action before roll.
- `Exploder`: fixed in `5aa6309`. Java `takeTurn()` synchronously increments
  private `turnCount` before queued damage or the empty UNKNOWN/BLOCK body and
  before `RollMoveAction`; Rust now emits the runtime update first in both
  attack and block turns. Tests also lock pre-battle Explosive amount 3 and the
  existing Explosive countdown suicide/damage ordering.
- `SpawnMonsterAction` / `PhilosopherStone`: fixed in `24e4618`. Java
  `SpawnMonsterAction.update()` calls relic `onSpawnMonster(m)` before
  `m.init()`, `m.applyPowers()`, and `addMonster(...)`, and
  `PhilosopherStone.onSpawnMonster` directly mutates `monster.addPower(...)`
  instead of queuing `ApplyPowerAction`. Rust now applies the spawn relic hook
  as immediate state mutation before insertion and before the spawned monster's
  first roll. `Darkling` reincarnate now uses the same direct hook semantics.
- `SpireShield` + `SpireSpear`: fixed/locked in `5fe09ea`. Shield
  pre-battle `Surrounded` now uses Java sentinel amount `-1`; existing
  `BackAttack` power now multiplies monster damage in the shared damage
  pipeline; tests lock Shield Bash Focus/Strength RNG timing, Fortify/Piercer
  all-monster loops without HP filtering, Spear Burn Strike queue order, Spear
  Artifact A18 gate, and runtime Skewer hit count.
- `CorruptHeart`: fixed/locked in `1879996`. Pre-battle Invincible /
  Beat of Death A19 gates are covered; first roll clears only `firstMove`;
  buff turns now emit the private `buffCount` runtime update before queued
  Strength/follow-up `ApplyPower` actions; tests lock negative-Strength cleanse
  and the `buffCount == 1` Beat of Death follow-up. Existing Invincible tests
  already cover Java `maxAmt` reset storage and ordinary/HP_LOSS damage caps.
- Monster factory constructor RNG: corrected in `3d4805e`. Java
  `setHp(int)` still calls `Random.random(hp, hp)`, so fixed-HP constructors
  consume one monster HP RNG roll. Java `bottomHumanoid()` / `bottomWildlife()`
  also construct unselected candidate monsters before selecting one; Rust now
  preserves those discarded candidate HP RNG and louse bite RNG consumptions.
- MonsterGroup pre-battle RNG stream: fixed in `894274a`. Rust group-level
  pre-battle now passes `MonsterHp` so Louse Curl Up consumes the same
  `AbstractDungeon.monsterHpRng` stream as Java. Java universal pre-battle
  Daily/Endless/blight hooks remain intentionally unmodeled.
- Monster `duringTurn()` lifecycle: fixed in `556788e`. Java
  `GameActionManager` calls `m.applyTurnPowers()` immediately after each
  monster `takeTurn()`, before the next monster is dequeued. Only Java
  `FadingPower` and `ExplosivePower` override `duringTurn()`, so Rust now
  handles those in a dedicated per-monster hook instead of the group-level
  `applyEndOfTurnPowers()` pass.
- Victory settlement basically-dead predicate: fixed in `4fd646b`. Rust
  `settle_victory_if_ready` now uses Java `MonsterGroup.areMonstersBasicallyDead()`
  semantics (`isDying || isEscaping`) instead of inferring victory from
  `current_hp <= 0`.
- Monster pre-turn `Invincible` / Poison timing: fixed in `10997a8`. Rust now
  resets `Invincible` in the Java start-of-turn power pass and routes monster
  `PoisonLoseHp` through the HP_LOSS damage pipeline so `Invincible` caps it
  before Poison decrements.
- Monster group end-of-turn / end-of-round queue timing: fixed in `ea1570c`.
  Java queues collective end-of-turn and round-end actions, then runs the
  following player start-of-turn hook methods and constructs `DrawCardAction`
  before the queued cleanup actions execute. Rust now preserves that queue
  order; `DrawReduction` expiration is the locked regression case.
- Regular new-turn post-draw hook action order: fixed in `012e056`. Rust's
  synthetic `PostDrawTrigger` now runs before the queued `DrawCards` so hook
  actions append behind the turn-start draw but ahead of draw-generated actions.
  `VoidCard.triggerWhenDrawn()` now uses bottom insertion like Java
  `addToBot(new LoseEnergyAction(1))`.

Source suspicion remaining after `5fe09ea`:

- Java automatic BackAttack application/removal is tied to `Surrounded`,
  `player.flipHorizontal`, `drawX`, and `AbstractMonster.applyBackAttack()`.
  Rust does not currently model player facing/drawX. Do not fake this inside
  Shield/Spear content; if needed, design a dedicated facing/BackAttack state
  packet and keep the multiplier behavior separate from automatic power
  creation.

Source suspicion resolved in `24e4618`:

- Java `SpawnMonsterAction.update()` calls relic `onSpawnMonster(m)` before the
  monster is inserted into `AbstractDungeon.getMonsters().monsters`. Rust now
  runs the modeled on-spawn relic hook before insertion. The only currently
  modeled on-spawn relic is Philosopher's Stone, and its hook is direct
  `addPower`-style mutation.

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
12. UI-tied but gameplay-relevant facing state, especially Act 4
    `Surrounded` / `BackAttack` creation and removal.

## Next Work Queue

Continue monster audit before jumping back to machine learning.

Recommended next packets:

1. Finish the mixed `SetMoveAction` / `RollMoveAction` monster sweep:
   - `AwakenedOne` was fixed in `30c73bb`.
   - `Darkling` was fixed in `c7a3546`.
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
   - `AwakenedOne` was fixed in `30c73bb`.
   - `Darkling` was fixed in `c7a3546`.
   - `Reptomancer` + `SnakeDagger` shared death/suicide interactions were fixed
     in `fcf0f0b`.
   - `Nemesis` was checked in `98ee287`.
   - `GiantHead` was checked in `9e6e73f`.
   - `TimeEater` was checked in `6c142a3`.
   - `Donu` + `Deca` were checked in `2aae03b`.
   - `Transient` was fixed in `d6a62f4`.
   - `Maw` was fixed in `bcbd851`.
   - `Spiker` was checked in `17d05fd`.
   - `SpireGrowth` was checked in `9ce0e12`.
   - `WrithingMass` was fixed in `87044fb`.
   - `OrbWalker` was checked in `945681d`.
   - `Repulsor` was checked in `a8e2118`.
   - `Exploder` was fixed in `5aa6309`.
   - Dedicated `Reptomancer` move/slot behavior was fixed/locked in
     `bf619c7`.
   - Java `SpawnMonsterAction.update()` hook ordering was fixed in `24e4618`.
   - Act 4 `SpireShield` + `SpireSpear` coordinated runtime/move audit was
     fixed/locked in `5fe09ea`.
   - `CorruptHeart` runtime/power audit was fixed/locked in `1879996`.
   - Final-act encounter/factory initialization audit initially reached the
     wrong fixed-HP RNG conclusion in `06e5f9f`; this was corrected in
     `3d4805e`.
   - Java `MonsterHelper` encounter composition and factory constructor RNG
     audit was fixed/locked in `3d4805e`, including discarded candidate
     construction for Exordium Thugs / Wildlife.
   - Java `MonsterGroup.usePreBattleAction()` RNG stream was fixed/locked in
     `894274a`.
   - Java `GameActionManager` per-monster `applyTurnPowers()` timing was
     fixed/locked in `556788e`.
   - Java `MonsterGroup.areMonstersBasicallyDead()` victory readiness was
     fixed/locked in `4fd646b`.
   - Java monster start-of-turn `Invincible` and `PoisonLoseHpAction`
     interaction was fixed/locked in `10997a8`.
   - Java collective end-of-turn / atEndOfRound action queue timing was
     fixed/locked in `ea1570c`, with `DrawReductionPower` as the regression
     case.
   - Regular new-turn `atStartOfTurnPostDraw` hook action order and
     `VoidCard.triggerWhenDrawn()` insertion were fixed/locked in `012e056`.
   - Next narrow packet: audit initial combat start hook ordering around
     `AbstractRoom.update()`:
     `GainEnergyAndEnableControlsAction`, `applyStartOfCombatPreDrawLogic`,
     initial `DrawCardAction`, `applyStartOfCombatLogic`,
     `applyStartOfTurnRelics`, `applyStartOfTurnPostDrawRelics`,
     `applyStartOfTurnCards`, `applyStartOfTurnPowers`,
     `applyStartOfTurnOrbs`, and Rust `PreBattleTrigger` /
     `BattleStartPreDrawTrigger` / `BattleStartTrigger`.
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
