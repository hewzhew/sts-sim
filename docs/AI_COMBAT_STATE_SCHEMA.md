# AI Combat State Schema

This document defines the combat-state schema used by `CombatKernel`.

It is not a demo schema, not a small-combat schema, and not a numbered
compatibility tier. The authoritative source for coverage is the decompiled game
source at:

```text
D:\rust\cardcrawl
```

The first executable probe may use a simple combat, but that probe must be an
instance of this complete schema. If implementation discovers missing combat
state, the schema is incomplete and must be repaired before the kernel is treated
as deterministic.

## Java Semantics, Rust Simulator

`D:\rust\cardcrawl` is the semantic source of truth. It is used to recover what
the original game tracks, when hooks fire, which RNG stream is consumed, how card
and monster effects compose, and which intermediate states can request player
input.

It is not a mandate to copy Java's object graph. The Rust simulator is the
runtime used by AI, search, replay, and training. Rust may replace Java globals,
inheritance, UI-coupled fields, mutable singletons, and render objects with
typed deterministic data structures. That redesign is allowed only when it
preserves the original mechanic.

The migration rule is:

```text
Preserve mechanic semantics.
Redesign implementation structure where Java's structure is bad for Rust.
Document every semantic divergence.
Do not drop a game feature because it is inconvenient.
```

This is a feature-preservation rule, not a Java-porting rule. The Java side is
read to recover the original design and all mechanic obligations. The Rust side
is written as a clean simulator for AI. A Java feature may be represented
differently in Rust only if the resulting simulator has the same public combat
semantics, hidden state transitions, hook order, and RNG consumption for the
covered case.

Selective porting is forbidden. A mechanic may be absent only when all of these
are true:

```text
1. The relevant Java source location is named.
2. The missing behavior is described precisely.
3. The omission is classified as unsupported_abort, not as implemented.
4. Any frame reaching it is non-trainable.
5. The limitation is listed in the release blockers.
```

Difficulty is not an impossibility proof. "Hard to model", "old Rust cannot
represent it", "Java design is ugly", or "not needed for the first probe" are
not valid reasons to drop a feature.

If a Java feature is impossible or inappropriate to migrate literally, the Rust
replacement must state:

```text
source class / method / field
original Java behavior
why literal migration is invalid
Rust representation
semantic equivalence test
known limitation, if any
```

`why literal migration is invalid` must be about the implementation structure,
not the mechanic. Examples that can justify redesign are Java UI coupling,
mutable global singletons, inheritance shape, pointer identity, render-only
state, or LibGDX runtime dependencies. They do not justify changing gameplay
semantics.

The existing Rust simulator does not receive compatibility protection. Any
module, file, function, type, fixture, or test helper may be changed or deleted
if it conflicts with this schema, with replay determinism, or with Java-derived
combat semantics.

## Coverage Rule

Every combat-relevant field, queue, counter, RNG, and screen state found in
`D:\rust\cardcrawl` must be classified in the schema inventory as one of:

```text
modeled:
  stored directly in CombatStateSnapshot

derived:
  recomputed deterministically from modeled state

render_only:
  UI/animation data that no combat mechanic reads

run_level_materialized:
  originates outside combat but must be converted into combat state before
  CombatKernel::start

non_combat:
  belongs only to map, shop, event, reward, rest-site, menu, or save systems

unsupported_abort:
  known source state not yet modeled; entering it is KernelAbort, not training
  data
```

Silent globals are forbidden. Untyped `run_state` is forbidden. A field needed
for replay, legal action generation, damage/block calculation, choice-screen
progress, combat hooks, or RNG consumption cannot be ignored. Existing Rust code
that cannot represent such a field must change.

## Source Inventory

The working source ledger lives in
`docs/AI_COMBAT_SOURCE_COVERAGE_LEDGER.md`.

The minimum source inventory is:

| Source file | Required schema coverage |
| --- | --- |
| `dungeons/AbstractDungeon.java` | combat-global player pointer, current room/node facts needed by combat, combat RNG streams, action manager, screen-open state, combat-relevant static counters and flags |
| `characters/AbstractPlayer.java` | player identity, HP/block/gold, master deck, draw/hand/discard/exhaust/limbo zones, relics, blights, potion belt, energy, orbs, stance, card-in-use, class-specific combat state |
| `core/AbstractCreature.java` | common creature HP/block/powers/lifecycle/last-damage/combat flags used by player and monsters |
| `cards/AbstractCard.java` | concrete card instance identity, cost, current turn cost, damage/block/magic/heal/draw/discard values, upgrade/misc/tags, target/type/color/rarity, one-shot flags, generated/autoplay state |
| `cards/CardGroup.java` | zone ordering, group type, queued/in-hand bookkeeping when mechanically relevant |
| `cards/CardQueueItem.java` | queued card, target, X-cost energy, ignore-energy flag, autoplay flag, random-target flag, end-turn autoplay flag |
| `actions/GameActionManager.java` | action queues, pre-turn actions, card queue, monster queue, cards played this turn/combat, orb/stance history, phase/control flags, damage/discard/energy/turn counters |
| `actions/AbstractGameAction.java` and subclasses | queued action class, action type, duration/done state, source/target refs, amount, and subclass payload needed to resume action execution |
| `rooms/AbstractRoom.java` and monster room subclasses | room phase, monster group, combat-over flags, cannot-lose flag, elite/combat-event/smoke/mug flags, reward timing flags, card reward rarity counters that are touched by combat end |
| `monsters/AbstractMonster.java` | monster identity, HP/block/powers/lifecycle, move bytes, move history, intent, damage list, escape/death flags, monster-specific payload |
| `monsters/MonsterGroup.java` | ordered monster list, spawned monster insertion, random monster selection semantics, hovered monster excluded as render-only unless a mechanic reads it |
| `monsters/EnemyMoveInfo.java` | next move, intent, base damage, multiplier, multi-damage flag |
| `powers/AbstractPower.java` and concrete powers | owner ref, power id, amount, priority, type, turn-based/post-action/negative flags, concrete power payloads |
| `relics/AbstractRelic.java` and concrete relics | relic id, counter, used-up/grayscale flags, energy-based flag, combat counters/charges, concrete relic payloads |
| `potions/AbstractPotion.java` and concrete potions | potion id, slot, potency, can-use/target-required/thrown/discarded flags, concrete potion payloads |
| `orbs/AbstractOrb.java` and concrete orbs | orb id, slot order, evoke/passive/base amounts, focus-applied values, concrete orb payloads |
| `stances/AbstractStance.java` and concrete stances | stance id and concrete stance counters/timers only when mechanically read |
| `screens/select/GridCardSelectScreen.java` | target group, selected cards, selection amount, confirm/cancel state, upgrade/transform/purge/any-number/clarity flags |
| `screens/select/HandCardSelectScreen.java` | selected cards, number to select, can-pick-zero/up-to/any-number/transform/upgrade flags, selection reason, retrieval flag |
| `random/Random.java` | `RandomXS128` state words and counter for every combat-consuming RNG stream |

This table is not decorative. Before an implementation claims schema coverage,
each concrete combat-relevant subclass under these directories must either map
to a typed payload or be listed as `unsupported_abort`.

## Rust Inventory

The working Rust migration ledger lives in
`docs/AI_COMBAT_RUST_MIGRATION_LEDGER.md`.

Rust implementation work must also inventory the current simulator before reuse.
At minimum, classify existing files under:

```text
src/runtime
src/engine
src/content
src/state
src/protocol
src/projection
src/semantics
src/testing
src/ai
```

Each reused Rust type must be classified as:

```text
kept:
  already matches the Java-derived semantic schema

rewritten:
  useful concept, wrong shape or incomplete state

deleted:
  seed-driven, fixture-driven, policy-polluted, or incompatible with deterministic
  replay

adapter_only:
  acceptable only as a temporary importer/exporter into CombatStateSnapshot
```

No Rust type is accepted because it already exists. It is accepted only if it can
be traced to Java-derived mechanics and produces deterministic replay under this
schema.

When existing Rust disagrees with Java-derived semantics, the default action is
rewrite. Keeping it requires a parity note proving that the disagreement is only
structural, not mechanical.

## Top-Level Snapshot

The initial Rust type skeleton lives in
`src/ai/combat_state_snapshot/mod.rs`. It is an audit schema boundary, not a
legacy engine wrapper.

```text
CombatStateSnapshot {
  source_manifest: SourceManifest,
  snapshot_origin: CombatSnapshotOrigin,
  dungeon_context: DungeonCombatContext,
  room_state: RoomCombatState,
  content_pools: CombatContentPoolState,
  global_temp: GlobalCombatTempState,
  action_manager: ActionManagerState,
  player: PlayerCombatState,
  monster_group: MonsterGroupState,
  card_store: CardInstanceStore,
  card_zones: CardZoneState,
  powers: PowerState,
  relics: RelicState,
  blights: BlightState,
  potions: PotionBeltState,
  orbs: OrbState,
  stance: StanceState,
  choice_screens: ChoiceScreenState,
  rng: CombatRngState,
  lifecycle: CombatLifecycleState,
  public_refs: PublicRefState,
  derived_values: DerivedCombatValues,
  source_coverage: SourceCoverageLedger,
}
```

`CombatSnapshotOrigin` may be authored, replay-extracted, or bridge-extracted.
That is provenance only. All origins use the same `CombatStateSnapshot` schema.

## Source Manifest

```text
SourceManifest {
  cardcrawl_root,
  game_version,
  decompile_manifest_hash,
  source_files,
  simulator_commit,
  schema_hash,
  content_manifest_hash,
}
```

`decompile_manifest_hash` is a canonical hash of the source files used for the
inventory. If `D:\rust\cardcrawl` changes, the manifest must change.

The manifest also carries per-file evidence:

```text
SourceFileManifestEntry {
  source_path,
  sha256,
  byte_len,
  line_count,
}
```

`source_path` is relative to `cardcrawl_root` for decompiled Java files. The
canonical manifest hash is computed from the ordered list of
`SourceFileManifestEntry` values. A schema or Rust implementation change that
depends on a Java source file must be traceable to one or more entries in this
list.

## Dungeon Combat Context

```text
DungeonCombatContext {
  dungeon_name,
  level_num,
  player_class,
  floor_num,
  act_num,
  ascension_level,
  is_ascension_mode,
  curr_map_node_ref,
  dungeon_id,
  boss_key,
  screen_state,
  combat_relevant_global_flags,
}
```

Combat code in `AbstractDungeon` uses many static fields. The schema must not
pretend they are absent. Run-level fields stay outside the kernel only if their
combat effects have already been materialized into player, room, relic, potion,
card, monster, or RNG state.

## Room Combat State

```text
RoomCombatState {
  room_kind,
  phase,
  map_symbol,
  monster_group_ref,
  is_battle_over,
  cannot_lose,
  elite_trigger,
  blizzard_potion_mod,
  mugged,
  smoked,
  combat_event,
  reward_allowed,
  reward_time,
  skip_monster_turn,
  base_rare_card_chance,
  base_uncommon_card_chance,
  rare_card_chance,
  uncommon_card_chance,
  combat_end_timer_state,
  reward_pop_out_timer_bits,
  wait_timer_bits,
}
```

`CombatKernel` stops at `CombatTerminalReport`. It must define whether combat-end
hooks have been applied and must not consume post-combat reward-generation RNG.

## Combat Content Pools

These fields materialize combat-usable pools from `AbstractDungeon`. They are not
permission to run reward or map generation inside `CombatKernel`.

```text
CombatContentPoolState {
  src_colorless_card_pool,
  src_curse_card_pool,
  src_common_card_pool,
  src_uncommon_card_pool,
  src_rare_card_pool,
  colorless_card_pool,
  curse_card_pool,
  common_card_pool,
  uncommon_card_pool,
  rare_card_pool,
  common_relic_pool,
  uncommon_relic_pool,
  rare_relic_pool,
  shop_relic_pool,
  boss_relic_pool,
  monster_list,
  elite_monster_list,
  boss_list,
}
```

Card pools are modeled because combat card generation and transform effects can
read them. Relic and monster pools are run-level materialized unless a combat
path proves it consumes them.

## Global Combat Temp State

These are Java globals that are mechanically relevant or must be explicitly
guarded at the combat boundary.

```text
GlobalCombatTempState {
  transformed_card_ref,
  loading_post_combat,
  is_victory,
  turn_phase_effect_active,
  colorless_rare_chance_bits,
  card_blizz_start_offset,
  card_blizz_randomizer,
  card_blizz_growth,
  card_blizz_max_offset,
  boss_count,
  relics_to_remove_on_start,
}
```

Java `float` fields stored in replay-critical state must be represented as raw
bits, not as ordinary float comparison keys.

## Action Manager State

```text
ActionManagerState {
  action_static_state,
  phase,
  has_control,
  turn_has_ended,
  using_card,
  monster_attacks_queued,
  current_action,
  previous_action,
  turn_start_current_action,
  next_combat_actions,
  actions,
  pre_turn_actions,
  card_queue,
  monster_queue,
  cards_played_this_turn,
  cards_played_this_combat,
  orbs_channeled_this_turn,
  orbs_channeled_this_combat,
  unique_stances_this_combat,
  mantra_gained,
  last_card_ref,
  total_discarded_this_turn,
  damage_received_this_turn,
  damage_received_this_combat,
  hp_loss_this_combat,
  player_hp_last_turn,
  energy_gained_this_combat,
  turn_index,
}
```

Action classes with Java `static` mechanical state are not allowed to hide that
state inside individual queued actions:

```text
ActionStaticState {
  draw_card_action_drawn_cards,
  discard_action_num_discarded,
  exhaust_action_num_exhausted,
  nightmare_action_num_discarded,
  put_on_deck_action_num_placed,
  put_on_bottom_of_deck_action_num_placed,
}
```

Queued actions are typed state, not opaque callbacks:

```text
ActionState {
  action_class,
  action_type,
  attack_effect,
  damage_type,
  duration_bits,
  start_duration_bits,
  is_done,
  source_ref,
  target_ref,
  amount,
  damage_info,
  card_ref,
  power_ref,
  relic_ref,
  potion_ref,
  action_payload,
  unsupported_subclass_payload,
}
```

`ActionState` models the shared `AbstractGameAction` fields. Java action
`duration` and `startDuration` are replay-critical `float` fields and must be
stored as raw bits. `action_type`, `attack_effect`, and `damage_type` must keep
the Java enum surface; collapsing missing variants into `Special` is forbidden.

Concrete action subclasses must migrate to typed payloads before they are
trainable or searchable. Initial source-backed payloads:

```text
ActionPayload {
  ApplyPoisonOnRandomMonster {
    starting_duration_bits,
    power_to_apply,
  }

  ApplyPower {
    power_to_apply,
    starting_duration_bits,
  }

  ApplyPowerToRandomEnemy {
    power_to_apply,
    is_fast,
    effect,
  }

  AttackDamageRandomEnemy {
    card_ref,
    effect,
  }

  BetterDiscardPileToHand {
    number_of_cards,
    optional,
    new_cost,
    set_cost,
  }

  BetterDrawPileToHand {
    number_of_cards,
    optional,
  }

  ChooseOneColorless {
    retrieve_card,
  }

  ConditionalDraw {
    restricted_type,
  }

  Damage {
    gold_amount,
    skip_wait,
    mute_sfx,
  }

  DamageAllEnemies {
    damage,
    base_damage,
    first_frame,
    utilize_base_damage,
  }

  DamageRandomEnemy {}

  DiscardToHand {
    card_ref,
  }

  DrawCard {
    shuffle_check,
    clear_draw_history,
    follow_up_action,
  }

  DrawPileToHand {
    type_to_check,
  }

  Discard {
    is_random,
    end_turn,
  }

  DiscardSpecificCard {
    target_card,
    group_zone_ref,
  }

  EmptyDeckShuffle {
    shuffled,
    vfx_done,
    count,
  }

  Exhaust {
    is_random,
    any_number,
    can_pick_zero,
  }

  ExhaustToHand {
    card_ref,
  }

  ExhaustSpecificCard {
    target_card,
    group_zone_ref,
    starting_duration_bits,
  }

  GainEnergy {
    energy_gain,
  }

  MakeTempCardInDiscard {
    card_to_make,
    num_cards,
    same_uuid,
  }

  MakeTempCardInDiscardAndDeck {
    card_to_make,
  }

  MakeTempCardInDrawPile {
    card_to_make,
    random_spot,
    auto_position,
    to_bottom,
    x_bits,
    y_bits,
  }

  MakeTempCardInHand {
    card_to_make,
    is_other_card_in_center,
    same_uuid,
  }

  ModifyBlock {
    target_uuid,
  }

  NewQueueCard {
    card_ref,
    random_target,
    immediate_card,
    autoplay_card,
  }

  PlayTopCard {
    exhaust_cards,
  }

  PutOnBottomOfDeck {
    is_random,
  }

  PutOnDeck {
    is_random,
  }

  PummelDamage {}

  QueueCard {
    card_ref,
  }

  ReApplyPowers {
    card_ref,
    monster_ref,
  }

  ReduceCost {
    target_uuid,
    card_ref,
  }

  ReduceCostForTurn {
    target_card,
  }

  ReducePower {
    power_id,
    power_ref,
  }

  RemoveSpecificPower {
    power_id,
    power_ref,
  }

  ResetFlags {
    card_ref,
  }

  ReviveMonster {
    healing_effect,
  }

  RollMove {
    monster_ref,
  }

  Scry {
    starting_duration_bits,
  }

  SetMove {
    monster_ref,
    next_move,
    next_intent,
    next_damage,
    next_name,
    multiplier,
    is_multiplier,
  }

  SetDontTrigger {
    card_ref,
    trigger,
  }

  ShakeScreen {
    start_duration_bits,
    shake_duration,
    intensity,
  }

  ShowCard {
    card_ref,
  }

  ShowCardAndPoof {
    card_ref,
  }

  Sfx {
    key,
    pitch_var_bits,
    adjust,
  }

  SpawnMonster {
    used,
    monster_ref,
    minion,
    target_slot,
    use_smart_positioning,
  }

  Suicide {
    monster_ref,
    relic_trigger,
  }

  TextAboveCreature {
    used,
    message,
  }

  TextCentered {
    used,
    message,
  }

  TransformCardInHand {
    replacement_card,
    hand_index,
  }

  Unlimbo {
    card_ref,
    exhaust,
  }

  UpdateCardDescription {
    target_card,
  }

  UseCard {
    target_card,
    card_target,
    exhaust_card,
    return_to_hand,
    rebound_card,
  }
}
```

Rust must keep a code-level source link for every typed payload:

```text
ActionPayload::java_source_class()
TYPED_ACTION_PAYLOAD_SOURCE_CLASSES
NO_EXTRA_ACTION_PAYLOAD_SOURCE_CLASSES
```

Adding an `ActionPayload` variant without a Java source class mapping is a
schema defect. A Java action class with no subclass fields beyond
`AbstractGameAction` must be listed in `NO_EXTRA_ACTION_PAYLOAD_SOURCE_CLASSES`
and in the source ledger, so absence of a payload is auditable rather than
accidental.

Any action subclass that cannot be serialized and restored must populate
`unsupported_subclass_payload` and make the frame `unsupported_abort` before it
is used for training or search. This field is not a generic escape hatch for
mechanics; it is a quarantine record for migration:

```text
UnsupportedActionPayload {
  source_class,
  source_fields,
  abort_reason,
}
```

`CardQueueItemState` must include:

```text
card_ref
monster_ref
energy_on_use
ignore_energy_total
autoplay_card
random_target
is_end_turn_auto_play
```

`MonsterQueueItemState` must include the queued monster ref. The queue item
exists as its own source object in Java and must not be silently flattened in
the schema.

```text
MonsterQueueItemState {
  monster_ref,
}
```

`DamageInfoState` preserves the Java `DamageInfo` object used by actions,
monster damage tables, and damage matrix calculation:

```text
DamageInfoState {
  owner,
  name,
  damage_type,
  output,
  base,
  is_modified,
}
```

`base` and `output` are both required. `is_modified` records whether power,
stance, blight, or final damage hooks changed the value from the base damage.

## Player Combat State

```text
PlayerCombatState {
  creature: CreatureState,
  player_class,
  starting_max_hp,
  master_deck_zone_ref,
  draw_pile_zone_ref,
  hand_zone_ref,
  discard_pile_zone_ref,
  exhaust_pile_zone_ref,
  limbo_zone_ref,
  relic_refs,
  blight_refs,
  potion_slot_refs,
  energy,
  is_ending_turn,
  end_turn_queued,
  master_hand_size,
  game_hand_size,
  master_max_orbs,
  max_orbs,
  orb_refs_in_order,
  stance_ref,
  card_in_use_ref,
  damaged_this_combat,
  deprecated_cards_played_this_turn_counter,
  custom_mods,
  class_specific_payload,
}
```

`EnergyState` covers both `EnergyManager` and `EnergyPanel` mechanical state:

```text
EnergyState {
  turn_energy,
  energy_master,
  panel_total_count,
}
```

`turn_energy` is `EnergyManager.energy`; `energy_master` is
`EnergyManager.energyMaster`; `panel_total_count` is `EnergyPanel.totalCount`.
They are distinct in Java: start-of-turn recharge and Ice Cream / Conserve
semantics read and write them differently.

`CreatureState` covers fields inherited through `AbstractCreature`:

```text
CreatureState {
  creature_ref,
  creature_id,
  name_id,
  is_player,
  hp,
  max_hp,
  block,
  gold,
  display_gold,
  powers,
  lifecycle,
  half_dead,
  is_bloodied,
  last_damage_taken,
  escape_state,
  escape_timer_bits,
  mechanically_relevant_flags,
}
```

Animation, hitbox, position, and color fields are render-only unless a source
method reads them for mechanics. If a mechanic reads one later, it moves from
`render_only` to `modeled`.

## Cards and Zones

All concrete card instances live in `CardInstanceStore`; zones only contain
refs.

```text
CardInstance {
  card_ref,
  source_uuid,
  card_id,
  name_id,
  original_name_id,
  color,
  type,
  rarity,
  target,
  tags,
  keywords,
  price,
  upgraded,
  times_upgraded,
  upgraded_cost,
  upgraded_damage,
  upgraded_block,
  upgraded_magic_number,
  misc,
  cost,
  cost_for_turn,
  charge_cost,
  is_cost_modified,
  is_cost_modified_for_turn,
  free_to_play_once,
  energy_on_use,
  ignore_energy_on_use,
  is_used,
  is_seen,
  is_locked,
  is_selected,
  show_evoke_value,
  show_evoke_orb_count,
  damage_type,
  damage_type_for_turn,
  base_damage,
  damage,
  is_damage_modified,
  base_block,
  block,
  is_block_modified,
  base_magic_number,
  magic_number,
  is_magic_number_modified,
  base_heal,
  heal,
  base_draw,
  draw,
  base_discard,
  discard,
  multi_damage,
  is_multi_damage,
  exhaust,
  ethereal,
  retain,
  self_retain,
  innate,
  return_to_hand,
  shuffle_back_into_draw_pile,
  exhaust_on_use_once,
  exhaust_on_fire,
  dont_trigger_on_use_card,
  purge_on_use,
  is_in_autoplay,
  in_bottle_flame,
  in_bottle_lightning,
  in_bottle_tornado,
  cant_use_message,
  generated_by,
  card_specific_payload,
}
```

```text
CardZoneState {
  master_deck,
  draw_pile,
  hand,
  discard_pile,
  exhaust_pile,
  limbo,
  card_in_play,
  temporary_generated_cards,
}

CardZone {
  zone_ref,
  zone_kind,
  ordered_card_refs,
  group_type,
  hand_positioning_map,
  queued_card_refs,
  in_hand_refs,
  public_visibility_mode,
}
```

Duplicate cards are represented by distinct `card_ref` values. Public
observations may hide draw order; the private combat state must still model the
real order for deterministic replay.

## Monster Group

```text
MonsterGroupState {
  group_ref,
  monsters_in_slot_order,
  hovered_monster_ref_if_mechanical,
}

MonsterState {
  creature: CreatureState,
  monster_ref,
  monster_id,
  enemy_type,
  slot,
  death_timer_bits,
  tint_fade_out_called,
  move_set,
  max_hp_roll_state,
  damage_entries,
  move_state,
  intent_state,
  move_history,
  escape_next,
  escaped,
  cannot_escape,
  half_dead,
  monster_specific_payload,
}

MonsterMoveState {
  next_move,
  move_byte,
  move_name_id,
  move_history,
  enemy_move_info,
}

IntentState {
  intent_kind,
  tip_intent_kind,
  base_damage,
  displayed_damage,
  damage_per_hit,
  hit_count,
  is_multi_damage,
  block_amount,
  debuffs,
  status_cards,
  summon_or_escape_flags,
  target_scope,
}
```

Intent is a structure, not a single damage number. Multi-hit, block, buff,
debuff, summon, sleep, escape, and unknown/hidden states must be represented
without overloading `visible_intent_damage`.

Duplicate monsters use distinct `monster_ref` values that are stable for the
entire combat and never reused.

## Powers

```text
PowerState {
  power_instances,
  owner_to_power_order,
}

PowerInstance {
  power_ref,
  power_id,
  name_id,
  description_id,
  owner_ref,
  amount,
  priority,
  power_type,
  is_turn_based,
  is_post_action_power,
  can_go_negative,
  concrete_payload,
}
```

Concrete power payloads are mandatory when the source class has additional
mechanical fields or behavior depending on stored state.

## Relics and Blights

```text
RelicState {
  relic_instances,
  relic_order,
}

RelicInstance {
  relic_ref,
  relic_id,
  name_id,
  description_id,
  cost,
  counter,
  tier,
  used_up,
  grayscale,
  energy_based,
  is_seen,
  is_done,
  is_animating,
  is_obtained,
  discarded,
  concrete_payload,
}

BlightState {
  blight_instances,
  blight_order,
}
```

Relic counters and concrete payloads are combat state whenever relic hooks can
trigger in combat or at combat end.

## Potions

```text
PotionBeltState {
  slots,
}

PotionSlotState {
  slot_index,
  potion_ref,
}

PotionInstance {
  potion_ref,
  potion_id,
  name_id,
  description_id,
  slot,
  potency,
  effect,
  color,
  rarity,
  size,
  can_use,
  target_required,
  is_obtained,
  discarded,
  is_thrown,
  concrete_payload,
}
```

Potion descriptors must distinguish targeted and untargeted use. Potion slots
are part of player public state; concrete potion internals are private unless
visible in game.

## Orbs and Stance

```text
OrbState {
  max_orbs,
  orb_refs_in_order,
  orb_instances,
}

OrbInstance {
  orb_ref,
  orb_id,
  name_id,
  description_id,
  slot,
  evoke_amount,
  passive_amount,
  base_evoke_amount,
  base_passive_amount,
  show_evoke_value,
  channel_anim_timer_bits,
  concrete_payload,
}

StanceState {
  stance_ref,
  stance_id,
  name_id,
  description_id,
  particle_timer_bits,
  particle_timer2_bits,
  concrete_payload,
}
```

Watcher and Defect state must be present even if the first probe uses Ironclad.
Empty is valid; undefined is not.

## Choice Screens

Choice screens are combat state when combat is waiting for input inside a card,
relic, potion, power, or action effect.

```text
ChoiceScreenState {
  active_screen,
  grid_select,
  hand_select,
  generated_choice,
  ordered_choice,
}
```

`GridSelectState` must cover the source fields from
`GridCardSelectScreen.java`:

```text
target_group_zone_ref
selected_card_refs
hovered_card_ref
num_cards
card_select_amount
can_cancel
for_upgrade
for_transform
for_purge
confirm_screen_up
is_just_for_confirming
any_number
for_clarity
cancel_was_on
cancel_text
tip_msg
last_tip
prev_deck_size
upgrade_preview_card_ref
```

`HandSelectState` must cover the source fields from
`HandCardSelectScreen.java`:

```text
num_cards_to_select
selected_card_refs
hovered_card_ref
upgrade_preview_card_ref
selection_reason
were_cards_retrieved
can_pick_zero
up_to
any_number
for_transform
for_upgrade
num_selected
message
hand_zone_ref
wait_then_close_if_mechanical
wait_to_close_timer_bits
```

Multi-step selection is a state machine. Fork, restore, cancel, confirm, and
replay must preserve partial selection progress.

## RNG State

Every combat-consuming RNG stream must store both `RandomXS128` state words and
the source `Random.counter`.

```text
RngStreamState {
  stream_id,
  xs128_state_0,
  xs128_state_1,
  counter,
}

CombatRngState {
  monster_rng,
  monster_hp_rng,
  ai_rng,
  shuffle_rng,
  card_random_rng,
  card_rng,
  misc_rng,
  potion_rng,
  relic_rng_if_combat_consumed,
  treasure_rng_if_combat_consumed,
  custom_streams,
}
```

If a combat path consumes an unmodeled RNG stream, replay has failed and the
schema must be repaired. Do not absorb that failure with a heuristic.

## Public Refs

```text
PublicRefState {
  next_card_ref,
  next_monster_ref,
  next_power_ref,
  next_relic_ref,
  next_potion_ref,
  tombstones,
  visibility_ledger,
}
```

Refs are engine-independent identities for trace and public observation:

```text
card_ref:
  unique for a concrete card instance while publicly trackable; never reused
  within a combat trace

monster_ref:
  stable for the whole combat; never reused after death, escape, or spawn slot
  replacement

potion_ref:
  stable while a potion occupies a slot

power/relic/orb/stance refs:
  stable while the instance exists
```

Refs must not reveal hidden draw order.

## Derived Combat Values

```text
DerivedCombatValues {
  rendered_card_values,
  legal_playability_cache,
  visible_intents,
  public_zone_summaries,
}
```

These values may be cached only if they are recomputable from modeled state and
their cache invalidation is deterministic. If a cached value affects legal
actions or training observations, it must be hashed.

## Lifecycle

```text
CombatLifecycleState {
  combat_started,
  pre_battle_actions_applied,
  monster_pre_battle_actions_applied,
  player_start_combat_hooks_applied,
  turn_start_hooks_applied_for_turn,
  combat_end_hooks_applied,
  terminal_reached,
  reward_generation_started,
  reward_screen_reached,
}
```

For `CombatKernel`, `reward_generation_started` and `reward_screen_reached` must
remain false. If the engine tries to advance there, the kernel must stop at
`CombatTerminalReport` before reward RNG is consumed.

## Source Coverage Ledger

```text
SourceCoverageLedger {
  source_file,
  source_class,
  source_field_or_state,
  classification,
  schema_path,
  public_visibility,
  replay_required,
  rust_owner_module,
  rust_status,
  migration_decision,
  notes,
}
```

This ledger is the guard against architectural hand-waving. A field is not
"handled" until it has an entry and a Rust owner or an explicit
`unsupported_abort` decision.

## Migration Ledger

Java-to-Rust migration decisions are tracked separately from field coverage:

```text
MigrationLedgerEntry {
  java_source,
  java_methods,
  java_fields,
  java_semantic_behavior,
  rust_module,
  rust_type,
  migration_kind,
  preserved_features,
  intentional_structural_changes,
  semantic_equivalence_tests,
  unsupported_cases,
}
```

`migration_kind` is one of:

```text
direct_model:
  Rust stores the same semantic state directly.

derived_model:
  Rust computes the value from other modeled state.

structural_redesign:
  Rust intentionally replaces Java's structure while preserving behavior.

unsupported_abort:
  known combat path not yet implemented; not trainable.
```

`structural_redesign` is not a shortcut. It requires a test that demonstrates the
same mechanic outcome as the Java source for the covered cases.

`unsupported_abort` is not a release-quality implementation. It is a named hole
that prevents trainable rollout through that path. It may be used to stage work,
but it cannot be counted as preserving a feature.

## First Executable Probe

The first probe is allowed to use a simple authored combat because execution must
start somewhere. It does not define the schema.

The probe must:

```text
1. Construct one CombatStateSnapshot using this full schema.
2. Initialize the real Rust combat engine from that snapshot.
3. Execute a fixed legal public action script.
4. Record full state hashes, public decision hashes, action-set hashes, RNG
   hashes, and terminal report after each accepted action and boundary.
5. Repeat the run five times.
6. Produce identical ordered hash sequences.
7. Record every missing source field as a schema defect.
8. Record every reused Rust module in the migration ledger.
```

No `DecisionFrame`, Python adapter, trainer, search, or planner is allowed to
become the reason this probe passes.

## Forbidden Shortcuts

- Do not introduce a reduced schema name for the first probe.
- Do not define combat state as `deck + monsters + seed`.
- Do not pass raw Java/Rust engine objects to task code.
- Do not leave action queues opaque.
- Do not leave choice screens as an unspecified pending state.
- Do not hide run-level requirements inside an untyped `run_state`.
- Do not consume reward-generation RNG inside `CombatKernel`.
- Do not treat render-only classification as permanent if a mechanic reads the
  field later.
- Do not preserve existing Rust APIs for compatibility when they conflict with
  Java-derived mechanics or deterministic replay.
