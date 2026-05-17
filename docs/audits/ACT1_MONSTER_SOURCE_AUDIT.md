# Act 1 Monster Source Audit

Date: 2026-05-16

This audit is Java-source-driven. The Java source root is `D:/rust/cardcrawl/monsters/exordium`,
and the Rust implementation root is `src/content/monsters/exordium`.

The goal is not to preserve render, animation, sound, or screen-shake code. Those are omitted
unless they mutate combat state or consume gameplay RNG. `MathUtils` UI randomness stays out of
the Rust simulator. `AbstractDungeon.aiRng`, combat state mutations, queued action order, monster
private fields, and public powers must be preserved.

## Current Pass Scope

Covered in this pass:

- Cultist
- Jaw Worm
- Fungi Beast
- Louses
- Acid Slime / Spike Slime family
- Act 1 gremlin family
- Looter / Mugger
- Blue Slaver / Red Slaver
- Gremlin Nob
- Sentry
- Lagavulin
- Hexaghost
- The Guardian
- Slime Boss

## Source Findings

### RollMoveAction After Death Or Split

Java `RollMoveAction.update()` calls `monster.rollMove()` without checking `isDying` or
`isEscaping`. This matters when a roll was already queued before thorns, suicide, or split marks
the monster as dying.

Rust now preserves that behavior in `handle_roll_monster_move`: queued rolls still consume
`aiRng.random(99)`, update the planned move, and append to move history.

### Acid Slime Probability

Java Acid Slime uses `AbstractDungeon.aiRng.randomBoolean(0.4f / 0.5f / 0.6f)`.

Rust previously used an integer `random_range(0, 99) < percent` helper. That is not the same RNG
operation and can shift later monster moves. Rust now uses `random_boolean_chance(percent / 100.0)`.

### Gremlin Tsundere Ally Count

Java `GremlinTsundere.takeTurn()` counts monsters where `!m.isDying && !m.isEscaping`. It does not
check `currentHealth`.

Rust now matches that behavior so a zero-HP monster awaiting the queued death path still affects
the immediate Protect follow-up branch.

### SplitPower Amount

Java `SplitPower` constructs with `amount = -1`. Acid Slime L, Spike Slime L, and Slime Boss add
that power directly in their constructors.

Rust applies those powers during pre-battle setup, but their public amount now matches Java's
sentinel `-1`.

### Hexaghost BurnIncreaseAction

Java `BurnIncreaseAction` upgrades Burn cards in discard pile and draw pile only. It does not
iterate hand or exhaust pile.

Rust `UpgradeAllBurns` now matches that zone set. This matters after Hexaghost Inferno because
hand Burns should not be upgraded by that action.

### Sentry

Java first move uses the monster's index in the group:

- index 0 / 2: Bolt
- index 1: Beam

Rust uses `MonsterEntity.slot` for the same parity. The Three Sentries factory assigns slots in
group order, so this path is currently consistent. Sentry post-turn roll and Artifact pre-battle
behavior also match the Java source.

### Lagavulin

Lagavulin remains a runtime-state monster, not a move-history-only monster. Required Java private
state is represented explicitly:

- `idle_count`
- `debuff_turn_count`
- `is_out`
- `is_out_triggered`

Existing Rust tests cover Fiend Fire wake-up once and non-asleep event Lagavulin's initial roll /
pre-battle override ordering.

### The Guardian

The Java `ChangeState` names are not UI-only here: `Defensive Mode`, `Offensive Mode`, and
`Reset Threshold` mutate Mode Shift, block, threshold, Sharp Hide, and the next move. Rust models
the mechanical results as runtime updates and queued actions, not as UI state.

Existing Rust coverage includes the Twin Slam ordering where defensive block remains until after
the two attacks and reflected damage, matching Java's `ChangeState(Offensive Mode)` queue order.

### Red Slaver

Java has private `firstTurn` and `usedEntangle` fields, but both are recoverable from the Java
move-history model for simulator-owned combats: `setMove()` appends to `moveHistory` when a move
is planned, `firstTurn` is equivalent to empty history, and `usedEntangle` is equivalent to an
Entangle move already appearing in history. Rust keeps that representation and now has direct
coverage for the first move, one-time Entangle gate, post-Entangle Stab preference, A17 Scrape
repeat rule, and take-turn action order.

## Still Worth Rechecking

- Slime split behavior under multi-hit player attacks: Java sets split intent immediately and also
  queues `SetMoveAction`; Rust queues `SetMonsterMove` from the Split hook. This should converge at
  the stable decision boundary, but multi-hit interleaving is worth a dedicated regression.
- Guardian threshold behavior under multi-hit attacks should stay covered by the dedicated Guardian
  threshold matrix.

## Tests Added Or Relied On

- `roll_monster_move_still_executes_for_dying_monster_like_java_action`
- `percent_roll_uses_java_random_boolean_chance_not_integer_roll`
- `protect_followup_counts_zero_hp_not_yet_dying_monsters_like_java`
- `split_power_prebattle_uses_java_sentinel_amount`
- `burn_increase_upgrades_only_draw_and_discard_like_java`
- `red_slaver_roll_logic_matches_java_private_flags_from_move_history`
- `red_slaver_a17_scrape_cannot_repeat_immediately_like_java`
- `red_slaver_take_turn_actions_preserve_java_order_and_amounts`
- Existing Guardian and Lagavulin source-parity tests
