# AI Combat State V0 Schema

This schema is the grounding point for the first maintainable combat kernel
implementation. It exists to prevent `CombatOrigin`, `RunSnapshot`, and replay
hashing from becoming abstract placeholders.

V0 scope is one complete authored combat:

```text
Ironclad starter-style combat vs Jaw Worm
fixed RNG streams
fixed action script
repeatable full combat outcome
```

This is not a full run-state schema. `RunCombatSnapshot` remains unsupported
until a separate run-state snapshot spec exists.

## AuthoredCombatStateV0

```text
AuthoredCombatStateV0 {
  player_setup: PlayerSetupV0,
  combat_zones: CombatZonesV0,
  relic_setup: RelicSetupV0,
  potion_belt: PotionBeltV0,
  monster_group: MonsterGroupV0,
  combat_room_context: CombatRoomContextV0,
  rng_streams: CombatRngStreamsV0,
  combat_entry_flags: CombatEntryFlagsV0,
}
```

Every field must be deterministic under canonical serialization. Adding a field
because the Jaw Worm replay diverged is allowed; leaving an implicit engine
global is not.

## Player Setup

```text
PlayerSetupV0 {
  player_class,
  hp,
  max_hp,
  starting_block,
  starting_energy,
  powers,
  stance,
  orbs,
  class_specific_state,
}
```

`stance`, `orbs`, and `class_specific_state` exist in V0 even if the first
combat uses Ironclad. They may be empty but not undefined.

## Combat Zones

```text
CombatZonesV0 {
  master_deck_instances,
  draw_pile_order,
  hand,
  discard_pile,
  exhaust_pile,
  limbo,
  card_in_play,
}
```

Card instances must include:

```text
CardInstanceV0 {
  instance_id,
  card_id,
  upgraded,
  misc,
  cost_base,
  cost_for_turn,
  flags,
}
```

Duplicate cards must have distinct `instance_id` values. Public refs are derived
from visible instances without leaking hidden draw order.

## Relics and Potions

```text
RelicSetupV0 {
  relics: Vec<RelicInstanceV0>,
}

RelicInstanceV0 {
  relic_id,
  counters,
  charges,
  disabled_flags,
  misc,
}

PotionBeltV0 {
  slots: Vec<PotionSlotV0>,
}

PotionSlotV0 {
  slot_index,
  potion_id,
  usable,
  target_requirements,
}
```

V0 may start with no potion, but the structure must support potion slots because
targeted and untargeted potion descriptors are part of the acceptance gate.

## Monster Group

```text
MonsterGroupV0 {
  monsters: Vec<MonsterInstanceV0>,
  group_id,
}

MonsterInstanceV0 {
  instance_id,
  monster_id,
  slot,
  hp,
  max_hp,
  block,
  powers,
  lifecycle,
  move_state,
  intent_state,
}
```

Duplicate monsters must have distinct `instance_id` values. Public monster refs
are stable for the entire combat and independent of slot reuse.

## Combat Context

```text
CombatRoomContextV0 {
  act,
  floor,
  ascension,
  room_type,
  encounter_id,
  combat_phase,
  turn_index,
  combat_step_index,
}

CombatEntryFlagsV0 {
  combat_end_hooks_applied,
  reward_generation_started,
  reward_screen_reached,
}
```

For `CombatKernel`, `reward_generation_started` and `reward_screen_reached` must
remain false. If the underlying engine reaches a reward screen internally after a
win, the kernel maps it to `CombatTerminalReport` and stops before consuming
reward-generation RNG.

## RNG Streams

```text
CombatRngStreamsV0 {
  card_shuffle_rng,
  monster_ai_rng,
  potion_rng,
  misc_combat_rng,
}
```

The first implementation may collapse these to the actual engine RNG structure,
but the schema must name which combat decisions consume which stream. If an RNG
consumer is discovered during replay, it must be added here before the kernel is
considered deterministic.

## Required V0 Probe

The V0 probe must:

```text
1. Construct one complete AuthoredCombatStateV0 for Ironclad vs Jaw Worm.
2. Execute a fixed legal action script through the real combat engine.
3. Record state hashes after each accepted action and monster turn boundary.
4. Repeat the run five times.
5. Produce identical ordered hash sequences.
6. Record any missing state field as a schema defect, not a strategy defect.
```

No `DecisionFrame`, trainer, search, or Python adapter is required before this
probe passes.
