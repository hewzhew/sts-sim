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
Threshold damage now also follows the Java split between immediate `damage()` mutation and queued
`ChangeState(Defensive Mode)`: Mode Shift amount and `closeUpTriggered` update before the next
queued action, but removing Mode Shift and gaining the defensive block are queued only when the
Defensive Mode action itself resolves.

### Red Slaver

Java has private `firstTurn` and `usedEntangle` fields. They are not safe to reconstruct from the
short live protocol move-history window: Java `setMove()` appends to `moveHistory` while planning,
but the protocol only exports current / last / second-last move ids. Rust now stores both fields in
`SlaverRedRuntimeState`, `CommunicationMod` exports them as `monster.runtime_state.first_turn` and
`monster.runtime_state.used_entangle`, and state sync treats them as strict protocol truth.

Remaining move-history use is limited to Java's explicit `lastMove` / `lastTwoMoves` repeat rules
for Stab and Scrape. Entangle gating and first-turn behavior come from runtime truth only.

### Gremlin Nob

Java has a private `usedBellow` field that is flipped inside `getMove()` before the first Bellow
move is planned. This is hidden runtime truth, not a repeat rule. Rust now stores it in
`GremlinNobRuntimeState`, `CommunicationMod` exports it as `monster.runtime_state.used_bellow`, and
state sync treats it as strict protocol truth.

Remaining move-history use is limited to Java's explicit `lastMove` / `lastMoveBefore` /
`lastTwoMoves` rules for Bull Rush and Skull Bash. The one-time Bellow gate comes from runtime
truth only.

### Looter

Java Looter escape sets `AbstractDungeon.getCurrRoom().mugged = true` before queuing
`EscapeAction`, regardless of whether the monster actually stole any gold. Rust now keys the
combat mugged flag off the escaping monster type for Looter/Mugger rather than the stolen-gold
amount. Gremlin Thief escape remains separate and does not mark the room mugged in Java.

### Slime Split Interrupt

Java large slimes and Slime Boss run their split interrupt from `damage()` after `super.damage`.
The interrupt is guarded by `!isDying`, half-HP, and `nextMove != 3`; when it fires, Java calls
`setMove(Split)` immediately and also queues a `SetMoveAction` to the bottom. Rust now mirrors the
mechanical part: killing hits do not queue Split, threshold hits update the planned split intent
immediately, and the queued `SetMonsterMove` stays behind existing multi-hit damage rather than
jumping to the front.

## Still Worth Rechecking

- Guardian threshold behavior now has direct queue-state coverage, but the broader multi-hit /
  target-power matrix should still be kept in mind while auditing unusual damage actions.

## Tests Added Or Relied On

- `roll_monster_move_still_executes_for_dying_monster_like_java_action`
- `percent_roll_uses_java_random_boolean_chance_not_integer_roll`
- `protect_followup_counts_zero_hp_not_yet_dying_monsters_like_java`
- `split_power_prebattle_uses_java_sentinel_amount`
- `mode_shift_threshold_keeps_power_until_defensive_change_state_resolves`
- `burn_increase_upgrades_only_draw_and_discard_like_java`
- `red_slaver_roll_logic_matches_java_private_flags_from_move_history`
- `red_slaver_used_entangle_is_private_runtime_not_truncated_history`
- `red_slaver_first_turn_is_private_runtime_not_empty_history`
- `red_slaver_a17_scrape_cannot_repeat_immediately_like_java`
- `red_slaver_take_turn_actions_preserve_java_order_and_amounts`
- `gremlin_nob_first_roll_uses_private_bellow_latch_and_marks_it`
- `gremlin_nob_used_bellow_is_private_runtime_not_empty_history`
- `gremlin_nob_bellow_latch_is_not_inferred_from_nonempty_history`
- `gremlin_nob_a18_keeps_java_skull_bash_sequence_after_bellow`
- `looter_escape_marks_room_mugged_even_without_stolen_gold_like_java`
- `gremlin_thief_escape_does_not_mark_room_mugged_like_java`
- `killing_large_slime_does_not_queue_split_like_java_damage_override`
- `large_slime_split_sets_intent_immediately_but_keeps_existing_multi_hit_queue`
- Existing Guardian and Lagavulin source-parity tests
